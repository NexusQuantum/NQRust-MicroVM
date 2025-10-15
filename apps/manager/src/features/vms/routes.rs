use crate::AppState;
use axum::{extract::Path, Extension, Json};
use nexus_types::{
    BalloonConfig, BalloonStatsConfig, CpuConfigReq, CreateDriveReq, CreateNicReq, CreateVmReq,
    CreateVmResponse, EntropyConfigReq, GetVmResponse, ListDrivesResponse, ListNicsResponse,
    ListVmsResponse, LoggerUpdateReq, MachineConfigPatchReq, MmdsConfigReq, MmdsDataReq,
    OkResponse, SerialConfigReq, UpdateDriveReq, UpdateNicReq, Vm, VmDrive, VmNic, VmPathParams,
    VsockConfigReq,
};
use reqwest::StatusCode;
use serde::Serialize;
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
            if err.to_string().contains("already exists") {
                axum::http::StatusCode::BAD_REQUEST
            } else if err
                .to_string()
                .contains("not within the configured image root")
            {
                axum::http::StatusCode::BAD_REQUEST
            } else if err.to_string().contains("not found") {
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
            if err.to_string().contains("does not belong") {
                axum::http::StatusCode::BAD_REQUEST
            } else if err
                .to_string()
                .contains("not within the configured image root")
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
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
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
        let state = crate::AppState {
            db: pool.clone(),
            hosts: hosts.clone(),
            images,
            snapshots,
            allow_direct_image_paths: true,
            storage,
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
        let storage = crate::features::storage::LocalStorage::new();
        storage.init().await.unwrap();
        let state = crate::AppState {
            db: pool,
            hosts,
            images,
            snapshots,
            allow_direct_image_paths: true,
            storage,
        };
        let Json(body) = super::delete(Extension(state), Path(VmPathParams { id: Uuid::new_v4() }))
            .await
            .unwrap();
        assert_eq!(body, OkResponse::default());
    }
}
