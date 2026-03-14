use crate::features::licensing::device::get_or_create_device_id;
use crate::features::licensing::repo::LicensingRepository;
use base64::Engine as _;
use nexus_types::LicenseState;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Configuration read from environment variables.
#[derive(Debug, Clone)]
pub struct LicenseConfig {
    pub server_url: String,
    pub api_key: String,
    pub grace_period_days: i64,
    pub persist_dir: String,
    pub license_key: Option<String>,
    /// PEM-encoded Ed25519 public key for offline .lic verification.
    pub public_key: Option<String>,
}

impl LicenseConfig {
    pub fn from_env() -> Self {
        // Load public key from env var, or from a file path
        let public_key = std::env::var("LICENSE_PUBLIC_KEY").ok().or_else(|| {
            std::env::var("LICENSE_PUBLIC_KEY_FILE")
                .ok()
                .and_then(|f| std::fs::read_to_string(f).ok())
        });

        Self {
            server_url: std::env::var("LICENSE_SERVER_URL")
                .unwrap_or_else(|_| "https://billing.nexusquantum.id".to_string()),
            api_key: std::env::var("LICENSE_API_KEY").unwrap_or_default(),
            grace_period_days: std::env::var("LICENSE_GRACE_PERIOD_DAYS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(7),
            persist_dir: std::env::var("LICENSE_PERSIST_DIR")
                .unwrap_or_else(|_| ".data".to_string()),
            license_key: std::env::var("LICENSE_KEY").ok(),
            public_key,
        }
    }
}

/// Structures matching the billing server's JSON responses.
#[derive(Debug, Deserialize)]
struct VerifyResponse {
    #[serde(default)]
    valid: bool,
    license: Option<VerifyLicense>,
    activations: Option<i32>,
    #[serde(rename = "maxActivations")]
    max_activations: Option<i32>,
    error: Option<String>,
    message: Option<String>,
}

fn deserialize_null_as_empty_vec<'de, D>(d: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<Vec<String>>::deserialize(d)?.unwrap_or_default())
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct VerifyLicense {
    #[serde(default)]
    key: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    product: String,
    #[serde(rename = "productId", default)]
    product_id: String,
    #[serde(default)]
    customer: String,
    #[serde(rename = "customerId", default)]
    customer_id: String,
    #[serde(default, deserialize_with = "deserialize_null_as_empty_vec")]
    features: Vec<String>,
    #[serde(rename = "createdAt", default)]
    created_at: String,
    #[serde(rename = "expiresAt", default)]
    expires_at: String,
}

#[derive(Debug, Serialize)]
struct VerifyRequest {
    #[serde(rename = "licenseKey")]
    license_key: String,
    #[serde(rename = "deviceId")]
    device_id: String,
    #[serde(rename = "deviceName")]
    device_name: String,
}

/// Payload decoded from the base64 body of a .lic file.
#[derive(Debug, Deserialize)]
struct LicOfflinePayload {
    #[serde(rename = "licenseId", default)]
    license_id: String,
    #[serde(rename = "customerName", default)]
    customer_name: String,
    #[serde(rename = "productName", default)]
    product_name: String,
    #[serde(default, deserialize_with = "deserialize_null_as_empty_vec")]
    features: Vec<String>,
    #[serde(rename = "maxActivations")]
    max_activations: Option<i32>,
    #[serde(rename = "expiresAt", default)]
    expires_at: String,
}

pub type SharedLicenseState = Arc<RwLock<LicenseState>>;

fn mask_key(key: &str) -> String {
    let parts: Vec<&str> = key.split('-').collect();
    if parts.len() != 4 {
        return "****".to_string();
    }
    format!("{}-****-****-{}", parts[0], parts[3])
}

/// Extract the content between `-----BEGIN <name>-----` and `-----END <name>-----`.
fn extract_lic_section<'a>(content: &'a str, name: &str) -> Option<&'a str> {
    let begin = format!("-----BEGIN {}-----", name);
    let end = format!("-----END {}-----", name);
    let start = content.find(&begin)? + begin.len();
    let finish = content.find(&end)?;
    Some(content[start..finish].trim())
}

