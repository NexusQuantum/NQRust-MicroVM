pub mod envelope;
pub mod repo;
pub mod routes;

use axum::{
    routing::{get, post},
    Router,
};

pub fn router() -> Router {
    Router::new()
        .route("/", post(routes::create).get(routes::list))
        .route(
            "/:id",
            get(routes::get_one)
                .patch(routes::update)
                .delete(routes::soft_delete),
        )
        .route("/:id/gc", post(routes::trigger_gc))
}
