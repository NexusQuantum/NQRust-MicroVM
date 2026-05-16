use crate::features::users::repo::AuthenticatedUser;
use crate::AppState;
use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, WebSocketUpgrade,
    },
    response::IntoResponse,
    Extension, Json,
};
use futures::{SinkExt, StreamExt};
use nexus_types::{
    BalloonConfig, BalloonStatsConfig, CpuConfigReq, CreateDriveReq, CreateNicReq, CreateVmReq,
    CreateVmResponse, EntropyConfigReq, GetVmResponse, ListDrivesResponse, ListNicsResponse,
    ListVmsResponse, LoggerUpdateReq, MachineConfigPatchReq, MmdsConfigReq, MmdsDataReq,
    OkResponse, SerialConfigReq, UpdateDriveReq, UpdateNicReq, UpdateVmReq, Vm, VmDrive, VmNic,
    VmPathParams, VsockConfigReq,
};
use reqwest::StatusCode;
use serde::Serialize;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message as WsMessage};
use uuid::Uuid;

fn extract_user_info(user: Option<Extension<AuthenticatedUser>>) -> (Option<Uuid>, String) {
    match user {
        Some(Extension(u)) => (Some(u.id), u.username),
        None => (None, "system".to_string()),
    }
}

#[utoipa::path(
    get,
    path = "/v1/vms/{id}/shell",
    params(VmPathParams),
    responses(
        (status = 200, description = "Shell credentials", body = VmShellCredentialResponse),
        (status = 404, description = "VM or credentials not found"),
        (status = 500, description = "Failed to fetch credentials"),
    ),
    tag = "VMs"
)]
pub async fn get_shell_credentials(
    Extension(st): Extension<AppState>,
    Path(VmPathParams { id }): Path<VmPathParams>,
) -> Result<Json<VmShellCredentialResponse>, StatusCode> {
    match st
        .shell_repo
        .get_credentials(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        Some(cred) => Ok(Json(VmShellCredentialResponse {
            username: cred.username,
            password: cred.password,
        })),
        None => Err(StatusCode::NOT_FOUND),
    }
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct VmShellCredentialResponse {
    pub username: String,
    pub password: String,
}

#[utoipa::path(
    get,
    path = "/v1/vms/{id}/shell/ws",
    params(VmPathParams),
    responses(
        (status = 101, description = "WebSocket connection established"),
        (status = 404, description = "VM not found"),
        (status = 502, description = "Failed to connect to agent"),
    ),
    tag = "VMs"
)]
pub async fn shell_websocket(
    ws: WebSocketUpgrade,
    Extension(st): Extension<AppState>,
    Path(VmPathParams { id }): Path<VmPathParams>,
) -> axum::response::Response {
    // Fetch VM to get host address
    let vm = match super::repo::get(&st.db, id).await {
        Ok(v) => v,
        Err(_) => {
            return (StatusCode::NOT_FOUND, "VM not found").into_response();
        }
    };

    // Upgrade the WebSocket connection
    ws.on_upgrade(move |socket| async move {
        if let Err(e) = proxy_to_agent_shell(vm.host_addr, id, socket).await {
            tracing::error!("WebSocket proxy error: {:?}", e);
        }
    })
}

/// Browser ↔ noVNC bridge. Mirrors `shell_websocket` but targets the
/// agent's VNC WS endpoint instead of the shell. Used by the UI's
/// in-browser noVNC client for graphical install / Windows access.
#[utoipa::path(
    get,
    path = "/v1/vms/{id}/console/vnc/ws",
    params(VmPathParams),
    responses(
        (status = 101, description = "WebSocket connection established"),
        (status = 404, description = "VM not found"),
        (status = 400, description = "VM has no VNC console"),
    ),
    tag = "VMs"
)]
pub async fn vnc_websocket(
    ws: WebSocketUpgrade,
    Extension(st): Extension<AppState>,
    Path(VmPathParams { id }): Path<VmPathParams>,
) -> axum::response::Response {
    let vm = match super::repo::get(&st.db, id).await {
        Ok(v) => v,
        Err(_) => return (StatusCode::NOT_FOUND, "VM not found").into_response(),
    };
    // Only QEMU VMs with console_kind='vnc' have a VNC endpoint.
    let vmm_kind: String = sqlx::query_scalar(r#"SELECT vmm_kind FROM vm WHERE id = $1"#)
        .bind(id)
        .fetch_one(&st.db)
        .await
        .unwrap_or_else(|_| "firecracker".into());
    if vmm_kind != "qemu" {
        return (StatusCode::BAD_REQUEST, "VNC console is qemu-only").into_response();
    }
    ws.on_upgrade(move |socket| async move {
        if let Err(e) = proxy_to_agent_vnc(vm.host_addr, id, socket).await {
            tracing::error!("VNC proxy error: {:?}", e);
        }
    })
}

async fn proxy_to_agent_vnc(
    host_addr: String,
    vm_id: Uuid,
    client_ws: WebSocket,
) -> Result<(), Box<dyn std::error::Error>> {
    let agent_url = format!(
        "ws://{}/agent/v1/vmm/{}/console/vnc/ws?vmm_kind=qemu",
        host_addr.trim_start_matches("http://"),
        vm_id
    );
    tracing::info!("Connecting to agent VNC at: {}", agent_url);
    let (agent_stream, _) = connect_async(&agent_url).await?;
    let (mut agent_write, mut agent_read) = agent_stream.split();
    let (mut client_write, mut client_read) = client_ws.split();

    let client_to_agent = async {
        while let Some(msg) = client_read.next().await {
            match msg {
                Ok(Message::Binary(data)) => {
                    if agent_write.send(WsMessage::Binary(data)).await.is_err() {
                        break;
                    }
                }
                Ok(Message::Text(text)) => {
                    if agent_write.send(WsMessage::Text(text)).await.is_err() {
                        break;
                    }
                }
                Ok(Message::Close(_)) | Err(_) => break,
                Ok(_) => {}
            }
        }
    };
    let agent_to_client = async {
        while let Some(msg) = agent_read.next().await {
            match msg {
                Ok(WsMessage::Binary(data)) => {
                    if client_write.send(Message::Binary(data)).await.is_err() {
                        break;
                    }
                }
                Ok(WsMessage::Text(text)) => {
                    if client_write.send(Message::Text(text)).await.is_err() {
                        break;
                    }
                }
                Ok(WsMessage::Close(_)) | Err(_) => break,
                _ => {}
            }
        }
    };
    tokio::select! {
        _ = client_to_agent => {}
        _ = agent_to_client => {}
    }
    Ok(())
}

async fn proxy_to_agent_shell(
    host_addr: String,
    vm_id: Uuid,
    client_ws: WebSocket,
) -> Result<(), Box<dyn std::error::Error>> {
    // Connect to agent's WebSocket endpoint
    let agent_url = format!(
        "ws://{}/agent/v1/vms/{}/shell/ws",
        host_addr.trim_start_matches("http://"),
        vm_id
    );
    tracing::info!("Connecting to agent shell at: {}", agent_url);

    let (agent_stream, _) = connect_async(&agent_url).await?;
    let (mut agent_write, mut agent_read) = agent_stream.split();
    let (mut client_write, mut client_read) = client_ws.split();

    // Proxy messages bidirectionally
    let client_to_agent = async {
        while let Some(msg) = client_read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if agent_write.send(WsMessage::Text(text)).await.is_err() {
                        break;
                    }
                }
                Ok(Message::Binary(data)) => {
                    if agent_write.send(WsMessage::Binary(data)).await.is_err() {
                        break;
                    }
                }
                Ok(Message::Ping(data)) => {
                    if agent_write.send(WsMessage::Ping(data)).await.is_err() {
                        break;
                    }
                }
                Ok(Message::Pong(data)) => {
                    if agent_write.send(WsMessage::Pong(data)).await.is_err() {
                        break;
                    }
                }
                Ok(Message::Close(_)) => break,
                Err(_) => break,
            }
        }
    };

    let agent_to_client = async {
        while let Some(msg) = agent_read.next().await {
            match msg {
                Ok(WsMessage::Text(text)) => {
                    if client_write.send(Message::Text(text)).await.is_err() {
                        break;
                    }
                }
                Ok(WsMessage::Binary(data)) => {
                    if client_write.send(Message::Binary(data)).await.is_err() {
                        break;
                    }
                }
                Ok(WsMessage::Ping(data)) => {
                    if client_write.send(Message::Ping(data)).await.is_err() {
                        break;
                    }
                }
                Ok(WsMessage::Pong(data)) => {
                    if client_write.send(Message::Pong(data)).await.is_err() {
                        break;
                    }
                }
                Ok(WsMessage::Close(_)) => break,
                Err(_) => break,
                _ => {}
            }
        }
    };

    tokio::select! {
        _ = client_to_agent => {},
        _ = agent_to_client => {},
    }

    Ok(())
}

