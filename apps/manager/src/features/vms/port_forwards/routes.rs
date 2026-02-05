use crate::AppState;
use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    Json,
};
use nexus_types::{
    CreatePortForwardReq, ListPortForwardsResponse, OkResponse, PortForward, PortForwardPathParams,
    VmPathParams,
};

#[utoipa::path(
    get,
    path = "/v1/vms/{id}/port-forwards",
    params(VmPathParams),
    responses(
        (status = 200, description = "Port forwards listed", body = ListPortForwardsResponse),
        (status = 500, description = "Internal error"),
    ),
    tag = "VMs"
)]
pub async fn list(
    Extension(st): Extension<AppState>,
    Path(VmPathParams { id }): Path<VmPathParams>,
) -> Result<Json<ListPortForwardsResponse>, StatusCode> {
    let rows = super::repo::list(&st.db, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ListPortForwardsResponse {
        items: rows.into_iter().map(Into::into).collect(),
    }))
}

#[utoipa::path(
    post,
    path = "/v1/vms/{id}/port-forwards",
    params(VmPathParams),
    request_body = CreatePortForwardReq,
    responses(
        (status = 201, description = "Port forward created", body = PortForward),
        (status = 409, description = "Port already in use"),
        (status = 500, description = "Internal error"),
    ),
    tag = "VMs"
)]
pub async fn create(
    Extension(st): Extension<AppState>,
    Path(VmPathParams { id }): Path<VmPathParams>,
    Json(req): Json<CreatePortForwardReq>,
) -> Result<(StatusCode, Json<PortForward>), (StatusCode, String)> {
    let row = super::repo::insert(
        &st.db,
        id,
        req.host_port,
        req.guest_port,
        &req.protocol,
        req.description.as_deref(),
    )
    .await
    .map_err(|e| {
        if e.to_string().contains("unique_host_port_protocol") {
            (
                StatusCode::CONFLICT,
                format!(
                    "Port {} ({}) is already in use",
                    req.host_port, req.protocol
                ),
            )
        } else {
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        }
    })?;

    // If VM is running and has a guest IP, apply the forward immediately
    if let Ok(vm) = super::super::repo::get(&st.db, id).await {
        if vm.state == "running" && vm.guest_ip.as_ref().is_some_and(|ip| !ip.is_empty()) {
            let guest_ip = vm.guest_ip.as_deref().unwrap();
            let _ = reqwest::Client::new()
                .post(format!(
                    "{}/agent/v1/vms/{}/port-forward",
                    vm.host_addr, vm.id
                ))
                .json(&serde_json::json!({
                    "guest_ip": guest_ip,
                    "host_port": req.host_port as u16,
                    "guest_port": req.guest_port as u16,
                    "protocol": req.protocol,
                }))
                .send()
                .await;
        }
    }

    Ok((StatusCode::CREATED, Json(row.into())))
}

#[utoipa::path(
    delete,
    path = "/v1/vms/{id}/port-forwards/{forward_id}",
    params(PortForwardPathParams),
    responses(
        (status = 200, description = "Port forward deleted", body = OkResponse),
        (status = 404, description = "Not found"),
        (status = 500, description = "Internal error"),
    ),
    tag = "VMs"
)]
pub async fn delete(
    Extension(st): Extension<AppState>,
    Path(PortForwardPathParams { id, forward_id }): Path<PortForwardPathParams>,
) -> Result<Json<OkResponse>, StatusCode> {
    let fwd = super::repo::get(&st.db, forward_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // If VM is running, remove the iptables rule
    if let Ok(vm) = super::super::repo::get(&st.db, id).await {
        if vm.state == "running" && vm.guest_ip.as_ref().is_some_and(|ip| !ip.is_empty()) {
            let guest_ip = vm.guest_ip.as_deref().unwrap();
            let _ = reqwest::Client::new()
                .delete(format!(
                    "{}/agent/v1/vms/{}/port-forward",
                    vm.host_addr, vm.id
                ))
                .json(&serde_json::json!({
                    "guest_ip": guest_ip,
                    "host_port": fwd.host_port as u16,
                    "guest_port": fwd.guest_port as u16,
                    "protocol": fwd.protocol,
                }))
                .send()
                .await;
        }
    }

    super::repo::delete(&st.db, forward_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(OkResponse::default()))
}
