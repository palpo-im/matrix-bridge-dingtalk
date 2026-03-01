use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomMapping {
    pub id: i64,
    pub matrix_room_id: String,
    pub dingtalk_conversation_id: String,
    pub dingtalk_conversation_name: Option<String>,
    pub dingtalk_conversation_type: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMapping {
    pub id: i64,
    pub matrix_user_id: String,
    pub dingtalk_user_id: String,
    pub dingtalk_username: Option<String>,
    pub dingtalk_avatar: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMapping {
    pub id: i64,
    pub matrix_event_id: String,
    pub dingtalk_message_id: String,
    pub room_id: String,
    pub sender_mxid: String,
    pub sender_dingtalk_id: String,
    pub content_hash: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedEvent {
    pub id: i64,
    pub event_id: String,
    pub event_type: String,
    pub source: String,
    pub processed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadLetterEvent {
    pub id: i64,
    pub source: String,
    pub event_type: String,
    pub dedupe_key: String,
    pub conversation_id: Option<String>,
    pub payload: String,
    pub error: String,
    pub status: String,
    pub replay_count: i64,
    pub last_replayed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaCacheEntry {
    pub id: i64,
    pub content_hash: String,
    pub media_kind: String,
    pub resource_key: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl RoomMapping {
    pub fn new(
        matrix_room_id: String,
        dingtalk_conversation_id: String,
        dingtalk_conversation_name: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: 0,
            matrix_room_id,
            dingtalk_conversation_id,
            dingtalk_conversation_name,
            dingtalk_conversation_type: "group".to_string(),
            created_at: now,
            updated_at: now,
        }
    }
}

impl UserMapping {
    pub fn new(
        matrix_user_id: String,
        dingtalk_user_id: String,
        dingtalk_username: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: 0,
            matrix_user_id,
            dingtalk_user_id,
            dingtalk_username,
            dingtalk_avatar: None,
            created_at: now,
            updated_at: now,
        }
    }
}

impl MessageMapping {
    pub fn new(
        matrix_event_id: String,
        dingtalk_message_id: String,
        room_id: String,
        sender_mxid: String,
        sender_dingtalk_id: String,
    ) -> Self {
        Self {
            id: 0,
            matrix_event_id,
            dingtalk_message_id,
            room_id,
            sender_mxid,
            sender_dingtalk_id,
            content_hash: None,
            created_at: Utc::now(),
        }
    }

    pub fn with_content_hash(mut self, content_hash: Option<String>) -> Self {
        self.content_hash = content_hash;
        self
    }
}
