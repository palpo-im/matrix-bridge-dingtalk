use std::sync::Arc;

#[cfg(any(feature = "postgres", feature = "mysql", feature = "sqlite"))]
use diesel::RunQueryDsl;
#[cfg(feature = "mysql")]
use diesel::mysql::MysqlConnection;
#[cfg(feature = "postgres")]
use diesel::pg::PgConnection;
#[cfg(any(feature = "postgres", feature = "mysql"))]
use diesel::r2d2::{self, ConnectionManager};

use crate::config::{DatabaseConfig as ConfigDatabaseConfig, DbType as ConfigDbType};
#[cfg(feature = "mysql")]
use crate::db::mysql::{MysqlMessageStore, MysqlRoomStore, MysqlUserStore};
#[cfg(feature = "postgres")]
use crate::db::postgres::{
    PostgresMessageStore, PostgresRoomStore, PostgresUserStore,
};
use crate::db::{DatabaseError, MessageStore, RoomStore, UserStore};

#[cfg(feature = "postgres")]
pub type Pool = r2d2::Pool<ConnectionManager<PgConnection>>;
#[cfg(feature = "mysql")]
pub type MysqlPool = r2d2::Pool<ConnectionManager<MysqlConnection>>;

#[cfg(feature = "sqlite")]
use diesel::Connection;
#[cfg(feature = "sqlite")]
use diesel::sqlite::SqliteConnection;

#[cfg(feature = "sqlite")]
use crate::db::sqlite::{SqliteMessageStore, SqliteRoomStore, SqliteUserStore};

#[derive(Clone)]
pub struct DatabaseManager {
    #[cfg(feature = "postgres")]
    postgres_pool: Option<Pool>,
    #[cfg(feature = "mysql")]
    mysql_pool: Option<MysqlPool>,
    #[cfg(feature = "sqlite")]
    sqlite_path: Option<String>,
    room_store: Arc<dyn RoomStore>,
    user_store: Arc<dyn UserStore>,
    message_store: Arc<dyn MessageStore>,
    db_type: DbType,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DbType {
    Postgres,
    Sqlite,
    Mysql,
}

impl From<ConfigDbType> for DbType {
    fn from(value: ConfigDbType) -> Self {
        match value {
            ConfigDbType::Postgres => DbType::Postgres,
            ConfigDbType::Sqlite => DbType::Sqlite,
            ConfigDbType::Mysql => DbType::Mysql,
        }
    }
}

impl DatabaseManager {
    pub async fn new(config: &ConfigDatabaseConfig) -> Result<Self, DatabaseError> {
        let db_type = DbType::from(config.db_type());

        match db_type {
            #[cfg(feature = "postgres")]
            DbType::Postgres => {
                let connection_string = config.connection_string();
                let max_connections = config.max_connections();
                let min_connections = config.min_connections();

                let manager = ConnectionManager::<PgConnection>::new(connection_string);

                let builder = r2d2::Pool::builder()
                    .max_size(max_connections.unwrap_or(10))
                    .min_idle(Some(min_connections.unwrap_or(1)));

                let pool = builder
                    .build(manager)
                    .map_err(|e| DatabaseError::Connection(e.to_string()))?;

                let room_store = Arc::new(PostgresRoomStore::new(pool.clone()));
                let user_store = Arc::new(PostgresUserStore::new(pool.clone()));
                let message_store = Arc::new(PostgresMessageStore::new(pool.clone()));

                Ok(Self {
                    postgres_pool: Some(pool),
                    #[cfg(feature = "mysql")]
                    mysql_pool: None,
                    #[cfg(feature = "sqlite")]
                    sqlite_path: None,
                    room_store,
                    user_store,
                    message_store,
                    db_type,
                })
            }

            #[cfg(feature = "sqlite")]
            DbType::Sqlite => {
                let sqlite_path = config.sqlite_path().unwrap_or_else(|| "dingtalk.db".to_string());

                let room_store = Arc::new(SqliteRoomStore::new(sqlite_path.clone()));
                let user_store = Arc::new(SqliteUserStore::new(sqlite_path.clone()));
                let message_store = Arc::new(SqliteMessageStore::new(sqlite_path.clone()));

                Ok(Self {
                    #[cfg(feature = "postgres")]
                    postgres_pool: None,
                    #[cfg(feature = "mysql")]
                    mysql_pool: None,
                    sqlite_path: Some(sqlite_path),
                    room_store,
                    user_store,
                    message_store,
                    db_type,
                })
            }

            #[cfg(feature = "mysql")]
            DbType::Mysql => {
                let connection_string = config.connection_string();
                let max_connections = config.max_connections();
                let min_connections = config.min_connections();

                let manager = ConnectionManager::<MysqlConnection>::new(connection_string);

                let builder = r2d2::Pool::builder()
                    .max_size(max_connections.unwrap_or(10))
                    .min_idle(Some(min_connections.unwrap_or(1)));

                let pool = builder
                    .build(manager)
                    .map_err(|e| DatabaseError::Connection(e.to_string()))?;

                let room_store = Arc::new(MysqlRoomStore::new(pool.clone()));
                let user_store = Arc::new(MysqlUserStore::new(pool.clone()));
                let message_store = Arc::new(MysqlMessageStore::new(pool.clone()));

                Ok(Self {
                    #[cfg(feature = "postgres")]
                    postgres_pool: None,
                    mysql_pool: Some(pool),
                    #[cfg(feature = "sqlite")]
                    sqlite_path: None,
                    room_store,
                    user_store,
                    message_store,
                    db_type,
                })
            }

            #[allow(unreachable_patterns)]
            _ => Err(DatabaseError::Connection(
                "Database type not supported or feature not enabled".to_string(),
            )),
        }
    }

