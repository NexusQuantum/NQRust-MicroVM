use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::{Path, State},
    http::StatusCode,
    response::Response,
    routing::get,
    Json, Router,
};
use futures::{stream::SplitSink, SinkExt, StreamExt};
use serde::Serialize;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
    sync::Mutex,
};

use crate::AppState;

#[derive(Serialize)]
pub struct ConsoleInfo {
    pub console_sock: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/:id/shell/info", get(get_console_info))
        .route("/:id/shell/ws", get(ws_console_proxy))
}

async fn get_console_info(
    State(st): State<AppState>,
    Path(vm_id): Path<String>,
) -> Result<Json<ConsoleInfo>, (StatusCode, String)> {
    let socket_path = PathBuf::from(&st.run_dir)
        .join("vms")
        .join(&vm_id)
        .join("sock/console.sock");

    if !socket_path.exists() {
        return Err((StatusCode::NOT_FOUND, "console socket not found".into()));
    }

    Ok(Json(ConsoleInfo {
        console_sock: socket_path.display().to_string(),
    }))
}

pub async fn proxy_console(socket: PathBuf, ws: WebSocket) -> Result<(), (StatusCode, String)> {
    let stream = UnixStream::connect(&socket)
        .await
        .map_err(|err| (StatusCode::BAD_GATEWAY, err.to_string()))?;

    let (mut read_socket, mut write_socket) = stream.into_split();
    let (ws_tx, mut ws_rx) = ws.split();
    let ws_tx: Arc<Mutex<SplitSink<WebSocket, Message>>> = Arc::new(Mutex::new(ws_tx));

    let ws_tx_clone = ws_tx.clone();
    let ws_to_sock = async {
        while let Some(msg) = ws_rx.next().await {
            let msg = msg.map_err(|err| (StatusCode::BAD_GATEWAY, err.to_string()))?;
            match msg {
                Message::Text(text) => {
                    write_socket
                        .write_all(text.as_bytes())
                        .await
                        .map_err(|err| (StatusCode::BAD_GATEWAY, err.to_string()))?;
                }
                Message::Binary(data) => {
                    write_socket
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

    let ws_tx_clone = ws_tx.clone();
    let sock_to_ws = async {
        let mut buf = [0u8; 1024];
        loop {
            let n = read_socket
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
        res = ws_to_sock => res?,
        res = sock_to_ws => res?,
    }

    Ok(())
}

async fn ws_console_proxy(
    ws: WebSocketUpgrade,
    State(st): State<AppState>,
    Path(vm_id): Path<String>,
) -> Response {
    let socket_path = PathBuf::from(&st.run_dir)
        .join("vms")
        .join(&vm_id)
        .join("sock/console.sock");

    ws.on_upgrade(move |socket| async move {
        if let Err(err) = proxy_console(socket_path, socket).await {
            tracing::warn!(error = ?err, "websocket proxy for console failed");
        }
    })
}
