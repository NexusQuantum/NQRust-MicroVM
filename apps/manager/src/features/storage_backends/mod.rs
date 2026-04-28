pub mod repo;
pub mod routes;

use axum::{routing::get, Router};

pub fn router() -> Router {
    Router::new()
        .route("/", get(routes::list))
        .route("/:id", get(routes::get_one))
}