#[utoipa::path(
    get,
    path = "/v1/vms/{id}/metrics/ws",
    params(VmPathParams),
    responses(
        (status = 101, description = "WebSocket connection established"),
        (status = 404, description = "VM not found"),
        (status = 400, description = "VM not running"),
    ),
    tag = "VMs"
)]
pub async fn metrics_websocket(
    ws: WebSocketUpgrade,
    Extension(st): Extension<AppState>,
    Path(VmPathParams { id }): Path<VmPathParams>,
) -> axum::response::Response {
    // Fetch VM to check if it's running
    let vm = match super::repo::get(&st.db, id).await {
        Ok(v) => v,
        Err(_) => {
            return (StatusCode::NOT_FOUND, "VM not found").into_response();
        }
    };

    if vm.state != "running" {
        return (
            StatusCode::BAD_REQUEST,
            "VM must be running to stream metrics",
        )
            .into_response();
    }

    // Upgrade the WebSocket connection
    ws.on_upgrade(move |socket| async move {
        if let Err(e) = stream_metrics(st, id, socket).await {
            tracing::error!(vm_id = %id, "Metrics WebSocket error: {:?}", e);
        }
    })
}

async fn stream_metrics(
    st: AppState,
    vm_id: Uuid,
    ws: WebSocket,
) -> Result<(), Box<dyn std::error::Error>> {
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tokio::net::unix::pipe;
    use tokio::time::{interval, timeout, Duration};

    let (mut sender, mut receiver) = ws.split();
    let metrics_path = format!("/srv/fc/vms/{}/logs/metrics.json", vm_id);

    let mut ticker = interval(Duration::from_secs(1));
    let mut last_metrics: Option<serde_json::Value> = None;

    // Open the FIFO once and keep it open for the entire session.
    // Previously the FIFO was opened/closed each tick, which caused Firecracker
    // to get EPIPE on writes (no reader present when FlushMetrics was called).
    // Using tokio::net::unix::pipe for proper async, non-blocking FIFO I/O.
    let fifo_rx = match pipe::OpenOptions::new().open_receiver(&metrics_path) {
        Ok(rx) => rx,
        Err(e) => {
            tracing::warn!(vm_id = %vm_id, error = %e, "Failed to open metrics FIFO — was the VM started with metrics enabled?");
            return Ok(());
        }
    };
    let mut reader = BufReader::new(fifo_rx);

    loop {
        tokio::select! {
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => {
                        tracing::info!(vm_id = %vm_id, "Metrics WebSocket client disconnected");
                        break;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        if sender.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    _ => {}
                }
            }

            _ = ticker.tick() => {
                // Flush metrics from Firecracker into the FIFO
                if let Err(e) = super::service::flush_vm_metrics(&st, vm_id).await {
                    tracing::debug!(vm_id = %vm_id, "Failed to flush metrics: {}", e);
                    continue;
                }

                // Read one JSON line with a timeout (Firecracker writes within ms of flush)
                let mut line = String::new();
                match timeout(Duration::from_millis(800), reader.read_line(&mut line)).await {
                    Ok(Ok(n)) if n > 0 => {
                        if let Ok(fc_metrics) = serde_json::from_str::<serde_json::Value>(&line) {
                            let (cpu_percent, memory_percent) = match super::service::get_process_stats(&st, vm_id).await {
                                Ok(stats) => (stats.cpu_percent, stats.memory_percent),
                                Err(e) => {
                                    tracing::debug!(vm_id = %vm_id, "Failed to get process stats: {}", e);
                                    (0.0, 0.0)
                                }
                            };

                            let simplified = simplify_firecracker_metrics(
                                &fc_metrics,
                                last_metrics.as_ref(),
                                cpu_percent,
                                memory_percent,
                            );

                            if let Ok(json) = serde_json::to_string(&simplified) {
                                if sender.send(Message::Text(json)).await.is_err() {
                                    break;
                                }
                            }

                            last_metrics = Some(fc_metrics);
                        } else {
                            tracing::warn!(vm_id = %vm_id, "Failed to parse Firecracker metrics JSON");
                        }
                    }
                    Ok(Err(e)) => {
                        tracing::debug!(vm_id = %vm_id, "Failed to read metrics FIFO: {}", e);
                    }
                    _ => {
                        // Timeout or empty read — skip this tick
                    }
                }
            }
        }
    }

    Ok(())
}

