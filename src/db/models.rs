use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomMapping {
    pub id: i64,
    pub matrix_room_id: String,
    pub dingtalk_chat_id: String,
    pub dingtalk_chat_name: String,
    pub dingtalk_conversation_type: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMapping {
    pub id: i64,
    pub matrix_user_id: String,
    pub dingtalk_user_id: String,
    pub dingtalk_username: String,
    pub dingtalk_nick: Option<String>,
    pub dingtalk_avatar: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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
pub struct MessageMapping {
    pub id: i64,
    pub dingtalk_message_id: String,
    pub matrix_room_id: String,
    pub matrix_event_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteRoomInfo {
    pub dingtalk_chat_id: String,
    pub dingtalk_chat_name: Option<String>,
    pub dingtalk_conversation_type: String,
    pub dingtalk_owner_user_id: Option<String>,
    pub plumbed: bool,
    pub update_name: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteUserInfo {
    pub dingtalk_user_id: String,
    pub displayname: Option<String>,
    pub avatar_url: Option<String>,
    pub avatar_mxc: Option<String>,
}
