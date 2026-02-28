use diesel::pg::PgConnection;
use diesel::prelude::*;

use super::DatabaseError;
use super::models::{MessageMapping, RoomMapping, UserMapping};
use crate::db::manager::Pool;
use crate::db::schema::{message_mappings, room_mappings, user_mappings};

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = room_mappings)]
struct DbRoomMapping {
    id: i64,
    matrix_room_id: String,
    dingtalk_chat_id: String,
    dingtalk_chat_name: String,
    dingtalk_conversation_type: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<DbRoomMapping> for RoomMapping {
    fn from(value: DbRoomMapping) -> Self {
        Self {
            id: value.id,
            matrix_room_id: value.matrix_room_id,
            dingtalk_chat_id: value.dingtalk_chat_id,
            dingtalk_chat_name: value.dingtalk_chat_name,
            dingtalk_conversation_type: value.dingtalk_conversation_type,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Insertable)]
#[diesel(table_name = room_mappings)]
struct NewRoomMapping<'a> {
    matrix_room_id: &'a str,
    dingtalk_chat_id: &'a str,
    dingtalk_chat_name: &'a str,
    dingtalk_conversation_type: &'a str,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = user_mappings)]
struct DbUserMapping {
    id: i64,
    matrix_user_id: String,
    dingtalk_user_id: String,
    dingtalk_username: String,
    dingtalk_nick: Option<String>,
    dingtalk_avatar: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<DbUserMapping> for UserMapping {
    fn from(value: DbUserMapping) -> Self {
        Self {
            id: value.id,
            matrix_user_id: value.matrix_user_id,
            dingtalk_user_id: value.dingtalk_user_id,
            dingtalk_username: value.dingtalk_username,
            dingtalk_nick: value.dingtalk_nick,
            dingtalk_avatar: value.dingtalk_avatar,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Insertable)]
#[diesel(table_name = user_mappings)]
struct NewUserMapping<'a> {
    matrix_user_id: &'a str,
    dingtalk_user_id: &'a str,
    dingtalk_username: &'a str,
    dingtalk_nick: Option<&'a str>,
    dingtalk_avatar: Option<&'a str>,
}

#[derive(AsChangeset)]
#[diesel(table_name = user_mappings)]
struct UpdateUserMapping<'a> {
    dingtalk_username: &'a str,
    dingtalk_nick: Option<&'a str>,
    dingtalk_avatar: Option<&'a str>,
    updated_at: &'a chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = message_mappings)]
struct DbMessageMapping {
    id: i64,
    dingtalk_message_id: String,
    matrix_room_id: String,
    matrix_event_id: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<DbMessageMapping> for MessageMapping {
    fn from(value: DbMessageMapping) -> Self {
        Self {
            id: value.id,
            dingtalk_message_id: value.dingtalk_message_id,
            matrix_room_id: value.matrix_room_id,
            matrix_event_id: value.matrix_event_id,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Insertable)]
#[diesel(table_name = message_mappings)]
struct NewMessageMapping<'a> {
    dingtalk_message_id: &'a str,
    matrix_room_id: &'a str,
    matrix_event_id: &'a str,
}

pub struct PostgresRoomStore {
    pool: Pool,
}

impl PostgresRoomStore {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl super::RoomStore for PostgresRoomStore {
    async fn create_room_mapping(
        &self,
        mapping: &RoomMapping,
    ) -> Result<RoomMapping, DatabaseError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        let new_mapping = NewRoomMapping {
            matrix_room_id: &mapping.matrix_room_id,
            dingtalk_chat_id: &mapping.dingtalk_chat_id,
            dingtalk_chat_name: &mapping.dingtalk_chat_name,
            dingtalk_conversation_type: &mapping.dingtalk_conversation_type,
        };

        let created: DbRoomMapping = diesel::insert_into(room_mappings::table)
            .values(&new_mapping)
            .returning(DbRoomMapping::as_returning())
            .get_result(&conn)
            .map_err(|e| DatabaseError::Query(e.to_string()))?;

        Ok(created.into())
    }

    async fn get_room_by_matrix_room(
        &self,
        matrix_room_id: &str,
    ) -> Result<Option<RoomMapping>, DatabaseError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        room_mappings::table
            .filter(room_mappings::matrix_room_id.eq(matrix_room_id))
            .select(DbRoomMapping::as_select())
            .first(&conn)
            .optional()
            .map(|opt| opt.map(|db| db.into()))
            .map_err(|e| DatabaseError::Query(e.to_string()))
    }

    async fn get_room_by_dingtalk_chat(
        &self,
        dingtalk_chat_id: &str,
    ) -> Result<Option<RoomMapping>, DatabaseError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        room_mappings::table
            .filter(room_mappings::dingtalk_chat_id.eq(dingtalk_chat_id))
            .select(DbRoomMapping::as_select())
            .first(&conn)
            .optional()
            .map(|opt| opt.map(|db| db.into()))
            .map_err(|e| DatabaseError::Query(e.to_string()))
    }

    async fn delete_room_mapping(&self, id: i64) -> Result<(), DatabaseError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        diesel::delete(room_mappings::table.filter(room_mappings::id.eq(id)))
            .execute(&conn)
            .map(|_| ())
            .map_err(|e| DatabaseError::Query(e.to_string()))
    }

    async fn count_rooms(&self) -> Result<i64, DatabaseError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        room_mappings::table
            .count()
            .get_result(&conn)
            .map_err(|e| DatabaseError::Query(e.to_string()))
    }
}

