use std::sync::Arc;

use crate::config::Config;
use crate::dingtalk::DingTalkService;

#[derive(Clone)]
pub struct MessageFlow {
    _config: Arc<Config>,
    _dingtalk_service: Arc<DingTalkService>,
}

impl MessageFlow {
    pub fn new(_config: Arc<Config>, dingtalk_service: Arc<DingTalkService>) -> Self {
        Self {
            _config,
            _dingtalk_service: dingtalk_service,
        }
    }

    pub async fn process_matrix_message(
        &self,
        _room_id: &str,
        _event_id: &str,
        _content: &str,
        _sender: &str,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    pub async fn process_dingtalk_message(
        &self,
        _conversation_id: &str,
        _sender_id: &str,
        _content: &str,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct MatrixInboundMessage {
    pub event_id: String,
    pub room_id: String,
    pub sender: String,
    pub content: String,
    pub msg_type: String,
}

#[derive(Debug, Clone)]
pub struct DingTalkInboundMessage {
    pub msg_id: String,
    pub conversation_id: String,
    pub sender_id: String,
    pub content: String,
    pub msg_type: String,
}

#[derive(Debug, Clone)]
pub struct OutboundMatrixMessage {
    pub room_id: String,
    pub content: String,
    pub msg_type: String,
}

#[derive(Debug, Clone)]
pub struct OutboundDingTalkMessage {
    pub webhook_url: String,
    pub content: String,
    pub msg_type: String,
}
