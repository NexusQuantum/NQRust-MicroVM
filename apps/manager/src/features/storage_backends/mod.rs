pub mod repo;
pub mod routes;

use axum::{routing::get, Router};

pub fn router() -> Router {
    Router::new()
        .route("/", get(routes::list))
        .route("/:id/groups", get(routes::list_groups))
        .route("/:id/groups/:group_id", get(routes::get_group_status))
        .route(
            "/:id/groups/:group_id/replicas/:node_id/repair",
            axum::routing::post(routes::repair_replica),
        )
        .route(
            "/:id/groups/:group_id/replicas/:node_id/repair_status",
            get(routes::repair_status),
        )
        .route("/:id/repair_queue", get(routes::list_repair_queue))
        .route("/:id", get(routes::get_one))
}
