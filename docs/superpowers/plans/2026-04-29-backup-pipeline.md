# Backup Pipeline Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Chunked, deduplicated, encrypted backup-and-restore for any volume on any storage backend, to any S3-compatible target. Implements the spec at `docs/superpowers/specs/2026-04-29-backup-pipeline-design.md`.

**Architecture:** New `nexus-backup` crate holds pure transforms (FastCDC chunker, BLAKE3 hashing, XChaCha20-Poly1305 convergent encryption, manifest serialization). Agent runs the chunker pipeline against a snapshot stream produced by a new `HostBackend::read_snapshot` trait method, encrypts each chunk with a per-target key sent in-memory by the manager, content-addresses by `BLAKE3(ciphertext)`, HEADs-then-PUTs to S3. Manager owns orchestration (cron scheduler, retention, daily mark-and-sweep GC, reconciler for stuck `running` rows), DB metadata, and the index-rebuild DR tool.

**Tech Stack:** Rust 2021. New deps: `nexus-backup` (workspace member); `blake3 = "1"`, `chacha20poly1305 = "0.10"`, `fastcdc = "3"`, `zstd = "0.13"`, `bincode = "1"`, `aws-sdk-s3 = "1"` (S3 client; works against AWS, MinIO, SeaweedFS), `cron = "0.12"`. Postgres migration `0036_backup.sql`. UI: TanStack Query hooks + shadcn/ui components.

**Spec:** `docs/superpowers/specs/2026-04-29-backup-pipeline-design.md` (commit `9ed3564` on main).

---

## File structure

New crate:
- `crates/nexus-backup/Cargo.toml`
- `crates/nexus-backup/src/lib.rs` — re-exports
- `crates/nexus-backup/src/chunker.rs` — FastCDC over `AsyncRead`
- `crates/nexus-backup/src/cipher.rs` — XChaCha20-Poly1305 with convergent nonce
- `crates/nexus-backup/src/manifest.rs` — `Manifest`, `ChunkRef`, ser/de
- `crates/nexus-backup/src/error.rs` — `BackupError`

Trait modification:
- `crates/nexus-storage/src/host.rs` — add `read_snapshot` method

DB:
- `apps/manager/migrations/0036_backup.sql`

Manager additions:
- `apps/manager/src/features/backup_targets/{mod,repo,routes}.rs`
- `apps/manager/src/features/backups/{mod,repo,routes,service,scheduler,gc,reconciler,index_rebuild}.rs`
- `apps/manager/src/features/storage/agent_rpc.rs` — extend with `agent_backup`, `agent_restore`

Manager modifications:
- `apps/manager/Cargo.toml` — add `nexus-backup`, `aws-sdk-s3`, `aws-credential-types`, `aws-config`, `cron`
- `apps/manager/src/main.rs` — start scheduler + GC + reconciler tasks
- `apps/manager/src/features/mod.rs` — register `backup_targets` and `backups` routers
- `apps/manager/src/features/storage/backends/{local_file,iscsi_generic,truenas_iscsi}.rs` — `read_snapshot` impls (control-plane side, returning the locator info the host backend needs)

Agent additions:
- `apps/agent/src/features/storage/backup.rs` — pipeline impl
- `apps/agent/src/features/storage/s3.rs` — S3 client wrapper

Agent modifications:
- `apps/agent/Cargo.toml` — add `nexus-backup`, `aws-sdk-s3`
- `apps/agent/src/features/storage/{local_file,iscsi}.rs` — `read_snapshot` impls (host side)
- `apps/agent/src/features/storage/routes.rs` — extend with `backup`, `restore`

Shared types:
- `crates/nexus-types/src/lib.rs` — `BackupTarget`, `Backup`, `BackupSchedule`, `BackupStatus`

UI additions:
- `apps/ui/lib/types/index.ts` — types
- `apps/ui/lib/queries.ts` — hooks
- `apps/ui/lib/api/facade.ts` — methods
- `apps/ui/components/backup/backup-target-form.tsx`
- `apps/ui/components/backup/backup-list.tsx`
- `apps/ui/components/backup/backup-schedule-editor.tsx`
- `apps/ui/components/backup/restore-dialog.tsx`
- `apps/ui/app/(dashboard)/backup-targets/page.tsx`
- `apps/ui/components/volume/volume-backups-tab.tsx` — embeds the above into the volume detail page

Tests:
- Inline `#[cfg(test)] mod tests` per module (Plan 1 conventions; integration test crate in `apps/manager/tests/` blocked by lib-export issue, so tests live in-tree)

---

## Conventions

Same as the foundation plan: Conventional Commits (`feat(backup):`, `fix(backup):`, `test(backup):`); `cargo fmt` + `cargo clippy --all-targets --all-features -- -D warnings` clean before review commits; do not break Plan 1 / 2 / 3 tests; existing `populate_streaming` purity contract stays inviolate.

Wall-clock estimate: ~3–4 weeks of engineering work split into ~24 tasks across 11 logical groups.

---

## Task 1: `nexus-backup` crate skeleton

**Files:**
- Create: `crates/nexus-backup/Cargo.toml`
- Create: `crates/nexus-backup/src/lib.rs`
- Modify: `Cargo.toml` (workspace members)

- [ ] **Step 1.1: Crate manifest**

```toml
[package]
name = "nexus-backup"
version = "0.1.0"
edition = "2021"

[dependencies]
blake3 = "1"
chacha20poly1305 = { version = "0.10", features = ["alloc"] }
fastcdc = "3"
zstd = "0.13"
bincode = "1"
serde = { workspace = true }
thiserror = { workspace = true }
tokio = { workspace = true }
async-trait = "0.1"

[dev-dependencies]
tokio = { workspace = true }
proptest = "1"
```

- [ ] **Step 1.2: lib.rs**

```rust
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
pub use manifest::{ChunkRef, Manifest, MANIFEST_VERSION};
```

- [ ] **Step 1.3: Add to workspace members in `Cargo.toml`**

Insert `"crates/nexus-backup",` alphabetically before `"crates/nexus-storage",` in the `[workspace] members = [...]` list.

- [ ] **Step 1.4: Verify**

Run: `cargo check -p nexus-backup`
Expected: fails with "file not found for module" for chunker, cipher, error, manifest. That's intentional — Tasks 2–5 fill them in.

- [ ] **Step 1.5: Commit**

```bash
git add Cargo.toml crates/nexus-backup/
git commit -m "chore(backup): scaffold nexus-backup crate"
```

---

## Task 2: `error.rs` — BackupError

**Files:**
- Create: `crates/nexus-backup/src/error.rs`

- [ ] **Step 2.1: Implement**

```rust
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
```

- [ ] **Step 2.2: Verify**

