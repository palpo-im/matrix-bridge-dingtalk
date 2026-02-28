use std::sync::Arc;

use crate::db::{DatabaseError, MessageMapping, ProcessedEvent, RoomMapping, UserMapping};

#[async_trait::async_trait]
pub trait RoomStore: Send + Sync {
    async fn create_room_mapping(
        &self,
        mapping: &RoomMapping,
    ) -> Result<RoomMapping, DatabaseError>;
    
    async fn get_room_by_matrix_room(
        &self,
        matrix_room_id: &str,
    ) -> Result<Option<RoomMapping>, DatabaseError>;
    
    async fn get_room_by_dingtalk_chat(
        &self,
        dingtalk_chat_id: &str,
    ) -> Result<Option<RoomMapping>, DatabaseError>;
    
    async fn delete_room_mapping(&self, id: i64) -> Result<(), DatabaseError>;
    
    async fn count_rooms(&self) -> Result<i64, DatabaseError>;
}

#[async_trait::async_trait]
pub trait UserStore: Send + Sync {
    async fn create_user_mapping(
        &self,
        mapping: &UserMapping,
    ) -> Result<UserMapping, DatabaseError>;
    
    async fn get_user_by_matrix_id(
        &self,
        matrix_user_id: &str,
    ) -> Result<Option<UserMapping>, DatabaseError>;
    
    async fn get_user_by_dingtalk_id(
        &self,
        dingtalk_user_id: &str,
    ) -> Result<Option<UserMapping>, DatabaseError>;
    
    async fn update_user_profile(
        &self,
        dingtalk_user_id: &str,
        username: &str,
        nick: Option<&str>,
        avatar: Option<&str>,
    ) -> Result<(), DatabaseError>;
    
    async fn delete_user_mapping(&self, id: i64) -> Result<(), DatabaseError>;
}

#[async_trait::async_trait]
pub trait MessageStore: Send + Sync {
    async fn create_message_mapping(
        &self,
        mapping: &MessageMapping,
    ) -> Result<MessageMapping, DatabaseError>;
    
    async fn get_by_matrix_event_id(
        &self,
        matrix_event_id: &str,
    ) -> Result<Option<MessageMapping>, DatabaseError>;
    
    async fn get_by_dingtalk_message_id(
        &self,
        dingtalk_message_id: &str,
    ) -> Result<Option<MessageMapping>, DatabaseError>;
    
    async fn delete_message_mapping(&self, id: i64) -> Result<(), DatabaseError>;
}
