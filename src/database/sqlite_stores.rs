use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::{prelude::*, sql_types, sqlite::SqliteConnection};

use super::models::{
    DeadLetterEvent, MediaCacheEntry, MessageMapping, ProcessedEvent, RoomMapping, UserMapping,
};
use super::stores::{DeadLetterStore, EventStore, MediaStore, MessageStore, RoomStore, UserStore};
use super::{DatabaseError, DatabaseResult};

type SqlitePool = Pool<ConnectionManager<SqliteConnection>>;
type SqlitePooledConnection = PooledConnection<ConnectionManager<SqliteConnection>>;

fn get_conn(pool: &SqlitePool) -> DatabaseResult<SqlitePooledConnection> {
    pool.get().map_err(|e| DatabaseError::Pool(e.to_string()))
}

#[derive(Clone)]
pub struct SqliteStores {
    pool: SqlitePool,
}

impl SqliteStores {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub fn room_store(&self) -> Arc<dyn RoomStore> {
        Arc::new(SqliteRoomStore::new(self.pool.clone()))
    }

    pub fn user_store(&self) -> Arc<dyn UserStore> {
        Arc::new(SqliteUserStore::new(self.pool.clone()))
    }

    pub fn message_store(&self) -> Arc<dyn MessageStore> {
        Arc::new(SqliteMessageStore::new(self.pool.clone()))
    }

    pub fn event_store(&self) -> Arc<dyn EventStore> {
        Arc::new(SqliteEventStore::new(self.pool.clone()))
    }

    pub fn dead_letter_store(&self) -> Arc<dyn DeadLetterStore> {
        Arc::new(SqliteDeadLetterStore::new(self.pool.clone()))
    }

    pub fn media_store(&self) -> Arc<dyn MediaStore> {
        Arc::new(SqliteMediaStore::new(self.pool.clone()))
    }
}

struct SqliteRoomStore {
    pool: SqlitePool,
}

impl SqliteRoomStore {
    fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RoomStore for SqliteRoomStore {
    async fn get_room_mapping(&self, matrix_room_id: &str) -> DatabaseResult<Option<RoomMapping>> {
        let pool = self.pool.clone();
        let matrix_room_id = matrix_room_id.to_string();

        tokio::task::spawn_blocking(move || {
            let mut conn = get_conn(&pool)?;
            let result: Option<RoomMappingRow> = diesel::sql_query(
                "SELECT id, matrix_room_id, dingtalk_conversation_id, dingtalk_conversation_name, 
                        dingtalk_conversation_type, created_at, updated_at 
                 FROM room_mappings WHERE matrix_room_id = ?",
            )
            .bind::<sql_types::Text, _>(&matrix_room_id)
            .get_result(&mut conn)
            .optional()?;

            Ok(result.map(|r| r.into_model()))
        })
        .await
        .map_err(|e| DatabaseError::Query(e.to_string()))?
    }

    async fn get_room_mapping_by_dingtalk(
        &self,
        dingtalk_conversation_id: &str,
    ) -> DatabaseResult<Option<RoomMapping>> {
        let pool = self.pool.clone();
        let dingtalk_conversation_id = dingtalk_conversation_id.to_string();

        tokio::task::spawn_blocking(move || {
            let mut conn = get_conn(&pool)?;
            let result: Option<RoomMappingRow> = diesel::sql_query(
                "SELECT id, matrix_room_id, dingtalk_conversation_id, dingtalk_conversation_name, 
                        dingtalk_conversation_type, created_at, updated_at 
                 FROM room_mappings WHERE dingtalk_conversation_id = ?",
            )
            .bind::<sql_types::Text, _>(&dingtalk_conversation_id)
            .get_result(&mut conn)
            .optional()?;

            Ok(result.map(|r| r.into_model()))
        })
        .await
        .map_err(|e| DatabaseError::Query(e.to_string()))?
    }

    async fn insert_room_mapping(&self, mapping: &RoomMapping) -> DatabaseResult<RoomMapping> {
        let pool = self.pool.clone();
        let matrix_room_id = mapping.matrix_room_id.clone();
        let dingtalk_conversation_id = mapping.dingtalk_conversation_id.clone();
        let dingtalk_conversation_name = mapping.dingtalk_conversation_name.clone();
        let dingtalk_conversation_type = mapping.dingtalk_conversation_type.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = get_conn(&pool)?;
            diesel::sql_query(
                "INSERT INTO room_mappings (matrix_room_id, dingtalk_conversation_id, dingtalk_conversation_name, dingtalk_conversation_type)
                 VALUES (?, ?, ?, ?)"
            )
            .bind::<sql_types::Text, _>(&matrix_room_id)
            .bind::<sql_types::Text, _>(&dingtalk_conversation_id)
            .bind::<sql_types::Nullable<sql_types::Text>, _>(&dingtalk_conversation_name)
            .bind::<sql_types::Text, _>(&dingtalk_conversation_type)
            .execute(&mut conn)?;
            
            let id: i64 = diesel::sql_query("SELECT CAST(last_insert_rowid() AS INTEGER) as id")
                .get_result::<IdRow>(&mut conn)?
                .id;
            
            Ok(RoomMapping {
                id,
                matrix_room_id,
                dingtalk_conversation_id,
                dingtalk_conversation_name,
                dingtalk_conversation_type,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            })
        }).await.map_err(|e| DatabaseError::Query(e.to_string()))?
    }

    async fn delete_room_mapping(&self, matrix_room_id: &str) -> DatabaseResult<bool> {
        let pool = self.pool.clone();
        let matrix_room_id = matrix_room_id.to_string();

        tokio::task::spawn_blocking(move || {
            let mut conn = get_conn(&pool)?;
            let affected = diesel::sql_query("DELETE FROM room_mappings WHERE matrix_room_id = ?")
                .bind::<sql_types::Text, _>(&matrix_room_id)
                .execute(&mut conn)?;
            Ok(affected > 0)
        })
        .await
        .map_err(|e| DatabaseError::Query(e.to_string()))?
    }

    async fn list_room_mappings(
        &self,
        limit: i64,
        offset: i64,
    ) -> DatabaseResult<Vec<RoomMapping>> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = get_conn(&pool)?;
            let results: Vec<RoomMappingRow> = diesel::sql_query(
                "SELECT id, matrix_room_id, dingtalk_conversation_id, dingtalk_conversation_name, 
                        dingtalk_conversation_type, created_at, updated_at 
                 FROM room_mappings ORDER BY created_at DESC LIMIT ? OFFSET ?",
            )
            .bind::<sql_types::BigInt, _>(&limit)
            .bind::<sql_types::BigInt, _>(&offset)
            .get_results(&mut conn)?;

            Ok(results.into_iter().map(|r| r.into_model()).collect())
        })
        .await
        .map_err(|e| DatabaseError::Query(e.to_string()))?
    }
}

