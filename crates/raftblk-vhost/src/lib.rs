//! Raft-replicated block backend for `vhost-user-blk`.
//!
//! This crate is the data plane that sits between a `vhost-user-backend`
//! daemon (the binary in `apps/raftblk-vhost`) and the agent's `RaftBlockState`
//! HTTP routes. It implements the *virtio-blk request translation* layer:
//! given a virtio-blk descriptor chain pulled off a virtqueue, dispatch
//! the appropriate read/write/flush against the Raft-replicated block group
//! and produce the matching status byte.
//!
//! Why a separate crate
//! --------------------
//! Three reasons:
//! 1. **Testability without rust-vmm.** Implementing the full vhost-user
//!    protocol requires kernel-level shared memory and a synthetic
//!    `vhost-user-master`. The translation layer here is plain Rust and is
//!    unit-testable in isolation, which is what proves B-II semantics — the
//!    actual vhost-user wiring is mechanical once the backend trait shape
//!    is stable.
//! 2. **Pluggable backends.** The `BlockBackend` trait abstracts away
//!    "where the bytes live". Today the only impl is `RaftBlockBackend`
//!    (HTTP -> agent -> Raft). Future impls (in-memory for tests, direct
//!    SPDK lvol bypass for non-replicated, NVMe-oF, etc.) drop in without
//!    touching the daemon.
//! 3. **Decoupled from the agent crate.** The daemon binary is a separate
//!    process from the agent (one daemon per attached VM disk). Sharing a
//!    library crate keeps the wire types in one place without forcing the
//!    agent to depend on rust-vmm crates.
//!
//! What's NOT here yet
//! -------------------
//! - The `vhost-user-backend` trait impl that turns `BlockBackend` into a
//!   live daemon. That's in the binary at `apps/raftblk-vhost` and is
//!   marked TODO until the real-microVM smoke runbook lands.
//! - SPDK-backed bytes. The Raft commit pipeline currently writes to the
//!   prototype JSON store on each replica; replacing that with an
//!   SPDK-lvol-backed store happens at the agent layer (see
//!   `RaftSpdkHostBackend::populate_streaming` for the wedge).

pub mod backend;
pub mod daemon;
pub mod request;

pub use backend::{BlockBackend, BlockBackendError, RaftBlockBackend, RaftBlockBackendConfig};
pub use daemon::RaftBlkVhostBackend;
pub use request::{BlockRequest, BlockRequestKind, BlockResponse, VirtioBlkStatus};

/// virtio-blk uses 512-byte logical sectors; this is the wire-level unit
/// for the `sector` field on virtio_blk_outhdr. Translating sector counts
/// to the Raft group's `block_size` is the responsibility of the dispatch
/// layer in `request.rs`.
pub const VIRTIO_BLK_SECTOR_SIZE: u64 = 512;
