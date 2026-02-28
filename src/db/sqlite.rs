use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::db::{
    schema_sqlite::{message_mappings, room_mappings, user_mappings},
    DatabaseError, MessageMapping, MessageStore, RoomMapping, RoomStore,
    UserMapping, UserStore,
};

pub struct SqliteRoomStore {
    db_path: String,
}

impl SqliteRoomStore {
    pub fn new(db_path: String) -> Self {
        Self { db_path }
    }

    fn get_connection(&self) -> Result<SqliteConnection, DatabaseError> {
        SqliteConnection::establish(&self.db_path)
            .map_err(|e| DatabaseError::Connection(e.to_string()))
    }
}

#[async_trait::async_trait]
impl RoomStore for SqliteRoomStore {
    async fn create_room_mapping(
        &self,
        mapping: &RoomMapping,
    ) -> Result<RoomMapping, DatabaseError> {
        let mut conn = self.get_connection()?;

        diesel::insert_into(room_mappings::table)
            .values((
                room_mappings::matrix_room_id.eq(&mapping.matrix_room_id),
                room_mappings::dingtalk_chat_id.eq(&mapping.dingtalk_chat_id),
                room_mappings::dingtalk_chat_name.eq(&mapping.dingtalk_chat_name),
                room_mappings::dingtalk_conversation_type.eq(&mapping.dingtalk_conversation_type),
            ))
            .execute(&mut conn)
            .map_err(|e| DatabaseError::Query(e.to_string()))?;

        let created = room_mappings::table
            .filter(room_mappings::matrix_room_id.eq(&mapping.matrix_room_id))
            .first::<RoomMapping>(&mut conn)
            .map_err(|e| DatabaseError::Query(e.to_string()))?;

        Ok(created)
    }

    async fn get_room_by_matrix_room(
        &self,
        matrix_room_id: &str,
    ) -> Result<Option<RoomMapping>, DatabaseError> {
        let mut conn = self.get_connection()?;

        room_mappings::table
            .filter(room_mappings::matrix_room_id.eq(matrix_room_id))
            .first::<RoomMapping>(&mut conn)
            .optional()
            .map_err(|e| DatabaseError::Query(e.to_string()))
    }

    async fn get_room_by_dingtalk_chat(
        &self,
        dingtalk_chat_id: &str,
    ) -> Result<Option<RoomMapping>, DatabaseError> {
        let mut conn = self.get_connection()?;

        room_mappings::table
            .filter(room_mappings::dingtalk_chat_id.eq(dingtalk_chat_id))
            .first::<RoomMapping>(&mut conn)
            .optional()
            .map_err(|e| DatabaseError::Query(e.to_string()))
    }

    async fn delete_room_mapping(&self, id: i64) -> Result<(), DatabaseError> {
        let mut conn = self.get_connection()?;

        diesel::delete(room_mappings::table.filter(room_mappings::id.eq(id)))
            .execute(&mut conn)
            .map(|_| ())
            .map_err(|e| DatabaseError::Query(e.to_string()))
    }

    async fn count_rooms(&self) -> Result<i64, DatabaseError> {
        let mut conn = self.get_connection()?;

        room_mappings::table
            .count()
            .get_result(&mut conn)
            .map_err(|e| DatabaseError::Query(e.to_string()))
    }
}

pub struct SqliteUserStore {
    db_path: String,
}

impl SqliteUserStore {
    pub fn new(db_path: String) -> Self {
        Self { db_path }
    }

    fn get_connection(&self) -> Result<SqliteConnection, DatabaseError> {
        SqliteConnection::establish(&self.db_path)
            .map_err(|e| DatabaseError::Connection(e.to_string()))
    }
}

#[async_trait::async_trait]
impl UserStore for SqliteUserStore {
    async fn create_user_mapping(
        &self,
        mapping: &UserMapping,
    ) -> Result<UserMapping, DatabaseError> {
        let mut conn = self.get_connection()?;

        diesel::insert_into(user_mappings::table)
            .values((
                user_mappings::matrix_user_id.eq(&mapping.matrix_user_id),
                user_mappings::dingtalk_user_id.eq(&mapping.dingtalk_user_id),
                user_mappings::dingtalk_username.eq(&mapping.dingtalk_username),
                user_mappings::dingtalk_nick.eq(&mapping.dingtalk_nick),
                user_mappings::dingtalk_avatar.eq(&mapping.dingtalk_avatar),
            ))
            .execute(&mut conn)
            .map_err(|e| DatabaseError::Query(e.to_string()))?;

        let created = user_mappings::table
            .filter(user_mappings::dingtalk_user_id.eq(&mapping.dingtalk_user_id))
            .first::<UserMapping>(&mut conn)
            .map_err(|e| DatabaseError::Query(e.to_string()))?;

        Ok(created)
    }