pub struct PostgresUserStore {
    pool: Pool,
}

impl PostgresUserStore {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl super::UserStore for PostgresUserStore {
    async fn create_user_mapping(
        &self,
        mapping: &UserMapping,
    ) -> Result<UserMapping, DatabaseError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        let new_mapping = NewUserMapping {
            matrix_user_id: &mapping.matrix_user_id,
            dingtalk_user_id: &mapping.dingtalk_user_id,
            dingtalk_username: &mapping.dingtalk_username,
            dingtalk_nick: mapping.dingtalk_nick.as_deref(),
            dingtalk_avatar: mapping.dingtalk_avatar.as_deref(),
        };

        let created: DbUserMapping = diesel::insert_into(user_mappings::table)
            .values(&new_mapping)
            .returning(DbUserMapping::as_returning())
            .get_result(&conn)
            .map_err(|e| DatabaseError::Query(e.to_string()))?;

        Ok(created.into())
    }

    async fn get_user_by_matrix_id(
        &self,
        matrix_user_id: &str,
    ) -> Result<Option<UserMapping>, DatabaseError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        user_mappings::table
            .filter(user_mappings::matrix_user_id.eq(matrix_user_id))
            .select(DbUserMapping::as_select())
            .first(&conn)
            .optional()
            .map(|opt| opt.map(|db| db.into()))
            .map_err(|e| DatabaseError::Query(e.to_string()))
    }

    async fn get_user_by_dingtalk_id(
        &self,
        dingtalk_user_id: &str,
    ) -> Result<Option<UserMapping>, DatabaseError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        user_mappings::table
            .filter(user_mappings::dingtalk_user_id.eq(dingtalk_user_id))
            .select(DbUserMapping::as_select())
            .first(&conn)
            .optional()
            .map(|opt| opt.map(|db| db.into()))
            .map_err(|e| DatabaseError::Query(e.to_string()))
    }

    async fn update_user_profile(
        &self,
        dingtalk_user_id: &str,
        username: &str,
        nick: Option<&str>,
        avatar: Option<&str>,
    ) -> Result<(), DatabaseError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        let now = chrono::Utc::now();
        let update = UpdateUserMapping {
            dingtalk_username: username,
            dingtalk_nick: nick,
            dingtalk_avatar: avatar,
            updated_at: &now,
        };

        diesel::update(user_mappings::table.filter(user_mappings::dingtalk_user_id.eq(dingtalk_user_id)))
            .set(&update)
            .execute(&conn)
            .map(|_| ())
            .map_err(|e| DatabaseError::Query(e.to_string()))
    }

    async fn delete_user_mapping(&self, id: i64) -> Result<(), DatabaseError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        diesel::delete(user_mappings::table.filter(user_mappings::id.eq(id)))
            .execute(&conn)
            .map(|_| ())
            .map_err(|e| DatabaseError::Query(e.to_string()))
    }
}

pub struct PostgresMessageStore {
    pool: Pool,
}

impl PostgresMessageStore {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl super::MessageStore for PostgresMessageStore {
    async fn create_message_mapping(
        &self,
        mapping: &MessageMapping,
    ) -> Result<MessageMapping, DatabaseError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        let new_mapping = NewMessageMapping {
            dingtalk_message_id: &mapping.dingtalk_message_id,
            matrix_room_id: &mapping.matrix_room_id,
            matrix_event_id: &mapping.matrix_event_id,
        };

        let created: DbMessageMapping = diesel::insert_into(message_mappings::table)
            .values(&new_mapping)
            .returning(DbMessageMapping::as_returning())
            .get_result(&conn)
            .map_err(|e| DatabaseError::Query(e.to_string()))?;

        Ok(created.into())
    }

    async fn get_by_matrix_event_id(
        &self,
        matrix_event_id: &str,
    ) -> Result<Option<MessageMapping>, DatabaseError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        message_mappings::table
            .filter(message_mappings::matrix_event_id.eq(matrix_event_id))
            .select(DbMessageMapping::as_select())
            .first(&conn)
            .optional()
            .map(|opt| opt.map(|db| db.into()))
            .map_err(|e| DatabaseError::Query(e.to_string()))
    }

    async fn get_by_dingtalk_message_id(
        &self,
        dingtalk_message_id: &str,
    ) -> Result<Option<MessageMapping>, DatabaseError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        message_mappings::table
            .filter(message_mappings::dingtalk_message_id.eq(dingtalk_message_id))
            .select(DbMessageMapping::as_select())
            .first(&conn)
            .optional()
            .map(|opt| opt.map(|db| db.into()))
            .map_err(|e| DatabaseError::Query(e.to_string()))
    }

    async fn delete_message_mapping(&self, id: i64) -> Result<(), DatabaseError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| DatabaseError::Connection(e.to_string()))?;

        diesel::delete(message_mappings::table.filter(message_mappings::id.eq(id)))
            .execute(&conn)
            .map(|_| ())
            .map_err(|e| DatabaseError::Query(e.to_string()))
    }
}
