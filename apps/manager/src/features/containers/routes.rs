use crate::AppState;
use axum::{
    extract::{Path, Query},
    http::StatusCode,
    Extension, Json,
};
use nexus_types::{
    ContainerLogsParams, ContainerLogsResp, ContainerPathParams, ContainerStatsResp,
    CreateContainerReq, CreateContainerResp, ExecCommandReq, ExecCommandResp, GetContainerResp,
    ListContainersParams, ListContainersResp, OkResponse, UpdateContainerReq,
};

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
    Json(req): Json<CreateContainerReq>,
) -> Result<Json<CreateContainerResp>, StatusCode> {
    let resp = super::service::create_container(&st, req)
        .await
        .map_err(|e| {
            eprintln!("Failed to create container: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
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
    Path(ContainerPathParams { id }): Path<ContainerPathParams>,
) -> Result<Json<OkResponse>, StatusCode> {
    super::service::delete_container(&st, id)
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
    Path(ContainerPathParams { id }): Path<ContainerPathParams>,
) -> Result<Json<OkResponse>, StatusCode> {
    super::service::start_container(&st, id)
        .await
        .map_err(|e| {
            eprintln!("Failed to start container: {}", e);
            if e.to_string().contains("already running") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;
    Ok(Json(OkResponse::default()))
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
    Path(ContainerPathParams { id }): Path<ContainerPathParams>,
) -> Result<Json<OkResponse>, StatusCode> {
    super::service::stop_container(&st, id).await.map_err(|e| {
        eprintln!("Failed to stop container: {}", e);
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
) -> Result<Json<OkResponse>, StatusCode> {
    super::service::restart_container(&st, id)
        .await
        .map_err(|e| {
            eprintln!("Failed to restart container: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(OkResponse::default()))
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
