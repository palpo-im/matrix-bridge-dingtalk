use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, anyhow};
use clap::{Args as ClapArgs, Parser, Subcommand};
use matrix_bridge_dingtalk::bridge::DingTalkBridge;
use matrix_bridge_dingtalk::config::Config;
use reqwest::Client;
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
        .with_max_level(Level::DEBUG)
        .pretty()
        .init();

    info!(
        "Starting Matrix-DingTalk bridge v{}",
        env!("CARGO_PKG_VERSION")
    );

    let bridge = DingTalkBridge::new(config).await?;
    let bridge = Arc::new(bridge);

    let bridge_clone = bridge.clone();
    tokio::select! {
        result = bridge_clone.start() => {
            if let Err(e) = result {
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
