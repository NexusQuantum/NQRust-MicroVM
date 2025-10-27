use axum::{routing::{get, post}, Router};

pub mod docker;
pub mod repo;
pub mod routes;
pub mod service;
pub mod vm;

pub fn router() -> Router {
    Router::new()
        .route("/", post(routes::create).get(routes::list))
        .route("/:id", get(routes::get).put(routes::update).delete(routes::delete))
        .route("/:id/start", post(routes::start))
        .route("/:id/stop", post(routes::stop))
        .route("/:id/restart", post(routes::restart))
        .route("/:id/pause", post(routes::pause))
        .route("/:id/resume", post(routes::resume))
        .route("/:id/logs", get(routes::logs))
        .route("/:id/stats", get(routes::stats))
        .route("/:id/exec", post(routes::exec))
}
