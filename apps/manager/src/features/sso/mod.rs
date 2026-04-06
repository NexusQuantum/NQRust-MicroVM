pub mod crypto;
pub mod oidc;
pub mod provisioning;
pub mod repo;
pub mod routes;
pub mod saml;

use axum::{
    routing::{get, post},
    Router,
};

/// Public SSO routes — no auth required (these ARE the auth flow).
pub fn public_router() -> Router {
    Router::new()
        .route("/providers", get(routes::list_enabled_providers))
        .route("/oidc/:slug/login", get(routes::oidc_login_initiate))
        .route("/oidc/:slug/callback", get(routes::oidc_callback))
        .route("/saml/:slug/login", get(routes::saml_login_initiate))
        .route("/saml/:slug/metadata", get(routes::saml_metadata))
        .route("/saml/:slug/acs", post(routes::saml_acs))
}

/// Admin SSO management routes — requires auth + admin.
pub fn admin_router() -> Router {
    Router::new()
        .route(
            "/providers",
            get(routes::admin_list_providers).post(routes::admin_create_provider),
        )
        .route(
            "/providers/:id",
            get(routes::admin_get_provider)
                .patch(routes::admin_update_provider)
                .delete(routes::admin_delete_provider),
        )
        .route("/providers/:id/test", post(routes::admin_test_provider))
}
