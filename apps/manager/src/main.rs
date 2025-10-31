pub mod core;
mod docs;
mod features;

use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;
use utoipa::OpenApi as _;

use crate::features::storage::LocalStorage;
use features::hosts::repo::HostRepository;
use features::images::repo::ImageRepository;
use features::snapshots::repo::SnapshotRepository;
use features::vms::shell::ShellRepository;

#[derive(Clone, Debug, serde::Serialize)]
pub struct DownloadProgress {
    pub image: String,
    pub status: String,
    pub current_bytes: i64,
    pub total_bytes: i64,
    pub completed: bool,
    pub error: Option<String>,
}

pub type DownloadProgressTracker = Arc<Mutex<HashMap<String, DownloadProgress>>>;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub hosts: HostRepository,
    pub images: ImageRepository,
    pub snapshots: SnapshotRepository,
    pub shell_repo: ShellRepository,
    pub allow_direct_image_paths: bool,
    pub storage: LocalStorage,
    pub download_progress: DownloadProgressTracker,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let db = PgPool::connect(&std::env::var("DATABASE_URL")?).await?;
    sqlx::migrate!("./migrations").run(&db).await?;

    let hosts = HostRepository::new(db.clone());
    let image_root =
        std::env::var("MANAGER_IMAGE_ROOT").unwrap_or_else(|_| "/srv/images".to_string());
    let images = ImageRepository::new(db.clone(), image_root);
    let snapshots = SnapshotRepository::new(db.clone());
    let shell_repo = ShellRepository::new(db.clone());
    let allow_direct_image_paths = std::env::var("MANAGER_ALLOW_IMAGE_PATHS")
        .map(|value| matches_ignore_case(value.trim()))
        .unwrap_or(false);
    let download_progress = Arc::new(Mutex::new(HashMap::new()));
    let state = AppState {
        db,
        hosts,
        images,
        snapshots,
        shell_repo,
        download_progress,
        allow_direct_image_paths,
        storage: LocalStorage::new(),
    };

    // Allow disabling the reconciler via env for test/debug to avoid races during VM creation
    let reconciler_disabled = std::env::var("MANAGER_RECONCILER_DISABLED")
        .map(|v| matches_ignore_case(v.trim()))
        .unwrap_or(false);
    if !reconciler_disabled {
        let _reconciler_handle = features::reconciler::spawn(state.clone());
    } else {
        warn!("reconciler disabled by MANAGER_RECONCILER_DISABLED");
    }

    let openapi = docs::ApiDoc::openapi();
    if let Err(err) = docs::write_openapi_yaml(&openapi).await {
        warn!(error = ?err, "failed to write OpenAPI specification to disk");
    }

    let app = features::router(state.clone())
        .merge(docs::router(openapi))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );
    let bind = std::env::var("MANAGER_BIND").unwrap_or_else(|_| "127.0.0.1:18080".into());
    info!(%bind, "manager listening");
    if let Ok(host_id) = std::env::var("MANAGER_HOST_ID") {
        let capabilities = serde_json::json!({
            "bridge": std::env::var("MANAGER_BRIDGE").unwrap_or_else(|_| "fcbr0".into())
        });
        let _ = state
            .hosts
            .register("manager-host", &host_id, capabilities)
            .await;
    }
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
