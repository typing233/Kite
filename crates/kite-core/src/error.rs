use thiserror::Error;

#[derive(Debug, Error)]
pub enum KiteError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Plugin error: {0}")]
    Plugin(String),
    #[error("Operation cancelled")]
    Cancelled,
    #[error("{0}")]
    Other(String),
}

pub type KiteResult<T> = Result<T, KiteError>;
