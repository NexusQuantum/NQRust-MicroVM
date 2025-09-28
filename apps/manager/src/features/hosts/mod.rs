use axum::{routing::post, Router};

pub mod repo;
pub mod routes;

pub fn router() -> Router {
    Router::new()
        .route("/register", post(routes::register))
        .route("/:id/heartbeat", post(routes::heartbeat))
}
