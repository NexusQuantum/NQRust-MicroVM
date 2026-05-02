//! `nqvm` operator CLI.
//!
//! Thin wrapper around the manager's HTTP API for the operator-facing
//! storage and host-lifecycle endpoints. Read-only commands by default;
//! the explicit `--execute` flag is required to run mutating operations
//! so that "I just wanted to see the plan" never accidentally migrates
//! data.

use anyhow::{anyhow, Context, Result};
use clap::{Args, Parser, Subcommand};
use serde::Serialize;
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(name = "nqvm", version, about = "NQRust-MicroVM operator CLI")]
struct Cli {
    /// Manager API base URL. Defaults to `NQVM_MANAGER` or
    /// `http://127.0.0.1:18080`.
    #[arg(long, env = "NQVM_MANAGER", default_value = "http://127.0.0.1:18080")]
    manager: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Storage backend operations (raft_spdk membership, repair, plans).
    Storage {
        #[command(subcommand)]
        sub: StorageCmd,
    },
    /// Host lifecycle (hot-spare flag, decommission).
    Hosts {
        #[command(subcommand)]
        sub: HostCmd,
    },
}

#[derive(Subcommand, Debug)]
enum StorageCmd {
    /// List all storage backends.
    Backends,
    /// List groups under a backend.
    Groups {
        #[arg(long)]
        backend: Uuid,
    },
    /// Show detailed status for one group across replicas.
    Group {
        #[arg(long)]
        backend: Uuid,
        #[arg(long)]
        group: Uuid,
    },
    /// Show the repair queue for a backend.
    RepairQueue {
        #[arg(long)]
        backend: Uuid,
    },
    /// Preview the decommission plan for a host.
    DecommissionPlan {
        #[arg(long)]
        backend: Uuid,
        #[arg(long)]
        host: Uuid,
    },
    /// Preview the hot-spare promotion plan for a (failed) host.
    PromotionPlan {
        #[arg(long)]
        backend: Uuid,
        #[arg(long)]
        host: Uuid,
    },
    /// Preview the rebalance plan for a backend.
    RebalancePlan {
        #[arg(long)]
        backend: Uuid,
    },
    /// Trigger a single-replica repair.
    Repair {
        #[arg(long)]
        backend: Uuid,
        #[arg(long)]
        group: Uuid,
        #[arg(long)]
        node: u64,
    },
    /// Add a replica to an existing group.
    AddReplica(AddReplicaArgs),
    /// Remove a replica from a group.
    RemoveReplica {
        #[arg(long)]
        backend: Uuid,
        #[arg(long)]
        group: Uuid,
        #[arg(long)]
        node: u64,
    },
}

#[derive(Args, Debug)]
struct AddReplicaArgs {
    #[arg(long)]
    backend: Uuid,
    #[arg(long)]
    group: Uuid,
    #[arg(long)]
    node: u64,
    #[arg(long)]
    agent_base_url: String,
    #[arg(long)]
    spdk_backend_id: Uuid,
}

