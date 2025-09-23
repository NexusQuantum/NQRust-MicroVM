use crate::AppState;
use axum::{Extension, Router};

pub mod health;
pub mod inventory;
pub mod tap;
pub mod vm;

pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(health::router())
        .merge(inventory::router())
        .nest("/agent/v1/vms", vm::router().merge(tap::router()))
        .layer(Extension(state))
}
