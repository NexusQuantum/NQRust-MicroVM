use crate::AppState;
use axum::{extract::Path, http::StatusCode, Extension, Json};
use nexus_types::{
    CreateTemplateReq, CreateTemplateResp, GetTemplateResp, InstantiateTemplateReq,
    InstantiateTemplateResp, ListTemplatesResp, OkResponse, TemplatePathParams, UpdateTemplateReq,
    UpdateTemplateResp,
};
use uuid::Uuid;

#[utoipa::path(
    post,
    path = "/v1/templates",
    request_body = CreateTemplateReq,
    responses(
        (status = 200, description = "Template created", body = CreateTemplateResp),
        (status = 500, description = "Failed to create template"),
    ),
    tag = "Templates"
)]
pub async fn create(
    Extension(st): Extension<AppState>,
    Json(req): Json<CreateTemplateReq>,
) -> Result<Json<CreateTemplateResp>, StatusCode> {
    let template = super::repo::insert(&st.db, &req)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(CreateTemplateResp { id: template.id }))
}

#[utoipa::path(
    get,
    path = "/v1/templates",
    responses(
        (status = 200, description = "Templates listed", body = ListTemplatesResp),
        (status = 500, description = "Failed to list templates"),
    ),
    tag = "Templates"
)]
pub async fn list(
    Extension(st): Extension<AppState>,
) -> Result<Json<ListTemplatesResp>, StatusCode> {
    let items = super::repo::list(&st.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(ListTemplatesResp { items }))
}

#[utoipa::path(
    get,
    path = "/v1/templates/{id}",
    params(TemplatePathParams),
    responses(
        (status = 200, description = "Template fetched", body = GetTemplateResp),
        (status = 404, description = "Template not found"),
        (status = 500, description = "Failed to fetch template"),
    ),
    tag = "Templates"
)]
pub async fn get(
    Extension(st): Extension<AppState>,
    Path(TemplatePathParams { id }): Path<TemplatePathParams>,
) -> Result<Json<GetTemplateResp>, StatusCode> {
    let template = super::repo::get(&st.db, id)
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })?;
    Ok(Json(GetTemplateResp { item: template }))
}

#[utoipa::path(
    put,
    path = "/v1/templates/{id}",
    params(TemplatePathParams),
    request_body = UpdateTemplateReq,
    responses(
        (status = 200, description = "Template updated", body = UpdateTemplateResp),
        (status = 404, description = "Template not found"),
        (status = 500, description = "Failed to update template"),
    ),
    tag = "Templates"
)]
pub async fn update(
    Extension(st): Extension<AppState>,
    Path(TemplatePathParams { id }): Path<TemplatePathParams>,
    Json(req): Json<UpdateTemplateReq>,
) -> Result<Json<UpdateTemplateResp>, StatusCode> {
    let template = super::repo::update(&st.db, id, &req)
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })?;
    Ok(Json(UpdateTemplateResp { item: template }))
}

#[utoipa::path(
    delete,
    path = "/v1/templates/{id}",
    params(TemplatePathParams),
    responses(
        (status = 200, description = "Template deleted", body = OkResponse),
        (status = 404, description = "Template not found"),
        (status = 500, description = "Failed to delete template"),
    ),
    tag = "Templates"
)]
pub async fn delete(
    Extension(st): Extension<AppState>,
    Path(TemplatePathParams { id }): Path<TemplatePathParams>,
) -> Result<Json<OkResponse>, StatusCode> {
    super::repo::delete(&st.db, id)
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })?;
    Ok(Json(OkResponse::default()))
}

#[utoipa::path(
    post,
    path = "/v1/templates/{id}/instantiate",
    params(TemplatePathParams),
    request_body = InstantiateTemplateReq,
    responses(
        (status = 200, description = "Template instantiated", body = InstantiateTemplateResp),
        (status = 404, description = "Template not found"),
        (status = 500, description = "Failed to instantiate template"),
    ),
    tag = "Templates"
)]
pub async fn instantiate(
    Extension(st): Extension<AppState>,
    Path(TemplatePathParams { id }): Path<TemplatePathParams>,
    Json(req): Json<InstantiateTemplateReq>,
) -> Result<Json<InstantiateTemplateResp>, StatusCode> {
    let template = super::repo::get(&st.db, id)
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })?;

    let vm_id = Uuid::new_v4();
    let vm_req = template.spec.into_vm_req(req.name);

    super::super::vms::service::create_and_start(&st, vm_id, vm_req, Some(template.id))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(InstantiateTemplateResp { id: vm_id }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::hosts::repo::HostRepository;
    use axum::{extract::Path, Extension};
    use nexus_types::{CreateTemplateReq, InstantiateTemplateReq, TemplateSpec};
    use serde_json::json;
    use std::convert::TryFrom;

    #[ignore]
    #[sqlx::test(migrations = "./migrations")]
    async fn instantiate_creates_vm_with_template(pool: sqlx::PgPool) {
        crate::features::vms::repo::reset_store();

        let hosts = HostRepository::new(pool.clone());
        hosts
            .register("test-host", "http://127.0.0.1:1", json!({}))
            .await
            .unwrap();
        let images = crate::features::images::repo::ImageRepository::new(pool.clone(), "/tmp");

        let snapshots = crate::features::snapshots::repo::SnapshotRepository::new(pool.clone());
        let users = crate::features::users::repo::UserRepository::new(pool.clone());
        let shell_repo = crate::features::vms::shell::ShellRepository::new(pool.clone());
        let storage = crate::features::storage::LocalStorage::new();
        storage.init().await.unwrap();
        let download_progress =
            std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
        let state = crate::AppState {
            db: pool.clone(),
            hosts: hosts.clone(),
            images: images.clone(),
            snapshots,
            users,
            shell_repo,
            allow_direct_image_paths: true,
            storage,
            download_progress,
        };

        let create_req = CreateTemplateReq {
            name: "ubuntu".into(),
            spec: TemplateSpec {
                vcpu: 2,
                mem_mib: 2048,
                kernel_image_id: None,
                rootfs_image_id: None,
                kernel_path: Some("/tmp/kernel".into()),
                rootfs_path: Some("/tmp/rootfs".into()),
            },
        };
        let spec = create_req.spec.clone();

        let Json(create_resp) = super::create(Extension(state.clone()), Json(create_req.clone()))
            .await
            .unwrap();
        let template_id = create_resp.id;

        let Json(inst_resp) = super::instantiate(
            Extension(state.clone()),
            Path(TemplatePathParams { id: template_id }),
            Json(InstantiateTemplateReq {
                name: "vm-from-template".into(),
            }),
        )
        .await
        .unwrap();

        let vm = crate::features::vms::repo::get(&state.db, inst_resp.id)
            .await
            .unwrap();
        assert_eq!(vm.template_id, Some(template_id));
        assert_eq!(vm.name, "vm-from-template");
        assert_eq!(vm.vcpu, i32::from(spec.vcpu));
        assert_eq!(vm.mem_mib, i32::try_from(spec.mem_mib).unwrap());
        assert_eq!(vm.kernel_path, spec.kernel_path.unwrap());
        assert_eq!(vm.rootfs_path, spec.rootfs_path.unwrap());
    }
}
