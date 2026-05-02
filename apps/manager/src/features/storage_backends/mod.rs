pub mod executor;
pub mod planner;
pub mod reconciler;
pub mod repo;
pub mod routes;

use axum::{routing::get, Router};

pub fn router() -> Router {
    Router::new()
        .route("/", get(routes::list))
        .route("/:id/groups", get(routes::list_groups))
        .route("/:id/groups/:group_id", get(routes::get_group_status))
        .route(
            "/:id/groups/:group_id/replicas",
            axum::routing::post(routes::add_replica),
        )
        .route(
            "/:id/groups/:group_id/replicas/:node_id/repair",
            axum::routing::post(routes::repair_replica),
        )
        .route(
            "/:id/groups/:group_id/replicas/:node_id/repair_status",
            get(routes::repair_status),
        )
        .route(
            "/:id/groups/:group_id/replicas/:node_id",
            axum::routing::delete(routes::remove_replica),
        )
        .route("/:id/repair_queue", get(routes::list_repair_queue))
        // B-III Task 6: decommission plan preview.
        .route("/:id/decommission_plan", get(routes::decommission_plan))
        // B-III Task 7: hot-spare promotion plan preview.
        .route("/:id/promotion_plan", get(routes::promotion_plan))
        // B-III Task 8: rebalance plan preview.
        .route("/:id/rebalance_plan", get(routes::rebalance_plan))
        // B-III plan execution: operator runs a previewed plan.
        .route(
            "/:id/execute_plan",
            axum::routing::post(routes::execute_plan),
        )
        .route("/:id", get(routes::get_one))
}
