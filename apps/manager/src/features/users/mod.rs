use axum::{
    routing::{delete, get, patch, post},
    Router,
};

pub mod audit;
pub mod authz;
pub mod middleware;
pub mod repo;
pub mod routes;

pub fn auth_router() -> Router {
    Router::new()
        .route("/login", post(routes::login))
        .route("/me", get(routes::me))
        .route(
            "/me/preferences",
            get(routes::get_preferences).patch(routes::update_preferences),
        )
        .route(
            "/me/profile",
            get(routes::get_profile).patch(routes::update_profile),
        )
        .route("/me/password", post(routes::change_password))
        .route(
            "/me/avatar",
            post(routes::upload_avatar).delete(routes::delete_avatar),
        )
        .route("/me/avatar", get(routes::get_my_avatar))
}

pub fn users_router() -> Router {
    use axum::middleware::from_fn;

    Router::new()
        .route("/", get(routes::list).post(routes::create))
        .route(
            "/:id",
            get(routes::get)
                .patch(routes::update)
                .delete(routes::delete),
        )
        .route("/:id/avatar", get(routes::get_user_avatar))
        .layer(from_fn(middleware::require_admin)) // Protect all user management routes - admin only
}
