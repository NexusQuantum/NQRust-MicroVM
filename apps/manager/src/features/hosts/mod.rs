use axum::{routing::post, Router};

pub mod repo;
mod routes;

pub fn router() -> Router {
    Router::new()
        .route("/register", post(routes::register))
        .route("/:id/heartbeat", post(routes::heartbeat))
}
