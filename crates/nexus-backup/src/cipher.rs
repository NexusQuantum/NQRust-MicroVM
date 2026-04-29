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

/// Encrypt the manifest with a random nonce. Returns nonce-prepended
/// ciphertext: `[nonce(24) | ciphertext+tag]`.
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

    fn key() -> ChunkKey { ChunkKey::from_bytes([0x42u8; 32]) }

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
        cipher[0] ^= 0x01;
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