struct SqliteUserStore {
    pool: SqlitePool,
}

impl SqliteUserStore {
    fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserStore for SqliteUserStore {
    async fn get_user_mapping(&self, matrix_user_id: &str) -> DatabaseResult<Option<UserMapping>> {
        let pool = self.pool.clone();
        let matrix_user_id = matrix_user_id.to_string();

        tokio::task::spawn_blocking(move || {
            let mut conn = get_conn(&pool)?;
            let result: Option<UserMappingRow> = diesel::sql_query(
                "SELECT id, matrix_user_id, dingtalk_user_id, dingtalk_username, dingtalk_avatar, created_at, updated_at
                 FROM user_mappings WHERE matrix_user_id = ?"
            )
            .bind::<sql_types::Text, _>(&matrix_user_id)
            .get_result(&mut conn)
            .optional()?;
            
            Ok(result.map(|r| r.into_model()))
        }).await.map_err(|e| DatabaseError::Query(e.to_string()))?
    }

    async fn get_user_mapping_by_dingtalk(
        &self,
        dingtalk_user_id: &str,
    ) -> DatabaseResult<Option<UserMapping>> {
        let pool = self.pool.clone();
        let dingtalk_user_id = dingtalk_user_id.to_string();

        tokio::task::spawn_blocking(move || {
            let mut conn = get_conn(&pool)?;
            let result: Option<UserMappingRow> = diesel::sql_query(
                "SELECT id, matrix_user_id, dingtalk_user_id, dingtalk_username, dingtalk_avatar, created_at, updated_at
                 FROM user_mappings WHERE dingtalk_user_id = ?"
            )
            .bind::<sql_types::Text, _>(&dingtalk_user_id)
            .get_result(&mut conn)
            .optional()?;
            
            Ok(result.map(|r| r.into_model()))
        }).await.map_err(|e| DatabaseError::Query(e.to_string()))?
    }

    async fn insert_user_mapping(&self, mapping: &UserMapping) -> DatabaseResult<UserMapping> {
        let pool = self.pool.clone();
        let matrix_user_id = mapping.matrix_user_id.clone();
        let dingtalk_user_id = mapping.dingtalk_user_id.clone();
        let dingtalk_username = mapping.dingtalk_username.clone();
        let dingtalk_avatar = mapping.dingtalk_avatar.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = get_conn(&pool)?;
            diesel::sql_query(
                "INSERT INTO user_mappings (matrix_user_id, dingtalk_user_id, dingtalk_username, dingtalk_avatar)
                 VALUES (?, ?, ?, ?)"
            )
            .bind::<sql_types::Text, _>(&matrix_user_id)
            .bind::<sql_types::Text, _>(&dingtalk_user_id)
            .bind::<sql_types::Nullable<sql_types::Text>, _>(&dingtalk_username)
            .bind::<sql_types::Nullable<sql_types::Text>, _>(&dingtalk_avatar)
            .execute(&mut conn)?;
            
            let id: i64 = diesel::sql_query("SELECT CAST(last_insert_rowid() AS INTEGER) as id")
                .get_result::<IdRow>(&mut conn)?
                .id;
            
            Ok(UserMapping {
                id,
                matrix_user_id,
                dingtalk_user_id,
                dingtalk_username,
                dingtalk_avatar,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            })
        }).await.map_err(|e| DatabaseError::Query(e.to_string()))?
    }

    async fn update_user_mapping(&self, mapping: &UserMapping) -> DatabaseResult<bool> {
        let pool = self.pool.clone();
        let matrix_user_id = mapping.matrix_user_id.clone();
        let dingtalk_username = mapping.dingtalk_username.clone();
        let dingtalk_avatar = mapping.dingtalk_avatar.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = get_conn(&pool)?;
            let affected = diesel::sql_query(
                "UPDATE user_mappings SET dingtalk_username = ?, dingtalk_avatar = ?, updated_at = datetime('now')
                 WHERE matrix_user_id = ?"
            )
            .bind::<sql_types::Nullable<sql_types::Text>, _>(&dingtalk_username)
            .bind::<sql_types::Nullable<sql_types::Text>, _>(&dingtalk_avatar)
            .bind::<sql_types::Text, _>(&matrix_user_id)
            .execute(&mut conn)?;
            Ok(affected > 0)
        }).await.map_err(|e| DatabaseError::Query(e.to_string()))?
    }

    async fn delete_user_mapping(&self, matrix_user_id: &str) -> DatabaseResult<bool> {
        let pool = self.pool.clone();
        let matrix_user_id = matrix_user_id.to_string();

        tokio::task::spawn_blocking(move || {
            let mut conn = get_conn(&pool)?;
            let affected = diesel::sql_query("DELETE FROM user_mappings WHERE matrix_user_id = ?")
                .bind::<sql_types::Text, _>(&matrix_user_id)
                .execute(&mut conn)?;
            Ok(affected > 0)
        })
        .await
        .map_err(|e| DatabaseError::Query(e.to_string()))?
    }
}

