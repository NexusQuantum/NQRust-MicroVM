use axum::{
    routing::{get, post},
    Router,
};

pub mod dockerhub;
pub mod preload;
pub mod repo;
pub mod routes;
pub mod scan;
pub mod upload;

pub fn router() -> Router {
    Router::new()
        .route("/", post(routes::create).get(routes::list))
        .route("/:id", get(routes::get).delete(routes::delete))
        .route("/upload", post(routes::upload_image))
        .route("/dockerhub/search", post(routes::dockerhub_search))
        .route("/dockerhub/tags", post(routes::dockerhub_tags))
        .route("/dockerhub/download", post(routes::dockerhub_download))
        .route(
            "/dockerhub/download/progress/:image_name",
            get(routes::dockerhub_download_progress),
        )
        .route("/dockerhub/preload", post(routes::dockerhub_preload))
}
