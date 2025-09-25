use axum::{
    routing::{get, post},
    Router,
};

pub mod repo;
pub mod routes;

pub fn router() -> Router {
    Router::new()
        .route("/", post(routes::create).get(routes::list))
        .route("/:id", get(routes::get))
        .route("/:id/instantiate", post(routes::instantiate))
}
