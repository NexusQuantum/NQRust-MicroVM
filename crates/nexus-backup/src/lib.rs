//! Pure-Rust backup transforms: FastCDC chunking, BLAKE3 hashing,
//! XChaCha20-Poly1305 convergent encryption, manifest serialization.
//! No I/O. Both manager and agent depend on this crate.

pub mod chunker;
pub mod cipher;
pub mod error;
pub mod manifest;

pub use chunker::{Chunk, Chunker, ChunkerParams};
pub use cipher::{decrypt_chunk, decrypt_manifest, encrypt_chunk, encrypt_manifest, ChunkKey};
pub use error::BackupError;
pub use manifest::{chunk_object_key, manifest_object_key, ChunkRef, Manifest, MANIFEST_VERSION};
