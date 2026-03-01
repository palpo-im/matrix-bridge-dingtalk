use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use matrix_bot_sdk::appservice::{Appservice, AppserviceHandler, Intent};
use matrix_bot_sdk::client::{MatrixAuth, MatrixClient};
use serde_json::Value;
use tokio::sync::RwLock;
use tracing::{error, info};
use url::Url;

use super::portal::{BridgePortal, PortalManager};
use super::puppet::{BridgePuppet, PuppetManager};
use super::user::{BridgeUser, UserSyncPolicy};
use super::{MatrixCommandHandler, PresenceHandler, ProvisioningCoordinator};
use crate::config::Config;
use crate::dingtalk::DingTalkService;

#[derive(Clone)]
pub struct DingTalkBridge {
    pub config: Arc<Config>,
    pub dingtalk_service: Arc<DingTalkService>,
    pub appservice: Arc<Appservice>,
    pub bot_intent: Intent,
    portal_manager: PortalManager,
    puppet_manager: Arc<RwLock<PuppetManager>>,
    users_by_mxid: Arc<RwLock<HashMap<String, BridgeUser>>>,
    intents: Arc<RwLock<HashMap<String, Intent>>>,
    command_handler: Arc<MatrixCommandHandler>,
    provisioning: Arc<ProvisioningCoordinator>,
    presence_handler: Arc<PresenceHandler>,
    started_at: Instant,
    user_sync_policy: UserSyncPolicy,
    user_last_synced_at: Arc<RwLock<HashMap<String, Instant>>>,
}

impl DingTalkBridge {
    pub async fn new(config: Config) -> anyhow::Result<Self> {
        let config = Arc::new(config);

        let webhook_url = std::env::var("DINGTALK_WEBHOOK_URL")
            .unwrap_or_else(|_| "https://oapi.dingtalk.com/robot/send".to_string());
        let access_token = std::env::var("DINGTALK_ACCESS_TOKEN")
            .unwrap_or_else(|_| String::new());
        let secret = std::env::var("DINGTALK_SECRET").ok();
        let callback_token = std::env::var("DINGTALK_CALLBACK_TOKEN").ok();

        let dingtalk_service = Arc::new(DingTalkService::new(
            webhook_url,
            access_token,
            secret,
            callback_token,
        ));

        let homeserver_url = Url::parse(&config.bridge.homeserver_url)?;
        let bot_mxid = format!(
            "@{}:{}",
            config.registration.sender_localpart, config.bridge.domain
        );

        let client = MatrixClient::new(
            homeserver_url,
            MatrixAuth::new(&config.registration.appservice_token).with_user_id(&bot_mxid),
        );

        let appservice = Appservice::new(
            config.registration.homeserver_token.clone(),
            config.registration.appservice_token.clone(),
            client,
        )
        .with_appservice_id(&config.registration.bridge_id)
        .with_protocols(["dingtalk"]);

        let bot_intent = Intent::new(&bot_mxid, appservice.client.clone());

        let command_handler = Arc::new(MatrixCommandHandler::new(true));
        let provisioning = Arc::new(ProvisioningCoordinator::new(300));
        let presence_handler = Arc::new(PresenceHandler::new(Some(50)));
        let user_sync_policy = UserSyncPolicy::default();

        Ok(Self {
            config,
            dingtalk_service,
            appservice: Arc::new(appservice),
            bot_intent,
            portal_manager: PortalManager::new(),
            puppet_manager: Arc::new(RwLock::new(PuppetManager::new())),
            users_by_mxid: Arc::new(RwLock::new(HashMap::new())),
            intents: Arc::new(RwLock::new(HashMap::new())),
            command_handler,
            provisioning,
            presence_handler,
            started_at: Instant::now(),
            user_sync_policy,
            user_last_synced_at: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn start(&self) -> anyhow::Result<()> {
        info!("Starting DingTalk bridge");

        self.bot_intent.ensure_registered().await?;

        let service = self.dingtalk_service.clone();
        let bridge_clone = Arc::new(self.clone());
        tokio::spawn(async move {
            if let Err(e) = service.start(bridge_clone).await {
                error!("DingTalk service error: {}", e);
            }
        });

        let maintenance_bridge = self.clone();
        tokio::spawn(async move {
            maintenance_bridge.run_user_sync_maintenance_loop().await;
        });

        info!("DingTalk bridge started");
        Ok(())
    }

    pub async fn stop(&self) {
        info!("Stopping DingTalk bridge");
        self.dingtalk_service.stop().await;
        info!("DingTalk bridge stopped");
    }

    pub fn started_at(&self) -> Instant {
        self.started_at
    }

    async fn run_user_sync_maintenance_loop(&self) {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(300)).await;
            self.sync_stale_users().await;
        }
    }

    async fn sync_stale_users(&self) {
        let users = self.users_by_mxid.read().await;
        for (mxid, user) in users.iter() {
            if user.needs_sync(&self.user_sync_policy) {
                info!("Syncing stale user: {}", mxid);
            }
        }
    }

    pub async fn get_portal(&self, room_id: &str) -> Option<BridgePortal> {
        self.portal_manager.get_by_matrix_room(room_id).await
    }

    pub async fn add_portal(&self, portal: BridgePortal) {
        self.portal_manager.add_portal(portal).await;
    }

    pub async fn remove_portal(&self, room_id: &str) {
        self.portal_manager.remove_portal(room_id).await;
    }

    pub async fn get_puppet(&self, dingtalk_user_id: &str) -> Option<Arc<BridgePuppet>> {
        let guard = self.puppet_manager.read().await;
        guard.get_puppet(dingtalk_user_id)
    }

    pub async fn add_puppet(&self, puppet: BridgePuppet) {
        let mut guard = self.puppet_manager.write().await;
        guard.add_puppet(puppet);
    }
}

pub struct BridgeHandler {
    bridge: Arc<DingTalkBridge>,
}

impl BridgeHandler {
    pub fn new(bridge: Arc<DingTalkBridge>) -> Self {
        Self { bridge }
    }
}

#[async_trait::async_trait]
impl AppserviceHandler for BridgeHandler {
    async fn on_transaction(&self, _txn_id: &str, body: &Value) -> anyhow::Result<()> {
        let events = body
            .get("events")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        for event in events {
            let event_type = event
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or_default();

            let room_id = event
                .get("room_id")
                .and_then(Value::as_str)
                .unwrap_or_default();

            let sender = event
                .get("sender")
                .and_then(Value::as_str)
                .unwrap_or_default();

            info!(
                "Received event: type={}, room={}, sender={}",
                event_type, room_id, sender
            );
        }

        Ok(())
    }
}
