use thiserror::Error;

pub type Result<T> = std::result::Result<T, BridgeError>;

#[derive(Error, Debug)]
pub enum BridgeError {
    #[error("Configuration error: {0}")]
    Config(#[from] crate::config::ConfigError),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Matrix error: {0}")]
    Matrix(String),

    #[error("DingTalk error: {0}")]
    DingTalk(String),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Room not found: {0}")]
    RoomNotFound(String),

    #[error("User not found: {0}")]
    UserNotFound(String),

    #[error("Message not found: {0}")]
    MessageNotFound(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<diesel::result::Error> for BridgeError {
    fn from(err: diesel::result::Error) -> Self {
        BridgeError::Database(err.to_string())
    }
}
