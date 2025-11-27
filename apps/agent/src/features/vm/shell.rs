use std::sync::Arc;

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::Path,
    http::StatusCode,
    response::Response,
    routing::get,
    Extension, Json, Router,
};
use futures::{stream::SplitSink, SinkExt, StreamExt};
use serde::Serialize;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
    sync::Mutex,
};

use crate::AppState;

#[derive(Serialize)]
pub struct ConsoleInfo {
    pub screen_session: String,
}

pub fn router() -> Router {
    Router::new()
        .route("/:id/shell/info", get(get_console_info))
        .route("/:id/shell/ws", get(ws_console_proxy))
}

async fn get_console_info(
    Extension(_st): Extension<AppState>,
    Path(vm_id): Path<String>,
) -> Result<Json<ConsoleInfo>, (StatusCode, String)> {
    // The screen session name is fc-{vm_id}.scope
    let screen_name = format!("fc-{}.scope", vm_id);

    // Check if screen session exists
    let output = Command::new("sudo")
        .args(["screen", "-ls", &screen_name])
        .output()
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    if !output.status.success() {
        return Err((StatusCode::NOT_FOUND, "screen session not found".into()));
    }

    Ok(Json(ConsoleInfo {
        screen_session: screen_name,
    }))
}

pub async fn proxy_console_screen(
    screen_name: String,
    ws: WebSocket,
) -> Result<(), (StatusCode, String)> {
    // Spawn screen -x to attach to the session
    // We use 'script' to allocate a PTY because screen requires a terminal
    // script -qfc "command" /dev/null runs command with a PTY and outputs to stdout
    // Set TERM=xterm-256color to ensure proper terminal emulation
    let mut child = Command::new("sudo")
        .args([
            "script",
            "-qfc",
            &format!("TERM=xterm-256color screen -x {}", screen_name),
            "/dev/null",
        ])
        .env("TERM", "xterm-256color")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to spawn screen: {}", err),
            )
        })?;

    let mut stdin = child.stdin.take().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "Failed to get stdin".to_string(),
    ))?;
    let mut stdout = child.stdout.take().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "Failed to get stdout".to_string(),
    ))?;

    let (ws_tx, mut ws_rx) = ws.split();
    let ws_tx: Arc<Mutex<SplitSink<WebSocket, Message>>> = Arc::new(Mutex::new(ws_tx));

    // WebSocket -> Screen stdin
    let ws_tx_clone = ws_tx.clone();
    let ws_to_screen = async {
        while let Some(msg) = ws_rx.next().await {
            let msg = msg.map_err(|err| (StatusCode::BAD_GATEWAY, err.to_string()))?;
            match msg {
                Message::Text(text) => {
                    stdin
                        .write_all(text.as_bytes())
                        .await
                        .map_err(|err| (StatusCode::BAD_GATEWAY, err.to_string()))?;
                }
                Message::Binary(data) => {
                    stdin
                        .write_all(&data)
                        .await
                        .map_err(|err| (StatusCode::BAD_GATEWAY, err.to_string()))?;
                }
                Message::Ping(payload) => {
                    ws_tx_clone
                        .lock()
                        .await
                        .send(Message::Pong(payload))
                        .await
                        .map_err(|err| (StatusCode::BAD_GATEWAY, err.to_string()))?;
                }
                Message::Pong(_) => {}
                Message::Close(_) => break,
            }
        }
        Ok::<_, (StatusCode, String)>(())
    };

    // Screen stdout -> WebSocket
    let ws_tx_clone = ws_tx.clone();
    let screen_to_ws = async {
        let mut buf = [0u8; 1024];
        loop {
            let n = stdout
                .read(&mut buf)
                .await
                .map_err(|err| (StatusCode::BAD_GATEWAY, err.to_string()))?;
            if n == 0 {
                ws_tx_clone
                    .lock()
                    .await
                    .send(Message::Close(None))
                    .await
                    .map_err(|err| (StatusCode::BAD_GATEWAY, err.to_string()))?;
                break;
            }
            ws_tx_clone
                .lock()
                .await
                .send(Message::Binary(buf[..n].to_vec()))
                .await
                .map_err(|err| (StatusCode::BAD_GATEWAY, err.to_string()))?;
        }
        Ok::<_, (StatusCode, String)>(())
    };

    tokio::select! {
        res = ws_to_screen => res?,
        res = screen_to_ws => res?,
    }

    // Kill the screen -x process when done
    let _ = child.kill().await;

    Ok(())
}

async fn ws_console_proxy(
    ws: WebSocketUpgrade,
    Extension(_st): Extension<AppState>,
    Path(vm_id): Path<String>,
) -> Response {
    // The screen session name is fc-{vm_id}.scope
    let screen_name = format!("fc-{}.scope", vm_id);

    ws.on_upgrade(move |socket| async move {
        if let Err(err) = proxy_console_screen(screen_name, socket).await {
            tracing::warn!(error = ?err, "websocket proxy for console failed");
        }
    })
}