struct SqliteMessageStore {
    pool: SqlitePool,
}

impl SqliteMessageStore {
    fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MessageStore for SqliteMessageStore {
    async fn get_message_mapping(
        &self,
        matrix_event_id: &str,
    ) -> DatabaseResult<Option<MessageMapping>> {
        let pool = self.pool.clone();
        let matrix_event_id = matrix_event_id.to_string();

        tokio::task::spawn_blocking(move || {
            let mut conn = get_conn(&pool)?;
            let result: Option<MessageMappingRow> = diesel::sql_query(
                "SELECT id, matrix_event_id, dingtalk_message_id, room_id, sender_mxid, sender_dingtalk_id, content_hash, created_at
                 FROM message_mappings WHERE matrix_event_id = ?"
            )
            .bind::<sql_types::Text, _>(&matrix_event_id)
            .get_result(&mut conn)
            .optional()?;
            
            Ok(result.map(|r| r.into_model()))
        }).await.map_err(|e| DatabaseError::Query(e.to_string()))?
    }

    async fn get_message_mapping_by_dingtalk(
        &self,
        dingtalk_message_id: &str,
    ) -> DatabaseResult<Option<MessageMapping>> {
        let pool = self.pool.clone();
        let dingtalk_message_id = dingtalk_message_id.to_string();

        tokio::task::spawn_blocking(move || {
            let mut conn = get_conn(&pool)?;
            let result: Option<MessageMappingRow> = diesel::sql_query(
                "SELECT id, matrix_event_id, dingtalk_message_id, room_id, sender_mxid, sender_dingtalk_id, content_hash, created_at
                 FROM message_mappings WHERE dingtalk_message_id = ?"
            )
            .bind::<sql_types::Text, _>(&dingtalk_message_id)
            .get_result(&mut conn)
            .optional()?;
            
            Ok(result.map(|r| r.into_model()))
        }).await.map_err(|e| DatabaseError::Query(e.to_string()))?
    }

    async fn insert_message_mapping(
        &self,
        mapping: &MessageMapping,
    ) -> DatabaseResult<MessageMapping> {
        let pool = self.pool.clone();
        let matrix_event_id = mapping.matrix_event_id.clone();
        let dingtalk_message_id = mapping.dingtalk_message_id.clone();
        let room_id = mapping.room_id.clone();
        let sender_mxid = mapping.sender_mxid.clone();
        let sender_dingtalk_id = mapping.sender_dingtalk_id.clone();
        let content_hash = mapping.content_hash.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = get_conn(&pool)?;
            diesel::sql_query(
                "INSERT INTO message_mappings (matrix_event_id, dingtalk_message_id, room_id, sender_mxid, sender_dingtalk_id, content_hash)
                 VALUES (?, ?, ?, ?, ?, ?)"
            )
            .bind::<sql_types::Text, _>(&matrix_event_id)
            .bind::<sql_types::Text, _>(&dingtalk_message_id)
            .bind::<sql_types::Text, _>(&room_id)
            .bind::<sql_types::Text, _>(&sender_mxid)
            .bind::<sql_types::Text, _>(&sender_dingtalk_id)
            .bind::<sql_types::Nullable<sql_types::Text>, _>(&content_hash)
            .execute(&mut conn)?;
            
            let id: i64 = diesel::sql_query("SELECT CAST(last_insert_rowid() AS INTEGER) as id")
                .get_result::<IdRow>(&mut conn)?
                .id;
            
            Ok(MessageMapping {
                id,
                matrix_event_id,
                dingtalk_message_id,
                room_id,
                sender_mxid,
                sender_dingtalk_id,
                content_hash,
                created_at: Utc::now(),
            })
        }).await.map_err(|e| DatabaseError::Query(e.to_string()))?
    }

    async fn delete_message_mapping(&self, matrix_event_id: &str) -> DatabaseResult<bool> {
        let pool = self.pool.clone();
        let matrix_event_id = matrix_event_id.to_string();

        tokio::task::spawn_blocking(move || {
            let mut conn = get_conn(&pool)?;
            let affected =
                diesel::sql_query("DELETE FROM message_mappings WHERE matrix_event_id = ?")
                    .bind::<sql_types::Text, _>(&matrix_event_id)
                    .execute(&mut conn)?;
            Ok(affected > 0)
        })
        .await
        .map_err(|e| DatabaseError::Query(e.to_string()))?
    }
}

