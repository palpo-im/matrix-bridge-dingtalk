#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(unused_imports)]

use std::sync::Arc;

use anyhow::Result;
use tracing::{error, info};

mod admin;
mod bridge;
mod cache;
mod cli;
mod config;
mod db;
mod dingtalk;
mod matrix;
mod media;
mod parsers;
mod utils;
mod web;

use config::Config;
use utils::logging::init_tracing;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let config = Arc::new(Config::load()?);
    info!("matrix-dingtalk bridge starting up");

    // TODO: Initialize database
    // let db_manager = Arc::new(db::DatabaseManager::new(&config.database).await?);
    // db_manager.migrate().await?;

    // TODO: Initialize Matrix client
    // let matrix_client = Arc::new(matrix::MatrixAppservice::new(config.clone()).await?);

    // TODO: Initialize DingTalk client
    // let dingtalk_client = Arc::new(dingtalk::DingTalkClient::new(config.clone()).await?);

    // TODO: Initialize bridge core
    // let bridge = Arc::new(bridge::BridgeCore::new(
    //     matrix_client.clone(),
    //     dingtalk_client.clone(),
    //     db_manager.clone(),
    // ));

    // TODO: Start web server
    // let web_server = web::WebServer::new(
    //     config.clone(),
    //     matrix_client.clone(),
    //     db_manager.clone(),
    //     bridge.clone(),
    // ).await?;

    info!("matrix-dingtalk bridge is ready (placeholder)");

    tokio::signal::ctrl_c().await?;
    info!("received Ctrl+C, beginning shutdown");

    info!("matrix-dingtalk bridge shutting down");
    Ok(())
}
