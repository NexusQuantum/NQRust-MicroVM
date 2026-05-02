use axum::{
    routing::{get, post},
    Router,
};

pub mod repo;
pub mod routes;

pub fn router() -> Router {
    Router::new()
        .route("/", get(routes::list))
        .route("/:id", get(routes::get).delete(routes::delete))
        .route("/register", post(routes::register))
        .route("/:id/heartbeat", post(routes::heartbeat))
        // B-III Task 5: toggle hot-spare flag.
        .route("/:id/hot_spare", post(routes::set_hot_spare))
        // B-III Task 6: begin host decommission.
        .route("/:id/decommission", post(routes::decommission))
}
