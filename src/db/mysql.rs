// MySQL implementation placeholder
// Similar to PostgreSQL implementation but with MySQL-specific types

use diesel::r2d2::{self, ConnectionManager};
use diesel::mysql::MysqlConnection;

use crate::db::{DatabaseError, MessageMapping, MessageStore, RoomMapping, RoomStore, UserMapping, UserStore};

pub type MysqlPool = r2d2::Pool<ConnectionManager<MysqlConnection>>;

pub struct MysqlRoomStore {
    _pool: MysqlPool,
}

impl MysqlRoomStore {
    pub fn new(pool: MysqlPool) -> Self {
        Self { _pool: pool }
    }
}

#[async_trait::async_trait]
impl RoomStore for MysqlRoomStore {
    async fn create_room_mapping(&self, _mapping: &RoomMapping) -> Result<RoomMapping, DatabaseError> {
        Err(DatabaseError::Query("MySQL not yet implemented".to_string()))
    }

    async fn get_room_by_matrix_room(&self, _matrix_room_id: &str) -> Result<Option<RoomMapping>, DatabaseError> {
        Err(DatabaseError::Query("MySQL not yet implemented".to_string()))
    }

    async fn get_room_by_dingtalk_chat(&self, _dingtalk_chat_id: &str) -> Result<Option<RoomMapping>, DatabaseError> {
        Err(DatabaseError::Query("MySQL not yet implemented".to_string()))
    }

    async fn delete_room_mapping(&self, _id: i64) -> Result<(), DatabaseError> {
        Err(DatabaseError::Query("MySQL not yet implemented".to_string()))
    }

    async fn count_rooms(&self) -> Result<i64, DatabaseError> {
        Err(DatabaseError::Query("MySQL not yet implemented".to_string()))
    }
}

pub struct MysqlUserStore {
    _pool: MysqlPool,
}

impl MysqlUserStore {
    pub fn new(pool: MysqlPool) -> Self {
        Self { _pool: pool }
    }
}

#[async_trait::async_trait]
impl UserStore for MysqlUserStore {
    async fn create_user_mapping(&self, _mapping: &UserMapping) -> Result<UserMapping, DatabaseError> {
        Err(DatabaseError::Query("MySQL not yet implemented".to_string()))
    }

    async fn get_user_by_matrix_id(&self, _matrix_user_id: &str) -> Result<Option<UserMapping>, DatabaseError> {
        Err(DatabaseError::Query("MySQL not yet implemented".to_string()))
    }

    async fn get_user_by_dingtalk_id(&self, _dingtalk_user_id: &str) -> Result<Option<UserMapping>, DatabaseError> {
        Err(DatabaseError::Query("MySQL not yet implemented".to_string()))
    }

    async fn update_user_profile(&self, _dingtalk_user_id: &str, _username: &str, _nick: Option<&str>, _avatar: Option<&str>) -> Result<(), DatabaseError> {
        Err(DatabaseError::Query("MySQL not yet implemented".to_string()))
    }

    async fn delete_user_mapping(&self, _id: i64) -> Result<(), DatabaseError> {
        Err(DatabaseError::Query("MySQL not yet implemented".to_string()))
    }
}

pub struct MysqlMessageStore {
    _pool: MysqlPool,
}

impl MysqlMessageStore {
    pub fn new(pool: MysqlPool) -> Self {
        Self { _pool: pool }
    }
}

#[async_trait::async_trait]
impl MessageStore for MysqlMessageStore {
    async fn create_message_mapping(&self, _mapping: &MessageMapping) -> Result<MessageMapping, DatabaseError> {
        Err(DatabaseError::Query("MySQL not yet implemented".to_string()))
    }

    async fn get_by_matrix_event_id(&self, _matrix_event_id: &str) -> Result<Option<MessageMapping>, DatabaseError> {
        Err(DatabaseError::Query("MySQL not yet implemented".to_string()))
    }

    async fn get_by_dingtalk_message_id(&self, _dingtalk_message_id: &str) -> Result<Option<MessageMapping>, DatabaseError> {
        Err(DatabaseError::Query("MySQL not yet implemented".to_string()))
    }

    async fn delete_message_mapping(&self, _id: i64) -> Result<(), DatabaseError> {
        Err(DatabaseError::Query("MySQL not yet implemented".to_string()))
    }
}