fn simplify_firecracker_metrics(
    fc_metrics: &serde_json::Value,
    _last_metrics: Option<&serde_json::Value>,
    cpu_percent: f64,
    memory_percent: f64,
) -> serde_json::Value {
    use serde_json::json;

    let obj = fc_metrics.as_object();

    // Extract network metrics - keys are like "net_eth0", "net_eth1", etc.
    // Note: Firecracker uses rx_bytes_count and tx_bytes_count
    // Firecracker resets counters after each flush, so values represent bytes since last flush
    let (network_rx, network_tx) = obj
        .map(|o| {
            let mut rx_total = 0u64;
            let mut tx_total = 0u64;

            for (key, value) in o {
                if key.starts_with("net_") {
                    if let Some(net_stats) = value.as_object() {
                        let rx = net_stats
                            .get("rx_bytes_count")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        let tx = net_stats
                            .get("tx_bytes_count")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        tracing::debug!("Found network interface {}: rx={}, tx={}", key, rx, tx);
                        rx_total += rx;
                        tx_total += tx;
                    }
                }
            }

            tracing::debug!("Network totals: rx={}, tx={}", rx_total, tx_total);
            (rx_total, tx_total)
        })
        .unwrap_or((0, 0));

    // Extract block device metrics - keys are like "block_rootfs", "block_sda", etc.
    // Firecracker resets counters after each flush, so values represent bytes since last flush
    let (disk_read, disk_write) = obj
        .map(|o| {
            let mut read_total = 0u64;
            let mut write_total = 0u64;

            for (key, value) in o {
                if key.starts_with("block_") {
                    if let Some(block_stats) = value.as_object() {
                        let rd = block_stats
                            .get("read_bytes")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        let wr = block_stats
                            .get("write_bytes")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        tracing::debug!("Found block device {}: read={}, write={}", key, rd, wr);
                        read_total += rd;
                        write_total += wr;
                    }
                }
            }

            tracing::debug!("Disk totals: read={}, write={}", read_total, write_total);
            (read_total, write_total)
        })
        .unwrap_or((0, 0));

    json!({
        "cpu_usage_percent": cpu_percent,  // From host-side process monitoring
        "memory_usage_percent": memory_percent,  // From host-side process monitoring
        "network_in_bytes": network_rx,
        "network_out_bytes": network_tx,
        "disk_read_bytes": disk_read,
        "disk_write_bytes": disk_write,
    })
}

#[utoipa::path(
    post,
    path = "/v1/vms",
    request_body = CreateVmReq,
    responses(
        (status = 200, description = "VM created", body = CreateVmResponse),
        (status = 500, description = "Failed to create VM"),
    ),
    tag = "VMs"
)]
pub async fn create(
    Extension(st): Extension<AppState>,
    user: Option<Extension<AuthenticatedUser>>,
    Json(req): Json<CreateVmReq>,
) -> Result<Json<CreateVmResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (user_id, username) = extract_user_info(user);
    let id = Uuid::new_v4();
    super::service::create_and_start(&st, id, req, None, user_id, &username)
        .await
        .map_err(|err| {
            tracing::error!(vm_id = %id, error = ?err, "create VM failed (full chain)");
            let chain: Vec<String> = err.chain().map(|e| e.to_string()).collect();
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to create VM".to_string(),
                    fault_message: Some(chain.join(" -> ")),
                }),
            )
        })?;
    Ok(Json(CreateVmResponse { id }))
}

#[utoipa::path(
    get,
    path = "/v1/vms",
    responses(
        (status = 200, description = "VMs listed", body = ListVmsResponse),
        (status = 500, description = "Failed to list VMs"),
    ),
    tag = "VMs"
)]
pub async fn list(
    Extension(st): Extension<AppState>,
) -> Result<Json<ListVmsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let items = super::repo::list(&st.db).await.map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to list VMs".to_string(),
                fault_message: Some(err.to_string()),
            }),
        )
    })?;
    let items = items.into_iter().map(Vm::from).collect();
    Ok(Json(ListVmsResponse { items }))
}

#[utoipa::path(
    get,
    path = "/v1/vms/{id}",
    params(VmPathParams),
    responses(
        (status = 200, description = "VM fetched", body = GetVmResponse),
        (status = 404, description = "VM not found"),
        (status = 500, description = "Failed to fetch VM"),
    ),
    tag = "VMs"
)]
pub async fn get(
    Extension(st): Extension<AppState>,
    Path(VmPathParams { id }): Path<VmPathParams>,
) -> Result<Json<GetVmResponse>, (StatusCode, Json<ErrorResponse>)> {
    let row = super::repo::get(&st.db, id).await.map_err(|_| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "VM not found".to_string(),
                fault_message: None,
            }),
        )
    })?;
    Ok(Json(GetVmResponse { item: row.into() }))
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fault_message: Option<String>,
}

#[utoipa::path(
    patch,
    path = "/v1/vms/{id}",
    params(VmPathParams),
    request_body = UpdateVmReq,
    responses(
        (status = 200, description = "VM updated", body = OkResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "VM not found"),
        (status = 500, description = "Failed to update VM"),
    ),
    tag = "VMs"
)]
pub async fn update(
    Extension(st): Extension<AppState>,
    user: Option<Extension<AuthenticatedUser>>,
    Path(VmPathParams { id }): Path<VmPathParams>,
    Json(req): Json<UpdateVmReq>,
) -> Result<Json<OkResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (user_id, username) = extract_user_info(user);
    super::service::update_vm_metadata(
        &st,
        id,
        req.name.as_deref(),
        req.tags.as_deref(),
        user_id,
        &username,
    )
    .await
    .map_err(|err| {
        let err_str = err.to_string();
        let status = if err_str.contains("not found") {
            StatusCode::NOT_FOUND
        } else if err_str.contains("cannot be empty") {
            StatusCode::BAD_REQUEST
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        };
        (
            status,
            Json(ErrorResponse {
                error: "Failed to update VM".to_string(),
                fault_message: Some(err_str),
            }),
        )
    })?;
    Ok(Json(OkResponse { ok: true }))
}

#[utoipa::path(
    post,
    path = "/v1/vms/{id}/start",
    params(VmPathParams),
    responses(
        (status = 200, description = "VM started", body = OkResponse),
        (status = 404, description = "VM not found"),
        (status = 500, description = "Failed to start VM"),
    ),
    tag = "VMs"
)]
pub async fn start(
    Extension(st): Extension<AppState>,
    user: Option<Extension<AuthenticatedUser>>,
    Path(VmPathParams { id }): Path<VmPathParams>,
) -> Result<Json<OkResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (user_id, username) = extract_user_info(user);
    super::service::start_vm_by_id_with_user(&st, id, user_id, &username)
        .await
        .map_err(|err| {
            let err_str = err.to_string();
            let status = if err_str.contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (
                status,
                Json(ErrorResponse {
                    error: "Failed to start VM".to_string(),
                    fault_message: Some(err_str),
                }),
            )
        })?;
    Ok(Json(OkResponse::default()))
}

#[utoipa::path(
    post,
    path = "/v1/vms/{id}/stop",
    params(VmPathParams),
    responses(
        (status = 200, description = "VM stopped", body = OkResponse),
        (status = 500, description = "Failed to stop VM"),
    ),
    tag = "VMs"
)]
pub async fn stop(
    Extension(st): Extension<AppState>,
    user: Option<Extension<AuthenticatedUser>>,
    Path(VmPathParams { id }): Path<VmPathParams>,
) -> Result<Json<OkResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (user_id, username) = extract_user_info(user);
    super::service::stop_only(&st, id, user_id, &username)
        .await
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to stop VM".to_string(),
                    fault_message: Some(err.to_string()),
                }),
            )
        })?;
    Ok(Json(OkResponse::default()))
}

