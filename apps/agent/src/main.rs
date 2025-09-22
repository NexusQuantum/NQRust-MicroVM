mod core;
mod features;

use axum::Router;
use tracing::info;

#[derive(Clone)]
pub struct AppState {
    pub run_dir: String,
    pub bridge: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let bind = std::env::var("AGENT_BIND").unwrap_or_else(|_| "127.0.0.1:9090".into());
    let state = AppState {
        run_dir: std::env::var("FC_RUN_DIR").unwrap_or_else(|_| "/srv/fc".into()),
        bridge: std::env::var("FC_BRIDGE").unwrap_or_else(|_| "fcbr0".into()),
    };

    let app = features::router(state);
    info!(%bind, "agent listening");
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    axum::serve(listener, app.into_make_service()).await?;
    Ok(())
}
