use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use serde_json::Value;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::client::DingTalkClient;
use super::types::*;
use crate::bridge::DingTalkBridge;

#[derive(Clone)]
pub struct DingTalkService {
    default_client: DingTalkClient,
    conversation_clients: Arc<RwLock<HashMap<String, DingTalkClient>>>,
    callback_token: Option<String>,
    bridge: Arc<RwLock<Option<Arc<DingTalkBridge>>>>,
}

impl DingTalkService {
    pub fn new(
        webhook_url: String,
        access_token: String,
        secret: Option<String>,
        callback_token: Option<String>,
        webhook_tokens: HashMap<String, String>,
    ) -> Self {
        let default_client =
            DingTalkClient::new(webhook_url.clone(), access_token, secret.clone());
        let mut conversation_clients = HashMap::new();
        for (conversation_id, webhook_value) in webhook_tokens {
            let client = if webhook_value.starts_with("https://")
                || webhook_value.starts_with("http://")
            {
                DingTalkClient::from_webhook_url(webhook_value, secret.clone())
            } else {
                DingTalkClient::new(webhook_url.clone(), webhook_value, secret.clone())
            };
            conversation_clients.insert(conversation_id, client);
        }

        Self {
            default_client,
            conversation_clients: Arc::new(RwLock::new(conversation_clients)),
            callback_token,
            bridge: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn set_bridge(&self, bridge: Arc<DingTalkBridge>) {
        let mut guard = self.bridge.write().await;
        *guard = Some(bridge);
    }

    pub async fn start(&self, bridge: Arc<DingTalkBridge>) -> Result<()> {
        self.set_bridge(bridge).await;
        info!("DingTalk service started");
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

        let event: DingTalkWebhookMessage =
            serde_json::from_str(body).map_err(|e| anyhow::anyhow!("Failed to parse callback: {}", e))?;

        self.process_event(event).await?;

        Ok(serde_json::json!({"errcode": 0, "errmsg": "success"}))
    }

    async fn process_event(&self, event: DingTalkWebhookMessage) -> Result<()> {
        let guard = self.bridge.read().await;
        let bridge = guard.as_ref().ok_or_else(|| anyhow::anyhow!("Bridge not initialized"))?;

        let msgtype = event.msgtype.as_deref().unwrap_or("unknown");
        let conversation_id = event.conversation_id.as_deref().unwrap_or("unknown");

        info!(
            "Processing DingTalk event: type={}, conversation={}",
            msgtype, conversation_id
        );

        match msgtype {
            "text" => {
                if let Some(text) = &event.text {
                    if let Some(content) = &text.content {
                        self.handle_text_message(bridge, content, &event).await?;
                    }
                }
            }
            _ => {
                warn!("Unsupported message type: {}", msgtype);
            }
        }

        Ok(())
    }

    async fn handle_text_message(
        &self,
        bridge: &Arc<DingTalkBridge>,
        content: &str,
        event: &DingTalkWebhookMessage,
    ) -> Result<()> {
        debug!("Handling text message: {}", content);

        let sender_id = event.sender_id.as_deref().unwrap_or("unknown");
        let conversation_id = event.conversation_id.as_deref().unwrap_or("unknown");

        info!(
            "Text message from {} in {}: {}",
            sender_id, conversation_id, content
        );

        match bridge
            .forward_dingtalk_text(
                conversation_id,
                sender_id,
                content,
                event.msg_id.as_deref(),
            )
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
        self.send_markdown_to_conversation(
            None,
            title,
            text,
            at_mobiles,
            at_user_ids,
            is_at_all,
        )
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
        client
            .send_link(title, text, message_url, pic_url)
            .await
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