#[utoipa::path(
    post,
    path = "/v1/vms/{id}/pause",
    params(VmPathParams),
    responses(
        (status = 200, description = "VM paused", body = OkResponse),
        (status = 400, description = "VM must be running to pause"),
        (status = 404, description = "VM not found"),
        (status = 500, description = "Failed to pause VM"),
    ),
    tag = "VMs"
)]
pub async fn pause(
    Extension(st): Extension<AppState>,
    user: Option<Extension<AuthenticatedUser>>,
    Path(VmPathParams { id }): Path<VmPathParams>,
) -> Result<Json<OkResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (user_id, username) = extract_user_info(user);
    super::service::pause_vm(&st, id, user_id, &username)
        .await
        .map_err(|err| {
            let err_str = err.to_string();
            let status = if err_str.contains("must be running") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (
                status,
                Json(ErrorResponse {
                    error: "Failed to pause VM".to_string(),
                    fault_message: Some(err_str),
                }),
            )
        })?;
    Ok(Json(OkResponse::default()))
}

#[utoipa::path(
    post,
    path = "/v1/vms/{id}/resume",
    params(VmPathParams),
    responses(
        (status = 200, description = "VM resumed", body = OkResponse),
        (status = 400, description = "VM must be paused to resume"),
        (status = 404, description = "VM not found"),
        (status = 500, description = "Failed to resume VM"),
    ),
    tag = "VMs"
)]
pub async fn resume(
    Extension(st): Extension<AppState>,
    user: Option<Extension<AuthenticatedUser>>,
    Path(VmPathParams { id }): Path<VmPathParams>,
) -> Result<Json<OkResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (user_id, username) = extract_user_info(user);
    super::service::resume_vm(&st, id, user_id, &username)
        .await
        .map_err(|err| {
            let err_str = err.to_string();
            let status = if err_str.contains("must be paused") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (
                status,
                Json(ErrorResponse {
                    error: "Failed to resume VM".to_string(),
                    fault_message: Some(err_str),
                }),
            )
        })?;
    Ok(Json(OkResponse::default()))
}

/// Back up a VM. For VMs whose rootfs lives on a registered storage volume
/// (the production path), delegates to the existing volume-backup pipeline
/// which handles chunked-encrypted upload via nexus-backup. For QEMU VMs
/// using a local qcow2 overlay (no storage backend), drives the agent's
/// /backup/disk primitive to write the qcow2 to a backup target directory.
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct BackupVmRequest {
    /// Backup target UUID (for volume-backed VMs — uses the existing
    /// nexus-backup chunked upload pipeline). Required when the VM has a
    /// volume_attachment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_id: Option<Uuid>,
    /// Destination path on the agent host (for overlay-backed QEMU VMs).
    /// Should live on a network-mounted backup share. Required when the VM
    /// has no volume_attachment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destination_path: Option<String>,
    /// `qcow2` or `raw`. Defaults to qcow2 for a compact backup.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// Pass `-c` to qemu-img for compressed backup output.
    #[serde(default)]
    pub compress: bool,
}

#[utoipa::path(
    post,
    path = "/v1/vms/{id}/backup",
    params(VmPathParams),
    request_body = BackupVmRequest,
    responses(
        (status = 200, description = "Backup created", body = OkResponse),
        (status = 400, description = "Invalid request"),
        (status = 502, description = "Agent reported failure"),
    ),
    tag = "VMs"
)]
pub async fn backup_vm(
    Extension(st): Extension<AppState>,
    Path(VmPathParams { id }): Path<VmPathParams>,
    Json(req): Json<BackupVmRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let vm = super::repo::get(&st.db, id).await.map_err(|_| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "VM not found".into(),
                fault_message: None,
            }),
        )
    })?;
    // Find the rootfs volume_attachment, if any.
    let vol_id: Option<Uuid> = sqlx::query_scalar(
        r#"SELECT volume_id FROM volume_attachment
             WHERE vm_id = $1 AND drive_id = 'rootfs'
             ORDER BY attached_at DESC LIMIT 1"#,
    )
    .bind(id)
    .fetch_optional(&st.db)
    .await
    .ok()
    .flatten();

    // Path A: volume-backed → use existing chunked-encrypted backup pipeline.
    if let (Some(volume_id), Some(target_id)) = (vol_id, req.target_id) {
        let backup_id = crate::features::backups::service::create_backup(&st, volume_id, target_id)
            .await
            .map_err(|err| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "volume backup failed".into(),
                        fault_message: Some(err.to_string()),
                    }),
                )
            })?;
        return Ok(Json(serde_json::json!({
            "ok": true,
            "mode": "volume-backup",
            "backup_id": backup_id,
            "volume_id": volume_id,
        })));
    }

    // Path B: local overlay → ask the agent to qemu-img convert to the
    // backup destination. Caller is responsible for the destination path
    // being on a backup-safe filesystem (network share, etc.).
    let destination = req.destination_path.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "destination_path required for overlay-backed VMs".into(),
                fault_message: Some(
                    "VM has no volume_attachment; provide destination_path on a backup-target filesystem"
                        .into(),
                ),
            }),
        )
    })?;
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(1800)) // up to 30 min for large disks
        .build()
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "http client".into(),
                    fault_message: Some(err.to_string()),
                }),
            )
        })?;
    let resp = http
        .post(format!(
            "{}/agent/v1/vmm/{}/backup/disk",
            vm.host_addr, vm.id
        ))
        .json(&serde_json::json!({
            "vmm_kind": "qemu",
            "source": vm.rootfs_path,
            "destination": destination,
            "format": req.format.clone().unwrap_or_else(|| "qcow2".into()),
            "compress": req.compress,
        }))
        .send()
        .await
        .map_err(|err| {
            (
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse {
                    error: "agent backup request failed".into(),
                    fault_message: Some(err.to_string()),
                }),
            )
        })?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err((
            StatusCode::BAD_GATEWAY,
            Json(ErrorResponse {
                error: "agent backup returned non-2xx".into(),
                fault_message: Some(format!("{status}: {body}")),
            }),
        ));
    }
    let body: serde_json::Value = resp.json().await.unwrap_or_default();
    Ok(Json(serde_json::json!({
        "ok": true,
        "mode": "overlay-backup",
        "destination": destination,
        "size_bytes": body.get("size_bytes"),
    })))
}

/// Reschedule a QEMU VM onto a different host (HA / host-death recovery).
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct RescheduleRequest {
    pub target_host_id: Uuid,
}

#[utoipa::path(
    post,
    path = "/v1/vms/{id}/reschedule",
    params(VmPathParams),
    request_body = RescheduleRequest,
    responses(
        (status = 200, description = "VM rescheduled", body = OkResponse),
        (status = 400, description = "Reschedule preconditions not met"),
    ),
    tag = "VMs"
)]
pub async fn reschedule(
    Extension(st): Extension<AppState>,
    Path(VmPathParams { id }): Path<VmPathParams>,
    Json(req): Json<RescheduleRequest>,
) -> Result<Json<OkResponse>, (StatusCode, Json<ErrorResponse>)> {
    super::qemu_service::reschedule(&st, id, req.target_host_id)
        .await
        .map_err(|err| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "Reschedule failed".to_string(),
                    fault_message: Some(err.to_string()),
                }),
            )
        })?;
    Ok(Json(OkResponse::default()))
}

/// Live-migrate a QEMU VM to another host.
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct MigrateRequest {
    /// UUID of the target host. Must have `qemu` in vmm_kinds_installed.
    pub target_host_id: Uuid,
    /// TCP port the target QEMU is listening on with `-incoming`. The
    /// operator pre-launches the target VM in incoming mode for now;
    /// full target-side orchestration is a 0.5.x follow-up.
    pub target_port: u16,
}

