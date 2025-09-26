use crate::AppState;
use axum::{extract::Path, http::StatusCode, Extension, Json};
use nexus_types::{
    CreateTemplateReq, CreateTemplateResp, GetTemplateResp, InstantiateTemplateReq,
    InstantiateTemplateResp, ListTemplatesResp,
};
use uuid::Uuid;

pub async fn create(
    Extension(st): Extension<AppState>,
    Json(req): Json<CreateTemplateReq>,
) -> Result<Json<CreateTemplateResp>, StatusCode> {
    let template = super::repo::insert(&st.db, &req)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(CreateTemplateResp { id: template.id }))
}

pub async fn list(
    Extension(st): Extension<AppState>,
) -> Result<Json<ListTemplatesResp>, StatusCode> {
    let items = super::repo::list(&st.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(ListTemplatesResp { items }))
}

pub async fn get(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<GetTemplateResp>, StatusCode> {
    let template = super::repo::get(&st.db, id)
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })?;
    Ok(Json(GetTemplateResp { item: template }))
}

pub async fn instantiate(
    Extension(st): Extension<AppState>,
    Path(id): Path<Uuid>,
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

    #[sqlx::test(migrations = "./migrations")]
    async fn instantiate_creates_vm_with_template(pool: sqlx::PgPool) {
        crate::features::vms::repo::reset_store();

        let hosts = HostRepository::new(pool.clone());
        hosts
            .register("test-host", "http://127.0.0.1:1", json!({}))
            .await
            .unwrap();
        let images = crate::features::images::repo::ImageRepository::new(pool.clone(), "/tmp");

        let state = crate::AppState {
            db: pool.clone(),
            hosts: hosts.clone(),
            images: images.clone(),
            allow_direct_image_paths: true,
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
            Path(template_id),
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
