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
use features::users::repo::UserRepository;
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
    pub users: UserRepository,
    pub shell_repo: ShellRepository,
    pub allow_direct_image_paths: bool,
    pub storage: LocalStorage,
    pub download_progress: DownloadProgressTracker,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("warn,manager=info")
            .add_directive("hyper_util=warn".parse().unwrap())
            .add_directive("sqlx=warn".parse().unwrap())
    });
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let db = PgPool::connect(&std::env::var("DATABASE_URL")?).await?;
    sqlx::migrate!("./migrations").run(&db).await?;

    // Initialize default admin user if no users exist
    let users_repo = UserRepository::new(db.clone());
    let user_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
        .fetch_one(&db)
        .await?;
    if user_count == 0 {
        info!("No users found, creating default admin user (username: root, password: root)");
        match users_repo
            .create_user("root", "root", nexus_types::Role::Admin)
            .await
        {
            Ok(user) => {
                info!(username = %user.username, user_id = %user.id, "Default admin user created");
            }
            Err(e) => {
                warn!(error = ?e, "Failed to create default admin user");
            }
        }
    }

    let hosts = HostRepository::new(db.clone());
    let image_root =
        std::env::var("MANAGER_IMAGE_ROOT").unwrap_or_else(|_| "/srv/images".to_string());
    let images = ImageRepository::new(db.clone(), &image_root);
    let snapshots = SnapshotRepository::new(db.clone());
    let users = UserRepository::new(db.clone());
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
        users,
        shell_repo,
        download_progress,
        allow_direct_image_paths,
        storage: LocalStorage::new(),
    };

    // Auto-register base images found in the image root directory
    match features::images::scan::scan_and_register_base_images(&state.images).await {
        Ok(count) => {
            if count > 0 {
                info!("Auto-registered {} base images from {}", count, image_root);
            }
        }
        Err(e) => {
            warn!(error = ?e, "Failed to scan and register base images");
        }
    }

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
                .allow_headers(Any)
                .max_age(std::time::Duration::from_secs(3600)),
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
