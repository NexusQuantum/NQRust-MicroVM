use axum::{
    routing::{get, post},
    Router,
};

pub mod device;
pub mod license_service;
pub mod repo;
pub mod routes;
pub mod service;

pub const CURRENT_EULA_VERSION: &str = "1.0.0";
pub const AVAILABLE_LANGUAGES: [&str; 2] = ["en", "id"];

/// Public routes that don't require authentication
pub fn public_router() -> Router {
    Router::new()
        .route("/eula", get(routes::get_eula_info))
        .route("/eula/status", get(routes::get_eula_status))
        .route("/eula/accept", post(routes::accept_eula))
        .route("/license/status", get(routes::get_license_status))
        .route("/license/activate", post(routes::activate_license))
        .route(
            "/license/activate-file",
            post(routes::activate_license_file),
        )
}
