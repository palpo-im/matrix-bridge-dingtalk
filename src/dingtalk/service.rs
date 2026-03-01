use std::sync::Arc;

use anyhow::Result;
use serde_json::Value;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use super::client::DingTalkClient;
use super::types::*;
use crate::bridge::DingTalkBridge;

#[derive(Clone)]
pub struct DingTalkService {
    client: DingTalkClient,
    callback_token: Option<String>,
    bridge: Arc<RwLock<Option<Arc<DingTalkBridge>>>>,
}

impl DingTalkService {
    pub fn new(
        webhook_url: String,
        access_token: String,
        secret: Option<String>,
        callback_token: Option<String>,
    ) -> Self {
        let client = DingTalkClient::new(webhook_url, access_token, secret);

        Self {
            client,
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
        &self.client
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
        _bridge: &Arc<DingTalkBridge>,
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

        Ok(())
    }

    pub async fn send_text(
        &self,
        content: &str,
        at_mobiles: Option<Vec<String>>,
        at_user_ids: Option<Vec<String>>,
        is_at_all: bool,
    ) -> Result<DingTalkResponse> {
        self.client
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
        self.client
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
        self.client
            .send_link(title, text, message_url, pic_url)
            .await
    }
}
