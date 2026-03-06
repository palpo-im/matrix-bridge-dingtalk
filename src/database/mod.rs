pub mod error;
pub mod models;
pub mod sqlite_stores;
pub mod stores;

pub use error::{DatabaseError, DatabaseResult};
pub use models::{
    DeadLetterEvent, MediaCacheEntry, MessageMapping, ProcessedEvent, RoomMapping, UserMapping,
};
pub use sqlite_stores::SqliteStores;
pub use stores::{DeadLetterStore, EventStore, MediaStore, MessageStore, RoomStore, UserStore};

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use diesel::connection::SimpleConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use tracing::info;

pub type SqlitePool = Pool<ConnectionManager<SqliteConnection>>;

#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
    db_type: String,
}

impl Database {
    pub async fn connect(
        db_type: &str,
        db_uri: &str,
        max_open: u32,
        max_idle: u32,
    ) -> Result<Self> {
        info!("Connecting to {} database: {}", db_type, db_uri);

        let db_kind = db_type.trim().to_ascii_lowercase();
        let max_size = max_open.max(1);
        let min_idle = Some(max_idle.min(max_size));

        if db_kind != "sqlite" {
            anyhow::bail!("database type '{}' is not supported; use sqlite", db_type);
        }

        let db_path = sqlite_path_from_uri(db_uri)?;
        let is_memory = db_path == Path::new(":memory:");
        let db_existed = is_memory || db_path.exists();

        if !is_memory {
            if let Some(parent) = db_path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)?;
                }
            }
        }

        let db_url = db_path.to_string_lossy().to_string();
        let pool = tokio::task::spawn_blocking(move || -> Result<SqlitePool> {
            let manager = ConnectionManager::<SqliteConnection>::new(db_url);
            let pool = Pool::builder()
                .max_size(max_size)
                .min_idle(min_idle)
                .build(manager)?;
            Ok(pool)
        })
        .await
        .context("sqlite pool init task panicked")??;

        if !db_existed {
            info!("Created new {} database", db_type);
        }

        Ok(Self {
            pool,
            db_type: db_kind,
        })
    }

    pub async fn run_migrations(&self) -> Result<()> {
        info!("Running database migrations for {}", self.db_type);

        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || -> Result<()> {
            let mut conn = pool.get()?;
            conn.batch_execute(SQLITE_MIGRATIONS)?;
            Ok(())
        })
        .await
        .context("sqlite migration task panicked")??;

        info!("Database migrations completed");
        Ok(())
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub fn stores(&self) -> SqliteStores {
        SqliteStores::new(self.pool.clone())
    }
}

fn sqlite_path_from_uri(db_uri: &str) -> Result<PathBuf> {
    if db_uri.is_empty() {
        anyhow::bail!("database uri cannot be empty");
    }

    let path = db_uri
        .strip_prefix("sqlite://")
        .or_else(|| db_uri.strip_prefix("sqlite:"))
        .unwrap_or(db_uri);

    if path.is_empty() {
        anyhow::bail!("database uri '{}' does not contain a sqlite path", db_uri);
    }

    Ok(PathBuf::from(path))
}

const SQLITE_MIGRATIONS: &str = r#"
CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY,
    mxid TEXT NOT NULL UNIQUE,
    dingtalk_user_id TEXT,
    is_whitelisted BOOLEAN NOT NULL DEFAULT FALSE,
    is_admin BOOLEAN NOT NULL DEFAULT FALSE,
    management_room TEXT,
    timezone TEXT,
    next_batch TEXT
);

