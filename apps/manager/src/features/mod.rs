use crate::AppState;
use axum::{Extension, Router};

pub mod hosts;
pub mod images;
pub mod logs; // A3 starter
pub mod reconciler;
pub mod snapshots;
pub mod templates;
pub mod vms; // A2 core

pub fn router(state: AppState) -> Router {
    Router::new()
        .nest("/v1/hosts", hosts::router())
        .nest("/v1/images", images::router())
        .nest("/v1/templates", templates::router())
        .nest("/v1/vms", vms::router())
        .nest("/v1/snapshots", snapshots::router())
        .route(
            "/v1/vms/:id/snapshots",
            axum::routing::post(snapshots::routes::create).get(snapshots::routes::list_for_vm),
        )
        .nest("/v1/logs", logs::router())
        .layer(Extension(state))
}