#[utoipa::path(
    post,
    path = "/v1/vms/{id}/migrate",
    params(VmPathParams),
    request_body = MigrateRequest,
    responses(
        (status = 200, description = "Migration succeeded", body = OkResponse),
        (status = 400, description = "Invalid migration target"),
        (status = 502, description = "Agent reported migration failure"),
    ),
    tag = "VMs"
)]
pub async fn migrate(
    Extension(st): Extension<AppState>,
    Path(VmPathParams { id }): Path<VmPathParams>,
    Json(req): Json<MigrateRequest>,
) -> Result<Json<OkResponse>, (StatusCode, Json<ErrorResponse>)> {
    super::qemu_service::live_migrate(&st, id, req.target_host_id, req.target_port)
        .await
        .map_err(|err| {
            (
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse {
                    error: "Failed to migrate VM".to_string(),
                    fault_message: Some(err.to_string()),
                }),
            )
        })?;
    Ok(Json(OkResponse::default()))
}

/// Mark an `installing` VM as install-complete:
/// - Calls the agent to QMP-eject the installer CD-ROM (drive_id="installer")
/// - Transitions vm.state from "installing" → "running"
/// On a subsequent stop+start the VM will boot from the disk image without
/// the installer ISO still attached.
#[utoipa::path(
    post,
    path = "/v1/vms/{id}/install-complete",
    params(VmPathParams),
    responses(
        (status = 200, description = "Install marked complete", body = OkResponse),
        (status = 400, description = "VM is not in installing state"),
        (status = 404, description = "VM not found"),
        (status = 502, description = "Agent eject failed"),
    ),
    tag = "VMs"
)]
pub async fn install_complete(
    Extension(st): Extension<AppState>,
    Path(VmPathParams { id }): Path<VmPathParams>,
) -> Result<Json<OkResponse>, (StatusCode, Json<ErrorResponse>)> {
    let vm = super::repo::get(&st.db, id).await.map_err(|_| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "VM not found".into(),
                fault_message: None,
            }),
        )
    })?;
    // Only QEMU VMs in 'installing' state are valid targets.
    let vmm_kind: String = sqlx::query_scalar(r#"SELECT vmm_kind FROM vm WHERE id = $1"#)
        .bind(id)
        .fetch_one(&st.db)
        .await
        .unwrap_or_else(|_| "firecracker".into());
    if vmm_kind != "qemu" || vm.state != "installing" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "VM is not in installing state".into(),
                fault_message: Some(format!("vmm_kind={vmm_kind} state={}", vm.state)),
            }),
        ));
    }
    let http = reqwest::Client::new();
    let url = format!("{}/agent/v1/vmm/{}/cdrom/eject", vm.host_addr, vm.id);
    let resp = http
        .post(&url)
        .json(&serde_json::json!({"vmm_kind": "qemu", "drive_id": "installer"}))
        .send()
        .await
        .map_err(|err| {
            (
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse {
                    error: "agent eject failed".into(),
                    fault_message: Some(err.to_string()),
                }),
            )
        })?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err((
            StatusCode::BAD_GATEWAY,
            Json(ErrorResponse {
                error: "agent eject returned non-2xx".into(),
                fault_message: Some(format!("{status}: {body}")),
            }),
        ));
    }
    let _ = sqlx::query(r#"UPDATE vm SET state = 'running', updated_at = now() WHERE id = $1"#)
        .bind(id)
        .execute(&st.db)
        .await;
    Ok(Json(OkResponse::default()))
}

#[utoipa::path(
    delete,
    path = "/v1/vms/{id}",
    params(VmPathParams),
    responses(
        (status = 200, description = "VM deleted", body = OkResponse),
        (status = 500, description = "Failed to delete VM"),
    ),
    tag = "VMs"
)]
pub async fn delete(
    Extension(st): Extension<AppState>,
    user: Option<Extension<AuthenticatedUser>>,
    Path(VmPathParams { id }): Path<VmPathParams>,
) -> Result<Json<OkResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (user_id, username) = extract_user_info(user);
    super::service::stop_and_delete_with_user(&st, id, user_id, &username)
        .await
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to delete VM".to_string(),
                    fault_message: Some(err.to_string()),
                }),
            )
        })?;
    Ok(Json(OkResponse::default()))
}

#[utoipa::path(
    patch,
    path = "/v1/vms/{id}/machine-config",
    params(VmPathParams),
    request_body = MachineConfigPatchReq,
    responses(
        (status = 200, description = "Machine config patched", body = OkResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "VM not found"),
    ),
    tag = "VM configuration"
)]
pub async fn patch_machine_config(
    Extension(st): Extension<AppState>,
    Path(VmPathParams { id }): Path<VmPathParams>,
    Json(req): Json<MachineConfigPatchReq>,
) -> Result<Json<OkResponse>, axum::http::StatusCode> {
    super::service::patch_machine_config(&st, id, req)
        .await
        .map_err(|err| {
            if err
                .to_string()
                .contains("not within the configured image root")
            {
                axum::http::StatusCode::BAD_REQUEST
            } else if err.to_string().contains("not found") {
                axum::http::StatusCode::NOT_FOUND
            } else {
                axum::http::StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;
    Ok(Json(OkResponse::default()))
}

#[utoipa::path(
    put,
    path = "/v1/vms/{id}/cpu-config",
    params(VmPathParams),
    request_body = CpuConfigReq,
    responses(
        (status = 200, description = "CPU config applied", body = OkResponse),
        (status = 404, description = "VM not found"),
    ),
    tag = "VM configuration"
)]
pub async fn put_cpu_config(
    Extension(st): Extension<AppState>,
    Path(VmPathParams { id }): Path<VmPathParams>,
    Json(req): Json<CpuConfigReq>,
) -> Result<Json<OkResponse>, axum::http::StatusCode> {
    super::service::put_cpu_config(&st, id, req)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(OkResponse::default()))
}

#[utoipa::path(
    put,
    path = "/v1/vms/{id}/vsock",
    params(VmPathParams),
    request_body = VsockConfigReq,
    responses(
        (status = 200, description = "Vsock configured", body = OkResponse),
        (status = 404, description = "VM not found"),
    ),
    tag = "VM configuration"
)]
pub async fn put_vsock(
    Extension(st): Extension<AppState>,
    Path(VmPathParams { id }): Path<VmPathParams>,
    Json(req): Json<VsockConfigReq>,
) -> Result<Json<OkResponse>, axum::http::StatusCode> {
    super::service::put_vsock(&st, id, req)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(OkResponse::default()))
}

#[utoipa::path(
    put,
    path = "/v1/vms/{id}/mmds",
    params(VmPathParams),
    request_body = MmdsDataReq,
    responses(
        (status = 200, description = "MMDS data updated", body = OkResponse),
        (status = 404, description = "VM not found"),
    ),
    tag = "VM configuration"
)]
pub async fn put_mmds(
    Extension(st): Extension<AppState>,
    Path(VmPathParams { id }): Path<VmPathParams>,
    Json(req): Json<MmdsDataReq>,
) -> Result<Json<OkResponse>, axum::http::StatusCode> {
    super::service::put_mmds(&st, id, req)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(OkResponse::default()))
}

