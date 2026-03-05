use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, anyhow};
use clap::{Args as ClapArgs, Parser, Subcommand};
use matrix_bridge_dingtalk::bridge::DingTalkBridge;
use matrix_bridge_dingtalk::config::Config;
use matrix_bridge_dingtalk::database::Database;
use matrix_bridge_dingtalk::web::{
    ProvisioningApi, dingtalk_callback, health_endpoint, metrics_endpoint,
};
use reqwest::Client;
use salvo::prelude::*;
use serde_json::{Value, json};
use tracing::{Level, info};
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

#[derive(Debug, Clone, Copy)]
enum TokenScope {
    Read,
    Write,
    Delete,
}

const EXAMPLE_CONFIG: &str = include_str!("../config/config.example.yaml");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = CliArgs::parse();
    let config_path = resolve_config_path(&args.config);

    if args.generate_config {
        println!("{}", EXAMPLE_CONFIG);
        return Ok(());
    }

    let config = Config::load_from_file(&config_path).with_context(|| {
        format!(
            "Failed to load config at '{}'; use --generate-config to print a template",
            config_path.display()
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

    let db_uri = config.database.connection_string();
    let db_max_open = config.database.max_connections().unwrap_or(20);
    let db_max_idle = config.database.min_connections().unwrap_or(2);
    let db = Database::connect(
        config.database.db_type_name(),
        &db_uri,
        db_max_open,
        db_max_idle,
    )
    .await
    .context("Failed to connect to database")?;

    db.run_migrations()
        .await
        .context("Failed to run database migrations")?;
    info!("Database initialized successfully");

    let bridge = DingTalkBridge::new(config.clone(), db)
        .await
        .context("Failed to initialize bridge")?;
    let bridge = Arc::new(bridge);

    let web_server = start_web_server(config.clone(), bridge.clone());
    tokio::spawn(web_server);

    bridge
        .start()
        .await
        .context("Failed to start bridge background services")?;
    info!("Bridge started");

    tokio::signal::ctrl_c()
        .await
        .context("Failed waiting for shutdown signal")?;
    info!("Received shutdown signal");

    bridge.stop().await;
    info!("Bridge stopped");

    Ok(())
}

async fn start_web_server(config: Config, bridge: Arc<DingTalkBridge>) {
    let bind_address: &'static str = Box::leak(config.bridge.bind_address.clone().into_boxed_str());
    let port = config.bridge.port;

    let default_token = std::env::var("MATRIX_BRIDGE_DINGTALK_PROVISIONING_TOKEN")
        .unwrap_or_else(|_| config.registration.appservice_token.clone());
    let read_token = std::env::var("MATRIX_BRIDGE_DINGTALK_PROVISIONING_READ_TOKEN")
        .ok()
        .or_else(|| Some(default_token.clone()));
    let write_token = std::env::var("MATRIX_BRIDGE_DINGTALK_PROVISIONING_WRITE_TOKEN")
        .ok()
        .or_else(|| Some(default_token.clone()));
    let delete_token = std::env::var("MATRIX_BRIDGE_DINGTALK_PROVISIONING_DELETE_TOKEN")
        .ok()
        .or_else(|| std::env::var("MATRIX_BRIDGE_DINGTALK_PROVISIONING_ADMIN_TOKEN").ok())
        .or_else(|| write_token.clone());
    let admin_token = std::env::var("MATRIX_BRIDGE_DINGTALK_PROVISIONING_ADMIN_TOKEN").ok();

    let provisioning_api = ProvisioningApi::new(
        bridge.clone(),
        read_token,
        write_token,
        delete_token,
        admin_token,
    );
    let appservice_router = bridge.clone().appservice_router();

    let mut router = Router::new()
        .push(appservice_router)
        .push(Router::with_path("health").get(health_endpoint))
        .push(Router::with_path("metrics").get(metrics_endpoint))
        .push(Router::with_path("admin").push(provisioning_api.router()));

    if config.dingtalk.callback.enabled {
        println!("[DEBUG] ==================================");
        println!("[DEBUG] DingTalk callback endpoint ENABLED");
        println!("[DEBUG]   Path: POST /dingtalk/callback");
        println!("[DEBUG]   Port: {}", port);
        println!("[DEBUG] ==================================");
        eprintln!("[DEBUG] DingTalk callback endpoint ENABLED at /dingtalk/callback");
        info!("DingTalk callback endpoint enabled (compatibility mode)");
        router = router.push(
            Router::with_path("dingtalk/callback")
                .hoop(affix_state::inject(bridge.clone()))
                .post(dingtalk_callback),
        );
    } else {
        println!("[DEBUG] ==================================");
        println!("[DEBUG] DingTalk callback endpoint DISABLED");
        println!("[DEBUG] Messages must come through stream mode");
        println!("[DEBUG] ==================================");
    }

    let acceptor = TcpListener::new((bind_address, port)).bind().await;
    let service = Service::new(router);

    info!("Web server listening on {}:{}", bind_address, port);
    Server::new(acceptor).serve(service).await;
}

async fn run_management_command(command: Command, config: &Config) -> anyhow::Result<()> {
    let client = Client::builder()
        .build()
        .context("failed to create HTTP client")?;

    match command {
        Command::Status(cmd) => {
            let (base, token) = resolve_admin_access(config, &cmd.target, TokenScope::Read);
            let response = api_get(&client, &format!("{base}/status"), &token).await?;
            print_json(&response)?;
        }
        Command::Mappings(cmd) => {
            let (base, token) = resolve_admin_access(config, &cmd.target, TokenScope::Read);
            let url = format!(
                "{base}/mappings?limit={}&offset={}",
                cmd.limit.max(1),
                cmd.offset.max(0)
            );
            let response = api_get(&client, &url, &token).await?;
            print_json(&response)?;
        }
        Command::Replay(cmd) => {
            let (base, token) = resolve_admin_access(config, &cmd.target, TokenScope::Write);
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
            let (base, token) = resolve_admin_access(config, &cmd.target, TokenScope::Delete);
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

fn resolve_admin_access(
    config: &Config,
    target: &AdminApiTarget,
    required_scope: TokenScope,
) -> (String, String) {
    let base = target.admin_api.clone().unwrap_or_else(|| {
        format!(
            "http://{}:{}/admin",
            config.bridge.bind_address, config.bridge.port
        )
    });

    let token = target
        .token
        .clone()
        .or_else(|| env_token_for_scope(required_scope))
        .unwrap_or_else(|| config.registration.appservice_token.clone());

    (base.trim_end_matches('/').to_string(), token)
}

fn resolve_config_path(cli_path: &std::path::Path) -> PathBuf {
    let default = std::path::Path::new("config.yaml");
    if cli_path == default {
        if let Ok(path) = std::env::var("CONFIG_PATH") {
            let trimmed = path.trim();
            if !trimmed.is_empty() {
                return PathBuf::from(trimmed);
            }
        }
    }
    cli_path.to_path_buf()
}

fn env_token_for_scope(scope: TokenScope) -> Option<String> {
    match scope {
        TokenScope::Read => std::env::var("MATRIX_BRIDGE_DINGTALK_PROVISIONING_READ_TOKEN")
            .or_else(|_| std::env::var("MATRIX_BRIDGE_DINGTALK_PROVISIONING_WRITE_TOKEN"))
            .or_else(|_| std::env::var("MATRIX_BRIDGE_DINGTALK_PROVISIONING_DELETE_TOKEN"))
            .or_else(|_| std::env::var("MATRIX_BRIDGE_DINGTALK_PROVISIONING_ADMIN_TOKEN"))
            .or_else(|_| std::env::var("MATRIX_BRIDGE_DINGTALK_PROVISIONING_TOKEN"))
            .ok(),
        TokenScope::Write => std::env::var("MATRIX_BRIDGE_DINGTALK_PROVISIONING_WRITE_TOKEN")
            .or_else(|_| std::env::var("MATRIX_BRIDGE_DINGTALK_PROVISIONING_DELETE_TOKEN"))
            .or_else(|_| std::env::var("MATRIX_BRIDGE_DINGTALK_PROVISIONING_ADMIN_TOKEN"))
            .or_else(|_| std::env::var("MATRIX_BRIDGE_DINGTALK_PROVISIONING_TOKEN"))
            .ok(),
        TokenScope::Delete => std::env::var("MATRIX_BRIDGE_DINGTALK_PROVISIONING_DELETE_TOKEN")
            .or_else(|_| std::env::var("MATRIX_BRIDGE_DINGTALK_PROVISIONING_ADMIN_TOKEN"))
            .or_else(|_| std::env::var("MATRIX_BRIDGE_DINGTALK_PROVISIONING_WRITE_TOKEN"))
            .or_else(|_| std::env::var("MATRIX_BRIDGE_DINGTALK_PROVISIONING_TOKEN"))
            .ok(),
    }
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
