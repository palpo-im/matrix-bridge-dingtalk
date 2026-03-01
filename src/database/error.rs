use thiserror::Error;

pub type DatabaseResult<T> = Result<T, DatabaseError>;

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Query error: {0}")]
    Query(String),

    #[error("Migration error: {0}")]
    Migration(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Already exists: {0}")]
    AlreadyExists(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Pool error: {0}")]
    Pool(String),

    #[error("{0}")]
    Other(String),
}

impl From<diesel::result::Error> for DatabaseError {
    fn from(err: diesel::result::Error) -> Self {
        match err {
            diesel::result::Error::NotFound => {
                DatabaseError::NotFound("Record not found".to_string())
            }
            diesel::result::Error::DatabaseError(_, _) => DatabaseError::Query(err.to_string()),
            _ => DatabaseError::Query(err.to_string()),
        }
    }
}

impl From<diesel::r2d2::Error> for DatabaseError {
    fn from(err: diesel::r2d2::Error) -> Self {
        DatabaseError::Pool(err.to_string())
    }
}

impl From<serde_json::Error> for DatabaseError {
    fn from(err: serde_json::Error) -> Self {
        DatabaseError::Serialization(err.to_string())
    }
}
