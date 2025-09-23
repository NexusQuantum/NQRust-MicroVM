use crate::AppState;
use axum::{Extension, Router};

pub mod hosts;
pub mod logs; // A3 starter
pub mod vms; // A2 core

pub fn router(state: AppState) -> Router {
    Router::new()
        .nest("/v1/hosts", hosts::router())
        .nest("/v1/vms", vms::router())
        .nest("/v1/logs", logs::router())
        .layer(Extension(state))
}
