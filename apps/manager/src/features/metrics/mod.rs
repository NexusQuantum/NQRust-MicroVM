pub mod collector;
pub mod repo;
mod routes;

use crate::AppState;
use axum::Router;

pub fn router() -> Router {
    Router::new()
        .route("/hosts/:id", axum::routing::get(routes::get_host_metrics))
        .route("/vms/:id", axum::routing::get(routes::get_vm_metrics))
        .route(
            "/containers/:id",
            axum::routing::get(routes::get_container_metrics),
        )
}

pub fn spawn_collector(state: AppState) -> tokio::task::JoinHandle<()> {
    collector::spawn(state)
}