CREATE TABLE IF NOT EXISTS puppets (
    id INTEGER PRIMARY KEY,
    dingtalk_id TEXT NOT NULL UNIQUE,
    mxid TEXT NOT NULL UNIQUE,
    displayname TEXT NOT NULL,
    avatar_url TEXT,
    next_batch TEXT,
    is_online BOOLEAN NOT NULL DEFAULT FALSE,
    name_set BOOLEAN NOT NULL DEFAULT FALSE,
    avatar_set BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE TABLE IF NOT EXISTS portals (
    id INTEGER PRIMARY KEY,
    dingtalk_conversation_id TEXT NOT NULL UNIQUE,
    mxid TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    topic TEXT,
    avatar_url TEXT,
    encrypted BOOLEAN NOT NULL DEFAULT FALSE,
    room_type TEXT NOT NULL DEFAULT 'group',
    creator_mxid TEXT NOT NULL,
    last_event TEXT
);

CREATE TABLE IF NOT EXISTS messages (
    id INTEGER PRIMARY KEY,
    mxid TEXT NOT NULL UNIQUE,
    dingtalk_message_id TEXT NOT NULL,
    room_id TEXT NOT NULL,
    sender TEXT NOT NULL,
    content TEXT NOT NULL,
    msg_type TEXT NOT NULL,
    timestamp INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS room_mappings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    matrix_room_id TEXT NOT NULL UNIQUE,
    dingtalk_conversation_id TEXT NOT NULL UNIQUE,
    dingtalk_conversation_name TEXT,
    dingtalk_conversation_type TEXT NOT NULL DEFAULT 'group',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS user_mappings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    matrix_user_id TEXT NOT NULL UNIQUE,
    dingtalk_user_id TEXT NOT NULL UNIQUE,
    dingtalk_username TEXT,
    dingtalk_avatar TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS message_mappings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    matrix_event_id TEXT NOT NULL UNIQUE,
    dingtalk_message_id TEXT NOT NULL UNIQUE,
    room_id TEXT NOT NULL,
    sender_mxid TEXT NOT NULL,
    sender_dingtalk_id TEXT NOT NULL,
    content_hash TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS processed_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id TEXT NOT NULL UNIQUE,
    event_type TEXT NOT NULL,
    source TEXT NOT NULL,
    processed_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS dead_letters (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source TEXT NOT NULL,
    event_type TEXT NOT NULL,
    dedupe_key TEXT NOT NULL UNIQUE,
    conversation_id TEXT,
    payload TEXT NOT NULL,
    error TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    replay_count INTEGER NOT NULL DEFAULT 0,
    last_replayed_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS media_cache (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    content_hash TEXT NOT NULL,
    media_kind TEXT NOT NULL,
    resource_key TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(content_hash, media_kind)
);

CREATE TABLE IF NOT EXISTS conversation_webhooks (
    conversation_id TEXT PRIMARY KEY,
    webhook_value TEXT NOT NULL,
    expires_at INTEGER,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_room_mappings_matrix_id ON room_mappings(matrix_room_id);
CREATE INDEX IF NOT EXISTS idx_room_mappings_dingtalk_id ON room_mappings(dingtalk_conversation_id);
CREATE INDEX IF NOT EXISTS idx_user_mappings_matrix_id ON user_mappings(matrix_user_id);
CREATE INDEX IF NOT EXISTS idx_user_mappings_dingtalk_id ON user_mappings(dingtalk_user_id);
CREATE INDEX IF NOT EXISTS idx_message_mappings_matrix_id ON message_mappings(matrix_event_id);
CREATE INDEX IF NOT EXISTS idx_message_mappings_dingtalk_id ON message_mappings(dingtalk_message_id);
CREATE INDEX IF NOT EXISTS idx_message_mappings_room ON message_mappings(room_id);
CREATE INDEX IF NOT EXISTS idx_processed_events_event_id ON processed_events(event_id);
CREATE INDEX IF NOT EXISTS idx_dead_letters_status ON dead_letters(status);
CREATE INDEX IF NOT EXISTS idx_media_cache_created_at ON media_cache(created_at);
CREATE INDEX IF NOT EXISTS idx_conversation_webhooks_expires_at ON conversation_webhooks(expires_at);
"#;
