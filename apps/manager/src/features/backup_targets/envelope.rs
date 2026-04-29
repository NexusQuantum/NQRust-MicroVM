//! AES-GCM(envelope_key) wrap/unwrap for backup target secrets.
//! Reuses the same MANAGER_ENVELOPE_KEY env var the SSO module uses.

use aes_gcm::{aead::Aead, Aes256Gcm, Key, KeyInit, Nonce};
use anyhow::{anyhow, Context, Result};

const NONCE_LEN: usize = 12;

fn cipher() -> Result<Aes256Gcm> {
    let raw = std::env::var("MANAGER_ENVELOPE_KEY").context("MANAGER_ENVELOPE_KEY not set")?;
    let bytes = hex::decode(raw).context("MANAGER_ENVELOPE_KEY must be hex-encoded")?;
    if bytes.len() != 32 {
        return Err(anyhow!(
            "MANAGER_ENVELOPE_KEY must be 32 bytes (64 hex chars), got {}",
            bytes.len()
        ));
    }
    let key = Key::<Aes256Gcm>::from_slice(&bytes);
    Ok(Aes256Gcm::new(key))
}

pub fn wrap(plaintext: &[u8]) -> Result<Vec<u8>> {
    use rand::RngCore;
    let c = cipher()?;
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ct = c
        .encrypt(nonce, plaintext)
        .map_err(|e| anyhow!("aes-gcm encrypt: {e}"))?;
    let mut out = Vec::with_capacity(NONCE_LEN + ct.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ct);
    Ok(out)
}

#[allow(dead_code)]
pub fn unwrap_to_string(blob: &[u8]) -> Result<String> {
    let bytes = unwrap(blob)?;
    String::from_utf8(bytes).context("decrypted secret is not utf-8")
}

#[allow(dead_code)]
pub fn unwrap_to_array<const N: usize>(blob: &[u8]) -> Result<[u8; N]> {
    let bytes = unwrap(blob)?;
    if bytes.len() != N {
        return Err(anyhow!(
            "decrypted blob is {} bytes, expected {}",
            bytes.len(),
            N
        ));
    }
    let mut out = [0u8; N];
    out.copy_from_slice(&bytes);
    Ok(out)
}

#[allow(dead_code)]
fn unwrap(blob: &[u8]) -> Result<Vec<u8>> {
    if blob.len() < NONCE_LEN {
        return Err(anyhow!("envelope blob too short"));
    }
    let c = cipher()?;
    let nonce = Nonce::from_slice(&blob[..NONCE_LEN]);
    c.decrypt(nonce, &blob[NONCE_LEN..])
        .map_err(|_| anyhow!("envelope decrypt: auth failed"))
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
