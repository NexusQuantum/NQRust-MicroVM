use axum::{
    routing::{delete, get, post, put},
    Router,
};

pub mod repo;
pub mod routes;

pub fn router() -> Router {
    Router::new()
        .route("/", post(routes::create).get(routes::list))
        .route(
            "/:id",
            get(routes::get).put(routes::update).delete(routes::delete),
        )
        .route("/:id/instantiate", post(routes::instantiate))
}
