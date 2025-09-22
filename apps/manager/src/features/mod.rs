use axum::{Router, Extension};
use crate::AppState;


pub mod vms; // A2 core
pub mod logs; // A3 starter


pub fn router(state: AppState) -> Router {
    Router::new()
        .nest("/v1/vms", vms::router())
        .nest("/v1/logs", logs::router())
        .layer(Extension(state))
}