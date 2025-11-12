use axum::{
    routing::{delete, get, patch, post},
    Router,
};

pub mod repo;
pub mod routes;

pub fn router() -> Router {
    Router::new()
        .route("/", post(routes::create).get(routes::list))
        .route("/:id", get(routes::get).patch(routes::update).delete(routes::delete))
        .route("/:id/vms", get(routes::get_vms))
}