/// Verify an offline .lic file using the configured Ed25519 public key.
/// Returns a LicenseState on success, or an error string.
pub fn verify_offline_lic(
    config: &LicenseConfig,
    lic_content: &str,
) -> Result<LicenseState, String> {
    use ed25519_dalek::Verifier;

    let public_key_pem = config.public_key.as_deref().ok_or_else(|| {
        "Offline verification is not configured (LICENSE_PUBLIC_KEY not set)".to_string()
    })?;

    // Parse Ed25519 public key from PEM (SubjectPublicKeyInfo / SPKI format).
    // Strip the PEM armor and decode; the raw 32-byte key is the last 32 bytes of the DER.
    // Handle both multi-line and single-line PEM formats (single-line occurs when
    // environment variables lose newlines, e.g. GitHub Actions secrets via option_env!).
    let pem_body = {
        let s = public_key_pem.trim();
        // Remove BEGIN/END markers regardless of line structure
        let s = s
            .replace("-----BEGIN PUBLIC KEY-----", "")
            .replace("-----END PUBLIC KEY-----", "");
        // Remove any whitespace/newlines left over
        s.chars().filter(|c| !c.is_whitespace()).collect::<String>()
    };
    let der = base64::engine::general_purpose::STANDARD
        .decode(pem_body.trim())
        .map_err(|e| format!("Invalid public key PEM encoding: {}", e))?;
    if der.len() < 32 {
        return Err("Public key DER is too short".to_string());
    }
    let raw_key: &[u8; 32] = der[der.len() - 32..]
        .try_into()
        .map_err(|_| "Failed to extract raw key bytes".to_string())?;
    let verifying_key = ed25519_dalek::VerifyingKey::from_bytes(raw_key)
        .map_err(|e| format!("Invalid public key bytes: {}", e))?;

    // Extract sections from the .lic file
    let payload_b64 = extract_lic_section(lic_content, "LICENSE")
        .ok_or_else(|| "Missing LICENSE section in .lic file".to_string())?;
    let sig_b64 = extract_lic_section(lic_content, "SIGNATURE")
        .ok_or_else(|| "Missing SIGNATURE section in .lic file".to_string())?;

    // Decode base64
    let payload_bytes = base64::engine::general_purpose::STANDARD
        .decode(payload_b64)
        .map_err(|e| format!("Invalid payload encoding: {}", e))?;
    let sig_bytes = base64::engine::general_purpose::STANDARD
        .decode(sig_b64)
        .map_err(|e| format!("Invalid signature encoding: {}", e))?;

    // Verify Ed25519 signature
    let signature = ed25519_dalek::Signature::from_slice(&sig_bytes)
        .map_err(|e| format!("Invalid signature format: {}", e))?;
    verifying_key
        .verify(&payload_bytes, &signature)
        .map_err(|_| "Invalid signature — license file may be tampered or corrupted".to_string())?;

    // Parse the JSON payload
    let payload: LicOfflinePayload = serde_json::from_slice(&payload_bytes)
        .map_err(|e| format!("Invalid license payload: {}", e))?;

    // Check expiration
    if !payload.expires_at.is_empty() {
        let today = chrono::Utc::now().date_naive();
        let expires = chrono::NaiveDate::parse_from_str(&payload.expires_at, "%Y-%m-%d")
            .map_err(|e| format!("Invalid expiresAt date: {}", e))?;
        if expires < today {
            return Err(format!("License expired on {}", payload.expires_at));
        }
    }

    Ok(LicenseState {
        is_licensed: true,
        status: "active".to_string(),
        is_grace_period: false,
        grace_days_remaining: None,
        customer_name: Some(payload.customer_name),
        product: Some(payload.product_name),
        features: payload.features,
        expires_at: if payload.expires_at.is_empty() {
            None
        } else {
            Some(payload.expires_at)
        },
        activations: None,
        max_activations: payload.max_activations,
        verified_at: Some(chrono::Utc::now().to_rfc3339()),
        license_key: Some(mask_key(&payload.license_id)),
        error_message: None,
    })
}

