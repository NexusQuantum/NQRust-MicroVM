use crate::features::users::repo::AuthenticatedUser;
use crate::AppState;
use axum::{
    extract::{
        ws::{WebSocket, WebSocketUpgrade},
        Path, Query,
    },
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use nexus_types::{
    ContainerLogsParams, ContainerLogsResp, ContainerPathParams, ContainerStatsResp,
    CreateContainerReq, CreateContainerResp, ExecCommandReq, ExecCommandResp, GetContainerResp,
    ListContainersParams, ListContainersResp, OkResponse, UpdateContainerReq,
};
use serde::Serialize;
use tokio::time::{interval, Duration};

fn extract_user_info(user: Option<Extension<AuthenticatedUser>>) -> (Option<uuid::Uuid>, String) {
    match user {
        Some(Extension(u)) => (Some(u.id), u.username),
        None => (None, "system".to_string()),
    }
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    fault_message: Option<String>,
}

#[utoipa::path(
    post,
    path = "/v1/containers",
    request_body = CreateContainerReq,
    responses(
        (status = 200, description = "Container created", body = CreateContainerResp),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Failed to create container"),
    ),
    tag = "Containers"
)]
pub async fn create(
    Extension(st): Extension<AppState>,
    user: Option<Extension<AuthenticatedUser>>,
    Json(req): Json<CreateContainerReq>,
) -> Result<Json<CreateContainerResp>, (StatusCode, String)> {
    let (user_id, username) = extract_user_info(user);
    let resp = super::service::create_container(&st, req, user_id, &username)
        .await
        .map_err(|e| {
            let error_msg = e.to_string();
            eprintln!("Failed to create container: {}", error_msg);
            // Return 400 for validation errors (port conflicts, empty name, etc.)
            if error_msg.contains("already in use")
                || error_msg.contains("cannot be empty")
                || error_msg.contains("Port mapping failed")
            {
                (StatusCode::BAD_REQUEST, error_msg)
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, error_msg)
            }
        })?;
    Ok(Json(resp))
}

#[utoipa::path(
    get,
    path = "/v1/containers",
    params(ListContainersParams),
    responses(
        (status = 200, description = "Containers listed", body = ListContainersResp),
        (status = 500, description = "Failed to list containers"),
    ),
    tag = "Containers"
)]
pub async fn list(
    Extension(st): Extension<AppState>,
    Query(params): Query<ListContainersParams>,
) -> Result<Json<ListContainersResp>, StatusCode> {
    let resp = super::service::list_containers(&st.db, params.state, params.host_id)
        .await
        .map_err(|e| {
            eprintln!("Failed to list containers: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(resp))
}

#[utoipa::path(
    get,
    path = "/v1/containers/{id}",
    params(ContainerPathParams),
    responses(
        (status = 200, description = "Container fetched", body = GetContainerResp),
        (status = 404, description = "Container not found"),
        (status = 500, description = "Failed to fetch container"),
    ),
    tag = "Containers"
)]
pub async fn get(
    Extension(st): Extension<AppState>,
    Path(ContainerPathParams { id }): Path<ContainerPathParams>,
) -> Result<Json<GetContainerResp>, StatusCode> {
    let resp = super::service::get_container(&st.db, id)
        .await
        .map_err(|e| {
            eprintln!("Failed to get container: {}", e);
            if e.to_string().contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;
    Ok(Json(resp))
}

#[utoipa::path(
    put,
    path = "/v1/containers/{id}",
    params(ContainerPathParams),
    request_body = UpdateContainerReq,
    responses(
        (status = 200, description = "Container updated", body = GetContainerResp),
        (status = 404, description = "Container not found"),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Failed to update container"),
    ),
    tag = "Containers"
)]
pub async fn update(
    Extension(st): Extension<AppState>,
    Path(ContainerPathParams { id }): Path<ContainerPathParams>,
    Json(req): Json<UpdateContainerReq>,
) -> Result<Json<GetContainerResp>, StatusCode> {
    let resp = super::service::update_container(&st, id, req)
        .await
        .map_err(|e| {
            eprintln!("Failed to update container: {}", e);
            if e.to_string().contains("not found") {
                StatusCode::NOT_FOUND
            } else if e.to_string().contains("running") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;
    Ok(Json(resp))
}

#[utoipa::path(
    delete,
    path = "/v1/containers/{id}",
    params(ContainerPathParams),
    responses(
        (status = 200, description = "Container deleted", body = OkResponse),
        (status = 404, description = "Container not found"),
        (status = 500, description = "Failed to delete container"),
    ),
    tag = "Containers"
)]
pub async fn delete(
    Extension(st): Extension<AppState>,
    user: Option<Extension<AuthenticatedUser>>,
    Path(ContainerPathParams { id }): Path<ContainerPathParams>,
) -> Result<Json<OkResponse>, StatusCode> {
    let (user_id, username) = extract_user_info(user);
    super::service::delete_container(&st, id, user_id, &username)
        .await
        .map_err(|e| {
            eprintln!("Failed to delete container: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(OkResponse::default()))
}

#[utoipa::path(
    post,
    path = "/v1/containers/{id}/start",
    params(ContainerPathParams),
    responses(
        (status = 200, description = "Container started", body = OkResponse),
        (status = 404, description = "Container not found"),
        (status = 400, description = "Container already running"),
        (status = 500, description = "Failed to start container"),
    ),
    tag = "Containers"
)]
pub async fn start(
    Extension(st): Extension<AppState>,
    user: Option<Extension<AuthenticatedUser>>,
    Path(ContainerPathParams { id }): Path<ContainerPathParams>,
) -> impl IntoResponse {
    let (user_id, username) = extract_user_info(user);
    match super::service::start_container(&st, id, user_id, &username).await {
        Ok(_) => (StatusCode::OK, Json(OkResponse::default())).into_response(),
        Err(e) => {
            eprintln!("Failed to start container: {}", e);
            let error_msg = e.to_string();
            let status = if error_msg.contains("already running") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (
                status,
                Json(ErrorResponse {
                    error: "Failed to start container".to_string(),
                    fault_message: Some(error_msg),
                }),
            )
                .into_response()
        }
    }
}

#[utoipa::path(
    post,
    path = "/v1/containers/{id}/stop",
    params(ContainerPathParams),
    responses(
        (status = 200, description = "Container stopped", body = OkResponse),
        (status = 404, description = "Container not found"),
        (status = 400, description = "Container not running"),
        (status = 500, description = "Failed to stop container"),
    ),
    tag = "Containers"
)]
pub async fn stop(
    Extension(st): Extension<AppState>,
    user: Option<Extension<AuthenticatedUser>>,
    Path(ContainerPathParams { id }): Path<ContainerPathParams>,
) -> impl IntoResponse {
    let (user_id, username) = extract_user_info(user);
    match super::service::stop_container(&st, id, user_id, &username).await {
        Ok(_) => (StatusCode::OK, Json(OkResponse::default())).into_response(),
        Err(e) => {
            eprintln!("Failed to stop container: {}", e);
            let error_msg = e.to_string();
            let status = if error_msg.contains("not running") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (
                status,
                Json(ErrorResponse {
                    error: "Failed to stop container".to_string(),
                    fault_message: Some(error_msg),
                }),
            )
                .into_response()
        }
    }
}

