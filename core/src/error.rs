use crate::conversation::Id;

#[derive(thiserror::Error, Debug)]
pub enum CoreError {
    #[error("unsupported schema version: {0}")]
    UnsupportedSchemaVersion(u32),
    #[error("serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("conversation not found: {0}")]
    NotFound(Id),
    #[error("malformed vault: {0}")]
    MalformedVault(String),
    #[error("conversation is being written by another process: {0}")]
    ConversationBusy(Id),
    #[error("mcp protocol error: {0}")]
    Protocol(String),
    #[error("json-rpc error {code}: {message}")]
    JsonRpc { code: i64, message: String },
    #[error("transport closed")]
    TransportClosed,
    #[error("protocol version mismatch: server {0}")]
    VersionMismatch(String),
    #[error("request timed out: {method}")]
    Timeout { method: String },
}

pub type Result<T> = std::result::Result<T, CoreError>;
