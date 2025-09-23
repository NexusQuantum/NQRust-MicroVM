use axum::{
    routing::{get, post},
    Router,
};

pub mod repo; // db
pub mod routes; // handlers
pub mod service; // orchestration

pub fn router() -> Router {
    Router::new()
        .route("/", post(routes::create).get(routes::list))
        .route("/:id", get(routes::get).delete(routes::delete))
        .route("/:id/stop", post(routes::stop))
}
