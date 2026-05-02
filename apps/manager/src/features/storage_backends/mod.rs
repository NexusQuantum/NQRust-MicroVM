pub mod repo;
pub mod routes;

use axum::{routing::get, Router};

pub fn router() -> Router {
    Router::new()
        .route("/", get(routes::list))
        .route("/:id/groups", get(routes::list_groups))
        .route("/:id/groups/:group_id", get(routes::get_group_status))
        .route("/:id", get(routes::get_one))
}
