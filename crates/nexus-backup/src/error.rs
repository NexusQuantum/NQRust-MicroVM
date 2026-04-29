use thiserror::Error;

#[derive(Debug, Error)]
pub enum BackupError {
    #[error("chunker: {0}")]
    Chunker(String),

    #[error("cipher: {0}")]
    Cipher(String),

    #[error("manifest: {0}")]
    Manifest(String),

    #[error("authentication failed (Poly1305 MAC mismatch)")]
    AuthFailed,

    #[error("manifest version mismatch: got {got}, expected {expected}")]
    ManifestVersion { got: u32, expected: u32 },

    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("other: {0}")]
    Other(#[source] Box<dyn std::error::Error + Send + Sync>),
}
