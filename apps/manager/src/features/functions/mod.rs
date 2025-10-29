use axum::{
    routing::{get, post},
    Router,
};

pub mod repo;
pub mod routes;
pub mod service;
pub mod snapshots; // golden snapshot system for ultra-fast provisioning
pub mod vm;

pub fn router() -> Router {
    Router::new()
        .route("/", post(routes::create).get(routes::list))
        .route(
            "/:id",
            get(routes::get).put(routes::update).delete(routes::delete),
        )
        .route("/:id/invoke", post(routes::invoke))
        .route("/:id/logs", get(routes::logs))
        // Golden snapshot management
        .route("/snapshots", get(routes::list_golden_snapshots))
        .route("/snapshots/:runtime", post(routes::create_golden_snapshot).delete(routes::delete_golden_snapshot))
}
