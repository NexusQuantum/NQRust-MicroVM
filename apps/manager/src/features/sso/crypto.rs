use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use rand::RngCore;

/// Encrypt a plaintext string using AES-256-GCM.
/// Returns base64-encoded `nonce:ciphertext`.
pub fn encrypt(plaintext: &str, key: &[u8; 32]) -> Result<String> {
    let cipher = Aes256Gcm::new_from_slice(key).context("invalid AES key")?;

    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| anyhow::anyhow!("encryption failed: {}", e))?;

    let mut combined = nonce_bytes.to_vec();
    combined.extend_from_slice(&ciphertext);
    Ok(general_purpose::STANDARD.encode(&combined))
}

/// Decrypt a base64-encoded `nonce:ciphertext` string using AES-256-GCM.
pub fn decrypt(encrypted: &str, key: &[u8; 32]) -> Result<String> {
    let combined = general_purpose::STANDARD
        .decode(encrypted)
        .context("invalid base64")?;

    if combined.len() < 12 {
        anyhow::bail!("ciphertext too short");
    }

    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let cipher = Aes256Gcm::new_from_slice(key).context("invalid AES key")?;
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| anyhow::anyhow!("decryption failed: {}", e))?;

    String::from_utf8(plaintext).context("decrypted data is not valid UTF-8")
}

/// Derive a 32-byte key from an environment variable value using SHA-256.
pub fn derive_key(key_material: &str) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(key_material.as_bytes());
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}
