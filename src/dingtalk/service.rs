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
        if !self.stream_config.enabled {
            info!("DingTalk stream mode disabled; service stays in callback compatibility mode");
            return Ok(());
        }

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
        debug!("Received DingTalk callback: {}", body);

        let event: DingTalkWebhookMessage = serde_json::from_str(body)
            .map_err(|e| anyhow::anyhow!("Failed to parse callback: {}", e))?;

        self.process_event(event).await?;

        Ok(serde_json::json!({"errcode": 0, "errmsg": "success"}))
    }

    async fn start_stream_mode(&self) -> Result<()> {
        let client_id = self.stream_config.client_id.trim().to_string();
        let client_secret = self.stream_config.client_secret.trim().to_string();
        if client_id.is_empty() || client_secret.is_empty() {
            anyhow::bail!("stream mode enabled but stream.client_id/stream.client_secret is empty");
        }

        let mut stream_client =
            DingTalkStreamClient::new(client_id, client_secret).map_err(|err| {
                anyhow::anyhow!("failed to initialize dingtalk stream client: {}", err)
            })?;
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

        let service = self.clone();
        stream_client.register_callback_handler(TOPIC_BOT_MESSAGE_CALLBACK, move |frame| {
            let service = service.clone();
            async move { service.handle_stream_bot_message(frame).await }
        });

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

        stream_client
            .start()
            .await
            .map_err(|err| anyhow::anyhow!("dingtalk stream client stopped: {}", err))
    }

    async fn handle_stream_bot_message(
        &self,
        frame: DataFrame,
    ) -> dingtalk_sdk::Result<Option<DataFrameResponse>> {
        let payload_result = serde_json::from_str::<Value>(&frame.data);
        let payload = match payload_result {
            Ok(payload) => payload,
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

        Ok(Some(DataFrameResponse::success()))
    }

    async fn process_event(&self, event: DingTalkWebhookMessage) -> Result<()> {
        let guard = self.bridge.read().await;
        let bridge = guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Bridge not initialized"))?;

        let msgtype = event.msgtype.as_deref().unwrap_or("unknown");
        let msgtype_normalized = msgtype.to_ascii_lowercase();
        let conversation_id = event.effective_conversation_id().unwrap_or("unknown");

        if let (Some(conversation_id), Some(session_webhook)) = (
            event.effective_conversation_id(),
            event.session_webhook.as_deref(),
        ) {
            self.register_conversation_webhook(conversation_id, session_webhook)
                .await;
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
            _ => {
                warn!("Unsupported message type: {}", msgtype);
            }
        }

        Ok(())
    }

    async fn register_conversation_webhook(&self, conversation_id: &str, webhook_value: &str) {
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
        debug!("Handling text message: {}", content);

        let sender_id = event.effective_sender_id().unwrap_or("unknown");
        let conversation_id = event.effective_conversation_id().unwrap_or("unknown");

        info!(
            "Text message from {} in {}: {}",
            sender_id, conversation_id, content
        );

        match bridge
            .forward_dingtalk_text(conversation_id, sender_id, content, event.msg_id.as_deref())
            .await
        {
            Ok(matrix_event_id) => {
                info!(
                    "Forwarded DingTalk message to Matrix: dingtalk_conversation={} matrix_event_id={}",
                    conversation_id, matrix_event_id
                );
                Ok(())
            }
            Err(err) => {
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