struct SqliteEventStore {
    pool: SqlitePool,
}

impl SqliteEventStore {
    fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl EventStore for SqliteEventStore {
    async fn is_event_processed(&self, event_id: &str) -> DatabaseResult<bool> {
        let pool = self.pool.clone();
        let event_id = event_id.to_string();

        tokio::task::spawn_blocking(move || {
            let mut conn = get_conn(&pool)?;
            let count: i64 = diesel::sql_query(
                "SELECT COUNT(*) as count FROM processed_events WHERE event_id = ?",
            )
            .bind::<sql_types::Text, _>(&event_id)
            .get_result::<CountRow>(&mut conn)?
            .count;
            Ok(count > 0)
        })
        .await
        .map_err(|e| DatabaseError::Query(e.to_string()))?
    }

    async fn mark_event_processed(&self, event: &ProcessedEvent) -> DatabaseResult<()> {
        let pool = self.pool.clone();
        let event_id = event.event_id.clone();
        let event_type = event.event_type.clone();
        let source = event.source.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = get_conn(&pool)?;
            diesel::sql_query(
                "INSERT OR IGNORE INTO processed_events (event_id, event_type, source) VALUES (?, ?, ?)"
            )
            .bind::<sql_types::Text, _>(&event_id)
            .bind::<sql_types::Text, _>(&event_type)
            .bind::<sql_types::Text, _>(&source)
            .execute(&mut conn)?;
            Ok(())
        }).await.map_err(|e| DatabaseError::Query(e.to_string()))?
    }

    async fn cleanup_old_events(&self, before: DateTime<Utc>) -> DatabaseResult<u64> {
        let pool = self.pool.clone();
        let before_str = before.to_rfc3339();

        tokio::task::spawn_blocking(move || {
            let mut conn = get_conn(&pool)?;
            let affected = diesel::sql_query("DELETE FROM processed_events WHERE processed_at < ?")
                .bind::<sql_types::Text, _>(&before_str)
                .execute(&mut conn)?;
            Ok(affected as u64)
        })
        .await
        .map_err(|e| DatabaseError::Query(e.to_string()))?
    }
}

