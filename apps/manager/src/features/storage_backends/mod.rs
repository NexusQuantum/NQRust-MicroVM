pub mod discovery;
pub mod repo;
pub mod routes;

use axum::{routing::get, Router};

pub fn router() -> Router {
    Router::new()
        .route("/", get(routes::list).post(routes::create))
        .route("/scan/nfs", get(routes::scan_nfs))
        .route("/scan/iscsi", get(routes::scan_iscsi))
        .route("/:id", get(routes::get_one).delete(routes::delete))
}
