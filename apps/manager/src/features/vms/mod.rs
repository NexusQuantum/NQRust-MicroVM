use axum::{Router, routing::{get, post}};


pub mod routes; // handlers
pub mod service; // orchestration
pub mod repo; // db


pub fn router() -> Router {
    Router::new()
        .route("/", post(routes::create).get(routes::list))
        .route("/:id", get(routes::get))
        .route("/:id/stop", post(routes::stop))
}