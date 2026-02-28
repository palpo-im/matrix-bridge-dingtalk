use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Query error: {0}")]
    Query(String),

    #[error("Migration error: {0}")]
    Migration(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Duplicate entry: {0}")]
    Duplicate(String),

    #[error("Invalid data: {0}")]
    InvalidData(String),

    #[error("Pool error: {0}")]
    Pool(String),
}