    async fn get_user_by_matrix_id(
        &self,
        matrix_user_id: &str,
    ) -> Result<Option<UserMapping>, DatabaseError> {
        let mut conn = self.get_connection()?;

        user_mappings::table
            .filter(user_mappings::matrix_user_id.eq(matrix_user_id))
            .first::<UserMapping>(&mut conn)
            .optional()
            .map_err(|e| DatabaseError::Query(e.to_string()))
    }

    async fn get_user_by_dingtalk_id(
        &self,
        dingtalk_user_id: &str,
    ) -> Result<Option<UserMapping>, DatabaseError> {
        let mut conn = self.get_connection()?;

        user_mappings::table
            .filter(user_mappings::dingtalk_user_id.eq(dingtalk_user_id))
            .first::<UserMapping>(&mut conn)
            .optional()
            .map_err(|e| DatabaseError::Query(e.to_string()))
    }

    async fn update_user_profile(
        &self,
        dingtalk_user_id: &str,
        username: &str,
        nick: Option<&str>,
        avatar: Option<&str>,
    ) -> Result<(), DatabaseError> {
        let mut conn = self.get_connection()?;

        diesel::update(user_mappings::table.filter(user_mappings::dingtalk_user_id.eq(dingtalk_user_id)))
            .set((
                user_mappings::dingtalk_username.eq(username),
                user_mappings::dingtalk_nick.eq(nick),
                user_mappings::dingtalk_avatar.eq(avatar),
            ))
            .execute(&mut conn)
            .map(|_| ())
            .map_err(|e| DatabaseError::Query(e.to_string()))
    }

    async fn delete_user_mapping(&self, id: i64) -> Result<(), DatabaseError> {
        let mut conn = self.get_connection()?;

        diesel::delete(user_mappings::table.filter(user_mappings::id.eq(id)))
            .execute(&mut conn)
            .map(|_| ())
            .map_err(|e| DatabaseError::Query(e.to_string()))
    }
}

pub struct SqliteMessageStore {
    db_path: String,
}

impl SqliteMessageStore {
    pub fn new(db_path: String) -> Self {
        Self { db_path }
    }

    fn get_connection(&self) -> Result<SqliteConnection, DatabaseError> {
        SqliteConnection::establish(&self.db_path)
            .map_err(|e| DatabaseError::Connection(e.to_string()))
    }
}

#[async_trait::async_trait]
impl MessageStore for SqliteMessageStore {
    async fn create_message_mapping(
        &self,
        mapping: &MessageMapping,
    ) -> Result<MessageMapping, DatabaseError> {
        let mut conn = self.get_connection()?;

        diesel::insert_into(message_mappings::table)
            .values((
                message_mappings::dingtalk_message_id.eq(&mapping.dingtalk_message_id),
                message_mappings::matrix_room_id.eq(&mapping.matrix_room_id),
                message_mappings::matrix_event_id.eq(&mapping.matrix_event_id),
            ))
            .execute(&mut conn)
            .map_err(|e| DatabaseError::Query(e.to_string()))?;

        let created = message_mappings::table
            .filter(message_mappings::matrix_event_id.eq(&mapping.matrix_event_id))
            .first::<MessageMapping>(&mut conn)
            .map_err(|e| DatabaseError::Query(e.to_string()))?;

        Ok(created)
    }

    async fn get_by_matrix_event_id(
        &self,
        matrix_event_id: &str,
    ) -> Result<Option<MessageMapping>, DatabaseError> {
        let mut conn = self.get_connection()?;

        message_mappings::table
            .filter(message_mappings::matrix_event_id.eq(matrix_event_id))
            .first::<MessageMapping>(&mut conn)
            .optional()
            .map_err(|e| DatabaseError::Query(e.to_string()))
    }

    async fn get_by_dingtalk_message_id(
        &self,
        dingtalk_message_id: &str,
    ) -> Result<Option<MessageMapping>, DatabaseError> {
        let mut conn = self.get_connection()?;

        message_mappings::table
            .filter(message_mappings::dingtalk_message_id.eq(dingtalk_message_id))
            .first::<MessageMapping>(&mut conn)
            .optional()
            .map_err(|e| DatabaseError::Query(e.to_string()))
    }

    async fn delete_message_mapping(&self, id: i64) -> Result<(), DatabaseError> {
        let mut conn = self.get_connection()?;

        diesel::delete(message_mappings::table.filter(message_mappings::id.eq(id)))
            .execute(&mut conn)
            .map(|_| ())
            .map_err(|e| DatabaseError::Query(e.to_string()))
    }
}
