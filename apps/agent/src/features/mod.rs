use axum::{Router, Extension};
use crate::AppState;


pub mod health;
pub mod tap;
pub mod vm;


pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(health::router())
        .nest("/agent/v1/vms", vm::router().merge(tap::router()))
        .layer(Extension(state))
}