/// Activate via .lic file: verify signature, cache result, persist license ID.
pub async fn activate_offline_license(
    config: &LicenseConfig,
    repo: &LicensingRepository,
    shared_state: &SharedLicenseState,
    lic_content: &str,
) -> LicenseState {
    match verify_offline_lic(config, lic_content) {
        Ok(state) => {
            // Extract license ID from payload for DB caching
            let license_id = extract_lic_section(lic_content, "LICENSE")
                .and_then(|b64| base64::engine::general_purpose::STANDARD.decode(b64).ok())
                .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
                .and_then(|v| v["licenseId"].as_str().map(str::to_string))
                .unwrap_or_else(|| "offline".to_string());

            let features_json = serde_json::to_value(&state.features).unwrap_or_default();
            let expires = state
                .expires_at
                .as_deref()
                .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
            let device_id = get_or_create_device_id(&config.persist_dir);

            let _ = repo
                .upsert_license(
                    &license_id,
                    "active",
                    state.customer_name.as_deref(),
                    state.product.as_deref(),
                    features_json,
                    expires,
                    None,
                    state.max_activations,
                    &device_id,
                    true,
                )
                .await;

            // Persist .lic content to disk
            let lic_file = std::path::Path::new(&config.persist_dir).join("license.lic");
            let _ = std::fs::create_dir_all(&config.persist_dir);
            let _ = std::fs::write(&lic_file, lic_content);

            let mut guard = shared_state.write().await;
            *guard = state.clone();
            state
        }
        Err(e) => LicenseState {
            error_message: Some(e),
            ..Default::default()
        },
    }
}

/// Verify a license key against the billing server.
async fn verify_online(
    config: &LicenseConfig,
    license_key: &str,
    device_id: &str,
) -> Result<LicenseState, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let url = format!("{}/api/v1/licenses/verify", config.server_url);

    let body = VerifyRequest {
        license_key: license_key.to_string(),
        device_id: device_id.to_string(),
        device_name: "Nexus MicroVM".to_string(),
    };

    let resp = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", config.api_key))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    tracing::debug!("License verify response HTTP {}: {}", status, body);

    let data: VerifyResponse = serde_json::from_str(&body).map_err(|e| {
        format!(
            "Response parse error (HTTP {}): {} — body: {}",
            status, e, body
        )
    })?;

    if data.valid {
        if let Some(lic) = data.license {
            return Ok(LicenseState {
                is_licensed: true,
                status: "active".to_string(),
                is_grace_period: false,
                grace_days_remaining: None,
                customer_name: Some(lic.customer),
                product: Some(lic.product),
                features: lic.features,
                expires_at: if lic.expires_at.is_empty() {
                    None
                } else {
                    Some(lic.expires_at)
                },
                activations: data.activations,
                max_activations: data.max_activations,
                verified_at: Some(chrono::Utc::now().to_rfc3339()),
                license_key: Some(mask_key(license_key)),
                error_message: None,
            });
        }
    }

    let status = match data.error.as_deref() {
        Some("license_expired") => "expired",
        _ => "invalid",
    };

    Ok(LicenseState {
        is_licensed: false,
        status: status.to_string(),
        is_grace_period: false,
        grace_days_remaining: None,
        customer_name: None,
        product: None,
        features: vec![],
        expires_at: None,
        activations: None,
        max_activations: None,
        verified_at: None,
        license_key: Some(mask_key(license_key)),
        error_message: data
            .message
            .or(data.error)
            .or_else(|| Some("Verification failed".to_string())),
    })
}

/// Check grace period from cached DB row.
async fn check_grace_period(repo: &LicensingRepository, grace_days: i64) -> Option<LicenseState> {
    let cached = match repo.get_latest_license().await {
        Ok(Some(row)) => row,
        _ => return None,
    };

    if cached.status != "active" {
        return None;
    }

    let verified_at = cached.verified_at?;
    let now = chrono::Utc::now();
    let days_since = (now - verified_at).num_days();
    let today = now.date_naive();

    // Valid if: within the grace window, OR the license has a future expiry (or no expiry = perpetual)
    let in_grace_window = days_since <= grace_days;
    let expiry_ok = cached.expires_at.is_none_or(|d| d > today);

    if !in_grace_window && !expiry_ok {
        return None;
    }

    let features: Vec<String> = cached
        .features
        .and_then(|f| serde_json::from_value(f).ok())
        .unwrap_or_default();

    // Show grace_period flag only when outside the normal verification window
    // (signals "needs re-verification soon" but app remains operational)
    let is_grace_period = !in_grace_window;
    let grace_days_remaining = if in_grace_window {
        Some(grace_days - days_since)
    } else {
        None
    };

    Some(LicenseState {
        is_licensed: true,
        status: if is_grace_period {
            "grace_period".to_string()
        } else {
            "active".to_string()
        },
        is_grace_period,
        grace_days_remaining,
        customer_name: cached.customer_name,
        product: cached.product,
        features,
        expires_at: cached.expires_at.map(|d| d.to_string()),
        activations: cached.activations,
        max_activations: cached.max_activations,
        verified_at: Some(verified_at.to_rfc3339()),
        license_key: Some(mask_key(&cached.license_key)),
        error_message: None,
    })
}

