use std::path::Path as StdPath;

use crate::AppState;
use axum::{
    extract::{Path, Query},
    http::StatusCode,
    Extension, Json,
};
use nexus_types::{
    CreateImageReq, CreateImageResp, GetImageResp, ImageFilter, ImagePathParams, ListImagesResp,
    OkResponse,
};

#[utoipa::path(
    post,
    path = "/v1/images",
    request_body = CreateImageReq,
    responses(
        (status = 200, description = "Image registered", body = CreateImageResp),
        (status = 400, description = "Invalid image path"),
        (status = 500, description = "Failed to store image metadata"),
    ),
    tag = "Images"
)]
pub async fn create(
    Extension(st): Extension<AppState>,
    Json(req): Json<CreateImageReq>,
) -> Result<Json<CreateImageResp>, StatusCode> {
    if !st.images.is_path_allowed(StdPath::new(&req.host_path)) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let image = st
        .images
        .insert(&req)
        .await
        .map_err(|err| map_repo_error(err))?;

    Ok(Json(CreateImageResp { id: image.id }))
}

#[utoipa::path(
    get,
    path = "/v1/images",
    params(ImageFilter),
    responses(
        (status = 200, description = "Images listed", body = ListImagesResp),
        (status = 500, description = "Failed to list images"),
    ),
    tag = "Images"
)]
pub async fn list(
    Extension(st): Extension<AppState>,
    Query(filter): Query<ImageFilter>,
) -> Result<Json<ListImagesResp>, StatusCode> {
    let items = st
        .images
        .list(&filter)
        .await
        .map_err(|err| map_repo_error(err))?;
    Ok(Json(ListImagesResp { items }))
}

#[utoipa::path(
    get,
    path = "/v1/images/{id}",
    params(ImagePathParams),
    responses(
        (status = 200, description = "Image fetched", body = GetImageResp),
        (status = 404, description = "Image not found"),
        (status = 500, description = "Failed to fetch image"),
    ),
    tag = "Images"
)]
pub async fn get(
    Extension(st): Extension<AppState>,
    Path(ImagePathParams { id }): Path<ImagePathParams>,
) -> Result<Json<GetImageResp>, StatusCode> {
    let item = st.images.get(id).await.map_err(|err| map_repo_error(err))?;
    Ok(Json(GetImageResp { item }))
}

#[utoipa::path(
    delete,
    path = "/v1/images/{id}",
    params(ImagePathParams),
    responses(
        (status = 200, description = "Image deleted", body = OkResponse),
        (status = 404, description = "Image not found"),
        (status = 500, description = "Failed to delete image"),
    ),
    tag = "Images"
)]
pub async fn delete(
    Extension(st): Extension<AppState>,
    Path(ImagePathParams { id }): Path<ImagePathParams>,
) -> Result<Json<OkResponse>, StatusCode> {
    st.images
        .delete(id)
        .await
        .map_err(|err| map_repo_error(err))?;
    Ok(Json(OkResponse::default()))
}

fn map_repo_error(err: super::repo::ImageRepoError) -> StatusCode {
    match err {
        super::repo::ImageRepoError::InvalidPath(_) => StatusCode::BAD_REQUEST,
        super::repo::ImageRepoError::Sql(sqlx::Error::RowNotFound) => StatusCode::NOT_FOUND,
        super::repo::ImageRepoError::Sql(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::hosts::repo::HostRepository;
    use crate::features::images::repo::ImageRepository;
    use axum::{extract::Path, Extension};
    use nexus_types::CreateImageReq;

    #[sqlx::test(migrations = "./migrations")]
    async fn create_and_list_images(pool: sqlx::PgPool) {
        let hosts = HostRepository::new(pool.clone());
        let images = ImageRepository::new(pool.clone(), "/srv/images");
        let snapshots = crate::features::snapshots::repo::SnapshotRepository::new(pool.clone());
        let state = crate::AppState {
            db: pool.clone(),
            hosts,
            images: images.clone(),
            snapshots,
            allow_direct_image_paths: true,
        };

        let req = CreateImageReq {
            kind: "kernel".into(),
            name: "linux".into(),
            host_path: "/srv/images/vmlinux".into(),
            sha256: "deadbeef".into(),
            size: 1234,
            project: Some("default".into()),
        };

        let Json(resp) = super::create(Extension(state.clone()), Json(req.clone()))
            .await
            .unwrap();

        let Json(list) = super::list(Extension(state.clone()), Query(ImageFilter::default()))
            .await
            .unwrap();
        assert_eq!(list.items.len(), 1);
        assert_eq!(list.items[0].id, resp.id);

        let Json(item) = super::get(
            Extension(state.clone()),
            Path(ImagePathParams { id: resp.id }),
        )
        .await
        .unwrap();
        assert_eq!(item.item.name, req.name);

        let Json(ok) = super::delete(
            Extension(state.clone()),
            Path(ImagePathParams { id: resp.id }),
        )
        .await
        .unwrap();
        assert_eq!(ok, OkResponse::default());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn reject_out_of_root_path(pool: sqlx::PgPool) {
        let hosts = HostRepository::new(pool.clone());
        let images = ImageRepository::new(pool.clone(), "/srv/images");
        let snapshots = crate::features::snapshots::repo::SnapshotRepository::new(pool.clone());
        let state = crate::AppState {
            db: pool,
            hosts,
            images,
            snapshots,
            allow_direct_image_paths: true,
        };

        let req = CreateImageReq {
            kind: "kernel".into(),
            name: "bad".into(),
            host_path: "/etc/passwd".into(),
            sha256: "deadbeef".into(),
            size: 1234,
            project: None,
        };

        let result = super::create(Extension(state), Json(req)).await;
        assert_eq!(result.unwrap_err(), StatusCode::BAD_REQUEST);
    }
}