#[utoipa::path(
    put,
    path = "/v1/vms/{id}/mmds/config",
    params(VmPathParams),
    request_body = MmdsConfigReq,
    responses(
        (status = 200, description = "MMDS config updated", body = OkResponse),
        (status = 404, description = "VM not found"),
    ),
    tag = "VM configuration"
)]
pub async fn put_mmds_config(
    Extension(st): Extension<AppState>,
    Path(VmPathParams { id }): Path<VmPathParams>,
    Json(req): Json<MmdsConfigReq>,
) -> Result<Json<OkResponse>, axum::http::StatusCode> {
    super::service::put_mmds_config(&st, id, req)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(OkResponse::default()))
}

#[utoipa::path(
    put,
    path = "/v1/vms/{id}/entropy",
    params(VmPathParams),
    request_body = EntropyConfigReq,
    responses(
        (status = 200, description = "Entropy device configured", body = OkResponse),
        (status = 404, description = "VM not found"),
    ),
    tag = "VM configuration"
)]
pub async fn put_entropy(
    Extension(st): Extension<AppState>,
    Path(VmPathParams { id }): Path<VmPathParams>,
    Json(req): Json<EntropyConfigReq>,
) -> Result<Json<OkResponse>, axum::http::StatusCode> {
    super::service::put_entropy(&st, id, req)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(OkResponse::default()))
}

#[utoipa::path(
    put,
    path = "/v1/vms/{id}/serial",
    params(VmPathParams),
    request_body = SerialConfigReq,
    responses(
        (status = 200, description = "Serial device configured", body = OkResponse),
        (status = 404, description = "VM not found"),
    ),
    tag = "VM configuration"
)]
pub async fn put_serial(
    Extension(st): Extension<AppState>,
    Path(VmPathParams { id }): Path<VmPathParams>,
    Json(req): Json<SerialConfigReq>,
) -> Result<Json<OkResponse>, axum::http::StatusCode> {
    super::service::put_serial(&st, id, req)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(OkResponse::default()))
}

#[utoipa::path(
    put,
    path = "/v1/vms/{id}/logger",
    params(VmPathParams),
    request_body = LoggerUpdateReq,
    responses(
        (status = 200, description = "Logger updated", body = OkResponse),
        (status = 404, description = "VM not found"),
    ),
    tag = "VM configuration"
)]
pub async fn put_logger(
    Extension(st): Extension<AppState>,
    Path(VmPathParams { id }): Path<VmPathParams>,
    Json(req): Json<LoggerUpdateReq>,
) -> Result<Json<OkResponse>, axum::http::StatusCode> {
    super::service::patch_logger(&st, id, req)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(OkResponse::default()))
}

#[utoipa::path(
    put,
    path = "/v1/vms/{id}/balloon",
    params(VmPathParams),
    request_body = BalloonConfig,
    responses(
        (status = 200, description = "Balloon configured", body = OkResponse),
        (status = 404, description = "VM not found"),
    ),
    tag = "VM configuration"
)]
pub async fn put_balloon(
    Extension(st): Extension<AppState>,
    Path(VmPathParams { id }): Path<VmPathParams>,
    Json(req): Json<BalloonConfig>,
) -> Result<Json<OkResponse>, axum::http::StatusCode> {
    super::service::put_balloon(&st, id, req)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(OkResponse::default()))
}

#[utoipa::path(
    patch,
    path = "/v1/vms/{id}/balloon",
    params(VmPathParams),
    request_body = BalloonConfig,
    responses(
        (status = 200, description = "Balloon updated", body = OkResponse),
        (status = 404, description = "VM not found"),
    ),
    tag = "VM configuration"
)]
pub async fn patch_balloon(
    Extension(st): Extension<AppState>,
    Path(VmPathParams { id }): Path<VmPathParams>,
    Json(req): Json<BalloonConfig>,
) -> Result<Json<OkResponse>, axum::http::StatusCode> {
    super::service::patch_balloon(&st, id, req)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(OkResponse::default()))
}

#[utoipa::path(
    patch,
    path = "/v1/vms/{id}/balloon/statistics",
    params(VmPathParams),
    request_body = BalloonStatsConfig,
    responses(
        (status = 200, description = "Balloon stats updated", body = OkResponse),
        (status = 404, description = "VM not found"),
    ),
    tag = "VM configuration"
)]
pub async fn patch_balloon_statistics(
    Extension(st): Extension<AppState>,
    Path(VmPathParams { id }): Path<VmPathParams>,
    Json(req): Json<BalloonStatsConfig>,
) -> Result<Json<OkResponse>, axum::http::StatusCode> {
    super::service::patch_balloon_stats(&st, id, req)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(OkResponse::default()))
}

#[utoipa::path(
    get,
    path = "/v1/vms/{id}/drives",
    params(VmPathParams),
    responses(
        (status = 200, description = "Drives listed", body = ListDrivesResponse),
    ),
    tag = "VM devices"
)]
pub async fn list_drives(
    Extension(st): Extension<AppState>,
    Path(VmPathParams { id }): Path<VmPathParams>,
) -> Result<Json<ListDrivesResponse>, axum::http::StatusCode> {
    let items = super::service::list_drives(&st, id)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(ListDrivesResponse { items }))
}

#[utoipa::path(
    post,
    path = "/v1/vms/{id}/drives",
    params(VmPathParams),
    request_body = CreateDriveReq,
    responses(
        (status = 200, description = "Drive created", body = VmDrive),
        (status = 404, description = "VM not found"),
    ),
    tag = "VM devices"
)]
pub async fn create_drive(
    Extension(st): Extension<AppState>,
    Path(VmPathParams { id }): Path<VmPathParams>,
    Json(req): Json<CreateDriveReq>,
) -> Result<Json<VmDrive>, axum::http::StatusCode> {
    super::service::create_drive(&st, id, req)
        .await
        .map(Json)
        .map_err(|err| {
            let err_str = err.to_string();
            if err_str.contains("already exists")
                || err_str.contains("not within the configured image root")
            {
                axum::http::StatusCode::BAD_REQUEST
            } else if err_str.contains("not found") {
                axum::http::StatusCode::NOT_FOUND
            } else {
                axum::http::StatusCode::INTERNAL_SERVER_ERROR
            }
        })
}

