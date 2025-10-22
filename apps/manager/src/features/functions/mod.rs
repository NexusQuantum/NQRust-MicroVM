use axum::{routing::{get, post}, Router};

pub mod repo;
pub mod routes;
pub mod service;
pub mod vm;

pub fn router() -> Router {
    Router::new()
        .route("/", post(routes::create).get(routes::list))
        .route("/:id", get(routes::get).put(routes::update).delete(routes::delete))
        .route("/:id/invoke", post(routes::invoke))
        .route("/:id/logs", get(routes::logs))
}
