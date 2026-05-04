use crate::AppState;
use axum::{Extension, Router};
use std::sync::Arc;

pub mod health;
pub mod inventory;
pub mod networks;
pub mod storage;
pub mod tap;
pub mod vm;

pub fn router(state: AppState) -> Router {
    let storage_state = Arc::new(storage::routes::StorageState {
        registry: state.storage_registry.clone(),
        nfs_config: state.nfs_config.clone(),
    });
    Router::new()
        .merge(health::router())
        .merge(inventory::router())
        .nest("/agent/v1/vms", vm::router().merge(tap::router()))
        .nest("/agent/v1/networks", networks::router())
        .nest("/v1/storage", storage::routes::router(storage_state))
        .layer(Extension(state))
}
