use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use anyhow::Context;
use chrono::Utc;
use matrix_bot_sdk::appservice::{Appservice, AppserviceHandler, Intent};
use matrix_bot_sdk::client::{MatrixAuth, MatrixClient};
use serde_json::Value;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use url::Url;

use super::portal::{BridgePortal, PortalManager, RoomType};
use super::puppet::{BridgePuppet, PuppetManager};
use super::user::{BridgeUser, UserSyncPolicy};
use super::{MatrixCommandHandler, PresenceHandler, ProvisioningCoordinator};
use crate::config::Config;
use crate::database::{
    Database, DeadLetterEvent, DeadLetterStore, EventStore, MessageMapping, MessageStore,
    ProcessedEvent, RoomMapping, RoomStore, SqliteStores,
};
use crate::dingtalk::DingTalkService;

#[derive(Clone)]
pub struct DingTalkBridge {
    pub config: Arc<Config>,
    pub dingtalk_service: Arc<DingTalkService>,
    pub appservice: Arc<Appservice>,
    pub bot_intent: Intent,
    stores: Arc<SqliteStores>,
    room_store: Arc<dyn RoomStore>,
    message_store: Arc<dyn MessageStore>,
    event_store: Arc<dyn EventStore>,
    dead_letter_store: Arc<dyn DeadLetterStore>,
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
    pub async fn new(config: Config, db: Database) -> anyhow::Result<Self> {
        let config = Arc::new(config);

        let webhook_url = std::env::var("DINGTALK_WEBHOOK_URL")
            .unwrap_or_else(|_| "https://oapi.dingtalk.com/robot/send".to_string());
        let access_token = std::env::var("DINGTALK_ACCESS_TOKEN")
            .ok()
            .or_else(|| config.auth.webhooks.values().next().cloned())
            .unwrap_or_default();
        let secret = std::env::var("DINGTALK_SECRET")
            .ok()
            .or_else(|| config.auth.security.secret.clone());
        let callback_token = std::env::var("DINGTALK_CALLBACK_TOKEN").ok().or_else(|| {
            if config.callback.token.is_empty() {
                None
            } else {
                Some(config.callback.token.clone())
            }
        });
        let webhook_tokens = config.auth.webhooks.clone();

        let dingtalk_service = Arc::new(DingTalkService::new(
            webhook_url,
            access_token,
            secret,
            callback_token,
            webhook_tokens,
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

        let stores = Arc::new(db.stores());
        let room_store = stores.room_store();
        let message_store = stores.message_store();
        let event_store = stores.event_store();
        let dead_letter_store = stores.dead_letter_store();

        let command_handler = Arc::new(MatrixCommandHandler::new(true));
        let provisioning = Arc::new(ProvisioningCoordinator::new(300));
        let presence_handler = Arc::new(PresenceHandler::new(Some(50)));
        let user_sync_policy = UserSyncPolicy::default();

        Ok(Self {
            config,
            dingtalk_service,
            appservice: Arc::new(appservice),
            bot_intent,
            stores,
            room_store,
            message_store,
            event_store,
            dead_letter_store,
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
        self.load_portals_from_db().await?;

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

    pub async fn bridge_room(
        &self,
        matrix_room_id: &str,
        dingtalk_conversation_id: &str,
        dingtalk_conversation_name: Option<String>,
    ) -> anyhow::Result<RoomMapping> {
        if self
            .room_store
            .get_room_mapping(matrix_room_id)
            .await
            .context("query room mapping by matrix id failed")?
            .is_some()
        {
            anyhow::bail!("matrix room is already bridged");
        }

        if self
            .room_store
            .get_room_mapping_by_dingtalk(dingtalk_conversation_id)
            .await
            .context("query room mapping by dingtalk id failed")?
            .is_some()
        {
            anyhow::bail!("dingtalk conversation is already bridged");
        }

        let mapping = RoomMapping::new(
            matrix_room_id.to_string(),
            dingtalk_conversation_id.to_string(),
            dingtalk_conversation_name,
        );
        let saved = self
            .room_store
            .insert_room_mapping(&mapping)
            .await
            .context("insert room mapping failed")?;

        self.portal_manager
            .add_portal(Self::portal_from_mapping(&saved))
            .await;

        Ok(saved)
    }

    pub async fn unbridge_room(&self, matrix_room_id: &str) -> anyhow::Result<bool> {
        let removed = self
            .room_store
            .delete_room_mapping(matrix_room_id)
            .await
            .context("delete room mapping failed")?;
        if removed {
            self.portal_manager.remove_portal(matrix_room_id).await;
        }
        Ok(removed)
    }

    pub async fn list_room_mappings(&self, limit: i64, offset: i64) -> anyhow::Result<Vec<RoomMapping>> {
        self.room_store
            .list_room_mappings(limit.max(1), offset.max(0))
            .await
            .context("list room mappings failed")
    }

    pub async fn get_room_mapping_by_matrix(
        &self,
        matrix_room_id: &str,
    ) -> anyhow::Result<Option<RoomMapping>> {
        self.room_store
            .get_room_mapping(matrix_room_id)
            .await
            .context("query room mapping by matrix id failed")
    }

    pub async fn get_room_mapping_by_dingtalk(
        &self,
        conversation_id: &str,
    ) -> anyhow::Result<Option<RoomMapping>> {
        self.room_store
            .get_room_mapping_by_dingtalk(conversation_id)
            .await
            .context("query room mapping by dingtalk id failed")
    }

    pub async fn dead_letter_counts(&self) -> anyhow::Result<(i64, i64, i64, i64)> {
        let pending = self
            .dead_letter_store
            .count_dead_letters(Some("pending"))
            .await
            .context("count pending dead letters failed")?;
        let failed = self
            .dead_letter_store
            .count_dead_letters(Some("failed"))
            .await
            .context("count failed dead letters failed")?;
        let replayed = self
            .dead_letter_store
            .count_dead_letters(Some("replayed"))
            .await
            .context("count replayed dead letters failed")?;
        let total = self
            .dead_letter_store
            .count_dead_letters(None)
            .await
            .context("count dead letters failed")?;

        Ok((pending, failed, replayed, total))
    }

    pub async fn list_dead_letters(
        &self,
        status: Option<&str>,
        limit: i64,
    ) -> anyhow::Result<Vec<DeadLetterEvent>> {
        self.dead_letter_store
            .list_dead_letters(status, limit.max(1))
            .await
            .context("list dead letters failed")
    }

    pub async fn replay_dead_letter(&self, id: i64) -> anyhow::Result<()> {
        let Some(event) = self
            .dead_letter_store
            .get_dead_letter(id)
            .await
            .context("fetch dead letter failed")?
        else {
            anyhow::bail!("dead-letter id={} not found", id);
        };

        if event.status.eq_ignore_ascii_case("replayed") {
            return Ok(());
        }

        let payload: Value = serde_json::from_str(&event.payload)
            .with_context(|| format!("dead-letter id={} payload is not valid json", id))?;
        let conversation_id = event
            .conversation_id
            .clone()
            .or_else(|| {
                payload
                    .get("conversation_id")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
            })
            .or_else(|| {
                payload
                    .get("dingtalk_conversation_id")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
            });
        let content = payload
            .get("content")
            .and_then(Value::as_str)
            .or_else(|| payload.get("body").and_then(Value::as_str));

        match (conversation_id.as_deref(), content) {
            (conversation, Some(content)) => {
                self.dingtalk_service
                    .send_text_to_conversation(conversation, content, None, None, false)
                    .await
                    .with_context(|| format!("replay dead-letter id={} send failed", id))?;
                let _ = self
                    .dead_letter_store
                    .update_dead_letter_status(id, "replayed")
                    .await;
                Ok(())
            }
            (_, None) => {
                let _ = self
                    .dead_letter_store
                    .update_dead_letter_status(id, "failed")
                    .await;
                anyhow::bail!("dead-letter id={} missing content/body", id)
            }
        }
    }

    pub async fn replay_dead_letters(
        &self,
        status: &str,
        limit: i64,
    ) -> anyhow::Result<(usize, Vec<String>)> {
        let events = self
            .dead_letter_store
            .list_dead_letters(Some(status), limit.max(1))
            .await
            .context("list dead letters for replay failed")?;

        let mut replayed = 0usize;
        let mut errors = Vec::new();
        for event in events {
            match self.replay_dead_letter(event.id).await {
                Ok(()) => replayed += 1,
                Err(err) => errors.push(format!("id={}: {}", event.id, err)),
            }
        }

        Ok((replayed, errors))
    }

    pub async fn cleanup_dead_letters(
        &self,
        status: Option<&str>,
        older_than_hours: Option<i64>,
        limit: i64,
    ) -> anyhow::Result<u64> {
        self.dead_letter_store
            .cleanup_dead_letters(status, older_than_hours, limit.max(1))
            .await
            .context("cleanup dead letters failed")
    }

    pub async fn record_dead_letter(
        &self,
        source: &str,
        event_type: &str,
        dedupe_key: &str,
        conversation_id: Option<String>,
        payload: Value,
        error: &str,
    ) -> anyhow::Result<()> {
        let now = Utc::now();
        let dead_letter = DeadLetterEvent {
            id: 0,
            source: source.to_string(),
            event_type: event_type.to_string(),
            dedupe_key: dedupe_key.to_string(),
            conversation_id,
            payload: payload.to_string(),
            error: error.to_string(),
            status: "pending".to_string(),
            replay_count: 0,
            last_replayed_at: None,
            created_at: now,
            updated_at: now,
        };

        self.dead_letter_store
            .insert_dead_letter(&dead_letter)
            .await
            .context("insert dead letter failed")?;
        Ok(())
    }

    pub async fn mark_event_processed(
        &self,
        event_id: &str,
        event_type: &str,
        source: &str,
    ) -> anyhow::Result<()> {
        let event = ProcessedEvent {
            id: 0,
            event_id: event_id.to_string(),
            event_type: event_type.to_string(),
            source: source.to_string(),
            processed_at: Utc::now(),
        };
        self.event_store
            .mark_event_processed(&event)
            .await
            .context("mark event processed failed")
    }

    pub async fn is_event_processed(&self, event_id: &str) -> anyhow::Result<bool> {
        self.event_store
            .is_event_processed(event_id)
            .await
            .context("query processed event failed")
    }

    pub async fn save_message_mapping(
        &self,
        mapping: &MessageMapping,
    ) -> anyhow::Result<MessageMapping> {
        self.message_store
            .insert_message_mapping(mapping)
            .await
            .context("insert message mapping failed")
    }

    async fn load_portals_from_db(&self) -> anyhow::Result<()> {
        let mappings = self
            .room_store
            .list_room_mappings(i64::MAX, 0)
            .await
            .context("load room mappings failed")?;

        for mapping in mappings {
            self.portal_manager
                .add_portal(Self::portal_from_mapping(&mapping))
                .await;
        }

        Ok(())
    }

    fn portal_from_mapping(mapping: &RoomMapping) -> BridgePortal {
        let room_type = if mapping.dingtalk_conversation_type.eq_ignore_ascii_case("direct") {
            RoomType::Direct
        } else {
            RoomType::Group
        };

        let mut portal = BridgePortal::new(
            mapping.matrix_room_id.clone(),
            mapping.dingtalk_conversation_id.clone(),
            room_type,
        );
        portal.name = mapping.dingtalk_conversation_name.clone();
        portal
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

    pub fn provisioning(&self) -> Arc<ProvisioningCoordinator> {
        self.provisioning.clone()
    }

    pub fn stores(&self) -> Arc<SqliteStores> {
        self.stores.clone()
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
        let event_count = events.len();

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

        warn!(
            "Matrix transaction received but processing pipeline is not enabled yet; event count={}.",
            event_count
        );

        Ok(())
    }
}
