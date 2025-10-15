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
