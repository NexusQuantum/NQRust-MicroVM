use crate::core::systemd;
use crate::AppState;
use axum::http::StatusCode;
use axum::{
    extract::Path,
    routing::post,
    Router,
    Json,
    Extension,
};
use serde::Deserialize;
use tokio::{fs, io::AsyncWriteExt};

#[derive(Deserialize)]
struct SpawnReq {
    sock: String,
    log_path: String,
}

pub fn router() -> Router { Router::new().route("/:id/spawn", post(spawn_fc)) }

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
    systemd::spawn_fc_scope(&unit, &req.sock)
        .await
        .map_err(int)?;

    for _ in 0..80 {
        if std::path::Path::new(&req.sock).exists() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    if !std::path::Path::new(&req.sock).exists() {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "UDS not created".into()));
    }

    Ok(Json(serde_json::json!({"fc_unit": unit, "sock": req.sock})))
}
fn int<E: std::fmt::Display>(e: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}
