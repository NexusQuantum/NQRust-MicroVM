use crate::AppState;
use axum::{Extension, Router};
use std::sync::Arc;

pub mod health;
pub mod inventory;
pub mod networks;
pub mod raft_block;
pub mod storage;
pub mod tap;
pub mod vm;

pub fn router(state: AppState) -> Router {
    let raft_block_state = Arc::new(raft_block::RaftBlockState::new(state.run_dir.clone()));
    let storage_state = Arc::new(storage::routes::StorageState {
        registry: state.storage_registry.clone(),
    });
    Router::new()
        .merge(health::router())
        .merge(inventory::router())
        .nest("/agent/v1/vms", vm::router().merge(tap::router()))
        .nest("/agent/v1/networks", networks::router())
        .nest("/v1/raft_block", raft_block::router(raft_block_state))
        .nest("/v1/storage", storage::routes::router(storage_state))
        .layer(Extension(state))
}
