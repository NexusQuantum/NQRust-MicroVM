use crate::AppState;
use axum::{Extension, Router};

pub mod containers;
pub mod functions;
pub mod hosts;
pub mod images;
pub mod logs; // A3 starter
pub mod networks;
pub mod reconciler;
pub mod snapshots;
pub mod storage;
pub mod templates;
pub mod users;
pub mod vms; // A2 core
pub mod volumes;

pub fn router(state: AppState) -> Router {
    Router::new()
        .nest(
            "/v1/auth",
            users::auth_router().route_layer(axum::middleware::from_fn_with_state(
                state.clone(),
                users::middleware::auth_middleware,
            )),
        )
        .nest(
            "/v1/users",
            users::users_router()
                .layer(axum::middleware::from_fn(users::middleware::require_admin))
                .layer(axum::middleware::from_fn_with_state(
                    state.clone(),
                    users::middleware::auth_middleware,
                )),
        )
        .nest("/v1/hosts", hosts::router())
        .nest("/v1/images", images::router())
        .nest("/v1/networks", networks::router())
        .nest("/v1/templates", templates::router())
        .nest("/v1/vms", vms::router())
        .nest("/v1/snapshots", snapshots::router())
        .route(
            "/v1/vms/:id/snapshots",
            axum::routing::post(snapshots::routes::create).get(snapshots::routes::list_for_vm),
        )
        .nest("/v1/functions", functions::router())
        .nest("/v1/containers", containers::router())
        .nest("/v1/logs", logs::router())
        .nest("/v1/volumes", volumes::router())
        .layer(Extension(state))
}
