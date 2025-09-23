pub mod core;
mod features;

use sqlx::PgPool;
use tracing::info;

use features::hosts::repo::HostRepository;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub hosts: HostRepository,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let db = PgPool::connect(&std::env::var("DATABASE_URL")?).await?;
    sqlx::migrate!("./migrations").run(&db).await?;

    let hosts = HostRepository::new(db.clone());
    let state = AppState { db, hosts };

    let app = features::router(state.clone());
    let bind = std::env::var("MANAGER_BIND").unwrap_or_else(|_| "127.0.0.1:8080".into());
    info!(%bind, "manager listening");
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    axum::serve(listener, app.into_make_service()).await?;
    Ok(())
}