#[utoipa::path(
    post,
    path = "/v1/containers/{id}/restart",
    params(ContainerPathParams),
    responses(
        (status = 200, description = "Container restarted", body = OkResponse),
        (status = 404, description = "Container not found"),
        (status = 500, description = "Failed to restart container"),
    ),
    tag = "Containers"
)]
pub async fn restart(
    Extension(st): Extension<AppState>,
    Path(ContainerPathParams { id }): Path<ContainerPathParams>,
) -> impl IntoResponse {
    match super::service::restart_container(&st, id).await {
        Ok(_) => (StatusCode::OK, Json(OkResponse::default())).into_response(),
        Err(e) => {
            eprintln!("Failed to restart container: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to restart container".to_string(),
                    fault_message: Some(e.to_string()),
                }),
            )
                .into_response()
        }
    }
}

#[utoipa::path(
    post,
    path = "/v1/containers/{id}/pause",
    params(ContainerPathParams),
    responses(
        (status = 200, description = "Container paused", body = OkResponse),
        (status = 404, description = "Container not found"),
        (status = 400, description = "Container not running"),
        (status = 500, description = "Failed to pause container"),
    ),
    tag = "Containers"
)]
pub async fn pause(
    Extension(st): Extension<AppState>,
    Path(ContainerPathParams { id }): Path<ContainerPathParams>,
) -> Result<Json<OkResponse>, StatusCode> {
    super::service::pause_container(&st, id)
        .await
        .map_err(|e| {
            eprintln!("Failed to pause container: {}", e);
            if e.to_string().contains("not running") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;
    Ok(Json(OkResponse::default()))
}

#[utoipa::path(
    post,
    path = "/v1/containers/{id}/resume",
    params(ContainerPathParams),
    responses(
        (status = 200, description = "Container resumed", body = OkResponse),
        (status = 404, description = "Container not found"),
        (status = 400, description = "Container not paused"),
        (status = 500, description = "Failed to resume container"),
    ),
    tag = "Containers"
)]
pub async fn resume(
    Extension(st): Extension<AppState>,
    Path(ContainerPathParams { id }): Path<ContainerPathParams>,
) -> Result<Json<OkResponse>, StatusCode> {
    super::service::resume_container(&st, id)
        .await
        .map_err(|e| {
            eprintln!("Failed to resume container: {}", e);
            if e.to_string().contains("not paused") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;
    Ok(Json(OkResponse::default()))
}

#[utoipa::path(
    get,
    path = "/v1/containers/{id}/logs",
    params(ContainerPathParams, ContainerLogsParams),
    responses(
        (status = 200, description = "Container logs fetched", body = ContainerLogsResp),
        (status = 404, description = "Container not found"),
        (status = 500, description = "Failed to fetch logs"),
    ),
    tag = "Containers"
)]
pub async fn logs(
    Extension(st): Extension<AppState>,
    Path(ContainerPathParams { id }): Path<ContainerPathParams>,
    Query(params): Query<ContainerLogsParams>,
) -> Result<Json<ContainerLogsResp>, StatusCode> {
    let resp = super::service::get_container_logs(&st.db, id, params.tail)
        .await
        .map_err(|e| {
            eprintln!("Failed to get container logs: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(resp))
}

/// WebSocket endpoint for streaming container logs in real-time
pub async fn logs_stream(
    Extension(st): Extension<AppState>,
    Path(ContainerPathParams { id }): Path<ContainerPathParams>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_logs_stream(socket, st, id))
}

async fn handle_logs_stream(mut socket: WebSocket, st: AppState, container_id: uuid::Uuid) {
    // Poll for logs every 2 seconds
    let mut ticker = interval(Duration::from_secs(2));
    let mut last_timestamp = chrono::Utc::now().timestamp();

    loop {
        ticker.tick().await;

        // Get container to check state
        let container = match super::service::get_container(&st.db, container_id).await {
            Ok(resp) => resp.item,
            Err(e) => {
                let _ = socket
                    .send(axum::extract::ws::Message::Text(format!(
                        "{{\"error\": \"Failed to get container: {}\"}}",
                        e
                    )))
                    .await;
                break;
            }
        };

        // If container is in error or doesn't have a runtime ID, stop streaming
        if container.state == "error" || container.container_runtime_id.is_none() {
            let _ = socket
                .send(axum::extract::ws::Message::Text(format!(
                    "{{\"info\": \"Container in {} state\"}}",
                    container.state
                )))
                .await;
            break;
        }

        // Get new logs since last fetch
        match super::service::get_container_logs(&st.db, container_id, Some(100)).await {
            Ok(resp) => {
                for log in resp.items {
                    // Only send logs newer than last timestamp
                    if log.timestamp.timestamp() > last_timestamp {
                        let log_json = serde_json::json!({
                            "timestamp": log.timestamp,
                            "stream": log.stream,
                            "message": log.message
                        });

                        if socket
                            .send(axum::extract::ws::Message::Text(log_json.to_string()))
                            .await
                            .is_err()
                        {
                            // Client disconnected
                            return;
                        }

                        last_timestamp = log.timestamp.timestamp();
                    }
                }
            }
            Err(e) => {
                let _ = socket
                    .send(axum::extract::ws::Message::Text(format!(
                        "{{\"error\": \"Failed to fetch logs: {}\"}}",
                        e
                    )))
                    .await;
            }
        }

        // Check if client is still connected by trying to send a ping
        if socket
            .send(axum::extract::ws::Message::Ping(vec![]))
            .await
            .is_err()
        {
            break;
        }
    }
}

#[utoipa::path(
    get,
    path = "/v1/containers/{id}/stats",
    params(ContainerPathParams),
    responses(
        (status = 200, description = "Container stats fetched", body = ContainerStatsResp),
        (status = 404, description = "Container not found"),
        (status = 500, description = "Failed to fetch stats"),
    ),
    tag = "Containers"
)]
pub async fn stats(
    Extension(st): Extension<AppState>,
    Path(ContainerPathParams { id }): Path<ContainerPathParams>,
) -> Result<Json<ContainerStatsResp>, StatusCode> {
    let resp = super::service::get_container_stats(&st, id)
        .await
        .map_err(|e| {
            eprintln!("Failed to get container stats: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(resp))
}

#[utoipa::path(
    post,
    path = "/v1/containers/{id}/exec",
    params(ContainerPathParams),
    request_body = ExecCommandReq,
    responses(
        (status = 200, description = "Command executed", body = ExecCommandResp),
        (status = 404, description = "Container not found"),
        (status = 400, description = "Container not running"),
        (status = 500, description = "Failed to execute command"),
    ),
    tag = "Containers"
)]
pub async fn exec(
    Extension(st): Extension<AppState>,
    Path(ContainerPathParams { id }): Path<ContainerPathParams>,
    Json(req): Json<ExecCommandReq>,
) -> Result<Json<ExecCommandResp>, StatusCode> {
    let resp = super::service::exec_command(&st, id, req)
        .await
        .map_err(|e| {
            eprintln!("Failed to exec command: {}", e);
            if e.to_string().contains("must be running") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;
    Ok(Json(resp))
}
