use axum::Router;

pub mod balloon;
pub mod entropy;
pub mod metrics;
pub mod mmds;
pub mod port_forward;
pub mod proxy;
pub mod serial;
pub mod shell;
pub mod snapshot;
pub mod spawn;
pub mod stop;
pub mod system;
pub mod vsock;

pub fn router() -> Router {
    Router::new()
        .merge(spawn::router())
        .merge(stop::router())
        .merge(vsock::router())
        .merge(mmds::router())
        .merge(entropy::router())
        .merge(serial::router())
        .merge(proxy::router())
        .merge(snapshot::router())
        .merge(metrics::router())
        .merge(balloon::router())
        .merge(system::router())
        .merge(shell::router())
        .merge(port_forward::router())
}
