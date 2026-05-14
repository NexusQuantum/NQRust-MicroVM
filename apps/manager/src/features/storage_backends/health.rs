//! Per-backend reachability + capacity probe. Read-only — never
//! mutates anything on the storage system. Used by the UI to render
//! a status indicator next to each backend in the list view.

use crate::features::storage::backends::iscsi_lvm::IscsiLvmConfig;
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

pub async fn check_backend_health(
    row: &StorageBackendRow,
    default_agent_url: Option<&str>,
) -> BackendHealth {
    match row.kind.as_str() {
        "local_file" => probe_local_file(row).await,
        "nfs" => probe_nfs(row).await,
        "truenas_iscsi" => probe_truenas(row).await,
        "iscsi_lvm" => probe_iscsi_lvm(row, default_agent_url).await,
        "smb" => probe_smb(row, default_agent_url).await,
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

async fn probe_iscsi_lvm(
    row: &StorageBackendRow,
    default_agent_url: Option<&str>,
) -> BackendHealth {
    let cfg: IscsiLvmConfig = match serde_json::from_value(row.config_json.clone()) {
        Ok(c) => c,
        Err(e) => {
            return BackendHealth {
                reachable: false,
                status: format!("config decode: {e}"),
                used_bytes: None,
                total_bytes: None,
            };
        }
    };
    let agent_url_owned: String = cfg
        .agent_url
        .clone()
        .or_else(|| default_agent_url.map(|s| s.to_string()))
        .unwrap_or_default();
    if agent_url_owned.is_empty() {
        return BackendHealth {
            reachable: false,
            status: "no agent_url".into(),
            used_bytes: None,
            total_bytes: None,
        };
    }
    let base = {
        let with_scheme =
            if agent_url_owned.starts_with("http://") || agent_url_owned.starts_with("https://") {
                agent_url_owned.clone()
            } else {
                format!("http://{agent_url_owned}")
            };
        with_scheme.trim_end_matches('/').to_string()
    };
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
    {
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
    // Best-effort: a session may already be open; ignore login errors.
    let _ = client
        .post(format!("{base}/v1/storage/iscsi_lvm/login"))
        .json(&serde_json::json!({"iqn": cfg.iqn, "portal": cfg.portal}))
        .send()
        .await;
    match client
        .post(format!("{base}/v1/storage/iscsi_lvm/vg_status"))
        .json(&serde_json::json!({"vg": cfg.vg_name}))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => match resp.json::<serde_json::Value>().await {
            Ok(v) => {
                let size = v.get("size_bytes").and_then(|x| x.as_u64()).unwrap_or(0);
                let free = v.get("free_bytes").and_then(|x| x.as_u64()).unwrap_or(0);
                BackendHealth {
                    reachable: true,
                    status: "ok".into(),
                    used_bytes: Some(size.saturating_sub(free)),
                    total_bytes: Some(size),
                }
            }
            Err(e) => BackendHealth {
                reachable: false,
                status: format!("vg_status decode: {e}"),
                used_bytes: None,
                total_bytes: None,
            },
        },
        Ok(resp) => BackendHealth {
            reachable: false,
            status: format!("vg_status http {}", resp.status()),
            used_bytes: None,
            total_bytes: None,
        },
        Err(e) => BackendHealth {
            reachable: false,
            status: format!("vg_status: {e}"),
            used_bytes: None,
            total_bytes: None,
        },
    }
}

async fn probe_smb(row: &StorageBackendRow, default_agent_url: Option<&str>) -> BackendHealth {
    use crate::features::storage::backends::smb::{SmbConfig, SmbControlPlaneBackend};
    use nexus_storage::{BackendInstanceId, ControlPlaneBackend};

    let mut cfg: SmbConfig = match serde_json::from_value(row.config_json.clone()) {
        Ok(c) => c,
        Err(e) => {
            return BackendHealth {
                reachable: false,
                status: format!("config decode: {e}"),
                used_bytes: None,
                total_bytes: None,
            }
        }
    };
    if cfg.agent_url.is_none() {
        cfg.agent_url = default_agent_url.map(|s| s.to_string());
    }
    let mount_point = cfg.mount_point();
    let backend = SmbControlPlaneBackend {
        id: BackendInstanceId(row.id),
        config: cfg,
    };
    match backend.probe().await {
        Ok(()) => {
            // Probe succeeded; try to read disk usage from the mount.
            // Manager runs unprivileged — best-effort, may return None.
            let (used, total) = match tokio::process::Command::new("df")
                .args(["-B1", "--output=used,size", &mount_point.to_string_lossy()])
                .output()
                .await
            {
                Ok(out) if out.status.success() => {
                    let s = String::from_utf8_lossy(&out.stdout);
                    let line = s.lines().nth(1).unwrap_or("");
                    let mut parts = line.split_whitespace();
                    let used = parts.next().and_then(|n| n.parse::<u64>().ok());
                    let total = parts.next().and_then(|n| n.parse::<u64>().ok());
                    (used, total)
                }
                _ => (None, None),
            };
            BackendHealth {
                reachable: true,
                status: "ok".into(),
                used_bytes: used,
                total_bytes: total,
            }
        }
        Err(e) => BackendHealth {
            reachable: false,
            status: format!("probe: {e}"),
            used_bytes: None,
            total_bytes: None,
        },
    }
}
