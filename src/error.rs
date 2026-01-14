//! Error types for uEmacs

use thiserror::Error;

/// Result type alias for uEmacs operations
pub type Result<T> = std::result::Result<T, EditorError>;

/// Editor error types
#[derive(Error, Debug)]
pub enum EditorError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Buffer is read-only")]
    ReadOnly,

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Operation aborted")]
    Aborted,

    #[error("No such buffer: {0}")]
    NoSuchBuffer(String),

    #[error("Cannot delete last window")]
    LastWindow,

    #[error("{0}")]
    Message(String),
}
