use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DingTalkUser {
    pub userid: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub mobile: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DingTalkChat {
    pub chatid: String,
    pub name: String,
    pub owner: String,
    #[serde(default)]
    pub user_userid_list: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DingTalkChatProfile {
    pub chat_id: String,
    pub name: String,
    pub owner_user_id: String,
    #[serde(default)]
    pub user_id_list: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DingTalkMessageType {
    Text,
    Markdown,
    Link,
    ActionCard,
    FeedCard,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DingTalkMessage {
    pub msgtype: DingTalkMessageType,
    pub content: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct DingTalkTextMessage {
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at_mobiles: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at_user_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_at_all: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DingTalkMarkdownMessage {
    pub title: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at_mobiles: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at_user_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_at_all: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DingTalkLinkMessage {
    pub title: String,
    pub text: String,
    pub pic_url: Option<String>,
    pub message_url: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DingTalkActionCardMessage {
    pub title: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub single_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub single_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub btn_orientation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub btn_json_list: Option<Vec<DingTalkActionButton>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DingTalkActionButton {
    pub title: String,
    pub action_url: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DingTalkFeedCardMessage {
    pub links: Vec<DingTalkFeedCardLink>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DingTalkFeedCardLink {
    pub title: String,
    pub message_url: String,
    pub pic_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DingTalkResponse {
    pub errcode: i64,
    pub errmsg: String,
}

impl DingTalkResponse {
    pub fn is_success(&self) -> bool {
        self.errcode == 0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DingTalkWebhookMessage {
    #[serde(default)]
    pub msgtype: Option<String>,
    #[serde(default)]
    pub text: Option<DingTalkWebhookText>,
    #[serde(default)]
    #[serde(alias = "senderId")]
    pub sender_id: Option<String>,
    #[serde(default)]
    #[serde(alias = "conversationId")]
    pub conversation_id: Option<String>,
    #[serde(default)]
    #[serde(alias = "createTime")]
    pub create_time: Option<i64>,
    #[serde(default)]
    #[serde(alias = "msgId")]
    pub msg_id: Option<String>,
    #[serde(default)]
    #[serde(alias = "sessionWebhook")]
    pub session_webhook: Option<String>,
    #[serde(default)]
    #[serde(alias = "chatbotUserId")]
    pub chatbot_user_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DingTalkWebhookText {
    #[serde(default)]
    pub content: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DingTalkEventMessage {
    pub event_type: String,
    pub event_id: String,
    pub timestamp: DateTime<Utc>,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DingTalkSendMessageRequest {
    pub msgtype: String,
    #[serde(flatten)]
    pub content: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DingTalkSendMessageResponse {
    pub errcode: i64,
    pub errmsg: String,
    #[serde(default)]
    pub message_id: Option<String>,
}
