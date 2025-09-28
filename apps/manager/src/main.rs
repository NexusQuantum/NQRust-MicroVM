pub mod core;
mod docs;
mod features;

use sqlx::PgPool;
use tracing::{info, warn};
use utoipa::OpenApi as _;

use features::hosts::repo::HostRepository;
use features::images::repo::ImageRepository;
use features::snapshots::repo::SnapshotRepository;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub hosts: HostRepository,
    pub images: ImageRepository,
    pub snapshots: SnapshotRepository,
    pub allow_direct_image_paths: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let db = PgPool::connect(&std::env::var("DATABASE_URL")?).await?;
    sqlx::migrate!("./migrations").run(&db).await?;

    let hosts = HostRepository::new(db.clone());
    let image_root =
        std::env::var("MANAGER_IMAGE_ROOT").unwrap_or_else(|_| "/srv/images".to_string());
    let images = ImageRepository::new(db.clone(), image_root);
    let snapshots = SnapshotRepository::new(db.clone());
    let allow_direct_image_paths = std::env::var("MANAGER_ALLOW_IMAGE_PATHS")
        .map(|value| matches_ignore_case(value.trim()))
        .unwrap_or(false);
    let state = AppState {
        db,
        hosts,
        images,
        snapshots,
        allow_direct_image_paths,
    };

    let _reconciler_handle = features::reconciler::spawn(state.clone());

    let openapi = docs::ApiDoc::openapi();
    if let Err(err) = docs::write_openapi_yaml(&openapi).await {
        warn!(error = ?err, "failed to write OpenAPI specification to disk");
    }

    let app = features::router(state.clone()).merge(docs::router(openapi));
    let bind = std::env::var("MANAGER_BIND").unwrap_or_else(|_| "127.0.0.1:8080".into());
    info!(%bind, "manager listening");
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    axum::serve(listener, app.into_make_service()).await?;
    Ok(())
}

fn matches_ignore_case(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}
