use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Context;
use chrono::Utc;
use matrix_bot_sdk::appservice::{Appservice, AppserviceHandler, Intent};
use matrix_bot_sdk::client::{MatrixAuth, MatrixClient};
use salvo::prelude::Router;
use serde_json::Value;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use url::Url;

use super::matrix_event_parser::{MatrixEvent, MatrixEventParser, ParsedEvent};
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
use crate::formatter::{DingTalkToMatrixFormatter, MatrixToDingTalkFormatter};

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
    _intents: Arc<RwLock<HashMap<String, Intent>>>,
    command_handler: Arc<MatrixCommandHandler>,
    provisioning: Arc<ProvisioningCoordinator>,
    _presence_handler: Arc<PresenceHandler>,
    matrix_event_parser: MatrixEventParser,
    matrix_formatter: MatrixToDingTalkFormatter,
    dingtalk_formatter: DingTalkToMatrixFormatter,
    rate_limiter: Arc<RoomRateLimiter>,
    bot_user_id: String,
    started_at: Instant,
    user_sync_policy: UserSyncPolicy,
    _user_last_synced_at: Arc<RwLock<HashMap<String, Instant>>>,
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
            if config.dingtalk.callback.token.is_empty() {
                None
            } else {
                Some(config.dingtalk.callback.token.clone())
            }
        });
        let webhook_tokens = config.auth.webhooks.clone();

        let dingtalk_service = Arc::new(DingTalkService::new(
            webhook_url,
            access_token,
            secret,
            callback_token,
            config.dingtalk.clone(),
            webhook_tokens,
        ));

        let homeserver_url = Url::parse(&config.bridge.homeserver_url)?;
        let bot_mxid = format!(
            "@{}:{}",
            config.registration.sender_localpart, config.bridge.domain
        );

        println!("[DEBUG] Bridge initialized with bot MXID: {}", bot_mxid);
        println!("[DEBUG] Registration sender_localpart: {}, bridge domain: {}",
            config.registration.sender_localpart, config.bridge.domain);

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
        let matrix_event_parser = MatrixEventParser::new();
        let matrix_formatter = MatrixToDingTalkFormatter::new();
        let dingtalk_formatter = DingTalkToMatrixFormatter::new();
        let rate_limiter = Arc::new(RoomRateLimiter::new(
            config.bridge.message_limit,
            config.bridge.message_cooldown,
        ));

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
            _intents: Arc::new(RwLock::new(HashMap::new())),
            command_handler,
            provisioning,
            _presence_handler: presence_handler,
            matrix_event_parser,
            matrix_formatter,
            dingtalk_formatter,
            rate_limiter,
            bot_user_id: bot_mxid,
            started_at: Instant::now(),
            user_sync_policy,
            _user_last_synced_at: Arc::new(RwLock::new(HashMap::new())),
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

    pub fn appservice_router(self: Arc<Self>) -> Router {
        let handler = Arc::new(BridgeHandler::new(self.clone()));
        Appservice::new(
            self.config.registration.homeserver_token.clone(),
            self.config.registration.appservice_token.clone(),
            self.appservice.client.clone(),
        )
        .with_appservice_id(&self.config.registration.bridge_id)
        .with_protocols(["dingtalk"])
        .with_handler(handler)
        .router()
    }

    pub async fn handle_matrix_transaction(&self, body: &Value) -> anyhow::Result<()> {
        let events = body
            .get("events")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        println!("[DEBUG] Matrix transaction received: {} events", events.len());

        for raw_event in events {
            if let Err(err) = self.process_matrix_event(raw_event.clone()).await {
                let dedupe_key = raw_event
                    .get("event_id")
                    .and_then(Value::as_str)
                    .map(|event_id| format!("matrix:{}", event_id))
                    .unwrap_or_else(|| {
                        format!(
                            "matrix:fallback:{}",
                            raw_event
                                .get("origin_server_ts")
                                .and_then(Value::as_i64)
                                .unwrap_or_default()
                        )
                    });
                let conversation_id = raw_event
                    .get("room_id")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned);
                if let Err(record_err) = self
                    .record_dead_letter(
                        "matrix",
                        "transaction_event",
                        &dedupe_key,
                        conversation_id,
                        raw_event.clone(),
                        &err.to_string(),
                    )
                    .await
                {
                    warn!("Failed to record Matrix dead-letter: {}", record_err);
                }
                warn!("Failed to process matrix event: {}", err);
            }
        }

        Ok(())
    }

    async fn process_matrix_event(&self, raw_event: Value) -> anyhow::Result<()> {
        let event: MatrixEvent =
            serde_json::from_value(raw_event).context("invalid matrix event payload")?;
        let event_id = event.event_id.clone().unwrap_or_default();
        let room_id = event.room_id.clone().unwrap_or_default();
        let sender = event.sender.clone().unwrap_or_default();

        println!("[DEBUG] Processing Matrix event: event_id={}, room_id={}, sender={}", event_id, room_id, sender);

        if !event_id.is_empty() && self.is_event_processed(&event_id).await? {
            println!("[DEBUG] Matrix event already processed, skipping: {}", event_id);
            return Ok(());
        }

        match self.matrix_event_parser.parse(&event) {
            ParsedEvent::Message {
                msgtype,
                body,
                formatted_body,
                reply_to,
                edit_of,
            } => {
                println!("[DEBUG] Parsed as Message event: msgtype={}, body={}", msgtype, body);
                self.handle_matrix_message(
                    &event,
                    &msgtype,
                    &body,
                    formatted_body.as_deref(),
                    reply_to.as_deref(),
                    edit_of.as_deref(),
                )
                .await?;
            }
            ParsedEvent::Member {
                membership,
                user_id,
                state_key,
            } => {
                println!("[DEBUG] Parsed as Member event: membership={}, user_id={}, state_key={:?}", membership, user_id, state_key);
                self.handle_member_event(&event, &membership, &user_id, state_key.as_deref())
                    .await;
            }
            ParsedEvent::Redaction { redacts } => {
                println!("[DEBUG] Parsed as Redaction event: redacts={:?}", redacts);
                self.handle_redaction_event(redacts.as_deref()).await?;
            }
            ParsedEvent::Unknown(event_type) => {
                println!("[DEBUG] Parsed as Unknown event type: {}", event_type);
                debug!("Ignoring unsupported Matrix event type: {}", event_type);
            }
        }

        if !event_id.is_empty() {
            println!("[DEBUG] Marking Matrix event as processed: {}", event_id);
            self.mark_event_processed(&event_id, &event.event_type, "matrix")
                .await?;
        }

        println!("[DEBUG] Successfully processed Matrix event: {}", event_id);
        Ok(())
    }

    async fn handle_matrix_message(
        &self,
        event: &MatrixEvent,
        msgtype: &str,
        body: &str,
        formatted_body: Option<&str>,
        reply_to: Option<&str>,
        edit_of: Option<&str>,
    ) -> anyhow::Result<()> {
        let room_id = event.room_id.clone().unwrap_or_default();
        let sender = event.sender.clone().unwrap_or_default();
        let event_id = event.event_id.clone().unwrap_or_default();

        println!("[DEBUG] ===== Matrix message received =====");
        println!("[DEBUG] event_id={}, room_id={}, sender={}, msgtype={}", event_id, room_id, sender, msgtype);
        println!("[DEBUG] body={:?}", body);
        println!("[DEBUG] formatted_body={:?}", formatted_body);
        println!("[DEBUG] ==================================");

        if room_id.is_empty() || sender.is_empty() {
            anyhow::bail!("matrix event missing room_id or sender");
        }

        if self.is_bridge_bot_sender(&sender) {
            println!("[DEBUG] Ignoring message from bridge bot: {}", sender);
            return Ok(());
        }

        if !self.rate_limiter.allow(&room_id) {
            println!("[DEBUG] Rate limit exceeded for room: {}", room_id);
            warn!(
                room_id = %room_id,
                event_id = %event_id,
                "Dropping Matrix event due to room rate limit"
            );
            return Ok(());
        }

        if self.is_blocked_msgtype(msgtype) {
            println!("[DEBUG] Blocked message type: {}", msgtype);
            debug!(
                room_id = %room_id,
                event_id = %event_id,
                msgtype = %msgtype,
                "Skipping blocked Matrix msgtype"
            );
            return Ok(());
        }

        let Some(mapping) = self.get_room_mapping_by_matrix(&room_id).await? else {
            println!("[DEBUG] No room mapping found for Matrix room: {}", room_id);
            return Ok(());
        };

        println!("[DEBUG] Found room mapping: {} -> {}", room_id, mapping.dingtalk_conversation_id);

        let source_text = formatted_body.unwrap_or(body);
        if let Some(command) =
            MatrixCommandHandler::parse_command(source_text, room_id.clone(), sender.clone())
        {
            println!("[DEBUG] Command detected in message: {:?}", command);
            match self.command_handler.handle(command, &self.bot_intent).await {
                Ok(outcome) => {
                    println!("[DEBUG] Command handler outcome: {:?}", outcome);
                    let reply = match outcome {
                        super::MatrixCommandOutcome::Success(message) => Some(message),
                        super::MatrixCommandOutcome::Error(message) => Some(message),
                        super::MatrixCommandOutcome::NoAction => None,
                    };
                    if let Some(ref reply) = reply {
                        println!("[DEBUG] Sending command reply to room {}: {}", room_id, reply);
                        match self.bot_intent.send_text(&room_id, reply).await {
                            Ok(_) => println!("[DEBUG] Command reply sent successfully"),
                            Err(e) => println!("[DEBUG] Failed to send command reply: {}", e),
                        }
                    } else {
                        println!("[DEBUG] No reply to send (NoAction)");
                    }
                }
                Err(e) => {
                    println!("[DEBUG] Command handler error: {}", e);
                }
            }
            return Ok(());
        }

        println!("[DEBUG] Not a command, proceeding to forward to DingTalk");

        if msgtype.starts_with("m.image") && !self.config.bridge.allow_images {
            println!("[DEBUG] Image messages not allowed, skipping");
            return Ok(());
        }
        if msgtype.starts_with("m.video") && !self.config.bridge.allow_videos {
            println!("[DEBUG] Video messages not allowed, skipping");
            return Ok(());
        }
        if msgtype.starts_with("m.audio") && !self.config.bridge.allow_audio {
            println!("[DEBUG] Audio messages not allowed, skipping");
            return Ok(());
        }
        if (msgtype.starts_with("m.file") || msgtype == "m.sticker")
            && !self.config.bridge.allow_files
        {
            return Ok(());
        }

        let rendered = if msgtype == "m.emote" {
            self.matrix_formatter
                .format_text(&format!("* {}", source_text), &sender)
        } else if matches!(
            msgtype,
            "m.image" | "m.video" | "m.audio" | "m.file" | "m.sticker"
        ) {
            let media_label = msgtype.trim_start_matches("m.");
            self.matrix_formatter
                .format_text(&format!("[{}] {}", media_label, source_text), &sender)
        } else {
            self.matrix_formatter.format_text(source_text, &sender)
        };

        let mut outbound = rendered;
        if let Some(reply_event_id) = reply_to {
            if self.config.bridge.bridge_matrix_reply {
                outbound = format!("↪ {} \n{}", reply_event_id, outbound);
            }
        }
        if let Some(target_event_id) = edit_of {
            if !self.config.bridge.bridge_matrix_edit {
                return Ok(());
            }
            outbound = format!("(edit {})\n{}", target_event_id, outbound);
        }
        outbound = self.truncate_for_policy(&outbound);
        if outbound.trim().is_empty() {
            println!("[DEBUG] Outbound message empty after truncation, skipping");
            return Ok(());
        }

        println!("[DEBUG] Sending Matrix message to DingTalk: conversation_id={}, outbound={}", mapping.dingtalk_conversation_id, outbound);

        self.dingtalk_service
            .send_text_to_conversation(
                Some(&mapping.dingtalk_conversation_id),
                &outbound,
                None,
                None,
                false,
            )
            .await
            .with_context(|| {
                format!(
                    "send to dingtalk failed for conversation {}",
                    mapping.dingtalk_conversation_id
                )
            })?;

        println!("[DEBUG] Successfully sent message to DingTalk: conversation_id={}", mapping.dingtalk_conversation_id);

        if !event_id.is_empty() {
            let message_mapping = MessageMapping::new(
                event_id.clone(),
                format!("matrix:{}", event_id),
                room_id,
                sender,
                mapping.dingtalk_conversation_id.clone(),
            )
            .with_content_hash(Some(format!(
                "{}:{}:{}:{}:{}",
                mapping.dingtalk_conversation_id,
                msgtype,
                outbound,
                reply_to.unwrap_or_default(),
                edit_of.unwrap_or_default()
            )));
            let _ = self.save_message_mapping(&message_mapping).await;
        }

        Ok(())
    }

    async fn handle_member_event(
        &self,
        event: &MatrixEvent,
        membership: &str,
        sender: &str,
        state_key: Option<&str>,
    ) {
        println!("[DEBUG] handle_member_event called: membership={}, sender={}, state_key={:?}", membership, sender, state_key);

        if membership != "invite" {
            println!("[DEBUG] Not an invite event, skipping");
            return;
        }

        let room_id = event.room_id.as_deref().unwrap_or_default();
        if room_id.is_empty() {
            println!("[DEBUG] Room ID is empty, skipping");
            return;
        }

        println!("[DEBUG] Processing invite for room: {}", room_id);

        let state_key = state_key.unwrap_or_default();
        println!("[DEBUG] State key (invitee): {}", state_key);
        println!("[DEBUG] Bot MXID: {}", self.bot_user_id);

        if !self.is_bot_mxid(state_key) {
            println!("[DEBUG] Invite is not for the bot, skipping");
            return;
        }

        println!("[DEBUG] Invite is for bot, attempting to join room: {}", room_id);

        match self.bot_intent.join_room(room_id).await {
            Ok(joined_room_id) => {
                println!("[DEBUG] Successfully joined room: {}", joined_room_id);
                info!(
                    sender = %sender,
                    room_id = %room_id,
                    joined_room_id = %joined_room_id,
                    "Auto-joined Matrix room after invite"
                );
            },
            Err(err) => {
                println!("[DEBUG] Failed to join room: {}", err);
                warn!(
                    sender = %sender,
                    room_id = %room_id,
                    error = %err,
                    "Failed to auto-join Matrix room after invite"
                );
            }
        }
    }

    async fn handle_redaction_event(&self, redacts: Option<&str>) -> anyhow::Result<()> {
        if !self.config.bridge.bridge_matrix_redactions {
            return Ok(());
        }

        let Some(redacts) = redacts else {
            return Ok(());
        };

        if let Err(err) = self.message_store.delete_message_mapping(redacts).await {
            warn!(
                matrix_event_id = %redacts,
                error = %err,
                "Failed to delete message mapping for redacted Matrix event"
            );
        }

        Ok(())
    }

    fn is_blocked_msgtype(&self, msgtype: &str) -> bool {
        self.config
            .bridge
            .blocked_matrix_msgtypes
            .iter()
            .any(|blocked| blocked.eq_ignore_ascii_case(msgtype))
    }

    fn truncate_for_policy(&self, content: &str) -> String {
        let max = self.config.bridge.max_text_length;
        if max == 0 {
            return content.to_string();
        }

        let char_count = content.chars().count();
        if char_count <= max {
            return content.to_string();
        }

        let mut truncated: String = content.chars().take(max).collect();
        truncated.push_str(" …");
        truncated
    }

    fn is_bot_mxid(&self, mxid: &str) -> bool {
        println!("[DEBUG] is_bot_mxid check: mxid={}, bot_user_id={}", mxid, self.bot_user_id);

        if mxid.eq_ignore_ascii_case(&self.bot_user_id) {
            println!("[DEBUG] MXID matches bot_user_id, returning true");
            return true;
        }

        let configured_bot = format!(
            "@{}:{}",
            self.config.bridge.bot_username, self.config.bridge.domain
        );
        println!("[DEBUG] Configured bot MXID: {}", configured_bot);

        let result = mxid.eq_ignore_ascii_case(&configured_bot);
        println!("[DEBUG] MXID match result: {}", result);
        result
    }

    fn is_bridge_bot_sender(&self, sender: &str) -> bool {
        if self.is_bot_mxid(sender) {
            return true;
        }

        let sender = sender.trim();
        let Some(stripped) = sender.strip_prefix('@') else {
            return false;
        };
        let Some((localpart, domain)) = stripped.rsplit_once(':') else {
            return false;
        };
        if !domain.eq_ignore_ascii_case(&self.config.bridge.domain) {
            return false;
        }

        let Some((prefix, suffix)) = self
            .config
            .bridge
            .username_template
            .split_once("{{.}}")
            .or_else(|| self.config.bridge.username_template.split_once("{user_id}"))
        else {
            return false;
        };
        localpart.starts_with(prefix)
            && localpart.ends_with(suffix)
            && localpart.len() > prefix.len() + suffix.len()
    }

    pub async fn forward_dingtalk_text(
        &self,
        conversation_id: &str,
        sender_id: &str,
        content: &str,
        dingtalk_msg_id: Option<&str>,
    ) -> anyhow::Result<String> {
        println!("[DEBUG] Forwarding DingTalk text: conversation_id={}, sender_id={}, content={}, msg_id={:?}",
            conversation_id, sender_id, content, dingtalk_msg_id);

        let Some(mapping) = self.get_room_mapping_by_dingtalk(conversation_id).await? else {
            println!("[DEBUG] No Matrix mapping found for DingTalk conversation: {}", conversation_id);
            anyhow::bail!(
                "dingtalk conversation '{}' has no matrix mapping",
                conversation_id
            );
        };

        println!("[DEBUG] Found Matrix room mapping: {} -> {}", conversation_id, mapping.matrix_room_id);

        if let Some(msg_id) = dingtalk_msg_id {
            let dedupe_event_id = format!("dingtalk:{}", msg_id);
            if self.is_event_processed(&dedupe_event_id).await? {
                println!("[DEBUG] DingTalk message already processed, skipping: {}", msg_id);
                return Ok(String::new());
            }
        }

        let matrix_body = self.dingtalk_formatter.format_text(content, sender_id);
        let matrix_event_id = self
            .bot_intent
            .send_text(&mapping.matrix_room_id, &matrix_body)
            .await
            .with_context(|| {
                format!("send text to matrix room {} failed", mapping.matrix_room_id)
            })?;

        let dingtalk_message_id = dingtalk_msg_id
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| format!("dingtalk:{}:{}", conversation_id, matrix_event_id));

        let message_mapping = MessageMapping::new(
            matrix_event_id.clone(),
            dingtalk_message_id.clone(),
            mapping.matrix_room_id.clone(),
            self.bot_user_id.clone(),
            sender_id.to_string(),
        )
        .with_content_hash(Some(format!("{}:{}", sender_id, content)));
        let _ = self.save_message_mapping(&message_mapping).await;

        if let Some(msg_id) = dingtalk_msg_id {
            self.mark_event_processed(&format!("dingtalk:{}", msg_id), "dingtalk.text", "dingtalk")
                .await?;
        }

        Ok(matrix_event_id)
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

    pub async fn list_room_mappings(
        &self,
        limit: i64,
        offset: i64,
    ) -> anyhow::Result<Vec<RoomMapping>> {
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
                println!("[DEBUG] Replaying dead-letter: id={}, conversation={:?}, content={}", id, conversation, content);
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
        let room_type = if mapping
            .dingtalk_conversation_type
            .eq_ignore_ascii_case("direct")
        {
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

struct RoomRateLimiter {
    limit: usize,
    window: Duration,
    events_by_room: Mutex<HashMap<String, VecDeque<Instant>>>,
}

impl RoomRateLimiter {
    fn new(limit: u32, window_millis: u64) -> Self {
        Self {
            limit: limit as usize,
            window: Duration::from_millis(window_millis),
            events_by_room: Mutex::new(HashMap::new()),
        }
    }

    fn allow(&self, room_id: &str) -> bool {
        if self.limit == 0 || self.window.is_zero() {
            return true;
        }

        let now = Instant::now();
        let mut guard = self
            .events_by_room
            .lock()
            .expect("room rate limiter mutex poisoned");
        let queue = guard.entry(room_id.to_string()).or_default();

        while let Some(timestamp) = queue.front() {
            if now.duration_since(*timestamp) > self.window {
                queue.pop_front();
            } else {
                break;
            }
        }

        if queue.len() >= self.limit {
            return false;
        }

        queue.push_back(now);
        true
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
        self.bridge.handle_matrix_transaction(body).await
    }

    async fn query_user(&self, user_id: &str) -> anyhow::Result<Option<Value>> {
        let localpart = user_id
            .strip_prefix('@')
            .and_then(|value| value.split(':').next())
            .unwrap_or(user_id);
        let prefix = self
            .bridge
            .config
            .bridge
            .username_template
            .replace("{{.}}", "");
        if localpart.starts_with(&prefix) {
            return Ok(Some(serde_json::json!({
                "displayname": localpart,
            })));
        }
        Ok(None)
    }

    async fn query_room_alias(&self, room_alias: &str) -> anyhow::Result<Option<Value>> {
        let localpart = room_alias
            .strip_prefix('#')
            .and_then(|value| value.split(':').next())
            .unwrap_or(room_alias);
        if localpart.starts_with("dingtalk_") {
            return Ok(Some(serde_json::json!({
                "name": format!("DingTalk {}", localpart),
                "topic": "Bridged from DingTalk",
                "preset": "private_chat",
                "visibility": "private",
            })));
        }
        Ok(None)
    }

    async fn thirdparty_protocol(&self, _protocol: &str) -> anyhow::Result<Option<Value>> {
        Ok(Some(serde_json::json!({
            "user_fields": ["id", "name"],
            "location_fields": ["id", "name"],
            "icon": "mxc://example.org/dingtalk",
            "field_types": {
                "id": {
                    "regexp": ".*",
                    "placeholder": "DingTalk ID"
                },
                "name": {
                    "regexp": ".*",
                    "placeholder": "Display name"
                }
            },
            "instances": [{
                "network_id": "dingtalk",
                "bot_user_id": self.bridge.bot_user_id.clone(),
                "desc": "DingTalk",
                "icon": "mxc://example.org/dingtalk",
                "fields": {}
            }]
        })))
    }
}
