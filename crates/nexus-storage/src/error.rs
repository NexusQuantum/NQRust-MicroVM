use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("volume already attached")]
    AlreadyAttached,

    #[error("operation not supported: {0}")]
    NotSupported(String),

    #[error("volume not found")]
    NotFound,

    #[error("invalid volume locator: {0}")]
    InvalidLocator(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Catch-all for backend-specific failures that don't categorize cleanly.
    /// Use sparingly — prefer adding a typed variant when an error condition
    /// becomes load-bearing for callers.
    #[error("backend error: {0}")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync>),
}

impl StorageError {
    pub fn backend<E>(err: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Backend(Box::new(err))
    }
}
