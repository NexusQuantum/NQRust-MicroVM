pub mod repo;
pub mod routes;

use axum::{routing::get, Router};

pub fn router() -> Router {
    Router::new()
        .route("/", get(routes::list).post(routes::create))
        .route("/:id", get(routes::get_one).delete(routes::delete))
}
