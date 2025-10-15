use axum::{
    routing::{get, post},
    Router,
};

pub mod repo;
pub mod routes;

pub fn router() -> Router {
    Router::new()
        .route("/:id", get(routes::get).delete(routes::delete))
        .route("/:id/instantiate", post(routes::instantiate))
}
