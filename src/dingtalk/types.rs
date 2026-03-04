use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    #[serde(
        alias = "senderId",
        alias = "senderStaffId",
        alias = "senderUserId",
        alias = "staffId",
        alias = "userId"
    )]
    pub sender_id: Option<String>,
    #[serde(default)]
    #[serde(alias = "conversationId", alias = "openConversationId")]
    pub conversation_id: Option<String>,
    #[serde(default)]
    #[serde(alias = "createTime", alias = "createAt", alias = "time")]
    pub create_time: Option<i64>,
    #[serde(default)]
    #[serde(alias = "msgId", alias = "messageId")]
    pub msg_id: Option<String>,
    #[serde(default)]
    #[serde(alias = "sessionWebhook")]
    pub session_webhook: Option<String>,
    #[serde(default)]
    #[serde(alias = "chatbotUserId")]
    pub chatbot_user_id: Option<String>,
    #[serde(default)]
    #[serde(alias = "senderNick")]
    pub sender_nick: Option<String>,
    #[serde(default)]
    #[serde(alias = "conversationType")]
    pub conversation_type: Option<String>,
    #[serde(default)]
    #[serde(alias = "conversationTitle")]
    pub conversation_title: Option<String>,
    #[serde(default)]
    #[serde(alias = "sessionWebhookExpiredTime")]
    pub session_webhook_expired_time: Option<i64>,
    #[serde(default)]
    #[serde(alias = "content")]
    pub raw_content: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DingTalkWebhookText {
    #[serde(default)]
    pub content: Option<String>,
}

impl DingTalkWebhookMessage {
    pub fn effective_sender_id(&self) -> Option<&str> {
        self.sender_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }

    pub fn effective_conversation_id(&self) -> Option<&str> {
        self.conversation_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }

    pub fn effective_text_content(&self) -> Option<String> {
        if let Some(content) = self
            .text
            .as_ref()
            .and_then(|text| text.content.as_deref())
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return Some(content.to_string());
        }

        let raw = self.raw_content.as_ref()?;
        match raw {
            Value::String(value) => {
                let trimmed = value.trim();
                (!trimmed.is_empty()).then(|| trimmed.to_string())
            }
            Value::Object(map) => map
                .get("content")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned),
            _ => None,
        }
    }
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

#[cfg(test)]
mod tests {
    use super::DingTalkWebhookMessage;

    #[test]
    fn parse_callback_alias_fields() {
        let payload = serde_json::json!({
            "msgtype": "text",
            "text": { "content": "hello from dingtalk" },
            "senderId": "user_123",
            "conversationId": "conv_456",
            "msgId": "msg_789",
            "sessionWebhook": "https://example.com/session/webhook"
        });

        let event: DingTalkWebhookMessage =
            serde_json::from_value(payload).expect("callback payload should parse");

        assert_eq!(event.msgtype.as_deref(), Some("text"));
        assert_eq!(
            event.text.as_ref().and_then(|text| text.content.as_deref()),
            Some("hello from dingtalk")
        );
        assert_eq!(event.sender_id.as_deref(), Some("user_123"));
        assert_eq!(event.conversation_id.as_deref(), Some("conv_456"));
        assert_eq!(event.msg_id.as_deref(), Some("msg_789"));
        assert_eq!(
            event.session_webhook.as_deref(),
            Some("https://example.com/session/webhook")
        );
    }

    #[test]
    fn parse_stream_alias_fields() {
        let payload = serde_json::json!({
            "msgtype": "text",
            "text": { "content": "hello from stream" },
            "senderStaffId": "manager_001",
            "conversationId": "cid_stream",
            "msgId": "stream_msg_01",
            "sessionWebhook": "https://example.com/session/webhook",
            "conversationType": "2",
            "conversationTitle": "stream-group"
        });

        let event: DingTalkWebhookMessage =
            serde_json::from_value(payload).expect("stream payload should parse");

        assert_eq!(event.effective_sender_id(), Some("manager_001"));
        assert_eq!(event.effective_conversation_id(), Some("cid_stream"));
        assert_eq!(event.msg_id.as_deref(), Some("stream_msg_01"));
        assert_eq!(
            event.effective_text_content().as_deref(),
            Some("hello from stream")
        );
        assert_eq!(event.conversation_type.as_deref(), Some("2"));
        assert_eq!(event.conversation_title.as_deref(), Some("stream-group"));
    }
}
