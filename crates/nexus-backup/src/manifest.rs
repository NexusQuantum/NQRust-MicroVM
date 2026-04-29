use crate::error::BackupError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const MANIFEST_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChunkRef {
    pub plaintext_offset: u64,
    pub plaintext_length: u32,
    pub plaintext_hash: [u8; 32],
    pub chunk_id: [u8; 32],
    pub ciphertext_length: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Manifest {
    pub version: u32,
    pub backup_id: Uuid,
    pub source_volume_id: Uuid,
    pub source_snapshot_id: Option<Uuid>,
    pub total_plaintext_size: u64,
    pub created_at_unix_seconds: i64,
    pub chunks: Vec<ChunkRef>,
}

impl Manifest {
    pub fn serialize_compressed(&self) -> Result<Vec<u8>, BackupError> {
        let bytes =
            bincode::serialize(self).map_err(|e| BackupError::Manifest(format!("bincode: {e}")))?;
        let compressed = zstd::stream::encode_all(&bytes[..], 3)
            .map_err(|e| BackupError::Manifest(format!("zstd: {e}")))?;
        Ok(compressed)
    }

    pub fn deserialize_compressed(blob: &[u8]) -> Result<Self, BackupError> {
        let bytes = zstd::stream::decode_all(blob)
            .map_err(|e| BackupError::Manifest(format!("zstd decode: {e}")))?;
        let manifest: Manifest = bincode::deserialize(&bytes)
            .map_err(|e| BackupError::Manifest(format!("bincode decode: {e}")))?;
        if manifest.version != MANIFEST_VERSION {
            return Err(BackupError::ManifestVersion {
                got: manifest.version,
                expected: MANIFEST_VERSION,
            });
        }
        Ok(manifest)
    }
}

pub fn chunk_object_key(prefix: &str, chunk_id: &[u8; 32]) -> String {
    let hex = hex::encode(chunk_id);
    if prefix.is_empty() {
        format!("chunks/{}/{}", &hex[..2], hex)
    } else {
        format!(
            "{}/chunks/{}/{}",
            prefix.trim_end_matches('/'),
            &hex[..2],
            hex
        )
    }
}

pub fn manifest_object_key(prefix: &str, backup_id: &Uuid) -> String {
    if prefix.is_empty() {
        format!("manifests/{}.bin", backup_id)
    } else {
        format!(
            "{}/manifests/{}.bin",
            prefix.trim_end_matches('/'),
            backup_id
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manifest() -> Manifest {
        Manifest {
            version: MANIFEST_VERSION,
            backup_id: Uuid::nil(),
            source_volume_id: Uuid::nil(),
            source_snapshot_id: None,
            total_plaintext_size: 12345,
            created_at_unix_seconds: 1735689600,
            chunks: vec![
                ChunkRef {
                    plaintext_offset: 0,
                    plaintext_length: 4096,
                    plaintext_hash: [1u8; 32],
                    chunk_id: [2u8; 32],
                    ciphertext_length: 4128,
                },
                ChunkRef {
                    plaintext_offset: 4096,
                    plaintext_length: 8192,
                    plaintext_hash: [3u8; 32],
                    chunk_id: [4u8; 32],
                    ciphertext_length: 8224,
                },
            ],
        }
    }

    #[test]
    fn manifest_round_trip() {
        let m = sample_manifest();
        let blob = m.serialize_compressed().unwrap();
        let recovered = Manifest::deserialize_compressed(&blob).unwrap();
        assert_eq!(m, recovered);
    }

    #[test]
    fn manifest_version_mismatch_rejected() {
        let mut m = sample_manifest();
        m.version = 999;
        let blob = m.serialize_compressed().unwrap();
        let err = Manifest::deserialize_compressed(&blob).unwrap_err();
        assert!(matches!(
            err,
            BackupError::ManifestVersion {
                got: 999,
                expected: 1
            }
        ));
    }

    #[test]
    fn chunk_key_format() {
        let mut id = [0u8; 32];
        id[0] = 0xab;
        id[1] = 0xcd;
        let key = chunk_object_key("", &id);
        assert!(key.starts_with("chunks/ab/abcd"));
        let key2 = chunk_object_key("backup-prefix/", &id);
        assert!(key2.starts_with("backup-prefix/chunks/ab/abcd"));
    }

    #[test]
    fn manifest_key_format() {
        let id = Uuid::nil();
        assert_eq!(
            manifest_object_key("", &id),
            format!("manifests/{}.bin", id)
        );
        assert_eq!(
            manifest_object_key("p/", &id),
            format!("p/manifests/{}.bin", id)
        );
    }
}