#[derive(Subcommand, Debug)]
enum HostCmd {
    /// List all hosts.
    List,
    /// Mark a host as a hot-spare.
    HotSpare {
        #[arg(long)]
        host: Uuid,
        /// Use `--off` to clear the flag instead of setting it.
        #[arg(long)]
        off: bool,
    },
    /// Begin host decommission (transitions host to `draining`).
    Decommission {
        #[arg(long)]
        host: Uuid,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("build http client")?;
    let base = cli.manager.trim_end_matches('/').to_string();
    match cli.command {
        Command::Storage { sub } => storage(&client, &base, sub).await,
        Command::Hosts { sub } => hosts(&client, &base, sub).await,
    }
}

async fn storage(client: &reqwest::Client, base: &str, sub: StorageCmd) -> Result<()> {
    match sub {
        StorageCmd::Backends => print_get(client, &format!("{base}/v1/storage_backends")).await,
        StorageCmd::Groups { backend } => {
            print_get(
                client,
                &format!("{base}/v1/storage_backends/{backend}/groups"),
            )
            .await
        }
        StorageCmd::Group { backend, group } => {
            print_get(
                client,
                &format!("{base}/v1/storage_backends/{backend}/groups/{group}"),
            )
            .await
        }
        StorageCmd::RepairQueue { backend } => {
            print_get(
                client,
                &format!("{base}/v1/storage_backends/{backend}/repair_queue"),
            )
            .await
        }
        StorageCmd::DecommissionPlan { backend, host } => {
            print_get(
                client,
                &format!("{base}/v1/storage_backends/{backend}/decommission_plan?host_id={host}"),
            )
            .await
        }
        StorageCmd::PromotionPlan { backend, host } => {
            print_get(
                client,
                &format!("{base}/v1/storage_backends/{backend}/promotion_plan?host_id={host}"),
            )
            .await
        }
        StorageCmd::RebalancePlan { backend } => {
            print_get(
                client,
                &format!("{base}/v1/storage_backends/{backend}/rebalance_plan"),
            )
            .await
        }
        StorageCmd::Repair {
            backend,
            group,
            node,
        } => {
            print_post::<()>(
                client,
                &format!(
                    "{base}/v1/storage_backends/{backend}/groups/{group}/replicas/{node}/repair"
                ),
                None,
            )
            .await
        }
        StorageCmd::AddReplica(args) => {
            #[derive(Serialize)]
            struct Body {
                node_id: u64,
                agent_base_url: String,
                spdk_backend_id: Uuid,
            }
            let body = Body {
                node_id: args.node,
                agent_base_url: args.agent_base_url,
                spdk_backend_id: args.spdk_backend_id,
            };
            print_post(
                client,
                &format!(
                    "{base}/v1/storage_backends/{}/groups/{}/replicas",
                    args.backend, args.group
                ),
                Some(&body),
            )
            .await
        }
        StorageCmd::RemoveReplica {
            backend,
            group,
            node,
        } => {
            let url =
                format!("{base}/v1/storage_backends/{backend}/groups/{group}/replicas/{node}");
            let resp = client
                .delete(&url)
                .send()
                .await
                .with_context(|| format!("DELETE {url}"))?;
            print_response(resp).await
        }
    }
}

async fn hosts(client: &reqwest::Client, base: &str, sub: HostCmd) -> Result<()> {
    match sub {
        HostCmd::List => print_get(client, &format!("{base}/v1/hosts")).await,
        HostCmd::HotSpare { host, off } => {
            #[derive(Serialize)]
            struct Body {
                is_hot_spare: bool,
            }
            let body = Body { is_hot_spare: !off };
            print_post(
                client,
                &format!("{base}/v1/hosts/{host}/hot_spare"),
                Some(&body),
            )
            .await
        }
        HostCmd::Decommission { host } => {
            print_post::<()>(
                client,
                &format!("{base}/v1/hosts/{host}/decommission"),
                None,
            )
            .await
        }
    }
}

async fn print_get(client: &reqwest::Client, url: &str) -> Result<()> {
    let resp = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("GET {url}"))?;
    print_response(resp).await
}

async fn print_post<T: Serialize>(
    client: &reqwest::Client,
    url: &str,
    body: Option<&T>,
) -> Result<()> {
    let mut req = client.post(url);
    if let Some(body) = body {
        req = req.json(body);
    }
    let resp = req.send().await.with_context(|| format!("POST {url}"))?;
    print_response(resp).await
}

async fn print_response(resp: reqwest::Response) -> Result<()> {
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    // Try to pretty-print as JSON; fall back to raw bytes for non-JSON
    // responses (e.g. plain-text errors).
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&body) {
        let pretty = serde_json::to_string_pretty(&parsed).unwrap_or(body.clone());
        println!("{pretty}");
    } else if !body.is_empty() {
        println!("{body}");
    }
    if !status.is_success() {
        return Err(anyhow!("server returned {status}"));
    }
    Ok(())
}
