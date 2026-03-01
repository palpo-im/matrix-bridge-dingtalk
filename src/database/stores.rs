use async_trait::async_trait;

use super::models::{DeadLetterEvent, MediaCacheEntry, MessageMapping, ProcessedEvent, RoomMapping, UserMapping};
use super::DatabaseResult;

#[async_trait]
pub trait RoomStore: Send + Sync {
    async fn get_room_mapping(&self, matrix_room_id: &str) -> DatabaseResult<Option<RoomMapping>>;
    async fn get_room_mapping_by_dingtalk(&self, dingtalk_conversation_id: &str) -> DatabaseResult<Option<RoomMapping>>;
    async fn insert_room_mapping(&self, mapping: &RoomMapping) -> DatabaseResult<RoomMapping>;
    async fn delete_room_mapping(&self, matrix_room_id: &str) -> DatabaseResult<bool>;
    async fn list_room_mappings(&self, limit: i64, offset: i64) -> DatabaseResult<Vec<RoomMapping>>;
}

#[async_trait]
pub trait UserStore: Send + Sync {
    async fn get_user_mapping(&self, matrix_user_id: &str) -> DatabaseResult<Option<UserMapping>>;
    async fn get_user_mapping_by_dingtalk(&self, dingtalk_user_id: &str) -> DatabaseResult<Option<UserMapping>>;
    async fn insert_user_mapping(&self, mapping: &UserMapping) -> DatabaseResult<UserMapping>;
    async fn update_user_mapping(&self, mapping: &UserMapping) -> DatabaseResult<bool>;
    async fn delete_user_mapping(&self, matrix_user_id: &str) -> DatabaseResult<bool>;
}

#[async_trait]
pub trait MessageStore: Send + Sync {
    async fn get_message_mapping(&self, matrix_event_id: &str) -> DatabaseResult<Option<MessageMapping>>;
    async fn get_message_mapping_by_dingtalk(&self, dingtalk_message_id: &str) -> DatabaseResult<Option<MessageMapping>>;
    async fn insert_message_mapping(&self, mapping: &MessageMapping) -> DatabaseResult<MessageMapping>;
    async fn delete_message_mapping(&self, matrix_event_id: &str) -> DatabaseResult<bool>;
}

#[async_trait]
pub trait EventStore: Send + Sync {
    async fn is_event_processed(&self, event_id: &str) -> DatabaseResult<bool>;
    async fn mark_event_processed(&self, event: &ProcessedEvent) -> DatabaseResult<()>;
    async fn cleanup_old_events(&self, before: chrono::DateTime<chrono::Utc>) -> DatabaseResult<u64>;
}

#[async_trait]
pub trait DeadLetterStore: Send + Sync {
    async fn insert_dead_letter(&self, event: &DeadLetterEvent) -> DatabaseResult<DeadLetterEvent>;
    async fn get_dead_letter(&self, id: i64) -> DatabaseResult<Option<DeadLetterEvent>>;
    async fn count_dead_letters(&self, status: Option<&str>) -> DatabaseResult<i64>;
    async fn list_dead_letters(&self, status: Option<&str>, limit: i64) -> DatabaseResult<Vec<DeadLetterEvent>>;
    async fn update_dead_letter_status(&self, id: i64, status: &str) -> DatabaseResult<bool>;
    async fn delete_dead_letter(&self, id: i64) -> DatabaseResult<bool>;
    async fn cleanup_dead_letters(&self, status: Option<&str>, older_than_hours: Option<i64>, limit: i64) -> DatabaseResult<u64>;
}

#[async_trait]
pub trait MediaStore: Send + Sync {
    async fn get_media_cache(&self, content_hash: &str, media_kind: &str) -> DatabaseResult<Option<MediaCacheEntry>>;
    async fn insert_media_cache(&self, entry: &MediaCacheEntry) -> DatabaseResult<MediaCacheEntry>;
    async fn cleanup_old_media_cache(&self, before: chrono::DateTime<chrono::Utc>) -> DatabaseResult<u64>;
}
