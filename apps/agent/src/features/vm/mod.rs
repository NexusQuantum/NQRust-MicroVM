use axum::Router;

pub mod proxy;
pub mod spawn;
pub mod stop;

pub fn router() -> Router { Router::new().merge(spawn::router()).merge(stop::router()).merge(proxy::router()) }
