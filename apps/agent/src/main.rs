mod core;
mod features;

use serde_json::json;
use tracing::{info, warn};

#[derive(Clone)]
pub struct AppState {
    pub run_dir: String,
    pub bridge: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let bind = std::env::var("AGENT_BIND").unwrap_or_else(|_| "127.0.0.1:9090".into());
    let advertise_addr =
        std::env::var("AGENT_ADVERTISE_ADDR").unwrap_or_else(|_| format!("http://{bind}"));
    let manager_base =
        std::env::var("MANAGER_BASE").unwrap_or_else(|_| "http://127.0.0.1:8080".into());
    let host_name = std::env::var("AGENT_NAME").unwrap_or_else(|_| advertise_addr.clone());
    let state = AppState {
        run_dir: std::env::var("FC_RUN_DIR").unwrap_or_else(|_| "/srv/fc".into()),
        bridge: std::env::var("FC_BRIDGE").unwrap_or_else(|_| "fcbr0".into()),
    };

    let heartbeat_state = state.clone();
    let manager_base_clone = manager_base.clone();
    let advertise_addr_clone = advertise_addr.clone();
    tokio::spawn(async move {
        if let Err(err) = register_and_heartbeat(
            manager_base_clone,
            host_name,
            advertise_addr_clone,
            heartbeat_state,
        )
        .await
        {
            warn!(?err, "manager heartbeat task exited");
        }
    });

    let app = features::router(state);
    info!(%bind, "agent listening");
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    axum::serve(listener, app.into_make_service()).await?;
    Ok(())
}

async fn register_and_heartbeat(
    manager_base: String,
    name: String,
    addr: String,
    state: AppState,
) -> anyhow::Result<()> {
    use nexus_types::{RegisterHostRequest, RegisterHostResponse};
    use tokio::time::Duration;

    let client = reqwest::Client::new();

    loop {
        let capabilities = gather_capabilities(&state);
        match client
            .post(format!("{manager_base}/v1/hosts/register"))
            .json(&RegisterHostRequest {
                name: name.clone(),
                addr: addr.clone(),
                capabilities,
            })
            .send()
            .await
        {
            Ok(response) => match response.error_for_status() {
                Ok(success) => match success.json::<RegisterHostResponse>().await {
                    Ok(body) => {
                        info!(host_id = %body.id, "registered host with manager");
                        heartbeat_loop(&client, &manager_base, body.id, &state).await;
                    }
                    Err(err) => {
                        warn!(?err, "failed to parse register response");
                    }
                },
                Err(err) => {
                    warn!(?err, "host registration failed");
                }
            },
            Err(err) => {
                warn!(?err, "error registering host");
            }
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

async fn heartbeat_loop(
    client: &reqwest::Client,
    manager_base: &str,
    host_id: uuid::Uuid,
    state: &AppState,
) {
    use nexus_types::HostHeartbeatRequest;
    use tokio::time::{sleep, Duration};

    loop {
        let capabilities = gather_capabilities(state);
        match client
            .post(format!("{manager_base}/v1/hosts/{host_id}/heartbeat"))
            .json(&HostHeartbeatRequest {
                capabilities: Some(capabilities),
            })
            .send()
            .await
        {
            Ok(response) => {
                if let Err(err) = response.error_for_status() {
                    warn!(?err, "heartbeat rejected by manager");
                }
            }
            Err(err) => {
                warn!(?err, "failed to send heartbeat");
            }
        }
        sleep(Duration::from_secs(15)).await;
    }
}

fn gather_capabilities(state: &AppState) -> serde_json::Value {
    json!({
        "bridge": state.bridge.clone(),
        "run_dir": state.run_dir.clone(),
        "cpus": num_cpus::get(),
    })
}
