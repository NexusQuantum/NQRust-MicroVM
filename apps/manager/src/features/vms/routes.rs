use crate::AppState;
use axum::{extract::Path, Extension, Json};
use nexus_types::{
    CreateVmReq, CreateVmResponse, GetVmResponse, ListVmsResponse, OkResponse, Vm, VmPathParams,
};
use uuid::Uuid;

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
        let state = crate::AppState {
            db: pool.clone(),
            hosts: hosts.clone(),
            images,
            snapshots,
            allow_direct_image_paths: true,
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
        let state = crate::AppState {
            db: pool,
            hosts,
            images,
            snapshots,
            allow_direct_image_paths: true,
        };
        let Json(body) = super::delete(Extension(state), Path(VmPathParams { id: Uuid::new_v4() }))
            .await
            .unwrap();
        assert_eq!(body, OkResponse::default());
    }
}
