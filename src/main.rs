use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, anyhow};
use clap::{Args as ClapArgs, Parser, Subcommand};
use matrix_bridge_dingtalk::bridge::DingTalkBridge;
use matrix_bridge_dingtalk::config::Config;
use matrix_bridge_dingtalk::database::Database;
use matrix_bridge_dingtalk::web::{health_endpoint, metrics_endpoint, ProvisioningApi};
use reqwest::Client;
use salvo::prelude::*;
use serde_json::{Value, json};
use tracing::{Level, error, info};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser, Debug)]
#[command(name = "matrix-bridge-dingtalk")]
#[command(version)]
#[command(about = "A Matrix-DingTalk puppeting bridge")]
struct CliArgs {
    /// Path to config file
    #[arg(short, long, default_value = "config.yaml")]
    config: PathBuf,

    /// Generate example config and exit
    #[arg(long)]
    generate_config: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Show provisioning/admin runtime status
    Status(StatusCommand),
    /// List current Matrix <-> DingTalk mappings
    Mappings(MappingsCommand),
    /// Replay dead-letters
    Replay(ReplayCommand),
    /// Cleanup dead-letters
    DeadLetterCleanup(DeadLetterCleanupCommand),
}

#[derive(ClapArgs, Debug, Clone)]
struct AdminApiTarget {
    /// Admin API base URL
    #[arg(long)]
    admin_api: Option<String>,
    /// Bearer token
    #[arg(long)]
    token: Option<String>,
}

#[derive(ClapArgs, Debug)]
struct StatusCommand {
    #[command(flatten)]
    target: AdminApiTarget,
}

#[derive(ClapArgs, Debug)]
struct MappingsCommand {
    #[arg(long, default_value_t = 100)]
    limit: i64,
    #[arg(long, default_value_t = 0)]
    offset: i64,
    #[command(flatten)]
    target: AdminApiTarget,
}

#[derive(ClapArgs, Debug)]
struct ReplayCommand {
    /// Replay a specific dead-letter id
    #[arg(long)]
    id: Option<i64>,
    /// Batch replay filter status
    #[arg(long)]
    status: Option<String>,
    /// Batch replay size
    #[arg(long, default_value_t = 20)]
    limit: i64,
    #[command(flatten)]
    target: AdminApiTarget,
}

#[derive(ClapArgs, Debug)]
struct DeadLetterCleanupCommand {
    #[arg(long)]
    status: Option<String>,
    #[arg(long)]
    older_than_hours: Option<i64>,
    #[arg(long, default_value_t = 200)]
    limit: i64,
    #[arg(long)]
    dry_run: bool,
    #[command(flatten)]
    target: AdminApiTarget,
}

const EXAMPLE_CONFIG: &str = include_str!("../config/config.sample.yaml");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = CliArgs::parse();

    if args.generate_config {
        println!("{}", EXAMPLE_CONFIG);
        return Ok(());
    }

    let config_path = args.config.to_string_lossy().to_string();
    let config = Config::load_from_path(&config_path).with_context(|| {
        format!(
            "Failed to load config at '{}'; use --generate-config to print a template",
            config_path
        )
    })?;

    if let Some(command) = args.command {
        run_management_command(command, &config).await?;
        return Ok(());
    }

    FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .pretty()
        .init();

    info!(
        "Starting Matrix-DingTalk bridge v{}",
        env!("CARGO_PKG_VERSION")
    );

    // Initialize database
    let db = Database::connect(
        "sqlite",
        &config.database.connection_string(),
        20,
        2,
    ).await.context("Failed to connect to database")?;
    
    db.run_migrations().await.context("Failed to run database migrations")?;
    info!("Database initialized successfully");

    // Create bridge
    let bridge = DingTalkBridge::new(config.clone()).await?;
    let bridge = Arc::new(bridge);

    // Start web server
    let web_server = start_web_server(config.clone(), bridge.clone());
    tokio::spawn(web_server);

    // Start bridge
    let bridge_clone = bridge.clone();
    tokio::select! {
        result = bridge_clone.start() => {
            if let Err(e) = result {
                error!("Bridge error: {}", e);
                return Err(e);
            }
            info!("Bridge started");
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Received shutdown signal");
        }
    }

    bridge.stop().await;
    info!("Bridge stopped");

    Ok(())
}

