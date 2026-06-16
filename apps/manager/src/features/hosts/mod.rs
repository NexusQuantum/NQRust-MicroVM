use axum::{
    routing::{get, post},
    Router,
};

pub mod repo;
pub mod routes;

pub fn router() -> Router {
    Router::new()
        .route("/", get(routes::list))
        .route("/:id", get(routes::get).delete(routes::delete))
        .route("/:id/pci-devices", get(routes::pci_devices))
        .route("/register", post(routes::register))
        .route("/:id/heartbeat", post(routes::heartbeat))
}