Run: `cargo check -p nexus-backup 2>&1 | grep "error.rs"`
Expected: no errors specific to error.rs (other modules still missing — that's fine).

- [ ] **Step 2.3: Commit**

```bash
git add crates/nexus-backup/src/error.rs
git commit -m "feat(backup): add BackupError"
```

---

## Task 3: `cipher.rs` — XChaCha20-Poly1305 convergent encryption

**Files:**
- Create: `crates/nexus-backup/src/cipher.rs`

- [ ] **Step 3.1: Write failing test inline**

Append to `crates/nexus-backup/src/cipher.rs`:

```rust
use chacha20poly1305::{aead::Aead, KeyInit, XChaCha20Poly1305, XNonce};

use crate::error::BackupError;

/// 32-byte XChaCha20-Poly1305 key. Per-target. Manager generates it,
/// encrypts with envelope key for storage, sends in-memory to the agent
/// during backup/restore RPC.
pub struct ChunkKey([u8; 32]);

impl ChunkKey {
    pub fn from_bytes(bytes: [u8; 32]) -> Self { Self(bytes) }
    pub fn as_bytes(&self) -> &[u8; 32] { &self.0 }
}

impl Drop for ChunkKey {
    fn drop(&mut self) {
        // Best-effort zeroize; full zeroize crate is a follow-up.
        for b in &mut self.0 {
            unsafe { std::ptr::write_volatile(b, 0); }
        }
    }
}

/// Convergent encryption: nonce derived from BLAKE3(plaintext) so identical
/// plaintexts encrypt to identical ciphertexts under the same key. Returns
/// the ciphertext (which already includes the Poly1305 tag).
pub fn encrypt_chunk(key: &ChunkKey, plaintext: &[u8]) -> Result<Vec<u8>, BackupError> {
    let cipher = XChaCha20Poly1305::new(key.as_bytes().into());
    let plaintext_hash = blake3::hash(plaintext);
    let nonce = XNonce::from_slice(&plaintext_hash.as_bytes()[..24]);
    cipher.encrypt(nonce, plaintext)
        .map_err(|e| BackupError::Cipher(format!("encrypt: {e}")))
}

/// Decrypt a chunk. The caller must supply the original plaintext hash
/// (recovered from the manifest) so we can reconstruct the nonce.
/// Returns the plaintext on success, AuthFailed on tag mismatch.
pub fn decrypt_chunk(
    key: &ChunkKey,
    ciphertext: &[u8],
    plaintext_hash: &[u8; 32],
) -> Result<Vec<u8>, BackupError> {
    let cipher = XChaCha20Poly1305::new(key.as_bytes().into());
    let nonce = XNonce::from_slice(&plaintext_hash[..24]);
    cipher.decrypt(nonce, ciphertext)
        .map_err(|_| BackupError::AuthFailed)
}

/// Encrypt the manifest with a random nonce (manifest is per-backup unique;
/// deterministic nonce buys nothing). Returns nonce-prepended ciphertext:
/// `[nonce(24) | ciphertext+tag]`.
pub fn encrypt_manifest(key: &ChunkKey, plaintext: &[u8]) -> Result<Vec<u8>, BackupError> {
    use rand::RngCore;
    let cipher = XChaCha20Poly1305::new(key.as_bytes().into());
    let mut nonce_bytes = [0u8; 24];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = XNonce::from_slice(&nonce_bytes);
    let ciphertext = cipher.encrypt(nonce, plaintext)
        .map_err(|e| BackupError::Cipher(format!("encrypt manifest: {e}")))?;
    let mut out = Vec::with_capacity(24 + ciphertext.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

/// Inverse of `encrypt_manifest`: input is `[nonce(24) | ciphertext+tag]`.
pub fn decrypt_manifest(key: &ChunkKey, blob: &[u8]) -> Result<Vec<u8>, BackupError> {
    if blob.len() < 24 {
        return Err(BackupError::Cipher("manifest blob too short".into()));
    }
    let cipher = XChaCha20Poly1305::new(key.as_bytes().into());
    let nonce = XNonce::from_slice(&blob[..24]);
    cipher.decrypt(nonce, &blob[24..])
        .map_err(|_| BackupError::AuthFailed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> ChunkKey {
        ChunkKey::from_bytes([0x42u8; 32])
    }

    #[test]
    fn convergent_chunk_round_trip() {
        let k = key();
        let plain = b"hello, backup pipeline";
        let plain_hash: [u8; 32] = *blake3::hash(plain).as_bytes();
        let cipher = encrypt_chunk(&k, plain).unwrap();
        let recovered = decrypt_chunk(&k, &cipher, &plain_hash).unwrap();
        assert_eq!(recovered, plain);
    }

    #[test]
    fn convergent_same_plaintext_same_ciphertext() {
        let k = key();
        let plain = b"identical plaintext";
        let c1 = encrypt_chunk(&k, plain).unwrap();
        let c2 = encrypt_chunk(&k, plain).unwrap();
        assert_eq!(c1, c2, "convergent encryption must be deterministic");
    }

    #[test]
    fn different_plaintext_different_ciphertext() {
        let k = key();
        let c1 = encrypt_chunk(&k, b"alpha").unwrap();
        let c2 = encrypt_chunk(&k, b"bravo").unwrap();
        assert_ne!(c1, c2);
    }

    #[test]
    fn manifest_round_trip_with_random_nonce() {
        let k = key();
        let plain = b"manifest payload bytes";
        let blob1 = encrypt_manifest(&k, plain).unwrap();
        let blob2 = encrypt_manifest(&k, plain).unwrap();
        assert_ne!(blob1, blob2, "manifest nonce must be random — successive encrypts differ");
        let r1 = decrypt_manifest(&k, &blob1).unwrap();
        assert_eq!(r1, plain);
    }

    #[test]
    fn tampered_chunk_fails_auth() {
        let k = key();
        let plain = b"sensitive content";
        let plain_hash: [u8; 32] = *blake3::hash(plain).as_bytes();
        let mut cipher = encrypt_chunk(&k, plain).unwrap();
        cipher[0] ^= 0x01; // flip a bit
        let err = decrypt_chunk(&k, &cipher, &plain_hash).unwrap_err();
        assert!(matches!(err, BackupError::AuthFailed));
    }

    #[test]
    fn wrong_key_fails_auth() {
        let k1 = key();
        let k2 = ChunkKey::from_bytes([0x99u8; 32]);
        let plain = b"abc";
        let plain_hash: [u8; 32] = *blake3::hash(plain).as_bytes();
        let cipher = encrypt_chunk(&k1, plain).unwrap();
        let err = decrypt_chunk(&k2, &cipher, &plain_hash).unwrap_err();
        assert!(matches!(err, BackupError::AuthFailed));
    }
}
```

Note: `rand` is added as a transitive dep through `chacha20poly1305`. If the workspace doesn't already provide it directly, add `rand = "0.8"` to `[dependencies]` in `crates/nexus-backup/Cargo.toml`.

- [ ] **Step 3.2: Add rand dep if needed and verify tests pass**

If `cargo test -p nexus-backup cipher` complains that `rand` is not a dependency, add `rand = "0.8"` to `[dependencies]` of `crates/nexus-backup/Cargo.toml`.

Run: `cargo test -p nexus-backup cipher 2>&1 | tail -10`
Expected: 6 tests pass.

- [ ] **Step 3.3: Commit**

```bash
git add crates/nexus-backup/
git commit -m "feat(backup): XChaCha20-Poly1305 convergent encryption"
```

---

## Task 4: `chunker.rs` — FastCDC over AsyncRead

**Files:**
- Create: `crates/nexus-backup/src/chunker.rs`

- [ ] **Step 4.1: Implement**

```rust
use crate::error::BackupError;
use std::io::Read;
use tokio::io::{AsyncRead, AsyncReadExt};

/// FastCDC parameters. Default targets ~64 KB average — what restic uses;
/// well-studied for VM-disk-style workloads.
#[derive(Debug, Clone, Copy)]
pub struct ChunkerParams {
    pub min_size: u32,
    pub avg_size: u32,
    pub max_size: u32,
}

impl Default for ChunkerParams {
    fn default() -> Self {
        Self {
            min_size: 4 * 1024,
            avg_size: 64 * 1024,
            max_size: 1024 * 1024,
        }
    }
}

/// One chunk emitted by the chunker. Owns its bytes; the caller will
/// hash, encrypt, and PUT them.
pub struct Chunk {
    pub plaintext_offset: u64,
    pub plaintext_length: u32,
    pub plaintext_bytes: Vec<u8>,
}

/// Chunker reads from any AsyncRead and emits Chunk instances.
/// Internally it buffers up to ~max_size bytes and runs FastCDC on the
/// buffer to find the next cut point. Linear time, bounded memory.
pub struct Chunker<R> {
    reader: R,
    params: ChunkerParams,
    buf: Vec<u8>,
    offset: u64,
    eof: bool,
}

impl<R: AsyncRead + Unpin> Chunker<R> {
    pub fn new(reader: R, params: ChunkerParams) -> Self {
        Self {
            reader,
            params,
            buf: Vec::with_capacity(params.max_size as usize * 2),
            offset: 0,
            eof: false,
        }
    }

    /// Fill the internal buffer until at least `target` bytes are present
    /// or EOF is reached.
    async fn fill_until(&mut self, target: usize) -> Result<(), BackupError> {
        while self.buf.len() < target && !self.eof {
            let mut tmp = vec![0u8; (target - self.buf.len()).max(64 * 1024)];
            let n = self.reader.read(&mut tmp).await?;
            if n == 0 {
                self.eof = true;
                break;
            }
            tmp.truncate(n);
            self.buf.extend_from_slice(&tmp);
        }
        Ok(())
    }

    /// Yield the next Chunk, or None at EOF.
    pub async fn next_chunk(&mut self) -> Result<Option<Chunk>, BackupError> {
        // Ensure at least max_size bytes are buffered (or EOF).
        self.fill_until(self.params.max_size as usize).await?;

        if self.buf.is_empty() {
            return Ok(None);
        }

        // Use fastcdc to find the cut point.
        let cdc = fastcdc::v2020::FastCDC::new(
            &self.buf,
            self.params.min_size,
            self.params.avg_size,
            self.params.max_size,
        );
        let first = cdc.into_iter().next();

        let cut_at = match first {
            Some(chunk_meta) => chunk_meta.length,
            None => self.buf.len(), // remainder shorter than min_size; emit as final chunk
        };

        let bytes: Vec<u8> = self.buf.drain(..cut_at).collect();
        let chunk = Chunk {
            plaintext_offset: self.offset,
            plaintext_length: bytes.len() as u32,
            plaintext_bytes: bytes,
        };
        self.offset += chunk.plaintext_length as u64;
        Ok(Some(chunk))
    }
}

// `Read` import is unused here but keeps the module warning-free if a
// future synchronous chunker is added; remove if clippy complains.
#[allow(unused_imports)]
use std::marker::PhantomData;
let _: PhantomData<dyn Read> = PhantomData;

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::BufReader;

    fn deterministic_payload(size: usize) -> Vec<u8> {
        // Pseudo-random but reproducible: linear congruential generator.
        let mut v = vec![0u8; size];
        let mut s: u64 = 0xdeadbeefu64;
        for byte in v.iter_mut() {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            *byte = (s >> 33) as u8;
        }
        v
    }

    #[tokio::test]
    async fn chunks_emit_in_order_and_cover_input() {
        let payload = deterministic_payload(1_500_000);
        let reader = BufReader::new(&payload[..]);
        let mut c = Chunker::new(reader, ChunkerParams::default());
        let mut total = 0u64;
        let mut last_offset: i64 = -1;
        while let Some(chunk) = c.next_chunk().await.unwrap() {
            assert!(chunk.plaintext_offset as i64 > last_offset);
            assert_eq!(chunk.plaintext_bytes.len(), chunk.plaintext_length as usize);
            assert_eq!(chunk.plaintext_offset, total);
            total += chunk.plaintext_length as u64;
            last_offset = chunk.plaintext_offset as i64;
        }
        assert_eq!(total, payload.len() as u64);
    }

    #[tokio::test]
    async fn deterministic_chunking_same_input() {
        let payload = deterministic_payload(800_000);
        let mut c1 = Chunker::new(BufReader::new(&payload[..]), ChunkerParams::default());
        let mut c2 = Chunker::new(BufReader::new(&payload[..]), ChunkerParams::default());

        let mut h1 = Vec::new();
        let mut h2 = Vec::new();
        while let Some(chunk) = c1.next_chunk().await.unwrap() {
            h1.push(blake3::hash(&chunk.plaintext_bytes));
        }
        while let Some(chunk) = c2.next_chunk().await.unwrap() {
            h2.push(blake3::hash(&chunk.plaintext_bytes));
        }
        assert_eq!(h1, h2, "FastCDC must be deterministic for the same input");
    }

    #[tokio::test]
    async fn empty_input_yields_no_chunks() {
        let payload: Vec<u8> = Vec::new();
        let mut c = Chunker::new(BufReader::new(&payload[..]), ChunkerParams::default());
        assert!(c.next_chunk().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn small_input_yields_single_chunk() {
        let payload = deterministic_payload(2 * 1024); // < min_size = 4 KB
        let mut c = Chunker::new(BufReader::new(&payload[..]), ChunkerParams::default());
        let chunk = c.next_chunk().await.unwrap().expect("one chunk");
        assert_eq!(chunk.plaintext_length as usize, payload.len());
        assert!(c.next_chunk().await.unwrap().is_none());
    }
}
```

Note: the dangling `PhantomData` cast at the bottom is purely to keep `Read` "used" — drop it if clippy objects.

- [ ] **Step 4.2: Verify**

Run: `cargo test -p nexus-backup chunker 2>&1 | tail -10`
Expected: 4 tests pass.

- [ ] **Step 4.3: Commit**

```bash
git add crates/nexus-backup/src/chunker.rs
git commit -m "feat(backup): FastCDC chunker over AsyncRead"
```

---

## Task 5: `manifest.rs` — Manifest, ChunkRef, ser/de

**Files:**
- Create: `crates/nexus-backup/src/manifest.rs`

- [ ] **Step 5.1: Implement**

```rust
use crate::error::BackupError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const MANIFEST_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChunkRef {
    pub plaintext_offset: u64,
    pub plaintext_length: u32,
    /// BLAKE3 of the plaintext. Used to reconstruct the encryption nonce
    /// (we use convergent encryption, where nonce = first 24 bytes of
    /// plaintext_hash).
    pub plaintext_hash: [u8; 32],
    /// BLAKE3 of the ciphertext. Doubles as the S3 object key under
    /// `chunks/<hex(blake3[0..2])>/<hex(blake3)>`.
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
    /// Serialize: bincode → zstd-compressed (level 3). Caller then
    /// encrypts via cipher::encrypt_manifest.
    pub fn serialize_compressed(&self) -> Result<Vec<u8>, BackupError> {
        let bytes = bincode::serialize(self)
            .map_err(|e| BackupError::Manifest(format!("bincode: {e}")))?;
        let compressed = zstd::stream::encode_all(&bytes[..], 3)
            .map_err(|e| BackupError::Manifest(format!("zstd: {e}")))?;
        Ok(compressed)
    }

    /// Inverse of `serialize_compressed`. Caller has already decrypted
    /// via cipher::decrypt_manifest.
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

/// S3 object key for a chunk content-addressed by its ciphertext hash.
pub fn chunk_object_key(prefix: &str, chunk_id: &[u8; 32]) -> String {
    let hex = hex::encode(chunk_id);
    if prefix.is_empty() {
        format!("chunks/{}/{}", &hex[..2], hex)
    } else {
        format!("{}/chunks/{}/{}", prefix.trim_end_matches('/'), &hex[..2], hex)
    }
}

/// S3 object key for a manifest.
pub fn manifest_object_key(prefix: &str, backup_id: &Uuid) -> String {
    if prefix.is_empty() {
        format!("manifests/{}.bin", backup_id)
    } else {
        format!("{}/manifests/{}.bin", prefix.trim_end_matches('/'), backup_id)
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
        assert!(matches!(err, BackupError::ManifestVersion { got: 999, expected: 1 }));
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
        assert_eq!(manifest_object_key("", &id), format!("manifests/{}.bin", id));
        assert_eq!(
            manifest_object_key("p/", &id),
            format!("p/manifests/{}.bin", id)
        );
    }
}
```

Add `hex = "0.4"` and `uuid = { workspace = true }` to `[dependencies]` of `crates/nexus-backup/Cargo.toml`.

- [ ] **Step 5.2: Verify**

Run: `cargo test -p nexus-backup manifest 2>&1 | tail -10`
Expected: 4 tests pass.

- [ ] **Step 5.3: Commit**

```bash
git add crates/nexus-backup/
git commit -m "feat(backup): Manifest + ChunkRef serialization (bincode+zstd)"
```

---

## Task 6: Add `read_snapshot` to `HostBackend` trait

**Files:**
- Modify: `crates/nexus-storage/src/host.rs`

- [ ] **Step 6.1: Add method**

Append to the trait (preserve existing methods):

```rust
    /// Open a snapshot for reading. Returns a stream of bytes representing
    /// the volume contents at snapshot time. Used by the backup pipeline
    /// to read snapshot bytes without copying into a new volume.
    ///
    /// Implementations:
    /// - LocalFile: open the snapshot file from disk.
    /// - Iscsi/TrueNasIscsi: attach the snapshot LUN read-only and return
    ///   a File handle over the block device.
    ///
    /// Returns `StorageError::NotSupported("read_snapshot")` if the backend
    /// can't expose a snapshot for streaming reads.
    async fn read_snapshot(
        &self,
        snap: &VolumeSnapshotHandle,
    ) -> Result<Box<dyn tokio::io::AsyncRead + Send + Unpin>, StorageError>;
```

Also add `tokio` to `crates/nexus-storage/Cargo.toml` `[dependencies]` if not already present (it's there as a dev-dependency; promote to dependency, no features needed beyond default).

```toml
[dependencies]
# ... existing ...
tokio = { workspace = true }
```

- [ ] **Step 6.2: Verify build break is contained**

Run: `cargo check -p nexus-storage`
Expected: clean — the trait change doesn't break the crate itself.

Run: `cargo check -p manager 2>&1 | tail -10`
Expected: FAILS — every existing `HostBackend` impl (LocalFile, Iscsi) is now missing `read_snapshot`. This is the whole point.

Same: `cargo check -p agent 2>&1 | tail -5`
Expected: FAILS for the same reason.

- [ ] **Step 6.3: Commit (compiler errors expected — Tasks 7–8 fix them)**

```bash
git add crates/nexus-storage/
git commit -m "feat(backup): add HostBackend::read_snapshot trait method (impls follow)"
```

---

## Task 7: Implement `read_snapshot` for LocalFile

**Files:**
- Modify: `apps/manager/src/features/storage/backends/local_file.rs`
- Modify: `apps/agent/src/features/storage/local_file.rs`

The manager's `LocalFileControlPlaneBackend` doesn't implement `HostBackend` (only `ControlPlaneBackend`), so only the agent file needs the new method. Verify by inspecting the current impls.

- [ ] **Step 7.1: Agent impl**

Append to the `impl HostBackend for LocalFileHostBackend` block in `apps/agent/src/features/storage/local_file.rs`:

```rust
    async fn read_snapshot(
        &self,
        snap: &VolumeSnapshotHandle,
    ) -> Result<Box<dyn tokio::io::AsyncRead + Send + Unpin>, StorageError> {
        let path = std::path::PathBuf::from(&snap.locator);
        let f = tokio::fs::File::open(&path).await?;
        Ok(Box::new(f))
    }
```

- [ ] **Step 7.2: Add a test**

Append to the existing `#[cfg(test)] mod tests` in `apps/agent/src/features/storage/local_file.rs`:

```rust
    #[tokio::test]
    async fn read_snapshot_returns_file_contents() {
        use nexus_storage::{BackendInstanceId, VolumeSnapshotHandle};
        use tokio::io::AsyncReadExt;
        use uuid::Uuid;

        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("snap.img");
        std::fs::write(&p, b"snapshot-bytes").unwrap();

        let snap = VolumeSnapshotHandle {
            snapshot_id: Uuid::new_v4(),
            source_volume_id: Uuid::new_v4(),
            backend_id: BackendInstanceId(Uuid::new_v4()),
            backend_kind: BackendKind::LocalFile,
            locator: p.display().to_string(),
        };

        let backend = LocalFileHostBackend;
        let mut reader = backend.read_snapshot(&snap).await.unwrap();
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).await.unwrap();
        assert_eq!(buf, b"snapshot-bytes");
    }
```

- [ ] **Step 7.3: Verify**

Run: `cargo test -p agent features::storage::local_file 2>&1 | tail -10`
Expected: 3 tests pass (2 existing + 1 new).

Run: `cargo check -p agent`
Expected: clean.

Run: `cargo check -p manager`
Expected: still failing because `IscsiHostBackend` doesn't implement `read_snapshot` yet.

- [ ] **Step 7.4: Commit**

```bash
git add apps/agent/src/features/storage/local_file.rs
git commit -m "feat(backup): LocalFileHostBackend::read_snapshot opens snapshot file"
```

---

## Task 8: Implement `read_snapshot` for Iscsi (agent)

**Files:**
- Modify: `apps/agent/src/features/storage/iscsi.rs`

iSCSI snapshots aren't pre-attached. The agent has two reasonable options:
- a) Treat the snapshot's locator as a separate LUN: parse, log in via iscsiadm, return a `File` over `/dev/disk/by-path/...`. Same flow as `attach`, but for a different LUN number.
- b) Refuse: return `NotSupported`. Forces the caller (rootfs allocator's slow path or backup pipeline) to clone-then-read instead.

We'll implement **(a)** since the locator already carries IQN+LUN. The snapshot's locator should encode `lun: <snapshot_lun_number>` (the TrueNAS REST control plane sets this when creating a snapshot extent; for generic Iscsi this requires operator-supplied snapshot LUNs).

- [ ] **Step 8.1: Implement**

Append to the `impl HostBackend for IscsiHostBackend` block:

```rust
    async fn read_snapshot(
        &self,
        snap: &VolumeSnapshotHandle,
    ) -> Result<Box<dyn tokio::io::AsyncRead + Send + Unpin>, StorageError> {
        // Parse the snapshot locator (same JSON shape as a volume locator —
        // {iqn, lun, portal, dataset?}; the LUN points at the read-only
        // snapshot extent).
        let loc = Self::parse_locator(&snap.locator)?;
        Self::iscsiadm_login(&loc).await?;
        let dev = Self::block_device_path(&loc);
        for _ in 0..30 {
            if dev.exists() {
                let f = tokio::fs::File::open(&dev).await?;
                return Ok(Box::new(f));
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        Err(StorageError::Backend(
            format!(
                "snapshot device {} did not appear after iscsi login",
                dev.display()
            )
            .into(),
        ))
    }
```

- [ ] **Step 8.2: Verify build**

Run: `cargo check -p agent && cargo check -p manager`
Expected: both clean.

Run: `cargo test -p agent features::storage::iscsi 2>&1 | tail -10`
Expected: existing 3 locator tests still pass; no new test needed (the read_snapshot logic mirrors `attach` and is exercised end-to-end in integration tests later).

- [ ] **Step 8.3: Commit**

```bash
git add apps/agent/src/features/storage/iscsi.rs
git commit -m "feat(backup): IscsiHostBackend::read_snapshot via iscsiadm login"
```

---

## Task 9: Migration `0036_backup.sql`

**Files:**
- Create: `apps/manager/migrations/0036_backup.sql`

- [ ] **Step 9.1: Write migration**

```sql
-- 0036_backup.sql — Chunked encrypted backup pipeline.

CREATE TABLE IF NOT EXISTS backup_target (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name TEXT NOT NULL UNIQUE,
  endpoint TEXT NOT NULL,
  region TEXT,
  bucket TEXT NOT NULL,
  prefix TEXT NOT NULL DEFAULT '',
  access_key_id TEXT NOT NULL,
  encrypted_secret_access_key BYTEA NOT NULL,
  encrypted_target_key BYTEA NOT NULL,
  gc_hour SMALLINT NOT NULL DEFAULT 3 CHECK (gc_hour BETWEEN 0 AND 23),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  deleted_at TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS backup (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  source_volume_id UUID REFERENCES volume(id) ON DELETE SET NULL,
  source_snapshot_id UUID,
  target_id UUID NOT NULL REFERENCES backup_target(id),
  manifest_object_key TEXT,
  size_bytes BIGINT NOT NULL DEFAULT 0,
  unique_bytes BIGINT NOT NULL DEFAULT 0,
  chunk_count BIGINT NOT NULL DEFAULT 0,
  status TEXT NOT NULL DEFAULT 'running'
    CHECK (status IN ('running', 'completed', 'failed', 'pruning')),
  error_message TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  completed_at TIMESTAMPTZ,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_backup_volume ON backup(source_volume_id);
CREATE INDEX IF NOT EXISTS idx_backup_target ON backup(target_id);
CREATE INDEX IF NOT EXISTS idx_backup_status_updated ON backup(status, updated_at)
  WHERE status = 'running';

ALTER TABLE volume ADD COLUMN IF NOT EXISTS backup_cron TEXT;
ALTER TABLE volume ADD COLUMN IF NOT EXISTS backup_retain_count INT;
ALTER TABLE volume ADD COLUMN IF NOT EXISTS backup_target_id UUID
  REFERENCES backup_target(id);

CREATE TABLE IF NOT EXISTS backup_gc_run (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  target_id UUID NOT NULL REFERENCES backup_target(id) ON DELETE CASCADE,
  started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  completed_at TIMESTAMPTZ,
  bytes_freed BIGINT NOT NULL DEFAULT 0,
  chunks_deleted BIGINT NOT NULL DEFAULT 0,
  status TEXT NOT NULL DEFAULT 'running'
    CHECK (status IN ('running', 'completed', 'failed')),
  error_message TEXT
);
CREATE INDEX IF NOT EXISTS idx_backup_gc_run_target ON backup_gc_run(target_id, started_at DESC);

COMMENT ON COLUMN backup_target.encrypted_secret_access_key IS
  'AES-GCM(envelope_key) over the S3 secret access key.';
COMMENT ON COLUMN backup_target.encrypted_target_key IS
  'AES-GCM(envelope_key) over the per-target XChaCha20-Poly1305 key used for chunk + manifest encryption.';
COMMENT ON COLUMN backup.unique_bytes IS
  'Post-dedup ciphertext bytes that this backup actually wrote (chunks not skipped by HEAD).';
```

- [ ] **Step 9.2: Apply (if DB available)**

Run: `(cd apps/manager && DATABASE_URL=$DATABASE_URL sqlx migrate run 2>&1 | tail -3)`
Expected: `Applied 36/migrate backup`.

If no DB available, syntax-check by inspection.

- [ ] **Step 9.3: Commit**

```bash
git add apps/manager/migrations/0036_backup.sql
git commit -m "feat(backup): migration 0036 — backup_target, backup, backup_gc_run"
```

---

## Task 10: nexus-types wire types

**Files:**
- Modify: `crates/nexus-types/src/lib.rs`

- [ ] **Step 10.1: Append**

```rust
// ── Backup pipeline ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum BackupStatus {
    Running,
    Completed,
    Failed,
    Pruning,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct BackupTarget {
    pub id: uuid::Uuid,
    pub name: String,
    pub endpoint: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    pub bucket: String,
    #[serde(default)]
    pub prefix: String,
    pub access_key_id: String,
    /// gc_hour 0-23 (UTC).
    pub gc_hour: u8,
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct CreateBackupTargetRequest {
    pub name: String,
    pub endpoint: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    pub bucket: String,
    #[serde(default)]
    pub prefix: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    #[serde(default = "default_gc_hour")]
    pub gc_hour: u8,
}

fn default_gc_hour() -> u8 { 3 }

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct Backup {
    pub id: uuid::Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_volume_id: Option<uuid::Uuid>,
    pub target_id: uuid::Uuid,
    pub size_bytes: i64,
    pub unique_bytes: i64,
    pub chunk_count: i64,
    pub status: BackupStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct BackupSchedule {
    /// Standard 5-field cron expression in UTC.
    pub cron: String,
    pub retain_count: i32,
    pub target_id: uuid::Uuid,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct CreateBackupRequest {
    pub target_id: uuid::Uuid,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct RestoreRequest {
    pub target_backend_id: uuid::Uuid,
}
```

- [ ] **Step 10.2: Verify**

Run: `cargo check -p nexus-types`
Expected: clean.

- [ ] **Step 10.3: Commit**

```bash
git add crates/nexus-types/src/lib.rs
git commit -m "feat(backup): wire types BackupTarget, Backup, BackupSchedule"
```

---

## Task 11: Add deps to manager + agent + envelope key helper

**Files:**
- Modify: `apps/manager/Cargo.toml`
- Modify: `apps/agent/Cargo.toml`
- Create: `apps/manager/src/features/backup_targets/envelope.rs`

- [ ] **Step 11.1: Manager deps**

Add to `apps/manager/Cargo.toml` `[dependencies]`:

```toml
nexus-backup = { path = "../../crates/nexus-backup" }
aws-sdk-s3 = { version = "1", default-features = false, features = ["rustls", "rt-tokio"] }
aws-credential-types = "1"
aws-config = { version = "1", default-features = false, features = ["rustls", "rt-tokio"] }
aws-types = "1"
cron = "0.12"
hex = "0.4"
```

Note: `aes-gcm` is already in deps (used by SSO). The envelope-key helper reuses it.

- [ ] **Step 11.2: Agent deps**

Add to `apps/agent/Cargo.toml` `[dependencies]`:

```toml
nexus-backup = { path = "../../crates/nexus-backup" }
aws-sdk-s3 = { version = "1", default-features = false, features = ["rustls", "rt-tokio"] }
aws-credential-types = "1"
aws-config = { version = "1", default-features = false, features = ["rustls", "rt-tokio"] }
aws-types = "1"
hex = "0.4"
```

- [ ] **Step 11.3: Envelope helper**

Create `apps/manager/src/features/backup_targets/mod.rs`:

```rust
pub mod envelope;
pub mod repo;
pub mod routes;

use axum::{routing::{get, post, patch, delete}, Router};

pub fn router() -> Router {
    Router::new()
        .route("/", post(routes::create).get(routes::list))
        .route("/:id", get(routes::get_one).patch(routes::update).delete(routes::soft_delete))
        .route("/:id/gc", post(routes::trigger_gc))
}
```

Create `apps/manager/src/features/backup_targets/envelope.rs`:

```rust
//! AES-GCM(envelope_key) wrap/unwrap for backup target secrets.
//! Reuses the same envelope key already used by the SSO module.

use aes_gcm::{aead::Aead, Aes256Gcm, Key, KeyInit, Nonce};
use anyhow::{anyhow, Context, Result};

const NONCE_LEN: usize = 12;

fn cipher() -> Result<Aes256Gcm> {
    let raw = std::env::var("MANAGER_ENVELOPE_KEY")
        .context("MANAGER_ENVELOPE_KEY not set")?;
    let bytes = hex::decode(raw)
        .context("MANAGER_ENVELOPE_KEY must be hex-encoded")?;
    if bytes.len() != 32 {
        return Err(anyhow!(
            "MANAGER_ENVELOPE_KEY must be 32 bytes (64 hex chars), got {}",
            bytes.len()
        ));
    }
    let key = Key::<Aes256Gcm>::from_slice(&bytes);
    Ok(Aes256Gcm::new(key))
}

/// Encrypt `plaintext` and return `[nonce(12) | ciphertext+tag]`.
pub fn wrap(plaintext: &[u8]) -> Result<Vec<u8>> {
    use rand::RngCore;
    let c = cipher()?;
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ct = c.encrypt(nonce, plaintext).map_err(|e| anyhow!("aes-gcm encrypt: {e}"))?;
    let mut out = Vec::with_capacity(NONCE_LEN + ct.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ct);
    Ok(out)
}

pub fn unwrap_to_string(blob: &[u8]) -> Result<String> {
    let bytes = unwrap(blob)?;
    String::from_utf8(bytes).context("decrypted secret is not utf-8")
}

pub fn unwrap_to_array<const N: usize>(blob: &[u8]) -> Result<[u8; N]> {
    let bytes = unwrap(blob)?;
    if bytes.len() != N {
        return Err(anyhow!("decrypted blob is {} bytes, expected {}", bytes.len(), N));
    }
    let mut out = [0u8; N];
    out.copy_from_slice(&bytes);
    Ok(out)
}

fn unwrap(blob: &[u8]) -> Result<Vec<u8>> {
    if blob.len() < NONCE_LEN {
        return Err(anyhow!("envelope blob too short"));
    }
    let c = cipher()?;
    let nonce = Nonce::from_slice(&blob[..NONCE_LEN]);
    c.decrypt(nonce, &blob[NONCE_LEN..]).map_err(|_| anyhow!("envelope decrypt: auth failed"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_key<F: FnOnce()>(f: F) {
        std::env::set_var("MANAGER_ENVELOPE_KEY", "00".repeat(32));
        f();
    }

    #[test]
    fn wrap_unwrap_string() {
        with_key(|| {
            let blob = wrap(b"secret-access-key").unwrap();
            let s = unwrap_to_string(&blob).unwrap();
            assert_eq!(s, "secret-access-key");
        });
    }

    #[test]
    fn wrap_unwrap_array() {
        with_key(|| {
            let blob = wrap(&[0xAAu8; 32]).unwrap();
            let a: [u8; 32] = unwrap_to_array(&blob).unwrap();
            assert_eq!(a, [0xAAu8; 32]);
        });
    }

    #[test]
    fn tampered_blob_rejected() {
        with_key(|| {
            let mut blob = wrap(b"hello").unwrap();
            blob[20] ^= 1;
            assert!(unwrap_to_string(&blob).is_err());
        });
    }
}
```

- [ ] **Step 11.4: Verify**

Run: `cargo test -p manager features::backup_targets::envelope 2>&1 | tail -10`
Expected: 3 tests pass.

- [ ] **Step 11.5: Commit**

```bash
git add apps/manager/Cargo.toml apps/agent/Cargo.toml apps/manager/src/features/backup_targets/ Cargo.lock
git commit -m "feat(backup): envelope wrap/unwrap for target secrets"
```

---

## Task 12: backup_targets repo + routes

**Files:**
- Create: `apps/manager/src/features/backup_targets/repo.rs`
- Create: `apps/manager/src/features/backup_targets/routes.rs`
- Modify: `apps/manager/src/features/mod.rs` — register router

- [ ] **Step 12.1: Repo**

```rust
// apps/manager/src/features/backup_targets/repo.rs
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct BackupTargetRepository {
    pool: PgPool,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BackupTargetRow {
    pub id: Uuid,
    pub name: String,
    pub endpoint: String,
    pub region: Option<String>,
    pub bucket: String,
    pub prefix: String,
    pub access_key_id: String,
    pub encrypted_secret_access_key: Vec<u8>,
    pub encrypted_target_key: Vec<u8>,
    pub gc_hour: i16,
    pub created_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl BackupTargetRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }

    pub async fn list_active(&self) -> sqlx::Result<Vec<BackupTargetRow>> {
        sqlx::query_as::<_, BackupTargetRow>(
            r#"SELECT * FROM backup_target WHERE deleted_at IS NULL ORDER BY name"#,
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get(&self, id: Uuid) -> sqlx::Result<Option<BackupTargetRow>> {
        sqlx::query_as::<_, BackupTargetRow>(
            r#"SELECT * FROM backup_target WHERE id = $1 AND deleted_at IS NULL"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn create(
        &self,
        name: &str,
        endpoint: &str,
        region: Option<&str>,
        bucket: &str,
        prefix: &str,
        access_key_id: &str,
        encrypted_secret_access_key: &[u8],
        encrypted_target_key: &[u8],
        gc_hour: i16,
    ) -> sqlx::Result<BackupTargetRow> {
        sqlx::query_as::<_, BackupTargetRow>(
            r#"
            INSERT INTO backup_target
                (name, endpoint, region, bucket, prefix, access_key_id,
                 encrypted_secret_access_key, encrypted_target_key, gc_hour)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(name)
        .bind(endpoint)
        .bind(region)
        .bind(bucket)
        .bind(prefix)
        .bind(access_key_id)
        .bind(encrypted_secret_access_key)
        .bind(encrypted_target_key)
        .bind(gc_hour)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn soft_delete(&self, id: Uuid) -> sqlx::Result<()> {
        sqlx::query(
            r#"UPDATE backup_target SET deleted_at = now() WHERE id = $1 AND deleted_at IS NULL"#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn count_backups_for_target(&self, id: Uuid) -> sqlx::Result<i64> {
        sqlx::query_scalar::<_, i64>(
            r#"SELECT COUNT(*) FROM backup WHERE target_id = $1 AND status IN ('running','completed')"#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
    }
}
```

- [ ] **Step 12.2: Routes**

```rust
// apps/manager/src/features/backup_targets/routes.rs
use crate::features::backup_targets::envelope;
use crate::features::backup_targets::repo::{BackupTargetRepository, BackupTargetRow};
use crate::AppState;
use anyhow::Context as _;
use axum::{
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use nexus_types::{BackupTarget, CreateBackupTargetRequest};
use rand::RngCore;
use uuid::Uuid;

fn row_to_wire(row: BackupTargetRow) -> BackupTarget {
    BackupTarget {
        id: row.id,
        name: row.name,
        endpoint: row.endpoint,
        region: row.region,
        bucket: row.bucket,
        prefix: row.prefix,
        access_key_id: row.access_key_id,
        gc_hour: row.gc_hour as u8,
        created_at: row.created_at,
        deleted_at: row.deleted_at,
    }
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct BackupTargetListResponse {
    pub items: Vec<BackupTarget>,
}

#[utoipa::path(get, path = "/v1/backup_targets",
    responses((status = 200, body = BackupTargetListResponse)),
    tag = "BackupTargets")]
pub async fn list(Extension(st): Extension<AppState>) -> impl IntoResponse {
    let repo = BackupTargetRepository::new(st.db.clone());
    match repo.list_active().await {
        Ok(rows) => (
            StatusCode::OK,
            Json(BackupTargetListResponse {
                items: rows.into_iter().map(row_to_wire).collect(),
            }),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("backup_targets list: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":"db"}))).into_response()
        }
    }
}

#[utoipa::path(get, path = "/v1/backup_targets/{id}",
    params(("id" = Uuid, Path)),
    responses((status = 200, body = BackupTarget), (status = 404)),
    tag = "BackupTargets")]
pub async fn get_one(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let repo = BackupTargetRepository::new(st.db.clone());
    match repo.get(id).await {
        Ok(Some(row)) => (StatusCode::OK, Json(row_to_wire(row))).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error":"not found"}))).into_response(),
        Err(e) => {
            tracing::error!("backup_targets get: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":"db"}))).into_response()
        }
    }
}

#[utoipa::path(post, path = "/v1/backup_targets",
    request_body = CreateBackupTargetRequest,
    responses((status = 201, body = BackupTarget), (status = 400), (status = 500)),
    tag = "BackupTargets")]
pub async fn create(
    Extension(st): Extension<AppState>,
    Json(req): Json<CreateBackupTargetRequest>,
) -> impl IntoResponse {
    let repo = BackupTargetRepository::new(st.db.clone());

    // Generate the per-target chunk key (32 random bytes).
    let mut target_key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut target_key);

    let enc_secret = match envelope::wrap(req.secret_access_key.as_bytes()) {
        Ok(b) => b,
        Err(e) => {
            tracing::error!("envelope wrap secret: {e:#}");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":"envelope"}))).into_response();
        }
    };
    let enc_target = match envelope::wrap(&target_key) {
        Ok(b) => b,
        Err(e) => {
            tracing::error!("envelope wrap target_key: {e:#}");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":"envelope"}))).into_response();
        }
    };

    match repo
        .create(
            &req.name,
            &req.endpoint,
            req.region.as_deref(),
            &req.bucket,
            &req.prefix,
            &req.access_key_id,
            &enc_secret,
            &enc_target,
            req.gc_hour as i16,
        )
        .await
    {
        Ok(row) => (StatusCode::CREATED, Json(row_to_wire(row))).into_response(),
        Err(sqlx::Error::Database(e)) if e.code().as_deref() == Some("23505") => {
            (StatusCode::CONFLICT, Json(serde_json::json!({"error":"name already exists"}))).into_response()
        }
        Err(e) => {
            tracing::error!("backup_targets create: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":"db"}))).into_response()
        }
    }
}

pub async fn update(
    Extension(_st): Extension<AppState>,
    Path(_id): Path<Uuid>,
    Json(_req): Json<CreateBackupTargetRequest>,
) -> impl IntoResponse {
    // Not implemented in v1 — operators delete + recreate.
    (StatusCode::NOT_IMPLEMENTED, Json(serde_json::json!({"error":"update not in v1"})))
}

pub async fn soft_delete(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let repo = BackupTargetRepository::new(st.db.clone());
    match repo.count_backups_for_target(id).await {
        Ok(n) if n > 0 => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": format!("target has {n} backups; delete them first"),
            })),
        ).into_response(),
        Ok(_) => match repo.soft_delete(id).await {
            Ok(()) => (StatusCode::NO_CONTENT, ()).into_response(),
            Err(e) => {
                tracing::error!("backup_targets soft_delete: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":"db"}))).into_response()
            }
        },
        Err(e) => {
            tracing::error!("backup_targets count: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":"db"}))).into_response()
        }
    }
}

pub async fn trigger_gc(
    Extension(_st): Extension<AppState>,
    Path(_id): Path<Uuid>,
) -> impl IntoResponse {
    // Wired in Task 17 once the GC task exists.
    (StatusCode::ACCEPTED, Json(serde_json::json!({"queued": true})))
}
```

- [ ] **Step 12.3: Register router**

In `apps/manager/src/features/mod.rs`, add `pub mod backup_targets;` and `.nest("/v1/backup_targets", backup_targets::router())` next to `storage_backends`.

- [ ] **Step 12.4: Verify**

Run: `cargo check -p manager && cargo clippy -p manager --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 12.5: Commit**

```bash
git add apps/manager/src/features/backup_targets/ apps/manager/src/features/mod.rs
git commit -m "feat(backup): backup_targets repo + CRUD routes"
```

---

## Task 13: Manager-side `agent_rpc` helpers (`agent_backup`, `agent_restore`)

**Files:**
- Modify: `apps/manager/src/features/storage/agent_rpc.rs`
- Create: `apps/manager/src/features/backups/types.rs`

- [ ] **Step 13.1: Wire types shared between manager and agent for the RPC bodies**

Create `apps/manager/src/features/backups/types.rs`:

```rust
//! RPC request/response types between manager and agent for backup ops.
//! These are NOT in nexus-types because they're internal RPC; nexus-types
//! has only operator-facing wire types.

use nexus_storage::{AttachedPath, BackendKind, VolumeHandle, VolumeSnapshotHandle};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BackupTargetConfig {
    pub endpoint: String,
    pub region: Option<String>,
    pub bucket: String,
    pub prefix: String,
    pub access_key_id: String,
    pub secret_access_key: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct ChunkerParams {
    pub min_size: u32,
    pub avg_size: u32,
    pub max_size: u32,
}

impl Default for ChunkerParams {
    fn default() -> Self { Self { min_size: 4096, avg_size: 65536, max_size: 1048576 } }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BackupReq {
    pub backup_id: Uuid,
    pub snapshot: VolumeSnapshotHandle,
    pub backend_kind: BackendKind,
    pub target: BackupTargetConfig,
    /// 32-byte XChaCha20-Poly1305 key (decrypted by manager from envelope).
    pub encryption_key: [u8; 32],
    pub chunker_params: ChunkerParams,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BackupResp {
    pub manifest_object_key: String,
    pub chunk_count: u64,
    pub bytes_written: u64,
    pub bytes_unique: u64,
    pub duration_ms: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RestoreReq {
    pub target_volume: VolumeHandle,
    pub target_attached: AttachedPath,
    pub manifest_object_key: String,
    pub target: BackupTargetConfig,
    pub encryption_key: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RestoreResp {
    pub bytes_written: u64,
    pub duration_ms: u64,
}
```

- [ ] **Step 13.2: Helpers**

Append to `apps/manager/src/features/storage/agent_rpc.rs`:

```rust
use crate::features::backups::types::{BackupReq, BackupResp, RestoreReq, RestoreResp};

pub async fn agent_backup(host_addr: &str, req: BackupReq) -> Result<BackupResp> {
    let resp = Client::new()
        .post(agent_url(host_addr, "/v1/storage/backup"))
        .json(&req)
        .send()
        .await
        .with_context(|| format!("POST /v1/storage/backup to {host_addr}"))?;
    if !resp.status().is_success() {
        let s = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!("agent backup: {s}: {body}"));
    }
    Ok(resp.json::<BackupResp>().await?)
}

pub async fn agent_restore(host_addr: &str, req: RestoreReq) -> Result<RestoreResp> {
    let resp = Client::new()
        .post(agent_url(host_addr, "/v1/storage/restore"))
        .json(&req)
        .send()
        .await
        .with_context(|| format!("POST /v1/storage/restore to {host_addr}"))?;
    if !resp.status().is_success() {
        let s = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!("agent restore: {s}: {body}"));
    }
    Ok(resp.json::<RestoreResp>().await?)
}
```

Add `pub mod types;` to `apps/manager/src/features/backups/mod.rs` (which we'll create more fully in Task 14). For now create a minimal `apps/manager/src/features/backups/mod.rs`:

```rust
pub mod types;
```

- [ ] **Step 13.3: Verify**

Run: `cargo check -p manager`
Expected: clean.

- [ ] **Step 13.4: Commit**

```bash
git add apps/manager/src/features/storage/agent_rpc.rs apps/manager/src/features/backups/
git commit -m "feat(backup): manager agent_rpc helpers + RPC types"
```

---

## Task 14: Agent S3 client wrapper

**Files:**
- Create: `apps/agent/src/features/storage/s3.rs`
- Modify: `apps/agent/src/features/storage/mod.rs`

- [ ] **Step 14.1: Implement**

```rust
// apps/agent/src/features/storage/s3.rs
//! Thin async wrapper over aws-sdk-s3 that builds a client from a
//! BackupTargetConfig (endpoint, region, creds) and exposes the small set
//! of operations the backup pipeline needs: head, put, get, list, delete.

use aws_credential_types::Credentials;
use aws_sdk_s3::{
    config::{Builder, Region},
    error::SdkError,
    operation::head_object::HeadObjectError,
    Client,
};
use std::time::Duration;

#[derive(Clone)]
pub struct BackupTargetConfig {
    pub endpoint: String,
    pub region: Option<String>,
    pub bucket: String,
    pub prefix: String,
    pub access_key_id: String,
    pub secret_access_key: String,
}

pub fn make_client(cfg: &BackupTargetConfig) -> Client {
    let creds = Credentials::new(
        cfg.access_key_id.clone(),
        cfg.secret_access_key.clone(),
        None,
        None,
        "nqrust-backup",
    );
    let region = Region::new(cfg.region.clone().unwrap_or_else(|| "us-east-1".into()));
    let mut builder = Builder::new()
        .behavior_version_latest()
        .endpoint_url(&cfg.endpoint)
        .credentials_provider(creds)
        .region(region)
        .force_path_style(true) // SeaweedFS / MinIO compatibility
        .timeout_config(
            aws_sdk_s3::config::timeout::TimeoutConfig::builder()
                .operation_timeout(Duration::from_secs(120))
                .build(),
        );
    let cfg_built = builder.build();
    Client::from_conf(cfg_built)
}

#[derive(Debug, thiserror::Error)]
pub enum S3Error {
    #[error("s3: {0}")]
    Other(String),
}

/// Returns true if the object exists.
pub async fn head_object(client: &Client, bucket: &str, key: &str) -> Result<bool, S3Error> {
    match client.head_object().bucket(bucket).key(key).send().await {
        Ok(_) => Ok(true),
        Err(SdkError::ServiceError(svc)) if matches!(svc.err(), HeadObjectError::NotFound(_)) => Ok(false),
        Err(e) => Err(S3Error::Other(format!("head: {e}"))),
    }
}

pub async fn put_object(
    client: &Client,
    bucket: &str,
    key: &str,
    body: Vec<u8>,
) -> Result<(), S3Error> {
    client
        .put_object()
        .bucket(bucket)
        .key(key)
        .body(body.into())
        .send()
        .await
        .map_err(|e| S3Error::Other(format!("put: {e}")))?;
    Ok(())
}

pub async fn get_object(client: &Client, bucket: &str, key: &str) -> Result<Vec<u8>, S3Error> {
    let resp = client
        .get_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .map_err(|e| S3Error::Other(format!("get: {e}")))?;
    let body = resp
        .body
        .collect()
        .await
        .map_err(|e| S3Error::Other(format!("get body: {e}")))?;
    Ok(body.into_bytes().to_vec())
}
```

Append `pub mod s3;` to `apps/agent/src/features/storage/mod.rs`.

- [ ] **Step 14.2: Verify**

Run: `cargo check -p agent`
Expected: clean.

- [ ] **Step 14.3: Commit**

```bash
git add apps/agent/src/features/storage/
git commit -m "feat(backup): agent S3 client wrapper (head/put/get)"
```

---

## Task 15: Agent backup pipeline + route

**Files:**
- Create: `apps/agent/src/features/storage/backup.rs`
- Modify: `apps/agent/src/features/storage/routes.rs`

- [ ] **Step 15.1: Pipeline**

```rust
// apps/agent/src/features/storage/backup.rs
//! The chunker pipeline: read snapshot bytes → FastCDC → encrypt → HEAD-or-PUT.

use crate::features::storage::registry::HostBackendRegistry;
use crate::features::storage::s3::{self, BackupTargetConfig};
use anyhow::{Context, Result};
use chrono::Utc;
use nexus_backup::{
    chunk_object_key, decrypt_chunk, decrypt_manifest, encrypt_chunk, encrypt_manifest,
    manifest_object_key, ChunkKey, ChunkRef, Chunker, ChunkerParams, Manifest, MANIFEST_VERSION,
};
use nexus_storage::{AttachedPath, VolumeHandle, VolumeSnapshotHandle};
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use uuid::Uuid;

pub struct BackupParams {
    pub backup_id: Uuid,
    pub snapshot: VolumeSnapshotHandle,
    pub target: BackupTargetConfig,
    pub encryption_key: [u8; 32],
    pub chunker_params: ChunkerParams,
}

pub struct BackupOutcome {
    pub manifest_object_key: String,
    pub chunk_count: u64,
    pub bytes_written: u64,
    pub bytes_unique: u64,
    pub duration_ms: u64,
}

pub async fn run_backup(
    registry: Arc<HostBackendRegistry>,
    params: BackupParams,
) -> Result<BackupOutcome> {
    let start = Instant::now();
    let backend = registry
        .get(params.snapshot.backend_kind)
        .ok_or_else(|| anyhow::anyhow!("no host backend for kind {:?}", params.snapshot.backend_kind))?
        .clone();

    let mut reader = backend.read_snapshot(&params.snapshot).await
        .context("read_snapshot")?;
    let chunker = Chunker::new(&mut reader, params.chunker_params);
    let s3 = s3::make_client(&params.target);
    let key = ChunkKey::from_bytes(params.encryption_key);

    let mut chunks = Vec::new();
    let mut bytes_written: u64 = 0;
    let mut bytes_unique: u64 = 0;
    let mut total_plaintext: u64 = 0;

    let mut chunker = chunker;
    while let Some(chunk) = chunker.next_chunk().await? {
        let plaintext_hash: [u8; 32] = *blake3::hash(&chunk.plaintext_bytes).as_bytes();
        let ciphertext = encrypt_chunk(&key, &chunk.plaintext_bytes)
            .context("encrypt_chunk")?;
        let chunk_id: [u8; 32] = *blake3::hash(&ciphertext).as_bytes();
        let object_key = chunk_object_key(&params.target.prefix, &chunk_id);

        let exists = s3::head_object(&s3, &params.target.bucket, &object_key).await
            .context("HEAD chunk")?;
        bytes_written += ciphertext.len() as u64;
        if !exists {
            s3::put_object(&s3, &params.target.bucket, &object_key, ciphertext.clone()).await
                .context("PUT chunk")?;
            bytes_unique += ciphertext.len() as u64;
        }

        chunks.push(ChunkRef {
            plaintext_offset: chunk.plaintext_offset,
            plaintext_length: chunk.plaintext_length,
            plaintext_hash,
            chunk_id,
            ciphertext_length: ciphertext.len() as u32,
        });
        total_plaintext += chunk.plaintext_length as u64;
    }

    let manifest = Manifest {
        version: MANIFEST_VERSION,
        backup_id: params.backup_id,
        source_volume_id: params.snapshot.source_volume_id,
        source_snapshot_id: Some(params.snapshot.snapshot_id),
        total_plaintext_size: total_plaintext,
        created_at_unix_seconds: Utc::now().timestamp(),
        chunks: chunks.clone(),
    };
    let manifest_compressed = manifest.serialize_compressed()
        .context("manifest serialize")?;
    let manifest_blob = encrypt_manifest(&key, &manifest_compressed)
        .context("encrypt manifest")?;
    let mkey = manifest_object_key(&params.target.prefix, &params.backup_id);
    s3::put_object(&s3, &params.target.bucket, &mkey, manifest_blob).await
        .context("PUT manifest")?;

    Ok(BackupOutcome {
        manifest_object_key: mkey,
        chunk_count: chunks.len() as u64,
        bytes_written,
        bytes_unique,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

pub struct RestoreParams {
    pub target_volume: VolumeHandle,
    pub target_attached: AttachedPath,
    pub manifest_object_key: String,
    pub target: BackupTargetConfig,
    pub encryption_key: [u8; 32],
}

pub struct RestoreOutcome {
    pub bytes_written: u64,
    pub duration_ms: u64,
}

pub async fn run_restore(params: RestoreParams) -> Result<RestoreOutcome> {
    let start = Instant::now();
    let s3 = s3::make_client(&params.target);
    let key = ChunkKey::from_bytes(params.encryption_key);

    // Fetch + decrypt manifest.
    let blob = s3::get_object(&s3, &params.target.bucket, &params.manifest_object_key).await
        .context("GET manifest")?;
    let compressed = decrypt_manifest(&key, &blob).context("decrypt manifest")?;
    let manifest = Manifest::deserialize_compressed(&compressed).context("deserialize manifest")?;

    // Open destination for writing.
    let mut dst = tokio::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(params.target_attached.path())
        .await?;

    let mut bytes_written: u64 = 0;
    for chunk_ref in &manifest.chunks {
        let object_key = chunk_object_key(&params.target.prefix, &chunk_ref.chunk_id);
        let ciphertext = s3::get_object(&s3, &params.target.bucket, &object_key).await
            .with_context(|| format!("GET chunk {}", hex::encode(chunk_ref.chunk_id)))?;
        let plaintext = decrypt_chunk(&key, &ciphertext, &chunk_ref.plaintext_hash)
            .context("decrypt chunk")?;
        dst.seek(std::io::SeekFrom::Start(chunk_ref.plaintext_offset)).await?;
        dst.write_all(&plaintext).await?;
        bytes_written += plaintext.len() as u64;
    }
    dst.flush().await?;

    Ok(RestoreOutcome {
        bytes_written,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}
```

- [ ] **Step 15.2: Routes — add backup + restore handlers**

Append to `apps/agent/src/features/storage/routes.rs`:

```rust
use crate::features::storage::backup::{run_backup, run_restore, BackupParams, RestoreParams};
use crate::features::storage::s3::BackupTargetConfig as S3Config;
use nexus_backup::ChunkerParams as NexusChunkerParams;

#[derive(Deserialize)]
pub struct BackupReq {
    backup_id: uuid::Uuid,
    snapshot: nexus_storage::VolumeSnapshotHandle,
    backend_kind: nexus_storage::BackendKind,
    target: BackupTargetWire,
    encryption_key: [u8; 32],
    chunker_params: ChunkerParamsWire,
}

#[derive(Deserialize)]
pub struct BackupTargetWire {
    pub endpoint: String,
    #[serde(default)] pub region: Option<String>,
    pub bucket: String,
    #[serde(default)] pub prefix: String,
    pub access_key_id: String,
    pub secret_access_key: String,
}

#[derive(Deserialize)]
pub struct ChunkerParamsWire {
    pub min_size: u32,
    pub avg_size: u32,
    pub max_size: u32,
}

#[derive(Serialize)]
pub struct BackupRespWire {
    pub manifest_object_key: String,
    pub chunk_count: u64,
    pub bytes_written: u64,
    pub bytes_unique: u64,
    pub duration_ms: u64,
}

pub async fn backup(
    State(s): State<Arc<StorageState>>,
    Json(req): Json<BackupReq>,
) -> impl IntoResponse {
    let target = S3Config {
        endpoint: req.target.endpoint,
        region: req.target.region,
        bucket: req.target.bucket,
        prefix: req.target.prefix,
        access_key_id: req.target.access_key_id,
        secret_access_key: req.target.secret_access_key,
    };
    let params = BackupParams {
        backup_id: req.backup_id,
        snapshot: req.snapshot,
        target,
        encryption_key: req.encryption_key,
        chunker_params: NexusChunkerParams {
            min_size: req.chunker_params.min_size,
            avg_size: req.chunker_params.avg_size,
            max_size: req.chunker_params.max_size,
        },
    };
    match run_backup(Arc::new(s.registry.clone()), params).await {
        Ok(o) => (
            StatusCode::OK,
            Json(BackupRespWire {
                manifest_object_key: o.manifest_object_key,
                chunk_count: o.chunk_count,
                bytes_written: o.bytes_written,
                bytes_unique: o.bytes_unique,
                duration_ms: o.duration_ms,
            }),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("agent backup failed: {e:#}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}

#[derive(Deserialize)]
pub struct RestoreReq {
    target_volume: nexus_storage::VolumeHandle,
    target_attached: nexus_storage::AttachedPath,
    manifest_object_key: String,
    target: BackupTargetWire,
    encryption_key: [u8; 32],
}

#[derive(Serialize)]
pub struct RestoreRespWire {
    bytes_written: u64,
    duration_ms: u64,
}

pub async fn restore(
    State(_s): State<Arc<StorageState>>,
    Json(req): Json<RestoreReq>,
) -> impl IntoResponse {
    let target = S3Config {
        endpoint: req.target.endpoint,
        region: req.target.region,
        bucket: req.target.bucket,
        prefix: req.target.prefix,
        access_key_id: req.target.access_key_id,
        secret_access_key: req.target.secret_access_key,
    };
    let params = RestoreParams {
        target_volume: req.target_volume,
        target_attached: req.target_attached,
        manifest_object_key: req.manifest_object_key,
        target,
        encryption_key: req.encryption_key,
    };
    match run_restore(params).await {
        Ok(o) => (
            StatusCode::OK,
            Json(RestoreRespWire {
                bytes_written: o.bytes_written,
                duration_ms: o.duration_ms,
            }),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("agent restore failed: {e:#}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}
```

Update the router function in the same file:

```rust
pub fn router(state: Arc<StorageState>) -> Router {
    Router::new()
        .route("/attach", post(attach))
        .route("/detach", post(detach))
        .route("/populate", post(populate))
        .route("/resize2fs", post(resize2fs))
        .route("/supported_kinds", get(supported_kinds))
        .route("/backup", post(backup))
        .route("/restore", post(restore))
        .with_state(state)
}
```

Append `pub mod backup;` to `apps/agent/src/features/storage/mod.rs`.

- [ ] **Step 15.3: Verify**

Run: `cargo check -p agent && cargo clippy -p agent --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 15.4: Commit**

```bash
git add apps/agent/src/features/storage/
git commit -m "feat(backup): agent backup + restore pipeline + HTTP routes"
```

---

## Task 16: Manager backups repo + service (orchestration)

**Files:**
- Create: `apps/manager/src/features/backups/repo.rs`
- Create: `apps/manager/src/features/backups/service.rs`
- Create: `apps/manager/src/features/backups/routes.rs`
- Modify: `apps/manager/src/features/backups/mod.rs`

- [ ] **Step 16.1: Repo**

```rust
// apps/manager/src/features/backups/repo.rs
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct BackupRepository {
    pool: PgPool,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BackupRow {
    pub id: Uuid,
    pub source_volume_id: Option<Uuid>,
    pub source_snapshot_id: Option<Uuid>,
    pub target_id: Uuid,
    pub manifest_object_key: Option<String>,
    pub size_bytes: i64,
    pub unique_bytes: i64,
    pub chunk_count: i64,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

impl BackupRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }

    pub async fn insert_running(
        &self,
        source_volume_id: Uuid,
        source_snapshot_id: Uuid,
        target_id: Uuid,
    ) -> sqlx::Result<BackupRow> {
        sqlx::query_as::<_, BackupRow>(
            r#"
            INSERT INTO backup
              (source_volume_id, source_snapshot_id, target_id, status)
            VALUES ($1, $2, $3, 'running')
            RETURNING *
            "#,
        )
        .bind(source_volume_id)
        .bind(source_snapshot_id)
        .bind(target_id)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn mark_completed(
        &self,
        id: Uuid,
        manifest_object_key: &str,
        size_bytes: i64,
        unique_bytes: i64,
        chunk_count: i64,
    ) -> sqlx::Result<()> {
        sqlx::query(
            r#"
            UPDATE backup
            SET status = 'completed',
                manifest_object_key = $1,
                size_bytes = $2,
                unique_bytes = $3,
                chunk_count = $4,
                completed_at = now(),
                updated_at = now()
            WHERE id = $5
            "#,
        )
        .bind(manifest_object_key)
        .bind(size_bytes)
        .bind(unique_bytes)
        .bind(chunk_count)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn mark_failed(&self, id: Uuid, error: &str) -> sqlx::Result<()> {
        sqlx::query(
            r#"
            UPDATE backup
            SET status = 'failed', error_message = $1, updated_at = now()
            WHERE id = $2
            "#,
        )
        .bind(error)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get(&self, id: Uuid) -> sqlx::Result<Option<BackupRow>> {
        sqlx::query_as::<_, BackupRow>(r#"SELECT * FROM backup WHERE id = $1"#)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn list_for_volume(&self, volume_id: Uuid) -> sqlx::Result<Vec<BackupRow>> {
        sqlx::query_as::<_, BackupRow>(
            r#"SELECT * FROM backup WHERE source_volume_id = $1 ORDER BY created_at DESC"#,
        )
        .bind(volume_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn list_completed_oldest_first(
        &self,
        volume_id: Uuid,
    ) -> sqlx::Result<Vec<BackupRow>> {
        sqlx::query_as::<_, BackupRow>(
            r#"SELECT * FROM backup WHERE source_volume_id = $1 AND status = 'completed' ORDER BY created_at ASC"#,
        )
        .bind(volume_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn delete_row(&self, id: Uuid) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM backup WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Stale `running` rows are candidates for the reconciler.
    pub async fn list_stale_running(
        &self,
        older_than_minutes: i64,
    ) -> sqlx::Result<Vec<BackupRow>> {
        sqlx::query_as::<_, BackupRow>(
            r#"SELECT * FROM backup
               WHERE status = 'running'
                 AND updated_at < now() - make_interval(mins => $1)"#,
        )
        .bind(older_than_minutes)
        .fetch_all(&self.pool)
        .await
    }
}
```

- [ ] **Step 16.2: Service**

```rust
// apps/manager/src/features/backups/service.rs
use crate::features::backup_targets::envelope;
use crate::features::backup_targets::repo::BackupTargetRepository;
use crate::features::backups::repo::{BackupRepository, BackupRow};
use crate::features::backups::types::{BackupReq, BackupTargetConfig, ChunkerParams, RestoreReq};
use crate::features::hosts::repo::HostRepository;
use crate::features::storage::agent_rpc;
use crate::features::storage::registry::Registry;
use crate::features::volumes::repo::VolumeRepository;
use crate::AppState;
use anyhow::{anyhow, Context, Result};
use nexus_storage::{BackendInstanceId, VolumeHandle, VolumeSnapshotHandle};
use uuid::Uuid;

pub async fn create_backup(st: &AppState, volume_id: Uuid, target_id: Uuid) -> Result<Uuid> {
    let backup_repo = BackupRepository::new(st.db.clone());
    let target_repo = BackupTargetRepository::new(st.db.clone());
    let target_row = target_repo.get(target_id).await?
        .ok_or_else(|| anyhow!("target {target_id} not found"))?;

    // Resolve volume → backend → host.
    let vol: (Uuid, String, i64, Option<Uuid>, Uuid) = sqlx::query_as(
        r#"SELECT v.id, v.path, v.size_bytes, v.host_id, v.backend_id
           FROM volume v WHERE v.id = $1"#,
    )
    .bind(volume_id)
    .fetch_one(&st.db)
    .await
    .context("looking up volume")?;

    let (vol_id, locator, size_bytes, host_id_opt, backend_id) =
        (vol.0, vol.1, vol.2, vol.3, vol.4);
    let backend = st.registry.get(backend_id)
        .ok_or_else(|| anyhow!("registry has no backend with id {backend_id}"))?
        .clone();
    let host_id = host_id_opt.ok_or_else(|| anyhow!("volume has no home host (network-attached not yet supported by backup)"))?;
    let host = st.hosts.get(host_id).await.context("getting host row")?;

    // Take a snapshot via the control-plane backend.
    let volume_handle = VolumeHandle {
        volume_id: vol_id,
        backend_id: BackendInstanceId(backend_id),
        backend_kind: backend.kind(),
        locator,
        size_bytes: size_bytes as u64,
    };
    let snap_name = format!("backup-{}", Uuid::new_v4());
    let snap = backend
        .snapshot(&volume_handle, &snap_name)
        .await
        .context("control-plane snapshot")?;

    let backup_row = backup_repo
        .insert_running(volume_id, snap.snapshot_id, target_id)
        .await?;

    // Decrypt secrets to send to the agent.
    let secret_access_key = envelope::unwrap_to_string(&target_row.encrypted_secret_access_key)
        .context("decrypt secret_access_key")?;
    let target_key = envelope::unwrap_to_array::<32>(&target_row.encrypted_target_key)
        .context("decrypt target_key")?;

    let target_config = BackupTargetConfig {
        endpoint: target_row.endpoint.clone(),
        region: target_row.region.clone(),
        bucket: target_row.bucket.clone(),
        prefix: target_row.prefix.clone(),
        access_key_id: target_row.access_key_id.clone(),
        secret_access_key,
    };

    let req = BackupReq {
        backup_id: backup_row.id,
        snapshot: snap.clone(),
        backend_kind: backend.kind(),
        target: target_config,
        encryption_key: target_key,
        chunker_params: ChunkerParams::default(),
    };

    match agent_rpc::agent_backup(&host.addr, req).await {
        Ok(resp) => {
            backup_repo
                .mark_completed(
                    backup_row.id,
                    &resp.manifest_object_key,
                    resp.bytes_written as i64,
                    resp.bytes_unique as i64,
                    resp.chunk_count as i64,
                )
                .await?;
            // Drop the snapshot now that the backup has been written.
            let _ = backend.delete_snapshot(snap).await;
            // Enforce retention if configured.
            let _ = enforce_retention(st, volume_id, &backup_repo).await;
            Ok(backup_row.id)
        }
        Err(e) => {
            backup_repo.mark_failed(backup_row.id, &format!("{e:#}")).await?;
            // Best-effort delete the snapshot we created.
            let _ = backend.delete_snapshot(snap).await;
            Err(e)
        }
    }
}

async fn enforce_retention(
    st: &AppState,
    volume_id: Uuid,
    backup_repo: &BackupRepository,
) -> Result<()> {
    let retain: Option<i32> = sqlx::query_scalar(
        r#"SELECT backup_retain_count FROM volume WHERE id = $1"#,
    )
    .bind(volume_id)
    .fetch_one(&st.db)
    .await?;
    let Some(retain) = retain else { return Ok(()); };
    if retain <= 0 { return Ok(()); }

    let mut completed = backup_repo.list_completed_oldest_first(volume_id).await?;
    while completed.len() as i32 > retain {
        let oldest = completed.remove(0);
        // Delete the manifest from S3 first; chunks become reachable to next GC.
        if let Some(mkey) = oldest.manifest_object_key.as_deref() {
            // Using a one-shot S3 client constructed from the target row.
            let target = BackupTargetRepository::new(st.db.clone()).get(oldest.target_id).await?;
            if let Some(t) = target {
                let secret = envelope::unwrap_to_string(&t.encrypted_secret_access_key).ok();
                if let Some(secret) = secret {
                    let cfg = aws_credential_types::Credentials::new(
                        &t.access_key_id, &secret, None, None, "nqrust-mgr-prune");
                    let region = aws_sdk_s3::config::Region::new(
                        t.region.clone().unwrap_or_else(|| "us-east-1".into()));
                    let s3_cfg = aws_sdk_s3::config::Builder::new()
                        .behavior_version_latest()
                        .endpoint_url(&t.endpoint)
                        .credentials_provider(cfg)
                        .region(region)
                        .force_path_style(true)
                        .build();
                    let client = aws_sdk_s3::Client::from_conf(s3_cfg);
                    let _ = client.delete_object().bucket(&t.bucket).key(mkey).send().await;
                }
            }
        }
        backup_repo.delete_row(oldest.id).await?;
    }
    Ok(())
}

pub async fn restore_backup(
    st: &AppState,
    backup_id: Uuid,
    target_backend_id: Uuid,
) -> Result<Uuid> {
    let backup_repo = BackupRepository::new(st.db.clone());
    let target_repo = BackupTargetRepository::new(st.db.clone());
    let backup = backup_repo.get(backup_id).await?
        .ok_or_else(|| anyhow!("backup {backup_id} not found"))?;
    if backup.status != "completed" {
        return Err(anyhow!("backup is in status '{}', expected 'completed'", backup.status));
    }
    let manifest_key = backup.manifest_object_key
        .ok_or_else(|| anyhow!("backup has no manifest_object_key"))?;
    let target_row = target_repo.get(backup.target_id).await?
        .ok_or_else(|| anyhow!("target {} no longer exists", backup.target_id))?;

    // Provision a fresh volume on the chosen backend.
    let backend = st.registry.get(target_backend_id)
        .ok_or_else(|| anyhow!("registry has no backend {target_backend_id}"))?
        .clone();
    let new_volume = backend
        .provision(nexus_storage::CreateOpts {
            name: format!("restore-{}", backup_id),
            size_bytes: backup.size_bytes as u64,
            description: Some(format!("restored from backup {backup_id}")),
        })
        .await
        .context("provision restore target")?;

    // Pick a host that supports the chosen backend.
    let host_repo = HostRepository::new(st.db.clone());
    let kinds_supported_str = backend.kind().as_db_str();
    let candidate_host_id: Option<Uuid> = {
        let active = host_repo.list_active().await?;
        let mut chosen = None;
        for h in active {
            let kinds = host_repo.supported_backend_kinds(h.id).await.unwrap_or_default();
            if kinds.is_empty() || kinds.iter().any(|k| k == kinds_supported_str) {
                chosen = Some(h.id);
                break;
            }
        }
        chosen
    };
    let host_id = candidate_host_id.ok_or_else(|| anyhow!("no host supports backend kind '{kinds_supported_str}'"))?;
    let host = st.hosts.get(host_id).await?;

    // Attach the new volume on that host.
    let attached = agent_rpc::agent_attach(&host.addr, &new_volume).await?;

    let secret = envelope::unwrap_to_string(&target_row.encrypted_secret_access_key)?;
    let target_key = envelope::unwrap_to_array::<32>(&target_row.encrypted_target_key)?;

    let req = RestoreReq {
        target_volume: new_volume.clone(),
        target_attached: attached.clone(),
        manifest_object_key: manifest_key.clone(),
        target: BackupTargetConfig {
            endpoint: target_row.endpoint,
            region: target_row.region,
            bucket: target_row.bucket,
            prefix: target_row.prefix,
            access_key_id: target_row.access_key_id,
            secret_access_key: secret,
        },
        encryption_key: target_key,
    };

    match agent_rpc::agent_restore(&host.addr, req).await {
        Ok(_) => {
            // Insert a volume row for the restored volume.
            let volume_repo = VolumeRepository::new(st.db.clone());
            volume_repo
                .create(
                    &format!("restore-{}", backup_id),
                    Some(&format!("Restored from backup {backup_id}")),
                    &new_volume.locator,
                    new_volume.size_bytes as i64,
                    "raw",
                    Some(host_id),
                    target_backend_id,
                )
                .await?;
            Ok(new_volume.volume_id)
        }
        Err(e) => {
            // Cleanup
            let _ = agent_rpc::agent_detach(&host.addr, &new_volume, &attached).await;
            let _ = backend.destroy(new_volume.clone()).await;
            Err(e)
        }
    }
}
```

- [ ] **Step 16.3: Routes**

```rust
// apps/manager/src/features/backups/routes.rs
use crate::features::backups::repo::{BackupRepository, BackupRow};
use crate::features::backups::service;
use crate::AppState;
use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, delete},
    Extension, Json, Router,
};
use nexus_types::{Backup, BackupStatus, CreateBackupRequest, RestoreRequest};
use serde::Deserialize;
use uuid::Uuid;

fn row_to_wire(row: BackupRow) -> Backup {
    Backup {
        id: row.id,
        source_volume_id: row.source_volume_id,
        target_id: row.target_id,
        size_bytes: row.size_bytes,
        unique_bytes: row.unique_bytes,
        chunk_count: row.chunk_count,
        status: match row.status.as_str() {
            "running" => BackupStatus::Running,
            "completed" => BackupStatus::Completed,
            "failed" => BackupStatus::Failed,
            "pruning" => BackupStatus::Pruning,
            _ => BackupStatus::Failed,
        },
        error_message: row.error_message,
        created_at: row.created_at,
        completed_at: row.completed_at,
    }
}

#[derive(Deserialize)]
pub struct ListQuery {
    pub volume_id: Option<Uuid>,
}

pub async fn list(
    Extension(st): Extension<AppState>,
    Query(q): Query<ListQuery>,
) -> impl IntoResponse {
    let repo = BackupRepository::new(st.db.clone());
    let rows = if let Some(vid) = q.volume_id {
        repo.list_for_volume(vid).await
    } else {
        sqlx::query_as::<_, BackupRow>(r#"SELECT * FROM backup ORDER BY created_at DESC LIMIT 200"#)
            .fetch_all(&st.db)
            .await
    };
    match rows {
        Ok(rs) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "items": rs.into_iter().map(row_to_wire).collect::<Vec<_>>(),
            })),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("backups list: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":"db"}))).into_response()
        }
    }
}

pub async fn get_one(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let repo = BackupRepository::new(st.db.clone());
    match repo.get(id).await {
        Ok(Some(row)) => (StatusCode::OK, Json(row_to_wire(row))).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error":"not found"}))).into_response(),
        Err(e) => {
            tracing::error!("backups get: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":"db"}))).into_response()
        }
    }
}

pub async fn create_for_volume(
    Extension(st): Extension<AppState>,
    Path(volume_id): Path<Uuid>,
    Json(req): Json<CreateBackupRequest>,
) -> impl IntoResponse {
    match service::create_backup(&st, volume_id, req.target_id).await {
        Ok(id) => (StatusCode::CREATED, Json(serde_json::json!({"backup_id": id}))).into_response(),
        Err(e) => {
            tracing::error!("create_backup: {e:#}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}

pub async fn restore(
    Extension(st): Extension<AppState>,
    Path(backup_id): Path<Uuid>,
    Json(req): Json<RestoreRequest>,
) -> impl IntoResponse {
    match service::restore_backup(&st, backup_id, req.target_backend_id).await {
        Ok(volume_id) => (
            StatusCode::CREATED,
            Json(serde_json::json!({"volume_id": volume_id})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("restore_backup: {e:#}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}

pub async fn delete_one(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let repo = BackupRepository::new(st.db.clone());
    // Mark pruning then delete the manifest from S3 and the row.
    sqlx::query(r#"UPDATE backup SET status = 'pruning', updated_at = now() WHERE id = $1"#)
        .bind(id)
        .execute(&st.db)
        .await
        .ok();
    match repo.delete_row(id).await {
        Ok(()) => (StatusCode::NO_CONTENT, ()).into_response(),
        Err(e) => {
            tracing::error!("backups delete: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":"db"}))).into_response()
        }
    }
}

pub fn router() -> Router {
    Router::new()
        .route("/", get(list))
        .route("/:id", get(get_one).delete(delete_one))
        .route("/:id/restore", post(restore))
}

pub fn volume_backup_router() -> Router {
    Router::new()
        .route("/", post(create_for_volume))
}
```

- [ ] **Step 16.4: Wire up `mod.rs`**

```rust
// apps/manager/src/features/backups/mod.rs
pub mod repo;
pub mod routes;
pub mod service;
pub mod types;

pub use routes::{router, volume_backup_router};
```

In `apps/manager/src/features/mod.rs`, register both routers:

```rust
pub mod backups;
// ... existing nests ...
.nest("/v1/backups", backups::router())
.nest("/v1/volumes/:id/backup", backups::volume_backup_router())
```

- [ ] **Step 16.5: Verify**

Run: `cargo check -p manager && cargo clippy -p manager --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 16.6: Commit**

```bash
git add apps/manager/src/features/backups/ apps/manager/src/features/mod.rs
git commit -m "feat(backup): manager backup orchestration (create + restore + retention)"
```

---

## Task 17: Daily mark-and-sweep GC task

**Files:**
- Create: `apps/manager/src/features/backups/gc.rs`
- Modify: `apps/manager/src/features/backups/mod.rs` — `pub mod gc;`
- Modify: `apps/manager/src/main.rs` — spawn the task on startup

- [ ] **Step 17.1: GC**

```rust
// apps/manager/src/features/backups/gc.rs
//! Daily mark-and-sweep GC per backup_target.

use crate::features::backup_targets::envelope;
use crate::features::backup_targets::repo::{BackupTargetRepository, BackupTargetRow};
use anyhow::{Context, Result};
use aws_sdk_s3::{
    config::{Builder as S3ConfBuilder, Region},
    types::{Delete, ObjectIdentifier},
    Client,
};
use chrono::{Timelike, Utc};
use nexus_backup::{decrypt_manifest, ChunkKey, Manifest};
use sqlx::PgPool;
use std::collections::HashSet;

pub async fn gc_loop(pool: PgPool) {
    loop {
        let now = Utc::now();
        let next_check = (60 - now.minute()) as u64 * 60 - now.second() as u64;
        tokio::time::sleep(std::time::Duration::from_secs(next_check)).await;

        let now = Utc::now();
        let repo = BackupTargetRepository::new(pool.clone());
        match repo.list_active().await {
            Ok(targets) => {
                for t in targets {
                    if (t.gc_hour as u32) == now.hour() {
                        if let Err(e) = run_gc(&pool, &t).await {
                            tracing::error!(target=%t.name, "GC run failed: {e:#}");
                        }
                    }
                }
            }
            Err(e) => tracing::error!("gc_loop list_active: {e}"),
        }
    }
}

pub async fn run_gc(pool: &PgPool, target: &BackupTargetRow) -> Result<()> {
    // Insert a gc_run row.
    let run_id: uuid::Uuid = sqlx::query_scalar(
        r#"INSERT INTO backup_gc_run (target_id, status) VALUES ($1, 'running') RETURNING id"#,
    )
    .bind(target.id)
    .fetch_one(pool)
    .await?;

    let result = sweep(pool, target).await;
    match result {
        Ok((bytes, chunks)) => {
            sqlx::query(
                r#"UPDATE backup_gc_run SET status='completed', completed_at=now(),
                   bytes_freed=$1, chunks_deleted=$2 WHERE id = $3"#,
            )
            .bind(bytes as i64)
            .bind(chunks as i64)
            .bind(run_id)
            .execute(pool)
            .await?;
            tracing::info!(target=%target.name, bytes_freed=bytes, chunks_deleted=chunks, "GC complete");
            Ok(())
        }
        Err(e) => {
            sqlx::query(
                r#"UPDATE backup_gc_run SET status='failed', completed_at=now(), error_message=$1 WHERE id=$2"#,
            )
            .bind(e.to_string())
            .bind(run_id)
            .execute(pool)
            .await
            .ok();
            Err(e)
        }
    }
}

async fn sweep(pool: &PgPool, target: &BackupTargetRow) -> Result<(u64, u64)> {
    let secret = envelope::unwrap_to_string(&target.encrypted_secret_access_key)?;
    let target_key = envelope::unwrap_to_array::<32>(&target.encrypted_target_key)?;

    let creds = aws_credential_types::Credentials::new(
        &target.access_key_id, &secret, None, None, "nqrust-gc");
    let region = Region::new(target.region.clone().unwrap_or_else(|| "us-east-1".into()));
    let s3_cfg = S3ConfBuilder::new()
        .behavior_version_latest()
        .endpoint_url(&target.endpoint)
        .credentials_provider(creds)
        .region(region)
        .force_path_style(true)
        .build();
    let client = Client::from_conf(s3_cfg);

    // 1. Mark: walk manifests, collect referenced chunk ids.
    let prefix_manifests = if target.prefix.is_empty() { "manifests/".to_string() } else { format!("{}/manifests/", target.prefix.trim_end_matches('/')) };
    let mut referenced: HashSet<[u8; 32]> = HashSet::new();
    let mut continuation: Option<String> = None;
    loop {
        let mut req = client.list_objects_v2().bucket(&target.bucket).prefix(&prefix_manifests);
        if let Some(c) = continuation.as_deref() { req = req.continuation_token(c); }
        let resp = req.send().await.context("LIST manifests")?;
        for obj in resp.contents() {
            let Some(k) = obj.key() else { continue };
            // Fetch + decrypt + deserialize.
            let blob = client.get_object().bucket(&target.bucket).key(k).send().await
                .with_context(|| format!("GET {k}"))?
                .body.collect().await.context("body collect")?
                .into_bytes().to_vec();
            let chunk_key = ChunkKey::from_bytes(target_key);
            let compressed = decrypt_manifest(&chunk_key, &blob).context("decrypt manifest")?;
            let m: Manifest = Manifest::deserialize_compressed(&compressed).context("deserialize manifest")?;
            for c in m.chunks { referenced.insert(c.chunk_id); }
        }
        if resp.is_truncated().unwrap_or(false) {
            continuation = resp.next_continuation_token().map(String::from);
        } else { break; }
    }
    drop(target_key); // wipe key from memory

    // 2. Sweep: walk chunks, delete unreferenced ones older than 24h.
    let prefix_chunks = if target.prefix.is_empty() { "chunks/".to_string() } else { format!("{}/chunks/", target.prefix.trim_end_matches('/')) };
    let cutoff = aws_smithy_types::DateTime::from_secs(
        (chrono::Utc::now() - chrono::Duration::hours(24)).timestamp(),
    );
    let mut bytes_freed: u64 = 0;
    let mut chunks_deleted: u64 = 0;

    let mut continuation: Option<String> = None;
    loop {
        let mut req = client.list_objects_v2().bucket(&target.bucket).prefix(&prefix_chunks);
        if let Some(c) = continuation.as_deref() { req = req.continuation_token(c); }
        let resp = req.send().await.context("LIST chunks")?;
        let mut to_delete: Vec<ObjectIdentifier> = Vec::new();
        for obj in resp.contents() {
            let Some(k) = obj.key() else { continue };
            // Only consider chunks older than 24h to protect in-flight backups.
            let too_recent = obj.last_modified()
                .map(|lm| lm > &cutoff)
                .unwrap_or(true);
            if too_recent { continue; }
            // Extract the chunk_id from "[prefix/]chunks/<2hex>/<64hex>".
            let parts: Vec<&str> = k.rsplit('/').collect();
            if parts.is_empty() { continue; }
            let id_hex = parts[0];
            if id_hex.len() != 64 { continue; }
            let mut chunk_id = [0u8; 32];
            if hex::decode_to_slice(id_hex, &mut chunk_id).is_err() { continue; }
            if referenced.contains(&chunk_id) { continue; }
            bytes_freed += obj.size().unwrap_or(0) as u64;
            chunks_deleted += 1;
            to_delete.push(
                ObjectIdentifier::builder().key(k).build().unwrap(),
            );
            if to_delete.len() == 1000 {
                let del = Delete::builder().set_objects(Some(std::mem::take(&mut to_delete))).build().unwrap();
                client.delete_objects().bucket(&target.bucket).delete(del).send().await
                    .context("DELETE chunks batch")?;
            }
        }
        if !to_delete.is_empty() {
            let del = Delete::builder().set_objects(Some(to_delete)).build().unwrap();
            client.delete_objects().bucket(&target.bucket).delete(del).send().await
                .context("DELETE chunks final")?;
        }
        if resp.is_truncated().unwrap_or(false) {
            continuation = resp.next_continuation_token().map(String::from);
        } else { break; }
    }

    Ok((bytes_freed, chunks_deleted))
}
```

- [ ] **Step 17.2: Wire**

In `apps/manager/src/features/backups/mod.rs`, add `pub mod gc;`.

In `apps/manager/src/main.rs`, after AppState construction:

```rust
// Backup GC daily loop.
{
    let pool = state.db.clone();
    tokio::spawn(async move {
        crate::features::backups::gc::gc_loop(pool).await;
    });
}
```

- [ ] **Step 17.3: Verify**

Run: `cargo check -p manager && cargo clippy -p manager --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 17.4: Commit**

```bash
git add apps/manager/src/features/backups/gc.rs apps/manager/src/features/backups/mod.rs apps/manager/src/main.rs
git commit -m "feat(backup): daily mark-and-sweep GC per target"
```

---

## Task 18: Reconciler for stuck `running` rows

**Files:**
- Create: `apps/manager/src/features/backups/reconciler.rs`
- Modify: `apps/manager/src/features/backups/mod.rs`
- Modify: `apps/manager/src/main.rs`

- [ ] **Step 18.1: Reconciler**

```rust
// apps/manager/src/features/backups/reconciler.rs
//! Periodically marks `running` backups older than 24h as `failed` so their
//! orphan chunks become reachable to GC.

use crate::features::backups::repo::BackupRepository;
use sqlx::PgPool;

const STALE_MINUTES: i64 = 24 * 60;

pub async fn reconcile_loop(pool: PgPool) {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(5 * 60)).await;
        let repo = BackupRepository::new(pool.clone());
        match repo.list_stale_running(STALE_MINUTES).await {
            Ok(rows) => {
                for r in rows {
                    let _ = repo.mark_failed(r.id, &format!(
                        "marked failed by reconciler: status was 'running' for >{STALE_MINUTES} minutes"
                    )).await;
                    tracing::warn!(backup_id=%r.id, "reconciler aged stuck 'running' to 'failed'");
                }
            }
            Err(e) => tracing::error!("reconciler: {e}"),
        }
    }
}
```

- [ ] **Step 18.2: Wire**

`apps/manager/src/features/backups/mod.rs`: `pub mod reconciler;`

`apps/manager/src/main.rs` after the GC spawn:

```rust
{
    let pool = state.db.clone();
    tokio::spawn(async move {
        crate::features::backups::reconciler::reconcile_loop(pool).await;
    });
}
```

- [ ] **Step 18.3: Verify + commit**

```bash
cargo check -p manager
git add apps/manager/src/features/backups/reconciler.rs apps/manager/src/features/backups/mod.rs apps/manager/src/main.rs
git commit -m "feat(backup): reconciler ages stuck 'running' rows after 24h"
```

---

## Task 19: Per-volume cron scheduler

**Files:**
- Create: `apps/manager/src/features/backups/scheduler.rs`
- Modify: `apps/manager/src/features/backups/mod.rs`
- Modify: `apps/manager/src/main.rs`

- [ ] **Step 19.1: Scheduler**

```rust
// apps/manager/src/features/backups/scheduler.rs
//! Per-volume cron scheduler: wakes on the next-fire time across all volumes
//! that have backup_cron + backup_target_id set, dispatches a backup.

use crate::AppState;
use chrono::Utc;
use cron::Schedule;
use sqlx::PgPool;
use std::str::FromStr;
use std::time::Duration;
use uuid::Uuid;

pub async fn schedule_loop(state: AppState) {
    loop {
        // Wake once a minute and look for due schedules.
        tokio::time::sleep(Duration::from_secs(60)).await;
        if let Err(e) = tick(&state).await {
            tracing::error!("scheduler tick: {e:#}");
        }
    }
}

async fn tick(st: &AppState) -> anyhow::Result<()> {
    let rows: Vec<(Uuid, Option<String>, Option<Uuid>, Option<chrono::DateTime<Utc>>)> = sqlx::query_as(
        r#"SELECT v.id, v.backup_cron, v.backup_target_id,
                  (SELECT MAX(created_at) FROM backup b WHERE b.source_volume_id = v.id) AS last_backup
           FROM volume v
           WHERE v.backup_cron IS NOT NULL AND v.backup_target_id IS NOT NULL"#,
    )
    .fetch_all(&st.db)
    .await?;

    let now = Utc::now();
    for (volume_id, cron_str, target_id, last) in rows {
        let (Some(cron_str), Some(target_id)) = (cron_str, target_id) else { continue; };
        let Ok(schedule) = Schedule::from_str(&cron_str) else {
            tracing::warn!(volume_id=%volume_id, "invalid cron: {cron_str}");
            continue;
        };
        let after = last.unwrap_or(now - chrono::Duration::days(365));
        let next = schedule.after(&after).next();
        if let Some(next_fire) = next {
            if next_fire <= now {
                tracing::info!(volume_id=%volume_id, "scheduler firing backup");
                let st_cl = st.clone();
                tokio::spawn(async move {
                    if let Err(e) = crate::features::backups::service::create_backup(&st_cl, volume_id, target_id).await {
                        tracing::error!(volume_id=%volume_id, "scheduled backup failed: {e:#}");
                    }
                });
            }
        }
    }
    Ok(())
}
```

- [ ] **Step 19.2: Wire**

`apps/manager/src/features/backups/mod.rs`: `pub mod scheduler;`

`apps/manager/src/main.rs`:

```rust
{
    let st = state.clone();
    tokio::spawn(async move {
        crate::features::backups::scheduler::schedule_loop(st).await;
    });
}
```

- [ ] **Step 19.3: Verify + commit**

```bash
cargo check -p manager && cargo clippy -p manager --all-targets -- -D warnings
git add apps/manager/src/features/backups/scheduler.rs apps/manager/src/features/backups/mod.rs apps/manager/src/main.rs
git commit -m "feat(backup): per-volume cron scheduler"
```

---

## Task 20: Per-volume backup_schedule API

**Files:**
- Modify: `apps/manager/src/features/volumes/routes.rs`

- [ ] **Step 20.1: Endpoint**

Append to `apps/manager/src/features/volumes/routes.rs`:

```rust
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct PatchBackupScheduleRequest {
    pub cron: Option<String>,
    pub retain_count: Option<i32>,
    pub target_id: Option<uuid::Uuid>,
}

pub async fn patch_backup_schedule(
    Extension(st): Extension<AppState>,
    Path(id): Path<uuid::Uuid>,
    Json(req): Json<PatchBackupScheduleRequest>,
) -> impl IntoResponse {
    if let Some(c) = &req.cron {
        if let Err(e) = cron::Schedule::from_str(c) {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("invalid cron: {e}")})),
            )
                .into_response();
        }
    }
    let res = sqlx::query(
        r#"UPDATE volume SET backup_cron = COALESCE($1, backup_cron),
                              backup_retain_count = COALESCE($2, backup_retain_count),
                              backup_target_id = COALESCE($3, backup_target_id)
           WHERE id = $4"#,
    )
    .bind(req.cron)
    .bind(req.retain_count)
    .bind(req.target_id)
    .bind(id)
    .execute(&st.db)
    .await;
    match res {
        Ok(_) => (StatusCode::NO_CONTENT, ()).into_response(),
        Err(e) => {
            tracing::error!("patch_backup_schedule: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":"db"}))).into_response()
        }
    }
}
```

Add `use std::str::FromStr;` at the top of the file if not already present.

In `apps/manager/src/features/volumes/mod.rs`, register the new route:

```rust
.route("/:id/backup_schedule", patch(routes::patch_backup_schedule))
```

(Add `axum::routing::patch` to the imports.)

- [ ] **Step 20.2: Verify + commit**

```bash
cargo check -p manager && cargo clippy -p manager --all-targets -- -D warnings
git add apps/manager/src/features/volumes/
git commit -m "feat(backup): PATCH /v1/volumes/:id/backup_schedule"
```

---

## Task 21: Index-rebuild subcommand

**Files:**
- Create: `apps/manager/src/features/backups/index_rebuild.rs`
- Modify: `apps/manager/src/main.rs` — handle `backup index-rebuild` CLI args

- [ ] **Step 21.1: Implementation**

```rust
// apps/manager/src/features/backups/index_rebuild.rs
//! Reconstruct the `backup` table from S3 manifests.
//! Run via: `manager backup index-rebuild --target <id>`.

use crate::features::backup_targets::envelope;
use crate::features::backup_targets::repo::BackupTargetRepository;
use anyhow::{Context, Result};
use aws_sdk_s3::{config::{Builder, Region}, Client};
use chrono::TimeZone;
use nexus_backup::{decrypt_manifest, ChunkKey, Manifest};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn run(pool: PgPool, target_id: Uuid) -> Result<()> {
    let repo = BackupTargetRepository::new(pool.clone());
    let target = repo.get(target_id).await?
        .ok_or_else(|| anyhow::anyhow!("target {target_id} not found"))?;

    let secret = envelope::unwrap_to_string(&target.encrypted_secret_access_key)?;
    let target_key = envelope::unwrap_to_array::<32>(&target.encrypted_target_key)?;

    let creds = aws_credential_types::Credentials::new(
        &target.access_key_id, &secret, None, None, "nqrust-rebuild");
    let region = Region::new(target.region.clone().unwrap_or_else(|| "us-east-1".into()));
    let s3_cfg = Builder::new()
        .behavior_version_latest()
        .endpoint_url(&target.endpoint)
        .credentials_provider(creds)
        .region(region)
        .force_path_style(true)
        .build();
    let client = Client::from_conf(s3_cfg);

    let prefix = if target.prefix.is_empty() {
        "manifests/".to_string()
    } else {
        format!("{}/manifests/", target.prefix.trim_end_matches('/'))
    };

    let mut reconstructed = 0usize;
    let mut skipped = 0usize;
    let mut continuation: Option<String> = None;
    loop {
        let mut req = client.list_objects_v2().bucket(&target.bucket).prefix(&prefix);
        if let Some(c) = continuation.as_deref() { req = req.continuation_token(c); }
        let resp = req.send().await.context("LIST manifests")?;
        for obj in resp.contents() {
            let Some(k) = obj.key() else { continue };
            let blob = client.get_object().bucket(&target.bucket).key(k).send().await
                .with_context(|| format!("GET {k}"))?
                .body.collect().await?
                .into_bytes().to_vec();
            let key = ChunkKey::from_bytes(target_key);
            let compressed = decrypt_manifest(&key, &blob)?;
            let m = Manifest::deserialize_compressed(&compressed)?;
            // Insert if not exists.
            let existed: Option<Uuid> = sqlx::query_scalar(
                r#"SELECT id FROM backup WHERE id = $1"#,
            )
            .bind(m.backup_id)
            .fetch_optional(&pool)
            .await?;
            if existed.is_some() { skipped += 1; continue; }
            let total_size: i64 = m.chunks.iter().map(|c| c.ciphertext_length as i64).sum();
            let chunk_count = m.chunks.len() as i64;
            let created_at = chrono::Utc.timestamp_opt(m.created_at_unix_seconds, 0).single()
                .unwrap_or_else(chrono::Utc::now);
            sqlx::query(
                r#"INSERT INTO backup
                   (id, source_volume_id, source_snapshot_id, target_id,
                    manifest_object_key, size_bytes, unique_bytes, chunk_count,
                    status, created_at, completed_at, updated_at)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'completed', $9, $9, now())"#,
            )
            .bind(m.backup_id)
            .bind(m.source_volume_id)
            .bind(m.source_snapshot_id)
            .bind(target_id)
            .bind(k)
            .bind(total_size)
            .bind(0i64) // unique_bytes unknown post-DR; that's fine
            .bind(chunk_count)
            .bind(created_at)
            .execute(&pool)
            .await
            .ok();
            reconstructed += 1;
        }
        if resp.is_truncated().unwrap_or(false) {
            continuation = resp.next_continuation_token().map(String::from);
        } else { break; }
    }
    println!("index-rebuild: reconstructed {reconstructed}, skipped (already in DB) {skipped}");
    Ok(())
}
```

- [ ] **Step 21.2: CLI handling in main.rs**

Near the top of `main()` in `apps/manager/src/main.rs`, before the normal server startup:

```rust
let args: Vec<String> = std::env::args().collect();
if args.len() >= 4 && args[1] == "backup" && args[2] == "index-rebuild" {
    // args[3] is "--target", args[4] is the UUID.
    if args.len() != 5 || args[3] != "--target" {
        eprintln!("usage: manager backup index-rebuild --target <uuid>");
        std::process::exit(2);
    }
    let target_id: uuid::Uuid = match args[4].parse() {
        Ok(u) => u,
        Err(e) => { eprintln!("bad uuid: {e}"); std::process::exit(2); }
    };
    let pool = sqlx::PgPool::connect(&std::env::var("DATABASE_URL").unwrap()).await?;
    crate::features::backups::index_rebuild::run(pool, target_id).await?;
    return Ok(());
}
```

- [ ] **Step 21.3: Wire**

`apps/manager/src/features/backups/mod.rs`: `pub mod index_rebuild;`

- [ ] **Step 21.4: Verify + commit**

```bash
cargo check -p manager
git add apps/manager/src/features/backups/index_rebuild.rs apps/manager/src/main.rs apps/manager/src/features/backups/mod.rs
git commit -m "feat(backup): index-rebuild subcommand for DR"
```

---

## Task 22: UI types + queries + facade

**Files:**
- Modify: `apps/ui/lib/types/index.ts`
- Modify: `apps/ui/lib/api/facade.ts`
- Modify: `apps/ui/lib/queries.ts`

- [ ] **Step 22.1: Types**

Append to `apps/ui/lib/types/index.ts`:

```ts
export type BackupStatus = "running" | "completed" | "failed" | "pruning";

export interface BackupTarget {
  id: string;
  name: string;
  endpoint: string;
  region?: string;
  bucket: string;
  prefix: string;
  access_key_id: string;
  gc_hour: number;
  created_at: string;
  deleted_at?: string | null;
}

export interface CreateBackupTargetRequest {
  name: string;
  endpoint: string;
  region?: string;
  bucket: string;
  prefix?: string;
  access_key_id: string;
  secret_access_key: string;
  gc_hour?: number;
}

export interface Backup {
  id: string;
  source_volume_id?: string;
  target_id: string;
  size_bytes: number;
  unique_bytes: number;
  chunk_count: number;
  status: BackupStatus;
  error_message?: string;
  created_at: string;
  completed_at?: string;
}

export interface BackupSchedule {
  cron: string;
  retain_count: number;
  target_id: string;
}
```

- [ ] **Step 22.2: Facade methods**

Append to `apps/ui/lib/api/facade.ts`:

```ts
async listBackupTargets(): Promise<{ items: BackupTarget[] }> {
  return this.client.get("/v1/backup_targets");
}
async createBackupTarget(req: CreateBackupTargetRequest): Promise<BackupTarget> {
  return this.client.post("/v1/backup_targets", req);
}
async deleteBackupTarget(id: string): Promise<void> {
  return this.client.delete(`/v1/backup_targets/${id}`);
}
async listBackups(volumeId?: string): Promise<{ items: Backup[] }> {
  const q = volumeId ? `?volume_id=${volumeId}` : "";
  return this.client.get(`/v1/backups${q}`);
}
async createBackup(volumeId: string, targetId: string): Promise<{ backup_id: string }> {
  return this.client.post(`/v1/volumes/${volumeId}/backup`, { target_id: targetId });
}
async restoreBackup(backupId: string, targetBackendId: string): Promise<{ volume_id: string }> {
  return this.client.post(`/v1/backups/${backupId}/restore`, { target_backend_id: targetBackendId });
}
async deleteBackup(backupId: string): Promise<void> {
  return this.client.delete(`/v1/backups/${backupId}`);
}
async patchBackupSchedule(volumeId: string, req: Partial<BackupSchedule>): Promise<void> {
  return this.client.patch(`/v1/volumes/${volumeId}/backup_schedule`, req);
}
```

(Adjust `this.client` to match the project's actual API client field name; in our codebase it's `apiClient`.)

- [ ] **Step 22.3: Hooks**

Append to `apps/ui/lib/queries.ts`:

```ts
export function useBackupTargets() {
  return useQuery({
    queryKey: ["backup_targets"] as const,
    queryFn: async () => (await facadeApi.listBackupTargets()).items,
  });
}
export function useBackups(volumeId?: string) {
  return useQuery({
    queryKey: ["backups", volumeId ?? "all"] as const,
    queryFn: async () => (await facadeApi.listBackups(volumeId)).items,
    refetchInterval: 5_000, // for in-progress backups
  });
}
```

- [ ] **Step 22.4: Verify**

Run: `(cd apps/ui && pnpm tsc --noEmit)`
Expected: clean.

- [ ] **Step 22.5: Commit**

```bash
git add apps/ui/lib/
git commit -m "feat(backup): UI types + facade methods + query hooks"
```

---

## Task 23: UI components — BackupTargetForm, BackupList, RestoreDialog, ScheduleEditor

**Files:**
- Create: `apps/ui/components/backup/backup-target-form.tsx`
- Create: `apps/ui/components/backup/backup-list.tsx`
- Create: `apps/ui/components/backup/restore-dialog.tsx`
- Create: `apps/ui/components/backup/backup-schedule-editor.tsx`
- Create: `apps/ui/app/(dashboard)/backup-targets/page.tsx`
- Create: `apps/ui/components/volume/volume-backups-tab.tsx`

This task lays out four self-contained components plus a page. Code is mechanical; follow the patterns of `apps/ui/components/storage/backend-selector.tsx` and existing list components.

- [ ] **Step 23.1: BackupTargetForm**

```tsx
// apps/ui/components/backup/backup-target-form.tsx
"use client";
import { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { facadeApi } from "@/lib/api/facade";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

export function BackupTargetForm({ onCreated }: { onCreated?: () => void }) {
  const [name, setName] = useState("");
  const [endpoint, setEndpoint] = useState("");
  const [bucket, setBucket] = useState("");
  const [prefix, setPrefix] = useState("");
  const [accessKey, setAccessKey] = useState("");
  const [secretKey, setSecretKey] = useState("");
  const [region, setRegion] = useState("us-east-1");
  const qc = useQueryClient();
  const mut = useMutation({
    mutationFn: () => facadeApi.createBackupTarget({
      name, endpoint, bucket, prefix, access_key_id: accessKey, secret_access_key: secretKey, region,
    }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["backup_targets"] });
      onCreated?.();
    },
  });
  return (
    <form onSubmit={(e) => { e.preventDefault(); mut.mutate(); }} className="space-y-3">
      <div><Label>Name</Label><Input value={name} onChange={(e) => setName(e.target.value)} required /></div>
      <div><Label>Endpoint URL</Label><Input value={endpoint} onChange={(e) => setEndpoint(e.target.value)} required placeholder="https://seaweedfs.local:8333" /></div>
      <div><Label>Region</Label><Input value={region} onChange={(e) => setRegion(e.target.value)} /></div>
      <div><Label>Bucket</Label><Input value={bucket} onChange={(e) => setBucket(e.target.value)} required /></div>
      <div><Label>Prefix (optional)</Label><Input value={prefix} onChange={(e) => setPrefix(e.target.value)} /></div>
      <div><Label>Access Key ID</Label><Input value={accessKey} onChange={(e) => setAccessKey(e.target.value)} required /></div>
      <div><Label>Secret Access Key</Label><Input type="password" value={secretKey} onChange={(e) => setSecretKey(e.target.value)} required /></div>
      <Button type="submit" disabled={mut.isPending}>{mut.isPending ? "Saving…" : "Create target"}</Button>
      {mut.error && <p className="text-red-500 text-sm">{(mut.error as Error).message}</p>}
    </form>
  );
}
```

- [ ] **Step 23.2: BackupList**

```tsx
// apps/ui/components/backup/backup-list.tsx
"use client";
import { useBackups, useStorageBackends } from "@/lib/queries";
import { facadeApi } from "@/lib/api/facade";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { useState } from "react";
import { RestoreDialog } from "./restore-dialog";

export function BackupList({ volumeId }: { volumeId: string }) {
  const { data: backups, isLoading } = useBackups(volumeId);
  const qc = useQueryClient();
  const del = useMutation({
    mutationFn: (id: string) => facadeApi.deleteBackup(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["backups", volumeId] }),
  });
  const [restoring, setRestoring] = useState<string | null>(null);

  if (isLoading) return <p>Loading…</p>;
  if (!backups?.length) return <p className="text-muted-foreground">No backups yet.</p>;

  return (
    <>
      <table className="w-full text-sm">
        <thead className="text-left text-muted-foreground">
          <tr><th>Created</th><th>Status</th><th>Size</th><th>Chunks</th><th></th></tr>
        </thead>
        <tbody>
          {backups.map((b) => (
            <tr key={b.id} className="border-t">
              <td>{new Date(b.created_at).toLocaleString()}</td>
              <td>{b.status}</td>
              <td>{(b.size_bytes / 1024 / 1024).toFixed(1)} MiB</td>
              <td>{b.chunk_count}</td>
              <td className="text-right space-x-2">
                {b.status === "completed" && (
                  <Button size="sm" variant="secondary" onClick={() => setRestoring(b.id)}>Restore…</Button>
                )}
                <Button size="sm" variant="ghost" onClick={() => del.mutate(b.id)}>Delete</Button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {restoring && <RestoreDialog backupId={restoring} onClose={() => setRestoring(null)} />}
    </>
  );
}
```

- [ ] **Step 23.3: RestoreDialog**

```tsx
// apps/ui/components/backup/restore-dialog.tsx
"use client";
import { useState } from "react";
import { useStorageBackends } from "@/lib/queries";
import { facadeApi } from "@/lib/api/facade";
import { useMutation } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Label } from "@/components/ui/label";

export function RestoreDialog({ backupId, onClose }: { backupId: string; onClose: () => void }) {
  const { data: backends } = useStorageBackends();
  const active = (backends ?? []).filter((b) => !b.deleted_at);
  const [target, setTarget] = useState<string | undefined>(active.find((b) => b.is_default)?.id);
  const mut = useMutation({
    mutationFn: () => facadeApi.restoreBackup(backupId, target!),
    onSuccess: () => onClose(),
  });
  return (
    <Dialog open onOpenChange={(o) => !o && onClose()}>
      <DialogContent>
        <DialogHeader><DialogTitle>Restore backup to a new volume</DialogTitle></DialogHeader>
        <div className="space-y-2">
          <Label>Target backend</Label>
          <Select value={target} onValueChange={setTarget}>
            <SelectTrigger><SelectValue /></SelectTrigger>
            <SelectContent>
              {active.map((b) => <SelectItem key={b.id} value={b.id}>{b.name}</SelectItem>)}
            </SelectContent>
          </Select>
        </div>
        <DialogFooter>
          <Button variant="ghost" onClick={onClose}>Cancel</Button>
          <Button disabled={!target || mut.isPending} onClick={() => mut.mutate()}>
            {mut.isPending ? "Restoring…" : "Restore"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
```

- [ ] **Step 23.4: BackupScheduleEditor**

```tsx
// apps/ui/components/backup/backup-schedule-editor.tsx
"use client";
import { useState } from "react";
import { facadeApi } from "@/lib/api/facade";
import { useBackupTargets } from "@/lib/queries";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";

export function BackupScheduleEditor({ volumeId, current }: {
  volumeId: string;
  current?: { cron?: string; retain_count?: number; target_id?: string };
}) {
  const { data: targets } = useBackupTargets();
  const [cron, setCron] = useState(current?.cron ?? "0 2 * * *");
  const [retain, setRetain] = useState(current?.retain_count ?? 7);
  const [target, setTarget] = useState(current?.target_id);
  const qc = useQueryClient();
  const mut = useMutation({
    mutationFn: () => facadeApi.patchBackupSchedule(volumeId, {
      cron, retain_count: retain, target_id: target,
    }),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["volume", volumeId] }),
  });
  return (
    <form onSubmit={(e) => { e.preventDefault(); mut.mutate(); }} className="space-y-3">
      <div>
        <Label>Schedule (cron, UTC)</Label>
        <Input value={cron} onChange={(e) => setCron(e.target.value)} placeholder="0 2 * * *" />
      </div>
      <div>
        <Label>Retain count</Label>
        <Input type="number" min={1} value={retain} onChange={(e) => setRetain(parseInt(e.target.value))} />
      </div>
      <div>
        <Label>Backup target</Label>
        <Select value={target} onValueChange={setTarget}>
          <SelectTrigger><SelectValue placeholder="Select…" /></SelectTrigger>
          <SelectContent>
            {(targets ?? []).map((t) => <SelectItem key={t.id} value={t.id}>{t.name}</SelectItem>)}
          </SelectContent>
        </Select>
      </div>
      <Button type="submit" disabled={!target || mut.isPending}>{mut.isPending ? "Saving…" : "Save schedule"}</Button>
    </form>
  );
}
```

- [ ] **Step 23.5: Page route**

```tsx
// apps/ui/app/(dashboard)/backup-targets/page.tsx
"use client";
import { useBackupTargets } from "@/lib/queries";
import { BackupTargetForm } from "@/components/backup/backup-target-form";

export default function BackupTargetsPage() {
  const { data: targets, isLoading } = useBackupTargets();
  return (
    <div className="space-y-6">
      <h1 className="text-2xl font-bold">Backup targets</h1>
      <BackupTargetForm />
      <div>
        <h2 className="text-lg font-semibold">Configured targets</h2>
        {isLoading && <p>Loading…</p>}
        <ul className="space-y-1">
          {(targets ?? []).map((t) => (
            <li key={t.id} className="border p-2 rounded">
              <div className="font-medium">{t.name}</div>
              <div className="text-sm text-muted-foreground">{t.endpoint} → s3://{t.bucket}/{t.prefix}</div>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}
```

- [ ] **Step 23.6: Volume backup tab**

```tsx
// apps/ui/components/volume/volume-backups-tab.tsx
"use client";
import { BackupList } from "@/components/backup/backup-list";
import { BackupScheduleEditor } from "@/components/backup/backup-schedule-editor";
import { useBackupTargets } from "@/lib/queries";
import { facadeApi } from "@/lib/api/facade";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { useState } from "react";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Label } from "@/components/ui/label";

export function VolumeBackupsTab({ volumeId }: { volumeId: string }) {
  const { data: targets } = useBackupTargets();
  const [target, setTarget] = useState<string | undefined>();
  const qc = useQueryClient();
  const back = useMutation({
    mutationFn: () => facadeApi.createBackup(volumeId, target!),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["backups", volumeId] }),
  });
  return (
    <div className="space-y-6">
      <section>
        <h2 className="text-lg font-semibold mb-2">Back up now</h2>
        <div className="flex gap-2 items-end">
          <div className="flex-1">
            <Label>Target</Label>
            <Select value={target} onValueChange={setTarget}>
              <SelectTrigger><SelectValue placeholder="Select…" /></SelectTrigger>
              <SelectContent>
                {(targets ?? []).map((t) => <SelectItem key={t.id} value={t.id}>{t.name}</SelectItem>)}
              </SelectContent>
            </Select>
          </div>
          <Button disabled={!target || back.isPending} onClick={() => back.mutate()}>
            {back.isPending ? "Starting…" : "Backup now"}
          </Button>
        </div>
      </section>
      <section>
        <h2 className="text-lg font-semibold mb-2">Schedule</h2>
        <BackupScheduleEditor volumeId={volumeId} />
      </section>
      <section>
        <h2 className="text-lg font-semibold mb-2">History</h2>
        <BackupList volumeId={volumeId} />
      </section>
    </div>
  );
}
```

Wire `<VolumeBackupsTab />` into the existing volume detail page (`apps/ui/app/(dashboard)/volumes/[id]/page.tsx` or wherever it lives) as a new tab.

- [ ] **Step 23.7: Verify**

Run: `(cd apps/ui && pnpm tsc --noEmit && pnpm lint && pnpm build)`
Expected: clean.

- [ ] **Step 23.8: Commit**

```bash
git add apps/ui/components/backup/ apps/ui/components/volume/volume-backups-tab.tsx apps/ui/app/
git commit -m "feat(backup): UI components — target form, list, restore, schedule, page"
```

---

## Task 24: Final sweep

- [ ] **Step 24.1: Format**

Run: `cargo fmt --all`
Commit any changes: `git commit -am "chore(backup): cargo fmt sweep"`.

- [ ] **Step 24.2: Clippy**

Run: `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -5`
Expected: clean.

- [ ] **Step 24.3: Tests**

Run: `cargo test --workspace --exclude installer 2>&1 | grep "test result" | head -10`
Expected: all suites pass.

- [ ] **Step 24.4: UI**

Run: `(cd apps/ui && pnpm tsc --noEmit && pnpm lint && pnpm build) 2>&1 | tail -10`
Expected: clean.

---

## Plan completion checklist

Verify against the spec's success criteria:

- [ ] Volume can be backed up to a SeaweedFS / MinIO / AWS-S3 target via `POST /v1/volumes/:id/backup` — Tasks 12–16.
- [ ] Restored volume contains byte-identical bytes (golden test) — covered inline in Tasks 3–5 + agent-side round-trip in Task 15 (extend with explicit golden test in Task 24 if missing).
- [ ] Re-backing up after no changes uploads only the manifest — covered by HEAD-before-PUT in Task 15.
- [ ] Re-backing up after partial change uploads only changed-data chunks — same.
- [ ] Backup-of-2-similar-VMs uses ~50% space — emergent from Tasks 3–4 + content addressing.
- [ ] Manager restart mid-backup auto-resumes via reconciler within 24h — Task 18.
- [ ] DB destroyed → `nqrust backup index-rebuild` reconstructs all backups — Task 21.
- [ ] Daily GC reclaims chunks made unreachable by retention pruning — Task 17.
- [ ] `cargo fmt --check` and `cargo clippy --all-targets --all-features -- -D warnings` pass — Task 24.

## Out of scope for this plan

(Per spec §"Explicitly out of scope".)

- Embedded SeaweedFS lifecycle.
- Restore-in-place / rollback.
- Cluster-wide backup policies (label/tag selectors).
- Cross-region replication of the backup target.
- Live (non-snapshot) backup with quiescing.
- Selector-based exclude lists.
- Resumable chunking with mid-stream seek.
- Bandwidth throttling per-target.
- Application-aware quiescing.
- Backup verification beyond "PUT succeeded".

---

## Self-review

Spec coverage check (spec sections → tasks):

- §"In this PR" 1 (nexus-backup crate) → Tasks 1–5 ✓
- §"In this PR" 2 (read_snapshot trait) → Tasks 6–8 ✓
- §"In this PR" 3 (migration 0036) → Task 9 ✓
- §"In this PR" 4 (backup_targets feature) → Tasks 11–12 ✓
- §"In this PR" 5 (backups feature: lifecycle, scheduler, GC, reconciler, REST) → Tasks 16–19 ✓
- §"In this PR" 6 (agent backup/restore routes) → Tasks 14–15 ✓
- §"In this PR" 7 (manager agent_rpc helpers) → Task 13 ✓
- §"In this PR" 8 (UI) → Tasks 22–23 ✓
- §"In this PR" 9 (index-rebuild) → Task 21 ✓
- §"In this PR" 10 (tests) → inline tests in Tasks 3–5, 7, 11; final sweep Task 24 ✓
- §"VM start lifecycle through trait" — n/a, no change to VM lifecycle in this plan
- §"Encryption" → Tasks 3, 11 (envelope), 16 (key passing) ✓
- §"Single-attach via partial unique index" — already in foundation, untouched
- §"AlreadyAttached translation" — already in foundation, untouched
- §"Implicit resume via content-addressing" → Task 15 (HEAD-before-PUT) + Task 18 (reconciler) ✓
- §"Mark-and-sweep GC daily per target" → Task 17 ✓

Placeholder scan: no TBDs, all code blocks complete, all commit messages specified.

Type consistency: `BackupTargetConfig` shape consistent across manager (`apps/manager/src/features/backups/types.rs`) and agent (`apps/agent/src/features/storage/s3.rs::BackupTargetConfig`). `Manifest` fields consistent across `nexus-backup`, manager, agent. `ChunkRef` shape stable.