struct SqliteDeadLetterStore {
    pool: SqlitePool,
}

impl SqliteDeadLetterStore {
    fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DeadLetterStore for SqliteDeadLetterStore {
    async fn insert_dead_letter(&self, event: &DeadLetterEvent) -> DatabaseResult<DeadLetterEvent> {
        let pool = self.pool.clone();
        let source = event.source.clone();
        let event_type = event.event_type.clone();
        let dedupe_key = event.dedupe_key.clone();
        let conversation_id = event.conversation_id.clone();
        let payload = event.payload.clone();
        let error = event.error.clone();
        let status = event.status.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = get_conn(&pool)?;
            diesel::sql_query(
                "INSERT INTO dead_letters (source, event_type, dedupe_key, conversation_id, payload, error, status)
                 VALUES (?, ?, ?, ?, ?, ?, ?)"
            )
            .bind::<sql_types::Text, _>(&source)
            .bind::<sql_types::Text, _>(&event_type)
            .bind::<sql_types::Text, _>(&dedupe_key)
            .bind::<sql_types::Nullable<sql_types::Text>, _>(&conversation_id)
            .bind::<sql_types::Text, _>(&payload)
            .bind::<sql_types::Text, _>(&error)
            .bind::<sql_types::Text, _>(&status)
            .execute(&mut conn)?;
            
            let id: i64 = diesel::sql_query("SELECT CAST(last_insert_rowid() AS INTEGER) as id")
                .get_result::<IdRow>(&mut conn)?
                .id;
            
            Ok(DeadLetterEvent {
                id,
                source,
                event_type,
                dedupe_key,
                conversation_id,
                payload,
                error,
                status,
                replay_count: 0,
                last_replayed_at: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            })
        }).await.map_err(|e| DatabaseError::Query(e.to_string()))?
    }

    async fn get_dead_letter(&self, id: i64) -> DatabaseResult<Option<DeadLetterEvent>> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = get_conn(&pool)?;
            let result: Option<DeadLetterEventRow> = diesel::sql_query(
                "SELECT id, source, event_type, dedupe_key, conversation_id, payload, error, status, 
                        replay_count, last_replayed_at, created_at, updated_at
                 FROM dead_letters WHERE id = ?"
            )
            .bind::<sql_types::BigInt, _>(&id)
            .get_result(&mut conn)
            .optional()?;
            
            Ok(result.map(|r| r.into_model()))
        }).await.map_err(|e| DatabaseError::Query(e.to_string()))?
    }

    async fn count_dead_letters(&self, status: Option<&str>) -> DatabaseResult<i64> {
        let pool = self.pool.clone();
        let status = status.map(|s| s.to_string());

        tokio::task::spawn_blocking(move || {
            let mut conn = get_conn(&pool)?;
            let count: i64 = if let Some(status) = status {
                diesel::sql_query("SELECT COUNT(*) as count FROM dead_letters WHERE status = ?")
                    .bind::<sql_types::Text, _>(&status)
                    .get_result::<CountRow>(&mut conn)?
                    .count
            } else {
                diesel::sql_query("SELECT COUNT(*) as count FROM dead_letters")
                    .get_result::<CountRow>(&mut conn)?
                    .count
            };
            Ok(count)
        })
        .await
        .map_err(|e| DatabaseError::Query(e.to_string()))?
    }

    async fn list_dead_letters(
        &self,
        status: Option<&str>,
        limit: i64,
    ) -> DatabaseResult<Vec<DeadLetterEvent>> {
        let pool = self.pool.clone();
        let status = status.map(|s| s.to_string());

        tokio::task::spawn_blocking(move || {
            let mut conn = get_conn(&pool)?;
            let results: Vec<DeadLetterEventRow> = if let Some(status) = status {
                diesel::sql_query(
                    "SELECT id, source, event_type, dedupe_key, conversation_id, payload, error, status, 
                            replay_count, last_replayed_at, created_at, updated_at
                     FROM dead_letters WHERE status = ? ORDER BY created_at DESC LIMIT ?"
                )
                .bind::<sql_types::Text, _>(&status)
                .bind::<sql_types::BigInt, _>(&limit)
                .get_results(&mut conn)?
            } else {
                diesel::sql_query(
                    "SELECT id, source, event_type, dedupe_key, conversation_id, payload, error, status, 
                            replay_count, last_replayed_at, created_at, updated_at
                     FROM dead_letters ORDER BY created_at DESC LIMIT ?"
                )
                .bind::<sql_types::BigInt, _>(&limit)
                .get_results(&mut conn)?
            };
            
            Ok(results.into_iter().map(|r| r.into_model()).collect())
        }).await.map_err(|e| DatabaseError::Query(e.to_string()))?
    }

    async fn update_dead_letter_status(&self, id: i64, status: &str) -> DatabaseResult<bool> {
        let pool = self.pool.clone();
        let status = status.to_string();

        tokio::task::spawn_blocking(move || {
            let mut conn = get_conn(&pool)?;
            let affected = diesel::sql_query(
                "UPDATE dead_letters SET status = ?, updated_at = datetime('now') WHERE id = ?",
            )
            .bind::<sql_types::Text, _>(&status)
            .bind::<sql_types::BigInt, _>(&id)
            .execute(&mut conn)?;
            Ok(affected > 0)
        })
        .await
        .map_err(|e| DatabaseError::Query(e.to_string()))?
    }

    async fn delete_dead_letter(&self, id: i64) -> DatabaseResult<bool> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = get_conn(&pool)?;
            let affected = diesel::sql_query("DELETE FROM dead_letters WHERE id = ?")
                .bind::<sql_types::BigInt, _>(&id)
                .execute(&mut conn)?;
            Ok(affected > 0)
        })
        .await
        .map_err(|e| DatabaseError::Query(e.to_string()))?
    }

    async fn cleanup_dead_letters(
        &self,
        status: Option<&str>,
        older_than_hours: Option<i64>,
        limit: i64,
    ) -> DatabaseResult<u64> {
        let pool = self.pool.clone();
        let status = status.map(|s| s.to_string());
        let older_than = older_than_hours.map(|hours| {
            (Utc::now() - chrono::Duration::hours(hours.max(0)))
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
        });
        let limit = limit.max(1);

        tokio::task::spawn_blocking(move || {
            let mut conn = get_conn(&pool)?;

            let affected = match (status.as_ref(), older_than.as_ref()) {
                (Some(status), Some(older_than)) => diesel::sql_query(
                    "DELETE FROM dead_letters WHERE id IN (
                        SELECT id FROM dead_letters
                        WHERE status = ? AND created_at < ?
                        ORDER BY id ASC
                        LIMIT ?
                    )",
                )
                .bind::<sql_types::Text, _>(status)
                .bind::<sql_types::Text, _>(older_than)
                .bind::<sql_types::BigInt, _>(&limit)
                .execute(&mut conn)?,
                (Some(status), None) => diesel::sql_query(
                    "DELETE FROM dead_letters WHERE id IN (
                        SELECT id FROM dead_letters
                        WHERE status = ?
                        ORDER BY id ASC
                        LIMIT ?
                    )",
                )
                .bind::<sql_types::Text, _>(status)
                .bind::<sql_types::BigInt, _>(&limit)
                .execute(&mut conn)?,
                (None, Some(older_than)) => diesel::sql_query(
                    "DELETE FROM dead_letters WHERE id IN (
                        SELECT id FROM dead_letters
                        WHERE created_at < ?
                        ORDER BY id ASC
                        LIMIT ?
                    )",
                )
                .bind::<sql_types::Text, _>(older_than)
                .bind::<sql_types::BigInt, _>(&limit)
                .execute(&mut conn)?,
                (None, None) => diesel::sql_query(
                    "DELETE FROM dead_letters WHERE id IN (
                        SELECT id FROM dead_letters
                        ORDER BY id ASC
                        LIMIT ?
                    )",
                )
                .bind::<sql_types::BigInt, _>(&limit)
                .execute(&mut conn)?,
            };

            Ok(affected as u64)
        })
        .await
        .map_err(|e| DatabaseError::Query(e.to_string()))?
    }
}

