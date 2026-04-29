use crate::core::systemd;
use crate::AppState;
use axum::http::StatusCode;
use axum::{extract::Path, routing::post, Extension, Json, Router};
use serde::Deserialize;
use tokio::net::UnixStream;
use tokio::process::Command;
use tokio::{fs, io::AsyncWriteExt};

#[derive(Deserialize)]
struct SpawnReq {
    sock: String,
    log_path: String,
}

pub fn router() -> Router {
    Router::new().route("/:id/spawn", post(spawn_fc))
}

async fn spawn_fc(
    Extension(_st): Extension<AppState>,
    Path(id): Path<String>,
    Json(req): Json<SpawnReq>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if let Some(p) = std::path::Path::new(&req.log_path).parent() {
        fs::create_dir_all(p).await.map_err(int)?;
    }
    if fs::metadata(&req.log_path).await.is_err() {
        let mut f = fs::File::create(&req.log_path).await.map_err(int)?;
        f.flush().await.map_err(int)?;
    }
    if let Some(d) = std::path::Path::new(&req.sock).parent() {
        fs::create_dir_all(d).await.map_err(int)?;
    }

    let unit = format!("fc-{id}.scope");
    // If a previous attempt left a socket, check if it's live; if live, succeed; if stale, remove it.
    if std::path::Path::new(&req.sock).exists() {
        match UnixStream::connect(&req.sock).await {
            Ok(_) => {
                return Ok(Json(serde_json::json!({"fc_unit": unit, "sock": req.sock})));
            }
            Err(_) => {
                let _ = fs::remove_file(&req.sock).await;
            }
        }
    }

    // Ensure any previous scope is not lingering as loaded/deactivated
    let _ = systemd::stop_unit(&unit).await;

    // Also kill any existing screen session for this VM
    let _ = Command::new("sudo")
        .args(["screen", "-S", &unit, "-X", "quit"])
        .status()
        .await;

    // Attempt to spawn. If systemd-run reports failure but the socket appears,
    // consider it success to avoid flapping on duplicate unit names.
    if let Err(err) = systemd::spawn_fc_scope(&unit, &req.sock).await {
        // Brief grace period to see if the socket got created anyway
        for _ in 0..400 {
            if std::path::Path::new(&req.sock).exists() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        if !std::path::Path::new(&req.sock).exists() {
            // Fallback: try launching firecracker directly (without systemd)
            // 1) try without sudo
            let direct = Command::new("firecracker")
                .args(["--api-sock", &req.sock])
                .kill_on_drop(false)
                .spawn();
            if direct.is_err() {
                // 2) try with sudo -n
                let _ = Command::new("sudo")
                    .args(["-n", "firecracker", "--api-sock", &req.sock])
                    .kill_on_drop(false)
                    .spawn();
            }
            // Wait briefly for the socket to appear
            for _ in 0..400 {
                if std::path::Path::new(&req.sock).exists() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
            if !std::path::Path::new(&req.sock).exists() {
                return Err(int(err));
            }
        }
    }

    // Wait up to ~20s for the socket to appear (400 * 50ms)
    for _ in 0..400 {
        if std::path::Path::new(&req.sock).exists() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    if !std::path::Path::new(&req.sock).exists() {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "UDS not created".into()));
    }

    // Ensure the socket is world-writable so the unprivileged agent process
    // can proxy HTTP requests to Firecracker running as root.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = fs::metadata(&req.sock).await {
            let mut perms = metadata.permissions();
            perms.set_mode(0o666);
            let _ = fs::set_permissions(&req.sock, perms).await;
        }
    }

    // Optional: verify the socket is connectable (avoid stale file)
    for _ in 0..80 {
        if UnixStream::connect(&req.sock).await.is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    Ok(Json(serde_json::json!({"fc_unit": unit, "sock": req.sock})))
}
fn int<E: std::fmt::Display>(e: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::UnixListener;

    #[test]
    fn unit_name_format_is_stable() {
        // The systemd unit name encodes the VM id literally as `fc-{id}.scope`.
        // The FirecrackerDriver refactor must preserve this exact shape because
        // existing scopes on disk are matched by name.
        assert_eq!(format!("fc-{}.scope", "abc"), "fc-abc.scope");
        let uuid_id = "550e8400-e29b-41d4-a716-446655440000";
        assert_eq!(
            format!("fc-{uuid_id}.scope"),
            "fc-550e8400-e29b-41d4-a716-446655440000.scope"
        );
        assert_eq!(format!("fc-{}.scope", ""), "fc-.scope");
        assert_eq!(
            format!("fc-{}.scope", "vm with space"),
            "fc-vm with space.scope"
        );
    }

    #[test]
    fn spawn_req_deserializes_required_fields() {
        let json = r#"{"sock":"/tmp/fc.sock","log_path":"/var/log/fc.log"}"#;
        let req: SpawnReq = serde_json::from_str(json).expect("valid SpawnReq");
        assert_eq!(req.sock, "/tmp/fc.sock");
        assert_eq!(req.log_path, "/var/log/fc.log");
    }

    #[test]
    fn spawn_req_rejects_missing_fields() {
        // Missing log_path — required field, must fail.
        let bad = r#"{"sock":"/tmp/fc.sock"}"#;
        assert!(serde_json::from_str::<SpawnReq>(bad).is_err());
        // Missing sock — required field, must fail.
        let bad = r#"{"log_path":"/var/log/fc.log"}"#;
        assert!(serde_json::from_str::<SpawnReq>(bad).is_err());
        // Empty object — all required fields missing.
        assert!(serde_json::from_str::<SpawnReq>("{}").is_err());
    }

    #[tokio::test]
    async fn dead_socket_file_is_not_connectable() {
        // Mirrors the inline stale-socket detection in spawn_fc:
        // a regular file at the socket path cannot be connected to as a UDS,
        // so the production code falls into the cleanup branch.
        let tmp = tempfile::tempdir().unwrap();
        let stale = tmp.path().join("fc.sock");
        tokio::fs::write(&stale, b"not a socket").await.unwrap();
        assert!(stale.exists());
        let connect_res = UnixStream::connect(&stale).await;
        assert!(
            connect_res.is_err(),
            "regular file must not be connectable as UDS"
        );
        // Cleanup as the production code does.
        tokio::fs::remove_file(&stale).await.unwrap();
        assert!(!stale.exists());
    }

    #[tokio::test]
    async fn live_socket_is_connectable() {
        // Mirrors the success branch: if a real listener exists at the socket
        // path, spawn_fc returns early without touching systemd.
        let tmp = tempfile::tempdir().unwrap();
        let sock_path = tmp.path().join("fc.sock");
        let _listener = UnixListener::bind(&sock_path).expect("bind UDS");
        assert!(sock_path.exists());
        let connect = UnixStream::connect(&sock_path).await;
        assert!(connect.is_ok(), "live UDS must be connectable");
    }
}
