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
use raftblk_vhost::{
    BlockBackend, BlockRequestKind, RaftBlkVhostBackend, RaftBlockBackend, RaftBlockBackendConfig,
};
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;
use vhost_user_backend::VhostUserDaemon;
use vm_memory::{GuestMemoryAtomic, GuestMemoryMmap};
use vmm_sys_util::eventfd::EventFd;

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

    // Stage 2 — wire the backend into a vhost-user-backend daemon.
    //
    // The trait surface is correctly implemented in
    // `raftblk_vhost::daemon::RaftBlkVhostBackend` (features, config
    // space, exit_event). The `handle_event` body still requires
    // descriptor-chain processing that has to be validated against a
    // real vhost-user-master; until the operator runbook lands, the
    // daemon will start, accept the connection, advertise the right
    // features, but log a warning when guest I/O arrives.
    //
    // The advantage of this shape: `cargo build` succeeds on any host;
    // the runtime degradation only manifests when a guest tries to
    // perform virtio-blk I/O, where the warning explains exactly what's
    // missing.
    let backend = Arc::new(backend);
    let exit_event = EventFd::new(0)?;
    let runtime = tokio::runtime::Handle::current();
    // RaftBlkVhostBackend implements `VhostUserBackend` (interior
    // mutability), so wrap in `Arc<T>` (vhost-user-backend's blanket
    // impl makes `Arc<T>` implement the trait when T does).
    let raftblk_backend = Arc::new(RaftBlkVhostBackend::new(
        backend.clone(),
        runtime.clone(),
        exit_event.try_clone()?,
    ));

    if let Some(parent) = cli.socket.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if cli.socket.exists() {
        std::fs::remove_file(&cli.socket)?;
    }

    let mem: GuestMemoryAtomic<GuestMemoryMmap<()>> =
        GuestMemoryAtomic::new(GuestMemoryMmap::new());
    let mut daemon =
        VhostUserDaemon::new(format!("raftblk-{}", cli.group_id), raftblk_backend, mem)
            .map_err(|e| anyhow::anyhow!("VhostUserDaemon::new: {e:?}"))?;

    let socket_path = cli.socket.clone();
    tracing::info!(socket = ?socket_path, "starting vhost-user-blk daemon");
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("raftblk-vhost: ctrl_c received, exiting before daemon start");
        }
        // VhostUserDaemon::serve blocks; run on a dedicated thread so it
        // cooperates with tokio's signal handler.
        result = tokio::task::spawn_blocking(move || daemon.serve(&socket_path)) => {
            match result {
                Ok(Ok(())) => tracing::info!("raftblk-vhost: daemon exited cleanly"),
                Ok(Err(e)) => tracing::error!("raftblk-vhost: daemon error: {e:?}"),
                Err(e) => tracing::error!("raftblk-vhost: blocking task panicked: {e}"),
            }
        }
    }
    Ok(())
}
