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
    OkResponse, SerialConfigReq, UpdateDriveReq, UpdateNicReq, Vm, VmDrive, VmNic, VmPathParams,
    VsockConfigReq,
};
use reqwest::StatusCode;
use serde::Serialize;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message as WsMessage};
use uuid::Uuid;

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
    use tokio::fs::OpenOptions;
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tokio::time::{interval, Duration};

    let (mut sender, mut receiver) = ws.split();
    let metrics_path = format!("/srv/fc/vms/{}/logs/metrics.json", vm_id);

    // Create a ticker that will trigger metrics flush every second
    let mut ticker = interval(Duration::from_secs(1));

    // Track last metrics for rate calculations
    let mut last_metrics: Option<serde_json::Value> = None;

    loop {
        tokio::select! {
            // Check for client disconnect
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

            // Send metrics every second
            _ = ticker.tick() => {
                // First, flush metrics from Firecracker to the FIFO
                if let Err(e) = super::service::flush_vm_metrics(&st, vm_id).await {
                    tracing::debug!(vm_id = %vm_id, "Failed to flush metrics: {}", e);
                    continue;
                }

                // Try to read metrics from the FIFO
                match OpenOptions::new().read(true).open(&metrics_path).await {
                    Ok(file) => {
                        let mut reader = BufReader::new(file);
                        let mut line = String::new();

                        // Read one line from the FIFO (Firecracker writes one JSON object per flush)
                        match reader.read_line(&mut line).await {
                            Ok(n) if n > 0 => {
                                // Debug: log raw metrics
                                tracing::info!(vm_id = %vm_id, raw_metrics = %line.chars().take(500).collect::<String>(), "Received Firecracker metrics");

                                // Parse Firecracker metrics and convert to our format
                                if let Ok(fc_metrics) = serde_json::from_str::<serde_json::Value>(&line) {
                                    // Fetch process stats from agent for real CPU/memory metrics
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

                                    tracing::info!(vm_id = %vm_id, simplified = ?simplified, "Simplified metrics");

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
                            _ => {
                                // No data available or error, skip this tick
                            }
                        }
                    }
                    Err(e) => {
                        tracing::debug!(vm_id = %vm_id, "Failed to open metrics FIFO: {}", e);
                        // Metrics FIFO not available, skip this tick
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
    Json(req): Json<CreateVmReq>,
) -> Result<Json<CreateVmResponse>, axum::http::StatusCode> {
    let id = Uuid::new_v4();
    super::service::create_and_start(&st, id, req, None)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
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
) -> Result<Json<ListVmsResponse>, axum::http::StatusCode> {
    let items = super::repo::list(&st.db)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
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
) -> Result<Json<GetVmResponse>, axum::http::StatusCode> {
    let row = super::repo::get(&st.db, id)
        .await
        .map_err(|_| axum::http::StatusCode::NOT_FOUND)?;
    Ok(Json(GetVmResponse { item: row.into() }))
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
    Path(VmPathParams { id }): Path<VmPathParams>,
) -> Result<Json<OkResponse>, axum::http::StatusCode> {
    super::service::start_vm_by_id(&st, id)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
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
    Path(VmPathParams { id }): Path<VmPathParams>,
) -> Result<Json<OkResponse>, axum::http::StatusCode> {
    super::service::stop_only(&st, id)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
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
    Path(VmPathParams { id }): Path<VmPathParams>,
) -> Result<Json<OkResponse>, axum::http::StatusCode> {
    super::service::pause_vm(&st, id)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
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
    Path(VmPathParams { id }): Path<VmPathParams>,
) -> Result<Json<OkResponse>, axum::http::StatusCode> {
    super::service::resume_vm(&st, id)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
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
    Path(VmPathParams { id }): Path<VmPathParams>,
) -> Result<Json<OkResponse>, axum::http::StatusCode> {
    super::service::stop_and_delete(&st, id)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
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

    // Uses SQLx runtime DB with the same migrations as prod code.
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
        let state = crate::AppState {
            db: pool.clone(),
            hosts: hosts.clone(),
            images,
            snapshots,
            users,
            shell_repo,
            allow_direct_image_paths: true,
            storage,
            download_progress,
        };

        let Json(body) = super::delete(Extension(state), Path(VmPathParams { id }))
            .await
            .unwrap();
        assert_eq!(body, OkResponse::default());

        let fetched = super::super::repo::get(&pool, id).await;
        assert!(matches!(fetched, Err(sqlx::Error::RowNotFound)));
    }

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
        let state = crate::AppState {
            db: pool,
            hosts,
            images,
            snapshots,
            users,
            shell_repo,
            allow_direct_image_paths: true,
            storage,
            download_progress,
        };
        let Json(body) = super::delete(Extension(state), Path(VmPathParams { id: Uuid::new_v4() }))
            .await
            .unwrap();
        assert_eq!(body, OkResponse::default());
    }
}
