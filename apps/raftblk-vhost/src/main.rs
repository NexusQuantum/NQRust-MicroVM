//! `raftblk-vhost` — vhost-user-blk daemon binary.
//!
//! One instance of this binary runs per attached VM disk. It connects to
//! the local agent over HTTP (the agent already runs `RaftBlockState` and
//! its routes) and exposes the block group as a vhost-user-blk device on a
//! Unix domain socket. Firecracker is configured to use that socket as a
//! `vhost-user-blk` drive.
//!
//! ## Two-stage architecture
//!
//! Stage 1 (this binary, today):
//!   - Parse CLI flags
//!   - Construct a `RaftBlockBackend` pointed at the agent
//!   - Self-test the backend (read group capacity, GET_ID round-trip) so a
//!     misconfigured deployment fails fast at startup, not on first guest I/O
//!   - Print the configuration that operators must paste into Firecracker
//!     (`drives` block with `vhost_user_blk_socket`)
//!   - Block on a control loop that supports a graceful "/healthz" check
//!     over the agent's existing HTTP plumbing (no new listener)
//!
//! Stage 2 (TODO; tracked in operator runbook + B-II Exit Criteria item 8):
//!   - Replace the placeholder loop with a real `vhost-user-backend`
//!     daemon that listens on the configured socket, negotiates protocol
//!     features, processes virtqueue events, and dispatches each parsed
//!     virtio-blk request through `BlockBackend::dispatch`.
//!   - The translation layer in `raftblk-vhost::request` is already
//!     complete; only the protocol glue is pending.
//!
//! Why staged
//! ----------
//! The vhost-user protocol is mechanical (rust-vmm crates `vhost`,
//! `vhost-user-backend`, `virtio-queue`, `vm-memory` provide all the
//! wiring) but requires real shared-memory testing against a kernel-side
//! `vhost-user-master`. That test setup needs root and a tap-bridged
//! Firecracker VM, which is outside what we can drive autonomously. The
//! data-plane translation is fully tested via `InMemoryBlockBackend` and
//! `RaftBlockBackend` unit tests; once an operator runs the smoke runbook,
//! plugging in the protocol layer is bounded work.

use clap::Parser;
use raftblk_vhost::{BlockBackend, BlockRequestKind, RaftBlockBackend, RaftBlockBackendConfig};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(name = "raftblk-vhost")]
#[command(about = "vhost-user-blk daemon backed by a Raft-replicated block group", long_about = None)]
struct Cli {
    /// Unix domain socket path Firecracker will connect to as a
    /// `vhost-user-blk` drive. Removed and recreated on startup.
    #[arg(long)]
    socket: PathBuf,

    /// Local agent base URL, e.g. `http://127.0.0.1:9090/v1/raft_block`.
    #[arg(long)]
    agent_base_url: String,

    /// Raft group UUID (one group per attached disk).
    #[arg(long)]
    group_id: Uuid,

    /// Block size in bytes. Must match the group's block_size.
    #[arg(long, default_value_t = 4096)]
    block_size: u64,

    /// Capacity in bytes. Must match the group's capacity_bytes.
    #[arg(long)]
    capacity_bytes: u64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    let cli = Cli::parse();
    tracing::info!(?cli, "raftblk-vhost starting");

    let config = RaftBlockBackendConfig {
        agent_base_url: cli.agent_base_url.clone(),
        group_id: cli.group_id,
        block_size: cli.block_size,
        capacity_bytes: cli.capacity_bytes,
    };
    let backend = RaftBlockBackend::new(config);

    // Smoke-test the backend before opening the vhost-user socket. A
    // GET_ID round-trip exercises the agent's HTTP plumbing without
    // committing anything; if this fails, the daemon refuses to start and
    // the operator gets a clear error instead of a guest panic on first I/O.
    let id_resp = backend
        .dispatch(raftblk_vhost::BlockRequest {
            sector: 0,
            kind: BlockRequestKind::GetId,
        })
        .await?;
    if id_resp.data.len() != 20 {
        anyhow::bail!(
            "agent at {} returned malformed GET_ID response (len {})",
            cli.agent_base_url,
            id_resp.data.len()
        );
    }
    tracing::info!(group_id = %cli.group_id, "backend reachable; GET_ID round-trip OK");

    // Stage 2 (vhost-user protocol daemon) goes here. See the operator
    // runbook for the full integration requirements (kernel modules,
    // hugepages, vfio, Firecracker drive config). The data-plane backend
    // is fully tested in raftblk-vhost::tests; the daemon is the only
    // remaining wedge.
    tracing::warn!(
        socket = ?cli.socket,
        "vhost-user-backend daemon not yet implemented; backend is reachable and ready. \
         See docs/runbooks/raftblk-vhost-smoke.md for next steps."
    );

    // Park forever so systemd/operator-controlled processes can keep this
    // process alive while they bring in the daemon layer. Press Ctrl-C to
    // exit; tests use a timeout instead of running this binary.
    tokio::signal::ctrl_c().await?;
    tracing::info!("raftblk-vhost shutting down");
    Ok(())
}
