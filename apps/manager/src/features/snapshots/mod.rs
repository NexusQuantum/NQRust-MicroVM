use axum::{routing::get, Router};

pub mod repo;
pub mod routes;

pub fn router() -> Router {
    Router::new().route("/:id", get(routes::get))
}