async fn start_web_server(config: Config, bridge: Arc<DingTalkBridge>) {
    let bind_address: &'static str = Box::leak(config.bridge.bind_address.clone().into_boxed_str());
    let port = config.bridge.port;

    let provisioning_api = ProvisioningApi::new(
        bridge,
        std::env::var("PROVISIONING_READ_TOKEN").ok(),
        std::env::var("PROVISIONING_WRITE_TOKEN").ok(),
        std::env::var("PROVISIONING_ADMIN_TOKEN").ok(),
    );

    let router = Router::new()
        .push(
            Router::with_path("health")
                .get(health_endpoint)
        )
        .push(
            Router::with_path("metrics")
                .get(metrics_endpoint)
        )
        .push(
            Router::with_path("admin")
                .push(Router::with_path("status").get(crate::web::get_status))
                .push(Router::with_path("mappings").get(crate::web::mappings))
                .push(Router::with_path("bridge").post(crate::web::bridge_room))
                .hoop(affix_state::inject(provisioning_api))
        );

    let acceptor = TcpListener::new((bind_address, port)).bind().await;
    let service = Service::new(router);

    info!("Web server listening on {}:{}", bind_address, port);
    
    Server::new(acceptor).serve(service).await;
}

mod web {
    use salvo::prelude::*;
    use serde::{Deserialize, Serialize};
    use super::ProvisioningApi;

    #[derive(Debug, Serialize, Deserialize)]
    pub struct BridgeStatus {
        pub started_at: String,
        pub uptime_secs: u64,
        pub version: String,
    }

    #[handler]
    pub async fn get_status(req: &mut Request, res: &mut Response, depot: &mut Depot) {
        let api = depot.obtain::<ProvisioningApi>().cloned().unwrap();
        let token = req.header::<String>("Authorization").map(|s| {
            s.trim_start_matches("Bearer ").to_string()
        });

        if !api.validate_read_token(token.as_deref()) {
            res.status_code(StatusCode::UNAUTHORIZED);
            res.render(Json(serde_json::json!({
                "error": "Unauthorized"
            })));
            return;
        }

        let bridge = api.bridge();
        let started_at = bridge.started_at();
        let uptime = started_at.elapsed().as_secs();

        let resp = BridgeStatus {
            started_at: chrono::Utc::now()
                .checked_sub_signed(chrono::Duration::seconds(uptime as i64))
                .unwrap()
                .to_rfc3339(),
            uptime_secs: uptime,
            version: env!("CARGO_PKG_VERSION").to_string(),
        };

        res.render(Json(resp));
    }

    #[handler]
    pub async fn mappings(req: &mut Request, res: &mut Response, depot: &mut Depot) {
        let api = depot.obtain::<ProvisioningApi>().cloned().unwrap();
        let token = req.header::<String>("Authorization").map(|s| {
            s.trim_start_matches("Bearer ").to_string()
        });

        if !api.validate_read_token(token.as_deref()) {
            res.status_code(StatusCode::UNAUTHORIZED);
            res.render(Json(serde_json::json!({
                "error": "Unauthorized"
            })));
            return;
        }

        let limit: i64 = req.query("limit").unwrap_or(100);
        let offset: i64 = req.query("offset").unwrap_or(0);

        res.render(Json(serde_json::json!({
            "mappings": [],
            "total": 0,
            "limit": limit,
            "offset": offset
        })));
    }

    #[handler]
    pub async fn bridge_room(req: &mut Request, res: &mut Response, depot: &mut Depot) {
        let api = depot.obtain::<ProvisioningApi>().cloned().unwrap();
        let token = req.header::<String>("Authorization").map(|s| {
            s.trim_start_matches("Bearer ").to_string()
        });

        if !api.validate_write_token(token.as_deref()) {
            res.status_code(StatusCode::UNAUTHORIZED);
            res.render(Json(serde_json::json!({
                "error": "Unauthorized"
            })));
            return;
        }

        #[derive(Deserialize)]
        struct BridgeRequest {
            matrix_room_id: String,
            dingtalk_conversation_id: Option<String>,
        }

        let body: Result<BridgeRequest, _> = req.parse_json().await;

        match body {
            Ok(payload) => {
                res.render(Json(serde_json::json!({
                    "status": "pending",
                    "matrix_room_id": payload.matrix_room_id,
                    "dingtalk_conversation_id": payload.dingtalk_conversation_id
                })));
            }
            Err(e) => {
                res.status_code(StatusCode::BAD_REQUEST);
                res.render(Json(serde_json::json!({
                    "error": format!("Invalid request: {}", e)
                })));
            }
        }
    }
}

