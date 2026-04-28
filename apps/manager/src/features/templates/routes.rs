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

    super::super::vms::service::create_and_start(
        &st,
        vm_id,
        vm_req,
        Some(template.id),
        None,
        "system",
    )
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

    async fn test_registry(pool: &sqlx::PgPool) -> crate::features::storage::registry::Registry {
        crate::features::storage::registry::Registry::load(pool, None)
            .await
            .expect("registry")
    }

    fn full_spec() -> TemplateSpec {
        TemplateSpec {
            vcpu: 4,
            mem_mib: 4096,
            kernel_image_id: Some(Uuid::new_v4()),
            rootfs_image_id: Some(Uuid::new_v4()),
            kernel_path: Some("/srv/kernel".into()),
            rootfs_path: Some("/srv/rootfs".into()),
            rootfs_size_mb: Some(2048),
        }
    }

    #[test]
    fn template_spec_into_vm_req_round_trips_all_fields() {
        let spec = full_spec();
        let kernel_image_id = spec.kernel_image_id;
        let rootfs_image_id = spec.rootfs_image_id;
        let kernel_path = spec.kernel_path.clone();
        let rootfs_path = spec.rootfs_path.clone();
        let rootfs_size_mb = spec.rootfs_size_mb;

        let req = spec.into_vm_req("vm-from-template".into());

        assert_eq!(req.name, "vm-from-template");
        assert_eq!(req.vcpu, 4);
        assert_eq!(req.mem_mib, 4096);
        assert_eq!(req.kernel_image_id, kernel_image_id);
        assert_eq!(req.rootfs_image_id, rootfs_image_id);
        assert_eq!(req.kernel_path, kernel_path);
        assert_eq!(req.rootfs_path, rootfs_path);
        assert_eq!(req.rootfs_size_mb, rootfs_size_mb);
    }

    #[test]
    fn template_spec_into_vm_req_blanks_non_template_fields() {
        let req = full_spec().into_vm_req("any".into());

        // Fields not carried by TemplateSpec must be defaulted, not derived
        // from the template. This contract anchors the upcoming vmm_kind
        // refactor: anything new added to the round-trip should be asserted
        // explicitly here.
        assert!(req.source_snapshot_id.is_none());
        assert!(req.username.is_none());
        assert!(req.password.is_none());
        assert!(req.tags.is_empty());
        assert!(req.network_id.is_none());
        assert!(req.port_forwards.is_empty());
    }

    #[test]
    fn template_spec_into_vm_req_with_minimal_spec_keeps_optional_fields_none() {
        let spec = TemplateSpec {
            vcpu: 1,
            mem_mib: 256,
            kernel_image_id: None,
            rootfs_image_id: None,
            kernel_path: None,
            rootfs_path: None,
            rootfs_size_mb: None,
        };

        let req = spec.into_vm_req("tiny-vm".into());

        assert_eq!(req.name, "tiny-vm");
        assert_eq!(req.vcpu, 1);
        assert_eq!(req.mem_mib, 256);
        assert!(req.kernel_image_id.is_none());
        assert!(req.rootfs_image_id.is_none());
        assert!(req.kernel_path.is_none());
        assert!(req.rootfs_path.is_none());
        assert!(req.rootfs_size_mb.is_none());
    }

    #[test]
    fn template_spec_into_vm_req_propagates_name_verbatim() {
        // The instantiate handler forwards the user-supplied name through
        // into_vm_req unmodified; ensure trimming/casing/empty-string are
        // not silently mutated by the conversion.
        let spec = TemplateSpec {
            vcpu: 2,
            mem_mib: 1024,
            kernel_image_id: None,
            rootfs_image_id: None,
            kernel_path: None,
            rootfs_path: None,
            rootfs_size_mb: None,
        };
        let weird_name = "  Mixed-Case Name  ".to_string();
        let req = spec.clone().into_vm_req(weird_name.clone());
        assert_eq!(req.name, weird_name);

        let empty_req = spec.into_vm_req(String::new());
        assert_eq!(empty_req.name, "");
    }

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
        let registry = test_registry(&pool).await;
        let state = crate::AppState {
            db: pool.clone(),
            hosts: hosts.clone(),
            images: images.clone(),
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

        let create_req = CreateTemplateReq {
            name: "ubuntu".into(),
            spec: TemplateSpec {
                vcpu: 2,
                mem_mib: 2048,
                kernel_image_id: None,
                rootfs_image_id: None,
                kernel_path: Some("/tmp/kernel".into()),
                rootfs_path: Some("/tmp/rootfs".into()),
                rootfs_size_mb: None,
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
