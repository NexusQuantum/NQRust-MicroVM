use axum::Router;

pub mod proxy;
pub mod snapshot;
pub mod spawn;
pub mod stop;
pub mod metrics;

pub fn router() -> Router {
    Router::new()
        .merge(spawn::router())
        .merge(stop::router())
        .merge(proxy::router())
        .merge(snapshot::router())
        .merge(metrics::router())
}