#[utoipa::path(
    get,
    path = "/v1/vms/{id}/drives/{drive_id}",
    params(("id" = uuid::Uuid, Path, description = "VM ID"),
           ("drive_id" = uuid::Uuid, Path, description = "Drive record ID")),
    responses(
        (status = 200, description = "Drive fetched", body = VmDrive),
        (status = 404, description = "Drive not found"),
    ),
    tag = "VM devices"
)]
pub async fn get_drive(
    Extension(st): Extension<AppState>,
    Path((id, drive_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<VmDrive>, axum::http::StatusCode> {
    let drive = super::service::list_drives(&st, id)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .find(|d| d.id == drive_id)
        .ok_or(axum::http::StatusCode::NOT_FOUND)?;
    Ok(Json(drive))
}

#[utoipa::path(
    patch,
    path = "/v1/vms/{id}/drives/{drive_id}",
    params(("id" = uuid::Uuid, Path, description = "VM ID"),
           ("drive_id" = uuid::Uuid, Path, description = "Drive record ID")),
    request_body = UpdateDriveReq,
    responses(
        (status = 200, description = "Drive updated", body = VmDrive),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Drive not found"),
    ),
    tag = "VM devices"
)]
pub async fn update_drive(
    Extension(st): Extension<AppState>,
    Path((id, drive_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateDriveReq>,
) -> Result<Json<VmDrive>, axum::http::StatusCode> {
    super::service::update_drive(&st, id, drive_id, req)
        .await
        .map(Json)
        .map_err(|err| {
            let err_str = err.to_string();
            if err_str.contains("does not belong")
                || err_str.contains("not within the configured image root")
            {
                axum::http::StatusCode::BAD_REQUEST
            } else {
                axum::http::StatusCode::INTERNAL_SERVER_ERROR
            }
        })
}

#[utoipa::path(
    delete,
    path = "/v1/vms/{id}/drives/{drive_id}",
    params(("id" = uuid::Uuid, Path, description = "VM ID"),
           ("drive_id" = uuid::Uuid, Path, description = "Drive record ID")),
    responses(
        (status = 200, description = "Drive deleted", body = OkResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Drive not found"),
    ),
    tag = "VM devices"
)]
pub async fn delete_drive(
    Extension(st): Extension<AppState>,
    Path((id, drive_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<OkResponse>, axum::http::StatusCode> {
    super::service::delete_drive(&st, id, drive_id)
        .await
        .map_err(|err| {
            if err.to_string().contains("does not belong") {
                axum::http::StatusCode::BAD_REQUEST
            } else {
                axum::http::StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;
    Ok(Json(OkResponse::default()))
}

#[utoipa::path(
    get,
    path = "/v1/vms/{id}/nics",
    params(VmPathParams),
    responses(
        (status = 200, description = "NICs listed", body = ListNicsResponse),
    ),
    tag = "VM devices"
)]
pub async fn list_nics(
    Extension(st): Extension<AppState>,
    Path(VmPathParams { id }): Path<VmPathParams>,
) -> Result<Json<ListNicsResponse>, axum::http::StatusCode> {
    let items = super::service::list_nics(&st, id)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(ListNicsResponse { items }))
}

#[utoipa::path(
    post,
    path = "/v1/vms/{id}/nics",
    params(VmPathParams),
    request_body = CreateNicReq,
    responses(
        (status = 200, description = "NIC created", body = VmNic),
        (status = 404, description = "VM not found"),
    ),
    tag = "VM devices"
)]
pub async fn create_nic(
    Extension(st): Extension<AppState>,
    Path(VmPathParams { id }): Path<VmPathParams>,
    Json(req): Json<CreateNicReq>,
) -> Result<Json<VmNic>, axum::http::StatusCode> {
    super::service::create_nic(&st, id, req)
        .await
        .map(Json)
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)
}

#[utoipa::path(
    get,
    path = "/v1/vms/{id}/nics/{nic_id}",
    params(("id" = uuid::Uuid, Path, description = "VM ID"),
           ("nic_id" = uuid::Uuid, Path, description = "NIC record ID")),
    responses(
        (status = 200, description = "NIC fetched", body = VmNic),
        (status = 404, description = "NIC not found"),
    ),
    tag = "VM devices"
)]
pub async fn get_nic(
    Extension(st): Extension<AppState>,
    Path((id, nic_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<VmNic>, axum::http::StatusCode> {
    let nic = super::service::list_nics(&st, id)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .find(|n| n.id == nic_id)
        .ok_or(axum::http::StatusCode::NOT_FOUND)?;
    Ok(Json(nic))
}

#[utoipa::path(
    patch,
    path = "/v1/vms/{id}/nics/{nic_id}",
    params(("id" = uuid::Uuid, Path, description = "VM ID"),
           ("nic_id" = uuid::Uuid, Path, description = "NIC record ID")),
    request_body = UpdateNicReq,
    responses(
        (status = 200, description = "NIC updated", body = VmNic),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "NIC not found"),
    ),
    tag = "VM devices"
)]
pub async fn update_nic(
    Extension(st): Extension<AppState>,
    Path((id, nic_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateNicReq>,
) -> Result<Json<VmNic>, axum::http::StatusCode> {
    super::service::update_nic(&st, id, nic_id, req)
        .await
        .map(Json)
        .map_err(|err| {
            if err.to_string().contains("does not belong") {
                axum::http::StatusCode::BAD_REQUEST
            } else {
                axum::http::StatusCode::INTERNAL_SERVER_ERROR
            }
        })
}

#[utoipa::path(
    delete,
    path = "/v1/vms/{id}/nics/{nic_id}",
    params(("id" = uuid::Uuid, Path, description = "VM ID"),
           ("nic_id" = uuid::Uuid, Path, description = "NIC record ID")),
    responses(
        (status = 200, description = "NIC deleted", body = OkResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "NIC not found"),
    ),
    tag = "VM devices"
)]
pub async fn delete_nic(
    Extension(st): Extension<AppState>,
    Path((id, nic_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<OkResponse>, axum::http::StatusCode> {
    super::service::delete_nic(&st, id, nic_id)
        .await
        .map_err(|err| {
            if err.to_string().contains("does not belong") {
                axum::http::StatusCode::BAD_REQUEST
            } else {
                axum::http::StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;
    Ok(Json(OkResponse::default()))
}

#[utoipa::path(
    post,
    path = "/v1/vms/{id}/flush-metrics",
    params(VmPathParams),
    responses(
        (status = 200, description = "Metrics flushed", body = OkResponse),
        (status = 404, description = "VM not found"),
        (status = 500, description = "Failed to flush metrics"),
    ),
    tag = "VMs"
)]
pub async fn flush_metrics(
    Extension(st): Extension<AppState>,
    Path(VmPathParams { id }): Path<VmPathParams>,
) -> Result<Json<OkResponse>, axum::http::StatusCode> {
    super::service::flush_vm_metrics(&st, id)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(OkResponse::default()))
}

#[utoipa::path(
    post,
    path = "/v1/vms/{id}/ctrl-alt-del",
    params(VmPathParams),
    responses(
        (status = 200, description = "Ctrl-Alt-Del sent", body = OkResponse),
        (status = 400, description = "VM must be running"),
        (status = 404, description = "VM not found"),
        (status = 500, description = "Failed to send Ctrl-Alt-Del"),
    ),
    tag = "VMs"
)]
pub async fn ctrl_alt_del(
    Extension(st): Extension<AppState>,
    Path(VmPathParams { id }): Path<VmPathParams>,
) -> Result<Json<OkResponse>, axum::http::StatusCode> {
    super::service::send_ctrl_alt_del(&st, id)
        .await
        .map_err(|err| {
            if err.to_string().contains("must be running") {
                axum::http::StatusCode::BAD_REQUEST
            } else {
                axum::http::StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;
    Ok(Json(OkResponse::default()))
}

impl From<super::repo::VmRow> for Vm {
    fn from(row: super::repo::VmRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            state: row.state,
            host_id: row.host_id,
            template_id: row.template_id,
            host_addr: row.host_addr,
            api_sock: row.api_sock,
            tap: row.tap,
            log_path: row.log_path,
            http_port: row.http_port,
            fc_unit: row.fc_unit,
            vcpu: row.vcpu,
            mem_mib: row.mem_mib,
            kernel_path: row.kernel_path,
            rootfs_path: row.rootfs_path,
            source_snapshot_id: row.source_snapshot_id,
            guest_ip: row.guest_ip,
            tags: row.tags,
            created_by_user_id: row.created_by_user_id,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(serde::Deserialize)]
pub struct UpdateGuestIpReq {
    pub guest_ip: String,
}

#[utoipa::path(
    post,
    path = "/v1/vms/{id}/guest-ip",
    params(VmPathParams),
    request_body = UpdateGuestIpReq,
    responses(
        (status = 200, description = "Guest IP updated", body = OkResponse),
        (status = 404, description = "VM not found"),
        (status = 500, description = "Failed to update guest IP"),
    ),
    tag = "VMs"
)]
pub async fn update_guest_ip(
    Extension(st): Extension<AppState>,
    Path(VmPathParams { id }): Path<VmPathParams>,
    Json(req): Json<UpdateGuestIpReq>,
) -> Result<Json<OkResponse>, axum::http::StatusCode> {
    super::repo::update_guest_ip(&st.db, id, Some(&req.guest_ip))
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    tracing::info!(vm_id = %id, guest_ip = %req.guest_ip, "Updated VM guest IP");
    Ok(Json(OkResponse::default()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::hosts::repo::HostRepository;
    use axum::{extract::Path, Extension};
    use serde_json::json;

    async fn test_registry(pool: &sqlx::PgPool) -> crate::features::storage::registry::Registry {
        crate::features::storage::registry::Registry::load(pool, None)
            .await
            .expect("registry")
    }

    // Uses SQLx runtime DB with the same migrations as prod code.
    #[ignore]
    #[sqlx::test(migrations = "./migrations")]
    async fn delete_route_removes_vm(pool: sqlx::PgPool) {
        let id = Uuid::new_v4();
        let now = chrono::Utc::now();
        let hosts = HostRepository::new(pool.clone());
        let host_row = hosts
            .register("test-host", "http://127.0.0.1:1", json!({}))
            .await
            .unwrap();
        let row = super::super::repo::VmRow {
            id,
            name: "test-vm".into(),
            state: "running".into(),
            host_id: host_row.id,
            template_id: None,
            host_addr: host_row.addr.clone(), // unreachable; delete path ignores stop errors
            api_sock: "/tmp/test.sock".into(),
            tap: "tap-test".into(),
            log_path: "/tmp/log".into(),
            http_port: 0,
            fc_unit: "fc-test.scope".into(),
            created_by_user_id: None,
            guest_ip: None,
            tags: vec![],
            vcpu: 1,
            mem_mib: 512,
            kernel_path: "/tmp/kernel".into(),
            rootfs_path: "/tmp/rootfs".into(),
            source_snapshot_id: None,
            created_at: now,
            updated_at: now,
        };
        super::super::repo::insert(&pool, &row).await.unwrap();

        let images =
            crate::features::images::repo::ImageRepository::new(pool.clone(), "/srv/images");
        let snapshots = crate::features::snapshots::repo::SnapshotRepository::new(pool.clone());
        let storage = crate::features::storage::LocalStorage::new();
        storage.init().await.unwrap();
        let users = crate::features::users::repo::UserRepository::new(pool.clone());
        let shell_repo = crate::features::vms::shell::ShellRepository::new(pool.clone());
        let download_progress =
            std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
        let registry = test_registry(&pool).await;
        let state = crate::AppState {
            db: pool.clone(),
            hosts: hosts.clone(),
            images,
            snapshots,
            users,
            shell_repo,
            licensing: crate::features::licensing::repo::LicensingRepository::new(pool.clone()),
            allow_direct_image_paths: true,
            storage,
            registry,
            download_progress,
            license_state: std::sync::Arc::new(tokio::sync::RwLock::new(
                nexus_types::LicenseState::default(),
            )),
            license_config: crate::features::licensing::license_service::LicenseConfig::from_env(),
            sso_providers: crate::features::sso::repo::SsoProviderRepository::new(pool.clone()),
            user_identities: crate::features::sso::repo::UserIdentityRepository::new(pool.clone()),
            auth_states: crate::features::sso::repo::AuthStateRepository::new(pool.clone()),
            sso_base_url: "http://localhost:18080".to_string(),
            sso_frontend_url: "http://localhost:3000".to_string(),
            sso_encryption_key: crate::features::sso::crypto::derive_key("test-key"),
        };

        let Json(body) = super::delete(Extension(state), None, Path(VmPathParams { id }))
            .await
            .unwrap();
        assert_eq!(body, OkResponse::default());

        let fetched = super::super::repo::get(&pool, id).await;
        assert!(matches!(fetched, Err(sqlx::Error::RowNotFound)));
    }

    #[ignore]
    #[sqlx::test(migrations = "./migrations")]
    async fn delete_route_unknown_id_returns_ok(pool: sqlx::PgPool) {
        let hosts = HostRepository::new(pool.clone());
        let images =
            crate::features::images::repo::ImageRepository::new(pool.clone(), "/srv/images");
        let snapshots = crate::features::snapshots::repo::SnapshotRepository::new(pool.clone());
        let users = crate::features::users::repo::UserRepository::new(pool.clone());
        let shell_repo = crate::features::vms::shell::ShellRepository::new(pool.clone());
        let storage = crate::features::storage::LocalStorage::new();
        storage.init().await.unwrap();
        let download_progress =
            std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
        let registry = test_registry(&pool).await;
        let state = crate::AppState {
            db: pool.clone(),
            hosts,
            images,
            snapshots,
            users,
            shell_repo,
            licensing: crate::features::licensing::repo::LicensingRepository::new(pool.clone()),
            allow_direct_image_paths: true,
            storage,
            registry,
            download_progress,
            license_state: std::sync::Arc::new(tokio::sync::RwLock::new(
                nexus_types::LicenseState::default(),
            )),
            license_config: crate::features::licensing::license_service::LicenseConfig::from_env(),
            sso_providers: crate::features::sso::repo::SsoProviderRepository::new(pool.clone()),
            user_identities: crate::features::sso::repo::UserIdentityRepository::new(pool.clone()),
            auth_states: crate::features::sso::repo::AuthStateRepository::new(pool.clone()),
            sso_base_url: "http://localhost:18080".to_string(),
            sso_frontend_url: "http://localhost:3000".to_string(),
            sso_encryption_key: crate::features::sso::crypto::derive_key("test-key"),
        };
        let Json(body) = super::delete(
            Extension(state),
            None,
            Path(VmPathParams { id: Uuid::new_v4() }),
        )
        .await
        .unwrap();
        assert_eq!(body, OkResponse::default());
    }
}
