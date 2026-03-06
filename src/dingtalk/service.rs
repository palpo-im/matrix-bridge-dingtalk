use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use dingtalk_sdk::stream::{
    DataFrame, DataFrameResponse, DingTalkStreamClient, EVENT_HEADER_TYPE,
    TOPIC_BOT_MESSAGE_CALLBACK, event_success_response,
};
use serde_json::Value;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use super::client::DingTalkClient;
use super::types::*;
use crate::bridge::DingTalkBridge;
use crate::config::StreamConfig;

#[derive(Clone)]
pub struct DingTalkService {
    default_client: DingTalkClient,
    conversation_clients: Arc<RwLock<HashMap<String, DingTalkClient>>>,
    default_webhook_url: String,
    default_secret: Option<String>,
    callback_token: Option<String>,
    stream_config: StreamConfig,
    bridge: Arc<RwLock<Option<Arc<DingTalkBridge>>>>,
}

impl DingTalkService {
    pub fn new(
        webhook_url: String,
        access_token: String,
        secret: Option<String>,
        callback_token: Option<String>,
        stream_config: StreamConfig,
        webhook_tokens: HashMap<String, String>,
    ) -> Self {
        let default_client = DingTalkClient::new(webhook_url.clone(), access_token, secret.clone());
        let mut conversation_clients = HashMap::new();
        for (conversation_id, webhook_value) in webhook_tokens {
            let client =
                if webhook_value.starts_with("https://") || webhook_value.starts_with("http://") {
                    DingTalkClient::from_webhook_url(webhook_value, secret.clone())
                } else {
                    DingTalkClient::new(webhook_url.clone(), webhook_value, secret.clone())
                };
            conversation_clients.insert(conversation_id, client);
        }

        Self {
            default_client,
            conversation_clients: Arc::new(RwLock::new(conversation_clients)),
            default_webhook_url: webhook_url,
            default_secret: secret,
            callback_token,
            stream_config,
            bridge: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn set_bridge(&self, bridge: Arc<DingTalkBridge>) {
        let mut guard = self.bridge.write().await;
        *guard = Some(bridge);
    }

    pub async fn start(&self, bridge: Arc<DingTalkBridge>) -> Result<()> {
        self.set_bridge(bridge).await;
        if !self.stream_config.stream.enabled {
            println!("[DEBUG] ==================================");
            println!("[DEBUG] DingTalk stream mode DISABLED");
            println!("[DEBUG] Service is in callback compatibility mode only");
            println!("[DEBUG] ==================================");
            info!("DingTalk stream mode disabled; service stays in callback compatibility mode");
            return Ok(());
        }

        println!("[DEBUG] ==================================");
        println!("[DEBUG] DingTalk stream mode ENABLED");
        println!("[DEBUG] Starting stream client...");
        println!("[DEBUG] ==================================");
        self.start_stream_mode().await?;
        Ok(())
    }

    pub async fn stop(&self) {
        info!("DingTalk service stopped");
    }

    pub fn client(&self) -> &DingTalkClient {
        &self.default_client
    }

    pub fn validate_callback_token(&self, token: Option<&str>) -> bool {
        match (&self.callback_token, token) {
            (Some(expected), Some(provided)) => expected == provided,
            (Some(_), None) => false,
            (None, _) => true,
        }
    }

    pub async fn handle_callback(&self, body: &str) -> Result<Value> {
        println!("[DEBUG] ==================================");
        println!("[DEBUG] HANDLE_CALLBACK invoked");
        println!("[DEBUG]   payload_length: {} bytes", body.len());
        eprintln!("[DEBUG] HANDLE_CALLBACK invoked - payload_length: {} bytes", body.len());
        
        // Try to parse as JSON to show structure
        if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(body) {
            println!("[DEBUG] Payload JSON keys: {:?}", json_val.as_object().map(|obj| obj.keys().collect::<Vec<_>>()));
            if let Ok(pretty) = serde_json::to_string_pretty(&json_val) {
                println!("[DEBUG] Payload structure:\n{}", pretty);
            }
        }

        let event: DingTalkWebhookMessage = serde_json::from_str(body)
            .map_err(|e| {
                println!("[DEBUG] ERROR parsing callback payload");
                println!("[DEBUG]   error: {}", e);
                println!("[DEBUG]   raw_payload: {}", body);
                eprintln!("[DEBUG] Failed to parse callback: {}", e);
                anyhow::anyhow!("Failed to parse callback: {}", e)
            })?;

        println!("[DEBUG]   Parsed callback payload successfully");
        self.process_event(event).await?;
        println!("[DEBUG] HANDLE_CALLBACK completed successfully");
        println!("[DEBUG] ==================================");

        Ok(serde_json::json!({"errcode": 0, "errmsg": "success"}))
    }

    async fn start_stream_mode(&self) -> Result<()> {
        println!("[DEBUG] ==================================");
        println!("[DEBUG] start_stream_mode() initializing...");
        
        let client_id = self.stream_config.client_id.trim().to_string();
        let client_secret = self.stream_config.client_secret.trim().to_string();
        if client_id.is_empty() || client_secret.is_empty() {
            println!("[DEBUG] ERROR: client_id or client_secret is empty!");
            anyhow::bail!("stream mode enabled but stream.client_id/stream.client_secret is empty");
        }

        println!("[DEBUG] Creating DingTalkStreamClient...");
        let mut stream_client =
            DingTalkStreamClient::new(client_id, client_secret).map_err(|err| {
                println!("[DEBUG] ERROR creating stream client: {}", err);
                anyhow::anyhow!("failed to initialize dingtalk stream client: {}", err)
            })?;
        
        println!("[DEBUG] Setting stream client configuration...");
        {
            let cfg = stream_client.config_mut();
            cfg.openapi_host = self.stream_config.openapi_host.clone();
            cfg.keep_alive_idle =
                Duration::from_secs(self.stream_config.keep_alive_idle_secs.max(1));
            cfg.auto_reconnect = self.stream_config.auto_reconnect;
            cfg.reconnect_interval =
                Duration::from_secs(self.stream_config.reconnect_interval_secs.max(1));
            cfg.local_ip = self.stream_config.local_ip.clone();
        }
        println!("[DEBUG]   openapi_host: {}", self.stream_config.openapi_host);
        println!("[DEBUG]   keep_alive_idle_secs: {}", self.stream_config.keep_alive_idle_secs);
        println!("[DEBUG]   auto_reconnect: {}", self.stream_config.auto_reconnect);
        println!("[DEBUG]   reconnect_interval_secs: {}", self.stream_config.reconnect_interval_secs);

        println!("[DEBUG] Registering bot message callback handler...");
        let service = self.clone();
        stream_client.register_callback_handler(TOPIC_BOT_MESSAGE_CALLBACK, move |frame| {
            let service = service.clone();
            async move { service.handle_stream_bot_message(frame).await }
        });
        println!("[DEBUG] Bot message callback handler registered for topic: {}", TOPIC_BOT_MESSAGE_CALLBACK);

        println!("[DEBUG] Registering all event handler...");
        stream_client.register_all_event_handler(|frame| async move {
            let event_type = frame.header(EVENT_HEADER_TYPE).unwrap_or("unknown");
            debug!(
                topic = frame.topic().unwrap_or("unknown"),
                event_type, "Ignoring unsupported DingTalk EVENT frame"
            );
            Ok(Some(event_success_response()?))
        });

        info!(
            openapi_host = %self.stream_config.openapi_host,
            reconnect = self.stream_config.auto_reconnect,
            keep_alive_idle_secs = self.stream_config.keep_alive_idle_secs,
            reconnect_interval_secs = self.stream_config.reconnect_interval_secs,
            "Starting DingTalk stream client"
        );

        println!("[DEBUG] Starting DingTalk stream client...");
        println!("[DEBUG] ==================================");
        match stream_client.start().await {
            Ok(()) => {
                println!("[DEBUG] Stream client exited normally");
                Ok(())
            }
            Err(err) => {
                println!("[DEBUG] ERROR: stream client failed with error");
                println!("[DEBUG]   error: {}", err);
                println!("[DEBUG]   error_to_string: {}", err.to_string());
                eprintln!("[DEBUG] Stream client error: {}", err);
                
                // Check if this is a JSON parse error for 'time' field
                let err_str = err.to_string();
                if err_str.contains("missing field") && err_str.contains("time") {
                    eprintln!("[DEBUG] This is a DingTalk message parse error (missing 'time' field)");
                    eprintln!("[DEBUG] This may be a DingTalk SDK compatibility issue with some message types");
                    eprintln!("[DEBUG] The stream client will attempt to reconnect...");
                }
                
                Err(anyhow::anyhow!("dingtalk stream client stopped: {}", err))
            }
        }
    }

    async fn handle_stream_bot_message(
        &self,
        frame: DataFrame,
    ) -> dingtalk_sdk::Result<Option<DataFrameResponse>> {
        println!("[DEBUG] ==================================");
        println!("[DEBUG] DingTalk stream message received");
        println!("[DEBUG]   message_id: {:?}", frame.message_id());
        println!("[DEBUG]   topic: {:?}", frame.topic());
        println!("[DEBUG]   data_length: {} bytes", frame.data.len());

        let payload_result = serde_json::from_str::<Value>(&frame.data);
        let payload = match payload_result {
            Ok(payload) => {
                println!("[DEBUG] DingTalk stream payload parsed successfully");
                if let Some(obj) = payload.as_object() {
                    let keys: Vec<_> = obj.keys().collect();
                    println!("[DEBUG] Payload keys: {:?}", keys);
                }
                if let Ok(payload_str) = serde_json::to_string_pretty(&payload) {
                    println!("[DEBUG] Payload:\n{}", payload_str);
                }
                payload
            },
            Err(err) => {
                error!(
                    message_id = frame.message_id().unwrap_or_default(),
                    error = %err,
                    "Failed to parse DingTalk stream callback payload"
                );
                if let Some(bridge) = self.bridge.read().await.as_ref().cloned() {
                    let dedupe_key = format!(
                        "dingtalk:stream:{}",
                        frame.message_id().unwrap_or("unknown")
                    );
                    let _ = bridge
                        .record_dead_letter(
                            "dingtalk",
                            "stream_parse_error",
                            &dedupe_key,
                            None,
                            serde_json::json!({
                                "topic": frame.topic(),
                                "raw_data": frame.data,
                            }),
                            &err.to_string(),
                        )
                        .await;
                }
                return Ok(Some(DataFrameResponse::success()));
            }
        };

        let event: Result<DingTalkWebhookMessage> = serde_json::from_value(payload.clone())
            .map_err(|err| {
                println!("[DEBUG] ==================================");
                println!("[DEBUG] ERROR deserializing DingTalkWebhookMessage");
                println!("[DEBUG]   error: {}", err);
                println!("[DEBUG]   payload keys: {:?}", payload.as_object().map(|obj| obj.keys().collect::<Vec<_>>()));
                println!("[DEBUG]   full payload: {}", serde_json::to_string_pretty(&payload).unwrap_or_default());
                eprintln!("[DEBUG] Failed to deserialize: {}", err);
                anyhow::anyhow!(
                    "parse stream payload as DingTalkWebhookMessage failed: {}",
                    err
                )
            });

        match event {
            Ok(event) => {
                let conversation_id = event.effective_conversation_id().map(str::to_string);
                let msg_id = event.msg_id.clone();
                let dedupe_key = format!(
                    "dingtalk:stream:{}:{}",
                    msg_id.as_deref().unwrap_or("unknown"),
                    conversation_id.as_deref().unwrap_or("unknown")
                );
                println!("[DEBUG] DingTalk stream event deserialized successfully");
                println!("[DEBUG]   msgtype: {:?}", event.msgtype);
                println!("[DEBUG]   msg_id: {:?}", msg_id);
                println!("[DEBUG]   conversation_id: {:?}", conversation_id);
                println!("[DEBUG]   sender_id: {:?}", event.effective_sender_id());
                println!("[DEBUG]   sender_nick: {:?}", event.sender_nick);
                println!("[DEBUG]   conversation_type: {:?}", event.conversation_type);
                println!("[DEBUG]   conversation_title: {:?}", event.conversation_title);
                println!("[DEBUG]   create_time: {:?}", event.create_time);
                if let Err(err) = self.process_event(event).await {
                    error!(
                        message_id = frame.message_id().unwrap_or_default(),
                        conversation_id = conversation_id.as_deref().unwrap_or("unknown"),
                        error = %err,
                        "Failed to process DingTalk stream callback payload"
                    );
                    if let Some(bridge) = self.bridge.read().await.as_ref().cloned() {
                        let _ = bridge
                            .record_dead_letter(
                                "dingtalk",
                                "stream_callback_text",
                                &dedupe_key,
                                conversation_id,
                                payload,
                                &err.to_string(),
                            )
                            .await;
                    }
                }
            }
            Err(err) => {
                println!("[DEBUG] Failed to deserialize DingTalk stream payload");
                println!("[DEBUG]   error: {}", err);
                
                // Log available fields for debugging
                if let Some(obj) = payload.as_object() {
                    println!("[DEBUG]   available_fields: {:?}", obj.keys().collect::<Vec<_>>());
                    // Check if 'time' or its aliases exist
                    let has_time_field = obj.contains_key("time");
                    let has_create_time = obj.contains_key("createTime");
                    let has_create_at = obj.contains_key("createAt");
                    println!("[DEBUG]   has 'time' field: {}", has_time_field);
                    println!("[DEBUG]   has 'createTime' field: {}", has_create_time);
                    println!("[DEBUG]   has 'createAt' field: {}", has_create_at);
                }
                
                error!(
                    message_id = frame.message_id().unwrap_or_default(),
                    error = %err,
                    "Failed to deserialize DingTalk stream callback payload"
                );
                if let Some(bridge) = self.bridge.read().await.as_ref().cloned() {
                    let dedupe_key = format!(
                        "dingtalk:stream_deser:{}",
                        frame.message_id().unwrap_or("unknown")
                    );
                    let _ = bridge
                        .record_dead_letter(
                            "dingtalk",
                            "stream_deserialize_error",
                            &dedupe_key,
                            None,
                            payload,
                            &err.to_string(),
                        )
                        .await;
                }
            }
        }

        println!("[DEBUG] ==================================");
        // Always return success to keep stream alive even if processing failed
        Ok(Some(DataFrameResponse::success()))
    }

    async fn process_event(&self, event: DingTalkWebhookMessage) -> Result<()> {
        println!("[DEBUG] ==================================");
        println!("[DEBUG] Processing DingTalk event");
        println!("[DEBUG]   msgtype: {:?}", event.msgtype);
        println!("[DEBUG]   conversation_id: {:?}", event.conversation_id);
        println!("[DEBUG]   sender_id: {:?}", event.sender_id);
        println!("[DEBUG]   sender_nick: {:?}", event.sender_nick);
        println!("[DEBUG]   msg_id: {:?}", event.msg_id);
        println!("[DEBUG]   conversation_type: {:?}", event.conversation_type);
        println!("[DEBUG]   conversation_title: {:?}", event.conversation_title);
        println!("[DEBUG]   create_time: {:?}", event.create_time);
        println!("[DEBUG]   chatbot_user_id: {:?}", event.chatbot_user_id);

        let guard = self.bridge.read().await;
        let bridge = guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Bridge not initialized"))?;

        let msgtype = event.msgtype.as_deref().unwrap_or("unknown");
        let msgtype_normalized = msgtype.to_ascii_lowercase();
        let conversation_id = event.effective_conversation_id().unwrap_or("unknown");

        println!("[DEBUG] Event type normalized: {}", msgtype_normalized);

        if let (Some(conversation_id), Some(session_webhook)) = (
            event.effective_conversation_id(),
            event.session_webhook.as_deref(),
        ) {
            self.register_conversation_webhook(conversation_id, session_webhook)
                .await;
            if let Err(err) = bridge
                .cache_conversation_webhook(
                    conversation_id,
                    session_webhook,
                    event.session_webhook_expired_time,
                )
                .await
            {
                warn!(
                    "Failed to cache session webhook for conversation {}: {}",
                    conversation_id, err
                );
            }
        }

        info!(
            "Processing DingTalk event: type={}, conversation={}",
            msgtype, conversation_id
        );

        match msgtype_normalized.as_str() {
            "text" => {
                if let Some(content) = event.effective_text_content() {
                    self.handle_text_message(bridge, &content, &event).await?;
                } else {
                    warn!(
                        "DingTalk text event does not contain textual content: conversation={} msg_id={:?}",
                        conversation_id, event.msg_id
                    );
                }
            }
            "group_add" => {
                self.handle_group_add_event(bridge, &event).await?;
            }
            _ => {
                warn!("Unsupported message type: {}", msgtype);
            }
        }

        Ok(())
    }

    async fn handle_group_add_event(
        &self,
        _bridge: &Arc<DingTalkBridge>,
        event: &DingTalkWebhookMessage,
    ) -> Result<()> {
        let conversation_id = event.effective_conversation_id().unwrap_or("unknown");
        let conversation_name = event.conversation_title.as_deref().unwrap_or("Group");

        info!(
            "Bot added to DingTalk group: conversation_id={}, name={}",
            conversation_id, conversation_name
        );

        // Prepare welcome message with openConversationId
        let message = format!(
            "✅ Bot successfully joined the group!\n\nGroup Information:\n- Name: {}\n- OpenConversationId: {}\n\nYou can now use provisioning commands to bridge this group to Matrix.",
            conversation_name, conversation_id
        );

        println!("[DEBUG] Sending group-add message to DingTalk group: {}", conversation_id);

        match self
            .send_text_to_conversation(
                Some(conversation_id),
                &message,
                None,
                None,
                false,
            )
            .await
        {
            Ok(response) => {
                if response.is_success() {
                    info!(
                        "Successfully sent group-add message to group: {}",
                        conversation_id
                    );
                    println!("[DEBUG] Group-add message sent successfully to: {}", conversation_id);
                    Ok(())
                } else {
                    let error_msg = format!(
                        "Failed to send group-add message: {} ({})",
                        response.errmsg, response.errcode
                    );
                    warn!("{}", error_msg);
                    Err(anyhow::anyhow!(error_msg))
                }
            }
            Err(err) => {
                warn!(
                    "Error sending group-add message to {}: {}",
                    conversation_id, err
                );
                Err(err)
            }
        }
    }

    pub async fn register_conversation_webhook(&self, conversation_id: &str, webhook_value: &str) {
        let webhook_value = webhook_value.trim();
        if webhook_value.is_empty() {
            return;
        }

        let client = if webhook_value.starts_with("https://")
            || webhook_value.starts_with("http://")
        {
            DingTalkClient::from_webhook_url(webhook_value.to_string(), self.default_secret.clone())
        } else {
            DingTalkClient::new(
                self.default_webhook_url.clone(),
                webhook_value.to_string(),
                self.default_secret.clone(),
            )
        };

        let mut guard = self.conversation_clients.write().await;
        guard.insert(conversation_id.to_string(), client);
    }

    async fn handle_text_message(
        &self,
        bridge: &Arc<DingTalkBridge>,
        content: &str,
        event: &DingTalkWebhookMessage,
    ) -> Result<()> {
        println!("[DEBUG] ==================================");
        println!("[DEBUG] Handling DingTalk TEXT message");
        println!("[DEBUG]   msg_id: {:?}", event.msg_id);
        println!("[DEBUG]   sender_id: {}", event.effective_sender_id().unwrap_or("unknown"));
        println!("[DEBUG]   sender_nick: {:?}", event.sender_nick);
        println!("[DEBUG]   conversation_id: {}", event.effective_conversation_id().unwrap_or("unknown"));
        println!("[DEBUG]   conversation_title: {:?}", event.conversation_title);
        println!("[DEBUG]   conversation_type: {:?}", event.conversation_type);
        println!("[DEBUG]   create_time: {:?}", event.create_time);
        println!("[DEBUG]   content_length: {} chars", content.len());
        println!("[DEBUG]   content: {}", content);

        let sender_id = event.effective_sender_id().unwrap_or("unknown");
        let conversation_id = event.effective_conversation_id().unwrap_or("unknown");

        info!(
            "Text message from {} in {}: {}",
            sender_id, conversation_id, content
        );

        match bridge
            .forward_dingtalk_text(
                conversation_id,
                sender_id,
                event.sender_nick.as_deref(),
                content,
                event.msg_id.as_deref(),
            )
            .await
        {
            Ok(Some(matrix_event_id)) => {
                println!("[DEBUG] Successfully forwarded DingTalk message to Matrix");
                println!("[DEBUG]   matrix_event_id: {}", matrix_event_id);
                info!(
                    "Forwarded DingTalk message to Matrix: dingtalk_conversation={} matrix_event_id={}",
                    conversation_id, matrix_event_id
                );
                println!("[DEBUG] ==================================");
                Ok(())
            }
            Ok(None) => {
                println!(
                    "[DEBUG] DingTalk message skipped: no Matrix mapping or duplicate event"
                );
                info!(
                    "Skipped DingTalk message forwarding: dingtalk_conversation={}",
                    conversation_id
                );
                println!("[DEBUG] ==================================");
                Ok(())
            }
            Err(err) => {
                println!("[DEBUG] Failed to forward DingTalk message to Matrix");
                println!("[DEBUG]   error: {}", err);
                let dedupe_key = format!(
                    "dingtalk:{}:{}",
                    event.msg_id.as_deref().unwrap_or("unknown"),
                    conversation_id
                );
                if let Ok(payload) = serde_json::to_value(event) {
                    if let Err(record_err) = bridge
                        .record_dead_letter(
                            "dingtalk",
                            "callback_text",
                            &dedupe_key,
                            Some(conversation_id.to_string()),
                            payload,
                            &err.to_string(),
                        )
                        .await
                    {
                        warn!(
                            "Failed to record DingTalk callback dead-letter: {}",
                            record_err
                        );
                    }
                }
                println!("[DEBUG] ==================================");
                Err(err)
            }
        }
    }

    pub async fn send_text(
        &self,
        content: &str,
        at_mobiles: Option<Vec<String>>,
        at_user_ids: Option<Vec<String>>,
        is_at_all: bool,
    ) -> Result<DingTalkResponse> {
        self.send_text_to_conversation(None, content, at_mobiles, at_user_ids, is_at_all)
            .await
    }

    pub async fn send_text_to_conversation(
        &self,
        conversation_id: Option<&str>,
        content: &str,
        at_mobiles: Option<Vec<String>>,
        at_user_ids: Option<Vec<String>>,
        is_at_all: bool,
    ) -> Result<DingTalkResponse> {
        let client = self.resolve_client(conversation_id).await;
        client
            .send_text(content, at_mobiles, at_user_ids, is_at_all)
            .await
    }

    pub async fn send_markdown(
        &self,
        title: &str,
        text: &str,
        at_mobiles: Option<Vec<String>>,
        at_user_ids: Option<Vec<String>>,
        is_at_all: bool,
    ) -> Result<DingTalkResponse> {
        self.send_markdown_to_conversation(None, title, text, at_mobiles, at_user_ids, is_at_all)
            .await
    }

    pub async fn send_markdown_to_conversation(
        &self,
        conversation_id: Option<&str>,
        title: &str,
        text: &str,
        at_mobiles: Option<Vec<String>>,
        at_user_ids: Option<Vec<String>>,
        is_at_all: bool,
    ) -> Result<DingTalkResponse> {
        let client = self.resolve_client(conversation_id).await;
        client
            .send_markdown(title, text, at_mobiles, at_user_ids, is_at_all)
            .await
    }

    pub async fn send_link(
        &self,
        title: &str,
        text: &str,
        message_url: &str,
        pic_url: Option<&str>,
    ) -> Result<DingTalkResponse> {
        self.send_link_to_conversation(None, title, text, message_url, pic_url)
            .await
    }

    pub async fn send_link_to_conversation(
        &self,
        conversation_id: Option<&str>,
        title: &str,
        text: &str,
        message_url: &str,
        pic_url: Option<&str>,
    ) -> Result<DingTalkResponse> {
        let client = self.resolve_client(conversation_id).await;
        client.send_link(title, text, message_url, pic_url).await
    }

    async fn resolve_client(&self, conversation_id: Option<&str>) -> DingTalkClient {
        if let Some(conversation_id) = conversation_id {
            let guard = self.conversation_clients.read().await;
            if let Some(client) = guard.get(conversation_id) {
                return client.clone();
            }
        }
        self.default_client.clone()
    }
}
