use axum::{
    routing::{get, post},
    Router,
};

pub mod repo;
pub mod routes;
pub mod service;

pub fn router() -> Router {
    Router::new()
        .route("/", post(routes::create).get(routes::list))
        .route("/suggest", get(routes::suggest))
        .route("/interfaces", get(routes::list_interfaces))
        .route(
            "/:id",
            get(routes::get)
                .patch(routes::update)
                .delete(routes::delete),
        )
        .route("/:id/vms", get(routes::get_vms))
        .route("/:id/retry", post(routes::retry))
}
