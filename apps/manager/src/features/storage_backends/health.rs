//! Per-backend reachability + capacity probe. Read-only — never
//! mutates anything on the storage system. Used by the UI to render
//! a status indicator next to each backend in the list view.

use crate::features::storage_backends::repo::StorageBackendRow;
use serde::Serialize;
use std::time::Duration;

const PROBE_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct BackendHealth {
    pub reachable: bool,
    /// Human-readable status ("ok", "unreachable: <error>", "skipped: <kind>").
    pub status: String,
    /// Available + total capacity in bytes when the kind supports it.
    /// Both `None` for kinds we don't probe.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub used_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_bytes: Option<u64>,
}

pub async fn check_backend_health(row: &StorageBackendRow) -> BackendHealth {
    match row.kind.as_str() {
        "local_file" => probe_local_file(row).await,
        "nfs" => probe_nfs(row).await,
        "truenas_iscsi" => probe_truenas(row).await,
        "iscsi_lvm" => BackendHealth {
            reachable: false,
            status: "not yet implemented (Task 10)".into(),
            used_bytes: None,
            total_bytes: None,
        },
        other => BackendHealth {
            reachable: true,
            status: format!("skipped: {other} kind has no health probe"),
            used_bytes: None,
            total_bytes: None,
        },
    }
}

async fn probe_local_file(row: &StorageBackendRow) -> BackendHealth {
    let root = row
        .config_json
        .get("root_dir")
        .and_then(|v| v.as_str())
        .unwrap_or("/srv/fc/vms");
    match tokio::fs::metadata(root).await {
        Ok(_) => BackendHealth {
            reachable: true,
            status: "ok".into(),
            used_bytes: None,
            total_bytes: None,
        },
        Err(e) => BackendHealth {
            reachable: false,
            status: format!("unreachable: {e}"),
            used_bytes: None,
            total_bytes: None,
        },
    }
}

async fn probe_nfs(row: &StorageBackendRow) -> BackendHealth {
    let server = row.config_json.get("server").and_then(|v| v.as_str());
    let Some(server) = server else {
        return BackendHealth {
            reachable: false,
            status: "no server in config".into(),
            used_bytes: None,
            total_bytes: None,
        };
    };
    let target = format!("{server}:2049");
    match tokio::time::timeout(PROBE_TIMEOUT, tokio::net::TcpStream::connect(&target)).await {
        Ok(Ok(_)) => BackendHealth {
            reachable: true,
            status: "ok".into(),
            used_bytes: None,
            total_bytes: None,
        },
        Ok(Err(e)) => BackendHealth {
            reachable: false,
            status: format!("unreachable: {e}"),
            used_bytes: None,
            total_bytes: None,
        },
        Err(_) => BackendHealth {
            reachable: false,
            status: "unreachable: timeout".into(),
            used_bytes: None,
            total_bytes: None,
        },
    }
}

async fn probe_truenas(row: &StorageBackendRow) -> BackendHealth {
    let endpoint = row.config_json.get("endpoint").and_then(|v| v.as_str());
    let api_key_env = row
        .config_json
        .get("api_key_env")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let pool = row.config_json.get("pool").and_then(|v| v.as_str());
    let Some(endpoint) = endpoint else {
        return BackendHealth {
            reachable: false,
            status: "no endpoint in config".into(),
            used_bytes: None,
            total_bytes: None,
        };
    };
    let api_key = std::env::var(api_key_env).unwrap_or_default();
    if api_key.is_empty() {
        return BackendHealth {
            reachable: false,
            status: format!("api key env var '{api_key_env}' not set in manager"),
            used_bytes: None,
            total_bytes: None,
        };
    }
    let client = match reqwest::Client::builder().timeout(PROBE_TIMEOUT).build() {
        Ok(c) => c,
        Err(e) => {
            return BackendHealth {
                reachable: false,
                status: format!("reqwest builder: {e}"),
                used_bytes: None,
                total_bytes: None,
            };
        }
    };
    let url = format!("{}/api/v2.0/iscsi/global", endpoint.trim_end_matches('/'));
    match client
        .get(&url)
        .header("Authorization", format!("Bearer {api_key}"))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            // Best-effort capacity: GET pool/dataset/id/<pool>.
            let (used, total) = if let Some(pool) = pool {
                let dataset_url = format!(
                    "{}/api/v2.0/pool/dataset/id/{}",
                    endpoint.trim_end_matches('/'),
                    urlencoding::encode(pool)
                );
                match client
                    .get(&dataset_url)
                    .header("Authorization", format!("Bearer {api_key}"))
                    .send()
                    .await
                {
                    Ok(r) if r.status().is_success() => match r.json::<serde_json::Value>().await {
                        Ok(v) => {
                            let used = v
                                .get("used")
                                .and_then(|x| x.get("parsed"))
                                .and_then(|x| x.as_u64());
                            let avail = v
                                .get("available")
                                .and_then(|x| x.get("parsed"))
                                .and_then(|x| x.as_u64());
                            let total = match (used, avail) {
                                (Some(u), Some(a)) => Some(u + a),
                                _ => None,
                            };
                            (used, total)
                        }
                        Err(_) => (None, None),
                    },
                    _ => (None, None),
                }
            } else {
                (None, None)
            };
            BackendHealth {
                reachable: true,
                status: "ok".into(),
                used_bytes: used,
                total_bytes: total,
            }
        }
        Ok(resp) => BackendHealth {
            reachable: false,
            status: format!("unreachable: HTTP {}", resp.status()),
            used_bytes: None,
            total_bytes: None,
        },
        Err(e) => BackendHealth {
            reachable: false,
            status: format!("unreachable: {e}"),
            used_bytes: None,
            total_bytes: None,
        },
    }
}