struct SqliteMediaStore {
    pool: SqlitePool,
}

impl SqliteMediaStore {
    fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MediaStore for SqliteMediaStore {
    async fn get_media_cache(
        &self,
        content_hash: &str,
        media_kind: &str,
    ) -> DatabaseResult<Option<MediaCacheEntry>> {
        let pool = self.pool.clone();
        let content_hash = content_hash.to_string();
        let media_kind = media_kind.to_string();

        tokio::task::spawn_blocking(move || {
            let mut conn = get_conn(&pool)?;
            let result: Option<MediaCacheEntryRow> = diesel::sql_query(
                "SELECT id, content_hash, media_kind, resource_key, created_at, updated_at
                 FROM media_cache WHERE content_hash = ? AND media_kind = ?",
            )
            .bind::<sql_types::Text, _>(&content_hash)
            .bind::<sql_types::Text, _>(&media_kind)
            .get_result(&mut conn)
            .optional()?;

            Ok(result.map(|r| r.into_model()))
        })
        .await
        .map_err(|e| DatabaseError::Query(e.to_string()))?
    }

    async fn insert_media_cache(&self, entry: &MediaCacheEntry) -> DatabaseResult<MediaCacheEntry> {
        let pool = self.pool.clone();
        let content_hash = entry.content_hash.clone();
        let media_kind = entry.media_kind.clone();
        let resource_key = entry.resource_key.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = get_conn(&pool)?;
            diesel::sql_query(
                "INSERT OR REPLACE INTO media_cache (content_hash, media_kind, resource_key)
                 VALUES (?, ?, ?)",
            )
            .bind::<sql_types::Text, _>(&content_hash)
            .bind::<sql_types::Text, _>(&media_kind)
            .bind::<sql_types::Text, _>(&resource_key)
            .execute(&mut conn)?;

            let id: i64 = diesel::sql_query("SELECT CAST(last_insert_rowid() AS INTEGER) as id")
                .get_result::<IdRow>(&mut conn)?
                .id;

            Ok(MediaCacheEntry {
                id,
                content_hash,
                media_kind,
                resource_key,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            })
        })
        .await
        .map_err(|e| DatabaseError::Query(e.to_string()))?
    }

    async fn cleanup_old_media_cache(&self, before: DateTime<Utc>) -> DatabaseResult<u64> {
        let pool = self.pool.clone();
        let before_str = before.to_rfc3339();

        tokio::task::spawn_blocking(move || {
            let mut conn = get_conn(&pool)?;
            let affected = diesel::sql_query("DELETE FROM media_cache WHERE created_at < ?")
                .bind::<sql_types::Text, _>(&before_str)
                .execute(&mut conn)?;
            Ok(affected as u64)
        })
        .await
        .map_err(|e| DatabaseError::Query(e.to_string()))?
    }
}