async fn run_management_command(command: Command, config: &Config) -> anyhow::Result<()> {
    let client = Client::builder()
        .build()
        .context("failed to create HTTP client")?;

    match command {
        Command::Status(cmd) => {
            let (base, token) = resolve_admin_access(config, &cmd.target);
            let response = api_get(&client, &format!("{base}/status"), &token).await?;
            print_json(&response)?;
        }
        Command::Mappings(cmd) => {
            let (base, token) = resolve_admin_access(config, &cmd.target);
            let url = format!(
                "{base}/mappings?limit={}&offset={}",
                cmd.limit.max(1),
                cmd.offset.max(0)
            );
            let response = api_get(&client, &url, &token).await?;
            print_json(&response)?;
        }
        Command::Replay(cmd) => {
            let (base, token) = resolve_admin_access(config, &cmd.target);
            let response = if let Some(id) = cmd.id {
                api_post_json(
                    &client,
                    &format!("{base}/dead-letters/{id}/replay"),
                    &token,
                    json!({}),
                )
                .await?
            } else {
                api_post_json(
                    &client,
                    &format!("{base}/dead-letters/replay"),
                    &token,
                    json!({
                        "status": cmd.status,
                        "limit": cmd.limit.max(1),
                    }),
                )
                .await?
            };
            print_json(&response)?;
        }
        Command::DeadLetterCleanup(cmd) => {
            let (base, token) = resolve_admin_access(config, &cmd.target);
            let response = api_post_json(
                &client,
                &format!("{base}/dead-letters/cleanup"),
                &token,
                json!({
                    "status": cmd.status,
                    "older_than_hours": cmd.older_than_hours,
                    "limit": cmd.limit.max(1),
                    "dry_run": cmd.dry_run,
                }),
            )
            .await?;
            print_json(&response)?;
        }
    }

    Ok(())
}

fn resolve_admin_access(config: &Config, target: &AdminApiTarget) -> (String, String) {
    let base = target.admin_api.clone().unwrap_or_else(|| {
        format!(
            "http://{}:{}/admin",
            config.bridge.bind_address, config.bridge.port
        )
    });

    let token = target
        .token
        .clone()
        .or_else(|| std::env::var("MATRIX_BRIDGE_DINGTALK_PROVISIONING_TOKEN").ok())
        .unwrap_or_else(|| config.registration.appservice_token.clone());

    (base.trim_end_matches('/').to_string(), token)
}

async fn api_get(client: &Client, url: &str, token: &str) -> anyhow::Result<Value> {
    let response = client
        .get(url)
        .bearer_auth(token)
        .send()
        .await
        .with_context(|| format!("GET request failed: {url}"))?;
    decode_api_response(response).await
}

async fn api_post_json(
    client: &Client,
    url: &str,
    token: &str,
    body: Value,
) -> anyhow::Result<Value> {
    let response = client
        .post(url)
        .bearer_auth(token)
        .json(&body)
        .send()
        .await
        .with_context(|| format!("POST request failed: {url}"))?;
    decode_api_response(response).await
}

async fn decode_api_response(response: reqwest::Response) -> anyhow::Result<Value> {
    let status = response.status();
    let body = response
        .text()
        .await
        .context("failed to read API response body")?;
    let payload = if body.trim().is_empty() {
        json!({})
    } else {
        serde_json::from_str(&body).unwrap_or_else(|_| json!({ "raw": body }))
    };

    if !status.is_success() {
        return Err(anyhow!(
            "API request failed: status={} payload={}",
            status,
            payload
        ));
    }

    Ok(payload)
}

fn print_json(value: &Value) -> anyhow::Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}