/// Activate a license key: verify online, cache result, persist key.
pub async fn activate_license(
    config: &LicenseConfig,
    repo: &LicensingRepository,
    shared_state: &SharedLicenseState,
    license_key: &str,
) -> LicenseState {
    let device_id = get_or_create_device_id(&config.persist_dir);

    match verify_online(config, license_key, &device_id).await {
        Ok(state) => {
            if state.is_licensed {
                // Cache to DB
                let features_json = serde_json::to_value(&state.features).unwrap_or_default();
                let expires = state
                    .expires_at
                    .as_deref()
                    .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
                let _ = repo
                    .upsert_license(
                        license_key,
                        "active",
                        state.customer_name.as_deref(),
                        state.product.as_deref(),
                        features_json,
                        expires,
                        state.activations,
                        state.max_activations,
                        &device_id,
                        false,
                    )
                    .await;

                // Persist key to disk
                let key_file = std::path::Path::new(&config.persist_dir).join(".license-key");
                let _ = std::fs::create_dir_all(&config.persist_dir);
                let _ = std::fs::write(&key_file, license_key);
            }

            // Update shared state
            let mut guard = shared_state.write().await;
            *guard = state.clone();
            state
        }
        Err(e) => LicenseState {
            error_message: Some(e),
            license_key: Some(mask_key(license_key)),
            ..Default::default()
        },
    }
}

/// Full 3-tier license check: online → offline .lic → grace period.
pub async fn check_license(
    config: &LicenseConfig,
    repo: &LicensingRepository,
    shared_state: &SharedLicenseState,
) -> LicenseState {
    // Load persisted key from env or disk
    let license_key = config.license_key.clone().or_else(|| {
        let key_file = std::path::Path::new(&config.persist_dir).join(".license-key");
        std::fs::read_to_string(&key_file)
            .ok()
            .map(|s| s.trim().to_string())
    });

    let device_id = get_or_create_device_id(&config.persist_dir);

    if let Some(key) = license_key.filter(|k| !k.is_empty()) {
        // Tier 1: Online verification
        match verify_online(config, &key, &device_id).await {
            Ok(state) if state.is_licensed => {
                let features_json = serde_json::to_value(&state.features).unwrap_or_default();
                let expires = state
                    .expires_at
                    .as_deref()
                    .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
                let _ = repo
                    .upsert_license(
                        &key,
                        "active",
                        state.customer_name.as_deref(),
                        state.product.as_deref(),
                        features_json,
                        expires,
                        state.activations,
                        state.max_activations,
                        &device_id,
                        false,
                    )
                    .await;

                let mut guard = shared_state.write().await;
                *guard = state.clone();
                return state;
            }
            Ok(state) => {
                // Definitively invalid/expired from server
                let mut guard = shared_state.write().await;
                *guard = state.clone();
                return state;
            }
            Err(e) => {
                warn!("Online license verification failed: {}", e);
            }
        }
    }

    // Tier 2: Offline .lic file from disk
    let lic_file = std::path::Path::new(&config.persist_dir).join("license.lic");
    if let Ok(lic_content) = std::fs::read_to_string(&lic_file) {
        match verify_offline_lic(config, &lic_content) {
            Ok(state) => {
                info!("License verified from offline .lic file");
                let mut guard = shared_state.write().await;
                *guard = state.clone();
                return state;
            }
            Err(e) => {
                warn!("Offline .lic verification failed: {}", e);
            }
        }
    }

    // Tier 3: Grace period from cached DB
    if let Some(grace_state) = check_grace_period(repo, config.grace_period_days).await {
        info!(
            "License in grace period, {} days remaining",
            grace_state.grace_days_remaining.unwrap_or(0)
        );
        let mut guard = shared_state.write().await;
        *guard = grace_state.clone();
        return grace_state;
    }

    // All tiers failed
    let state = LicenseState {
        error_message: Some("Could not verify license".to_string()),
        ..Default::default()
    };
    let mut guard = shared_state.write().await;
    *guard = state.clone();
    state
}