#[derive(QueryableByName)]
struct IdRow {
    #[diesel(sql_type = sql_types::BigInt)]
    id: i64,
}

#[derive(QueryableByName)]
struct CountRow {
    #[diesel(sql_type = sql_types::BigInt)]
    count: i64,
}

#[derive(QueryableByName)]
struct RoomMappingRow {
    #[diesel(sql_type = sql_types::BigInt)]
    id: i64,
    #[diesel(sql_type = sql_types::Text)]
    matrix_room_id: String,
    #[diesel(sql_type = sql_types::Text)]
    dingtalk_conversation_id: String,
    #[diesel(sql_type = sql_types::Nullable<sql_types::Text>)]
    dingtalk_conversation_name: Option<String>,
    #[diesel(sql_type = sql_types::Text)]
    dingtalk_conversation_type: String,
    #[diesel(sql_type = sql_types::Text)]
    created_at: String,
    #[diesel(sql_type = sql_types::Text)]
    updated_at: String,
}

impl RoomMappingRow {
    fn into_model(self) -> RoomMapping {
        RoomMapping {
            id: self.id,
            matrix_room_id: self.matrix_room_id,
            dingtalk_conversation_id: self.dingtalk_conversation_id,
            dingtalk_conversation_name: self.dingtalk_conversation_name,
            dingtalk_conversation_type: self.dingtalk_conversation_type,
            created_at: self.created_at.parse().unwrap_or_else(|_| Utc::now()),
            updated_at: self.updated_at.parse().unwrap_or_else(|_| Utc::now()),
        }
    }
}

#[derive(QueryableByName)]
struct UserMappingRow {
    #[diesel(sql_type = sql_types::BigInt)]
    id: i64,
    #[diesel(sql_type = sql_types::Text)]
    matrix_user_id: String,
    #[diesel(sql_type = sql_types::Text)]
    dingtalk_user_id: String,
    #[diesel(sql_type = sql_types::Nullable<sql_types::Text>)]
    dingtalk_username: Option<String>,
    #[diesel(sql_type = sql_types::Nullable<sql_types::Text>)]
    dingtalk_avatar: Option<String>,
    #[diesel(sql_type = sql_types::Text)]
    created_at: String,
    #[diesel(sql_type = sql_types::Text)]
    updated_at: String,
}