    pub async fn migrate(&self) -> Result<(), DatabaseError> {
        match self.db_type {
            #[cfg(feature = "postgres")]
            DbType::Postgres => {
                use diesel::RunQueryDsl;
                let pool = self.postgres_pool.as_ref().ok_or_else(|| {
                    DatabaseError::Connection("Postgres pool not initialized".to_string())
                })?;
                let conn = pool
                    .get()
                    .map_err(|e| DatabaseError::Connection(e.to_string()))?;

                diesel::sql_query(include_str!("../../migrations/001_initial.sql"))
                    .execute(&conn)
                    .map_err(|e| DatabaseError::Migration(e.to_string()))?;

                tracing::info!("PostgreSQL migrations completed");
            }

            #[cfg(feature = "sqlite")]
            DbType::Sqlite => {
                let path = self.sqlite_path.as_ref().ok_or_else(|| {
                    DatabaseError::Connection("SQLite path not set".to_string())
                })?;
                let conn = SqliteConnection::establish(path)
                    .map_err(|e| DatabaseError::Connection(e.to_string()))?;

                diesel::sql_query(include_str!("../../migrations/001_initial_sqlite.sql"))
                    .execute(&conn)
                    .map_err(|e| DatabaseError::Migration(e.to_string()))?;

                tracing::info!("SQLite migrations completed");
            }

            #[cfg(feature = "mysql")]
            DbType::Mysql => {
                use diesel::RunQueryDsl;
                let pool = self.mysql_pool.as_ref().ok_or_else(|| {
                    DatabaseError::Connection("MySQL pool not initialized".to_string())
                })?;
                let conn = pool
                    .get()
                    .map_err(|e| DatabaseError::Connection(e.to_string()))?;

                diesel::sql_query(include_str!("../../migrations/001_initial.sql"))
                    .execute(&conn)
                    .map_err(|e| DatabaseError::Migration(e.to_string()))?;

                tracing::info!("MySQL migrations completed");
            }

            #[allow(unreachable_patterns)]
            _ => {}
        }

        Ok(())
    }

    pub fn room_store(&self) -> Arc<dyn RoomStore> {
        self.room_store.clone()
    }

    pub fn user_store(&self) -> Arc<dyn UserStore> {
        self.user_store.clone()
    }

    pub fn message_store(&self) -> Arc<dyn MessageStore> {
        self.message_store.clone()
    }
}
