pub mod discovery;
pub mod health;
pub mod initialize;
pub mod repo;
pub mod routes;

use axum::{
    routing::{get, post},
    Router,
};

pub fn router() -> Router {
    Router::new()
        .route("/", get(routes::list).post(routes::create))
        .route("/scan/nfs", get(routes::scan_nfs))
        .route("/scan/iscsi", get(routes::scan_iscsi))
        .route(
            "/:id",
            get(routes::get_one)
                .put(routes::update)
                .delete(routes::delete),
        )
        .route("/:id/config", get(routes::get_config))
        .route("/:id/health", get(routes::health))
        .route("/:id/initialize", post(initialize::initialize))
}