impl UserMappingRow {
    fn into_model(self) -> UserMapping {
        UserMapping {
            id: self.id,
            matrix_user_id: self.matrix_user_id,
            dingtalk_user_id: self.dingtalk_user_id,
            dingtalk_username: self.dingtalk_username,
            dingtalk_avatar: self.dingtalk_avatar,
            created_at: self.created_at.parse().unwrap_or_else(|_| Utc::now()),
            updated_at: self.updated_at.parse().unwrap_or_else(|_| Utc::now()),
        }
    }
}

#[derive(QueryableByName)]
struct MessageMappingRow {
    #[diesel(sql_type = sql_types::BigInt)]
    id: i64,
    #[diesel(sql_type = sql_types::Text)]
    matrix_event_id: String,
    #[diesel(sql_type = sql_types::Text)]
    dingtalk_message_id: String,
    #[diesel(sql_type = sql_types::Text)]
    room_id: String,
    #[diesel(sql_type = sql_types::Text)]
    sender_mxid: String,
    #[diesel(sql_type = sql_types::Text)]
    sender_dingtalk_id: String,
    #[diesel(sql_type = sql_types::Nullable<sql_types::Text>)]
    content_hash: Option<String>,
    #[diesel(sql_type = sql_types::Text)]
    created_at: String,
}

impl MessageMappingRow {
    fn into_model(self) -> MessageMapping {
        MessageMapping {
            id: self.id,
            matrix_event_id: self.matrix_event_id,
            dingtalk_message_id: self.dingtalk_message_id,
            room_id: self.room_id,
            sender_mxid: self.sender_mxid,
            sender_dingtalk_id: self.sender_dingtalk_id,
            content_hash: self.content_hash,
            created_at: self.created_at.parse().unwrap_or_else(|_| Utc::now()),
        }
    }
}

#[derive(QueryableByName)]
struct DeadLetterEventRow {
    #[diesel(sql_type = sql_types::BigInt)]
    id: i64,
    #[diesel(sql_type = sql_types::Text)]
    source: String,
    #[diesel(sql_type = sql_types::Text)]
    event_type: String,
    #[diesel(sql_type = sql_types::Text)]
    dedupe_key: String,
    #[diesel(sql_type = sql_types::Nullable<sql_types::Text>)]
    conversation_id: Option<String>,
    #[diesel(sql_type = sql_types::Text)]
    payload: String,
    #[diesel(sql_type = sql_types::Text)]
    error: String,
    #[diesel(sql_type = sql_types::Text)]
    status: String,
    #[diesel(sql_type = sql_types::BigInt)]
    replay_count: i64,
    #[diesel(sql_type = sql_types::Nullable<sql_types::Text>)]
    last_replayed_at: Option<String>,
    #[diesel(sql_type = sql_types::Text)]
    created_at: String,
    #[diesel(sql_type = sql_types::Text)]
    updated_at: String,
}

impl DeadLetterEventRow {
    fn into_model(self) -> DeadLetterEvent {
        DeadLetterEvent {
            id: self.id,
            source: self.source,
            event_type: self.event_type,
            dedupe_key: self.dedupe_key,
            conversation_id: self.conversation_id,
            payload: self.payload,
            error: self.error,
            status: self.status,
            replay_count: self.replay_count,
            last_replayed_at: self.last_replayed_at.and_then(|s| s.parse().ok()),
            created_at: self.created_at.parse().unwrap_or_else(|_| Utc::now()),
            updated_at: self.updated_at.parse().unwrap_or_else(|_| Utc::now()),
        }
    }
}

#[derive(QueryableByName)]
struct MediaCacheEntryRow {
    #[diesel(sql_type = sql_types::BigInt)]
    id: i64,
    #[diesel(sql_type = sql_types::Text)]
    content_hash: String,
    #[diesel(sql_type = sql_types::Text)]
    media_kind: String,
    #[diesel(sql_type = sql_types::Text)]
    resource_key: String,
    #[diesel(sql_type = sql_types::Text)]
    created_at: String,
    #[diesel(sql_type = sql_types::Text)]
    updated_at: String,
}

impl MediaCacheEntryRow {
    fn into_model(self) -> MediaCacheEntry {
        MediaCacheEntry {
            id: self.id,
            content_hash: self.content_hash,
            media_kind: self.media_kind,
            resource_key: self.resource_key,
            created_at: self.created_at.parse().unwrap_or_else(|_| Utc::now()),
            updated_at: self.updated_at.parse().unwrap_or_else(|_| Utc::now()),
        }
    }
}
