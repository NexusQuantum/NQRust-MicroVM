# Storage iSCSI Implementation Plan (Plan 2 of 3)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Prove the storage abstraction by adding two concrete iSCSI backends (`iscsi` generic + `truenas_iscsi` TrueNAS REST), the agent host-side `IscsiHostBackend`, the agent kind-handshake, and the slow-path rootfs allocator (provision + agent populate_streaming + caller-side resize2fs). After this plan a TrueNAS LUN can back a VM.

**Architecture:** Manager-side control-plane backends call TrueNAS REST (or operate as a generic IQN holder). Agent-side host backend shells out to `iscsiadm` to log in/out and produces an `AttachedPath::BlockDevice` for `/dev/disk/by-path/...`. Manager↔agent storage RPC: a small new endpoint on the agent (`POST /v1/storage/attach`, `POST /v1/storage/populate`, `POST /v1/storage/detach`) that delegates to the agent's host backend registry. Slow-path rootfs allocation orchestrates: provision (manager) → attach (agent over RPC) → populate_streaming (agent over RPC) → caller-side `resize2fs` (manager triggers via the agent if needed). Single-attach is enforced cluster-wide via the partial unique index already in place.

**Tech Stack:** Existing Rust toolchain. `reqwest` for TrueNAS REST. `iscsiadm` shell-out (host has `open-iscsi` installed). All scheduling via existing manager→agent HTTP API.

**Spec:** `docs/superpowers/specs/2026-04-28-storage-hci-design.md`. Builds on Plan 1 (`feature/storage-foundation` branch with 34 commits).

---

## File structure

Manager (additions):
- `apps/manager/src/features/storage/backends/iscsi_generic.rs` — generic iSCSI control-plane (provisioner is no-op; assumes operator pre-creates LUNs and registers them via API).
- `apps/manager/src/features/storage/backends/truenas_iscsi.rs` — TrueNAS REST control-plane.
- `apps/manager/src/features/storage/agent_rpc.rs` — manager-side RPC helpers (`agent_attach`, `agent_populate`, `agent_detach`, `agent_supported_kinds`).
- `apps/manager/src/features/hosts/repo.rs` — extend with `set_supported_backend_kinds` + `get_supported_backend_kinds`.
- `apps/manager/migrations/0035_host_backend_kinds.sql` — `host` table gains `supported_backend_kinds JSONB` (default `["local_file"]`).

Manager (modifications):
- `apps/manager/Cargo.toml` — `serde_with`, `tokio-retry` if needed.
- `apps/manager/src/features/storage/registry.rs` — `build_backend` instantiates iscsi + truenas_iscsi (replacing the Plan 2 placeholder Err).
- `apps/manager/src/features/storage/rootfs_allocator.rs` — slow path implemented (provision + agent attach + agent populate + caller-side resize2fs over RPC) with a separate `try_resize_ext4` helper.
- `apps/manager/src/features/vms/service.rs` — schedule decision rejects hosts that don't support the backend kind required by the volume.

Agent (additions):
- `apps/agent/src/features/storage/iscsi.rs` — `IscsiHostBackend` (iscsiadm shell-out).
- `apps/agent/src/features/storage/registry.rs` — host-side registry (Map<BackendKind, Arc<dyn HostBackend>>).
- `apps/agent/src/features/storage/routes.rs` — HTTP routes `POST /v1/storage/attach|populate|detach|supported_kinds`.
- `apps/agent/src/features/inventory/mod.rs` — extend heartbeat to include `supported_backend_kinds`.

Agent (modifications):
- `apps/agent/Cargo.toml` — no new deps (`reqwest`, `tokio` already there).
- `apps/agent/src/features/mod.rs` — register storage routes router.
- `apps/agent/src/main.rs` — initialize host backend registry, pass to AppState.

Tests:
- `apps/manager/tests/storage_iscsi.rs` — `#[ignore]`-gated integration tests against a TrueNAS sim (mockito-based) or skipped when env unset.
- Inline unit tests in each backend module.

---

## Conventions

Same as Plan 1: Conventional Commits (`feat(storage):`, `fix(storage):`, `test(storage):`); `cargo fmt`/`cargo clippy --all-targets --all-features -- -D warnings` before review commits; do not break Plan 1 tests.

Wall-clock estimate: ~1.5–2 weeks of engineering work; ~12 implementation tasks.

---

## Task 1: Migration `0035_host_backend_kinds.sql`

**Files:**
- Create: `apps/manager/migrations/0035_host_backend_kinds.sql`

- [ ] **Step 1.1: Write the migration**

```sql
-- 0035_host_backend_kinds.sql
-- Per-host advertised list of HostBackend kinds. Manager refuses to schedule
-- a VM on a host whose kind set doesn't include the volume's backend kind.

ALTER TABLE host
  ADD COLUMN IF NOT EXISTS supported_backend_kinds JSONB NOT NULL
    DEFAULT '["local_file"]'::jsonb;

COMMENT ON COLUMN host.supported_backend_kinds IS
  'JSON array of BackendKind db strings (e.g. ["local_file","iscsi"]) the agent advertises support for. Updated on agent registration / heartbeat.';
```

- [ ] **Step 1.2: Apply** (only if DB available): `(cd apps/manager && sqlx migrate run)`. Verify: `psql "$DATABASE_URL" -c "\d host"` shows the new column.

- [ ] **Step 1.3: Commit**

```bash
git add apps/manager/migrations/0035_host_backend_kinds.sql
git commit -m "feat(storage): migration 0035 — host.supported_backend_kinds"
```

---

## Task 2: Agent host-backend registry + LocalFile registration

**Files:**
- Create: `apps/agent/src/features/storage/registry.rs`
- Modify: `apps/agent/src/features/storage/mod.rs`
- Modify: `apps/agent/src/main.rs`

- [ ] **Step 2.1: Write the registry**

```rust
// apps/agent/src/features/storage/registry.rs
use nexus_storage::{BackendKind, HostBackend};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct HostBackendRegistry {
    by_kind: HashMap<BackendKind, Arc<dyn HostBackend>>,
}

impl HostBackendRegistry {
    pub fn empty() -> Self { Self { by_kind: HashMap::new() } }

    pub fn register(&mut self, backend: Arc<dyn HostBackend>) {
        self.by_kind.insert(backend.kind(), backend);
    }

    pub fn get(&self, kind: BackendKind) -> Option<&Arc<dyn HostBackend>> {
        self.by_kind.get(&kind)
    }

    pub fn supported_kinds(&self) -> Vec<BackendKind> {
        let mut v: Vec<_> = self.by_kind.keys().copied().collect();
        v.sort_by_key(|k| k.as_db_str());
        v
    }
}
```

- [ ] **Step 2.2: Wire into AppState in agent's main.rs**

Find the agent's `AppState` (or equivalent — check `apps/agent/src/main.rs` for the existing state struct or AppState). Add `pub storage_registry: HostBackendRegistry`. Build it before constructing AppState:

```rust
let mut storage_registry = features::storage::registry::HostBackendRegistry::empty();
storage_registry.register(std::sync::Arc::new(features::storage::local_file::LocalFileHostBackend));
```

- [ ] **Step 2.3: Update mod.rs**

Append `pub mod registry;` to `apps/agent/src/features/storage/mod.rs`.

- [ ] **Step 2.4: Verify** `cargo check -p agent && cargo clippy -p agent --all-targets -- -D warnings` clean.

- [ ] **Step 2.5: Commit**

```bash
git add apps/agent/src/features/storage/registry.rs apps/agent/src/features/storage/mod.rs apps/agent/src/main.rs
git commit -m "feat(storage): agent HostBackendRegistry + LocalFile registration"
```

---

## Task 3: Agent storage HTTP routes (attach/populate/detach/supported_kinds)

**Files:**
- Create: `apps/agent/src/features/storage/routes.rs`
- Modify: `apps/agent/src/features/storage/mod.rs`
- Modify: `apps/agent/src/features/mod.rs` — register router

- [ ] **Step 3.1: Routes**

```rust
// apps/agent/src/features/storage/routes.rs
use crate::features::storage::registry::HostBackendRegistry;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json, Router, routing::post, routing::get};
use nexus_storage::{AttachedPath, BackendKind, VolumeHandle};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone)]
pub struct StorageState {
    pub registry: HostBackendRegistry,
}

#[derive(Deserialize)]
pub struct AttachReq { pub volume: VolumeHandle }
#[derive(Serialize)]
pub struct AttachResp { pub attached: AttachedPath }

#[derive(Deserialize)]
pub struct DetachReq { pub volume: VolumeHandle, pub attached: AttachedPath }

#[derive(Deserialize)]
pub struct PopulateReq {
    pub attached: AttachedPath,
    pub source_path: PathBuf,
    pub target_size_bytes: u64,
}

pub async fn attach(State(s): State<Arc<StorageState>>, Json(req): Json<AttachReq>) -> impl IntoResponse {
    let backend = match s.registry.get(req.volume.backend_kind) {
        Some(b) => b,
        None => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error":"unsupported backend kind"}))).into_response(),
    };
    match backend.attach(&req.volume).await {
        Ok(attached) => (StatusCode::OK, Json(AttachResp { attached })).into_response(),
        Err(e) => {
            tracing::error!("attach failed: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response()
        }
    }
}

pub async fn detach(State(s): State<Arc<StorageState>>, Json(req): Json<DetachReq>) -> impl IntoResponse {
    let backend = match s.registry.get(req.volume.backend_kind) {
        Some(b) => b,
        None => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error":"unsupported backend kind"}))).into_response(),
    };
    match backend.detach(&req.volume, req.attached).await {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

pub async fn populate(State(s): State<Arc<StorageState>>, Json(req): Json<PopulateReq>) -> impl IntoResponse {
    let kind = match &req.attached {
        AttachedPath::File(_) => BackendKind::LocalFile,
        AttachedPath::BlockDevice(_) => BackendKind::Iscsi,
        AttachedPath::VhostUserSock(_) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error":"vhost-user-sock not supported"}))).into_response(),
    };
    let backend = match s.registry.get(kind) {
        Some(b) => b,
        None => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error":"unsupported kind"}))).into_response(),
    };
    match backend.populate_streaming(&req.attached, &req.source_path, req.target_size_bytes).await {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

pub async fn supported_kinds(State(s): State<Arc<StorageState>>) -> impl IntoResponse {
    let kinds: Vec<&'static str> = s.registry.supported_kinds().iter().map(|k| k.as_db_str()).collect();
    (StatusCode::OK, Json(serde_json::json!({"kinds": kinds}))).into_response()
}

pub fn router(state: Arc<StorageState>) -> Router {
    Router::new()
        .route("/attach", post(attach))
        .route("/detach", post(detach))
        .route("/populate", post(populate))
        .route("/supported_kinds", get(supported_kinds))
        .with_state(state)
}
```

NOTE: `populate`'s mapping from `AttachedPath` variant to `BackendKind` is heuristic (BlockDevice == Iscsi). When more block-device-backed kinds exist, plumb the kind explicitly through the request.

- [ ] **Step 3.2: Mount the router**

In `apps/agent/src/features/mod.rs`, add `pub mod storage;` (already added in Plan 1 T16 — verify) and a `.nest("/v1/storage", storage::routes::router(...))` call when the agent's main router is built. The state should come from agent's AppState.

- [ ] **Step 3.3: Verify** `cargo check -p agent && cargo clippy -p agent --all-targets -- -D warnings` clean.

- [ ] **Step 3.4: Commit**

```bash
git add apps/agent/src/features/storage/ apps/agent/src/features/mod.rs apps/agent/src/main.rs
git commit -m "feat(storage): agent HTTP routes /v1/storage/{attach,populate,detach,supported_kinds}"
```

---

## Task 4: Agent reports supported kinds at registration

**Files:**
- Modify: `apps/agent/src/features/inventory/mod.rs` (or wherever the agent's heartbeat/register payload is built)
- Modify: `apps/manager/src/features/hosts/{repo,routes}.rs` to receive + persist

- [ ] **Step 4.1: Agent side — include kinds in registration**

Find the existing register/heartbeat payload in the agent. Add a `supported_backend_kinds: Vec<&'static str>` field. Populate from `state.storage_registry.supported_kinds()`.

- [ ] **Step 4.2: Manager side — accept + persist**

Find the manager's host register/heartbeat handler. Add an optional `supported_backend_kinds: Option<Vec<String>>` to the request body. On register/heartbeat, write to `host.supported_backend_kinds`:

```rust
sqlx::query("UPDATE host SET supported_backend_kinds = $1 WHERE id = $2")
    .bind(serde_json::Value::from(req.supported_backend_kinds.unwrap_or_else(|| vec!["local_file".into()])))
    .bind(host_id)
    .execute(&state.db).await?;
```

Add a method to `HostRepository`:
```rust
pub async fn supported_backend_kinds(&self, host_id: Uuid) -> sqlx::Result<Vec<String>> {
    let v: serde_json::Value = sqlx::query_scalar(
        r#"SELECT supported_backend_kinds FROM host WHERE id = $1"#
    ).bind(host_id).fetch_one(&self.pool).await?;
    Ok(v.as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default())
}
```

- [ ] **Step 4.3: Verify** `cargo check + clippy` clean.

- [ ] **Step 4.4: Commit**

```bash
git add apps/agent/src/features/inventory/ apps/manager/src/features/hosts/
git commit -m "feat(storage): agent reports supported_backend_kinds; manager persists"
```

---

## Task 5: Manager-side agent RPC helpers

**Files:**
- Create: `apps/manager/src/features/storage/agent_rpc.rs`
- Modify: `apps/manager/src/features/storage/mod.rs`

- [ ] **Step 5.1: Helpers**

```rust
// apps/manager/src/features/storage/agent_rpc.rs
use anyhow::{anyhow, Context, Result};
use nexus_storage::{AttachedPath, VolumeHandle};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

fn agent_url(host_addr: &str, path: &str) -> String {
    let base = if host_addr.starts_with("http") {
        host_addr.to_string()
    } else {
        format!("http://{host_addr}")
    };
    format!("{base}{path}")
}

#[derive(Serialize)]
struct AttachReq<'a> { volume: &'a VolumeHandle }
#[derive(Deserialize)]
struct AttachResp { attached: AttachedPath }

pub async fn agent_attach(host_addr: &str, volume: &VolumeHandle) -> Result<AttachedPath> {
    let resp = Client::new().post(agent_url(host_addr, "/v1/storage/attach"))
        .json(&AttachReq { volume }).send().await
        .with_context(|| format!("POST /v1/storage/attach to {host_addr}"))?;
    if !resp.status().is_success() { return Err(anyhow!("agent attach: {}", resp.status())); }
    Ok(resp.json::<AttachResp>().await?.attached)
}

#[derive(Serialize)]
struct DetachReq<'a> { volume: &'a VolumeHandle, attached: &'a AttachedPath }

pub async fn agent_detach(host_addr: &str, volume: &VolumeHandle, attached: &AttachedPath) -> Result<()> {
    let resp = Client::new().post(agent_url(host_addr, "/v1/storage/detach"))
        .json(&DetachReq { volume, attached }).send().await
        .with_context(|| format!("POST /v1/storage/detach to {host_addr}"))?;
    if !resp.status().is_success() { return Err(anyhow!("agent detach: {}", resp.status())); }
    Ok(())
}

#[derive(Serialize)]
struct PopulateReq<'a> {
    attached: &'a AttachedPath,
    source_path: &'a PathBuf,
    target_size_bytes: u64,
}

pub async fn agent_populate(
    host_addr: &str,
    attached: &AttachedPath,
    source_path: &PathBuf,
    target_size_bytes: u64,
) -> Result<()> {
    let resp = Client::new().post(agent_url(host_addr, "/v1/storage/populate"))
        .json(&PopulateReq { attached, source_path, target_size_bytes })
        .send().await
        .with_context(|| format!("POST /v1/storage/populate to {host_addr}"))?;
    if !resp.status().is_success() {
        return Err(anyhow!("agent populate: {}", resp.status()));
    }
    Ok(())
}
```

- [ ] **Step 5.2: Register module** — append `pub mod agent_rpc;` to `apps/manager/src/features/storage/mod.rs`.

- [ ] **Step 5.3: Verify + commit**

```bash
git add apps/manager/src/features/storage/agent_rpc.rs apps/manager/src/features/storage/mod.rs
git commit -m "feat(storage): manager-side agent RPC helpers (attach, populate, detach)"
```

---

## Task 6: Slow-path rootfs allocator

**Files:**
- Modify: `apps/manager/src/features/storage/rootfs_allocator.rs`

- [ ] **Step 6.1: Implement the slow path**

Replace the `Err(...)` slow-path stub with a real implementation. Add a parameter `host_addr: &str` to `allocate_rootfs`:

```rust
pub async fn allocate_rootfs(
    registry: &Registry,
    backend_id: Uuid,
    host_addr: &str,
    source_image: &Path,
    target_size_bytes: u64,
    opts_name: &str,
) -> Result<AllocOutcome> {
    let backend = registry.get(backend_id).ok_or_else(|| anyhow!("no backend with id {backend_id}"))?;
    let opts = CreateOpts { name: opts_name.into(), size_bytes: target_size_bytes, description: None };

    if backend.capabilities().supports_clone_from_image {
        let h = backend.clone_from_image(source_image, opts).await
            .with_context(|| format!("clone_from_image failed on backend {backend_id}"))?;
        return Ok(AllocOutcome { volume_handle: h, attached_for_caller: None });
    }

    // Slow path: provision empty, attach via agent, populate via agent, optional resize.
    let h = backend.provision(opts).await.context("provision")?;
    let attached = crate::features::storage::agent_rpc::agent_attach(host_addr, &h).await
        .context("agent attach")?;
    crate::features::storage::agent_rpc::agent_populate(
        host_addr, &attached, &source_image.to_path_buf(), target_size_bytes,
    ).await.context("agent populate_streaming")?;

    // Caller-side resize2fs for ext4 rootfs images.
    if image_is_ext4_rootfs(source_image).await? {
        crate::features::storage::agent_rpc::agent_resize2fs(host_addr, &attached).await.ok();
        // Best-effort; non-fatal on backends where it doesn't apply.
    }

    Ok(AllocOutcome { volume_handle: h, attached_for_caller: Some(attached) })
}
```

Add `attached_for_caller: Option<AttachedPath>` to `AllocOutcome` (this addresses the I3 follow-up from Plan 1 review).

Add `image_is_ext4_rootfs`:
```rust
async fn image_is_ext4_rootfs(path: &Path) -> Result<bool> {
    let mut f = tokio::fs::File::open(path).await?;
    use tokio::io::AsyncReadExt;
    let mut sb = [0u8; 1024 + 8]; // ext4 superblock magic at offset 1024+0x38
    if f.read_exact(&mut sb).await.is_err() { return Ok(false); }
    // ext4 magic is 0xEF53 at offset 1024+0x38 = 1080.
    // We read 1032 bytes; that's not enough. Use a larger buffer.
    Ok(false) // refine in Task 7
}
```

(NOTE: leave `image_is_ext4_rootfs` as `Ok(false)` for now — extending into a real superblock check is Task 7. The slow path still works without filesystem-aware resize because most images we ship are already sized correctly.)

Add `agent_resize2fs` to `agent_rpc.rs` — a small POST to a new agent route `/v1/storage/resize2fs`. Implement it as best-effort:

```rust
pub async fn agent_resize2fs(host_addr: &str, attached: &AttachedPath) -> Result<()> {
    let resp = Client::new().post(agent_url(host_addr, "/v1/storage/resize2fs"))
        .json(&serde_json::json!({"attached": attached})).send().await?;
    if !resp.status().is_success() { return Err(anyhow!("resize2fs: {}", resp.status())); }
    Ok(())
}
```

In agent's `routes.rs`, add the resize2fs handler:
```rust
pub async fn resize2fs(Json(req): Json<serde_json::Value>) -> impl IntoResponse {
    let attached: AttachedPath = match serde_json::from_value(req["attached"].clone()) {
        Ok(a) => a,
        Err(_) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error":"bad attached"}))).into_response(),
    };
    let path = attached.path();
    let _fsck = tokio::process::Command::new("e2fsck").args(["-f", "-y"]).arg(path).output().await.ok();
    let resize = tokio::process::Command::new("resize2fs").arg(path).output().await;
    match resize {
        Ok(o) if o.status.success() => (StatusCode::OK, Json(serde_json::json!({}))).into_response(),
        Ok(o) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"stderr": String::from_utf8_lossy(&o.stderr).to_string()}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}
```

Update the router in `routes.rs` to include `.route("/resize2fs", post(resize2fs))`.

Update `vms/service.rs` to pass `host_addr` to `allocate_rootfs`. The `host_addr` is in `host` (the selected host) — pass `&host.addr`.

- [ ] **Step 6.2: Verify + commit**

```bash
cargo check + clippy
git add apps/manager/src/features/storage/ apps/agent/src/features/storage/
git commit -m "feat(storage): slow-path rootfs (provision + agent attach + agent populate + caller-side resize)"
```

---

## Task 7: Refine `image_is_ext4_rootfs`

**Files:** `apps/manager/src/features/storage/rootfs_allocator.rs`

- [ ] **Step 7.1: Read ext4 superblock**

The ext4 superblock magic `0xEF53` lives at offset 1080 (=1024+0x38) of any ext4 filesystem image. Re-implement:

```rust
async fn image_is_ext4_rootfs(path: &Path) -> Result<bool> {
    use tokio::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};
    let mut f = tokio::fs::File::open(path).await?;
    if f.seek(SeekFrom::Start(1080)).await.is_err() { return Ok(false); }
    let mut buf = [0u8; 2];
    if f.read_exact(&mut buf).await.is_err() { return Ok(false); }
    // little-endian u16
    Ok(buf[0] == 0x53 && buf[1] == 0xEF)
}
```

- [ ] **Step 7.2: Test**

Add a unit test: create a small dummy ext4 image (write magic at offset 1080), assert `true`. Create a dummy non-ext4 file, assert `false`.

- [ ] **Step 7.3: Commit**

```
git commit -m "feat(storage): detect ext4 rootfs by superblock magic"
```

---

## Task 8: `IscsiGenericControlPlaneBackend` — manager-side

**Files:**
- Create: `apps/manager/src/features/storage/backends/iscsi_generic.rs`
- Modify: `apps/manager/src/features/storage/backends/mod.rs`

- [ ] **Step 8.1: Implementation**

```rust
// Minimal generic iSCSI backend. Provisioning is no-op (operator must
// pre-create the LUN on the target). The volume's `path`/`locator` field
// stores the IQN+LUN as: "iqn.2024-01.com.example:tgt|lun=3"
//
// `clone_from_image` is unsupported (capability false). The slow path in
// rootfs_allocator handles populate via the agent.

use nexus_storage::{
    BackendInstanceId, BackendKind, Capabilities, ControlPlaneBackend, CreateOpts, StorageError,
    VolumeHandle, VolumeSnapshotHandle,
};
use serde::Deserialize;
use std::path::Path;
use uuid::Uuid;

#[derive(Deserialize, Clone)]
pub struct IscsiGenericConfig {
    pub target_iqn: String,
}

pub struct IscsiGenericControlPlaneBackend {
    pub id: BackendInstanceId,
    pub config: IscsiGenericConfig,
}

#[async_trait::async_trait]
impl ControlPlaneBackend for IscsiGenericControlPlaneBackend {
    fn kind(&self) -> BackendKind { BackendKind::Iscsi }
    fn capabilities(&self) -> Capabilities {
        Capabilities { ..Default::default() } // all false
    }
    async fn provision(&self, _opts: CreateOpts) -> Result<VolumeHandle, StorageError> {
        Err(StorageError::NotSupported(
            "generic iscsi: operator must pre-create LUNs and register them via API".into()
        ))
    }
    async fn destroy(&self, _h: VolumeHandle) -> Result<(), StorageError> { Ok(()) }
    async fn clone_from_image(&self, _: &Path, _: CreateOpts) -> Result<VolumeHandle, StorageError> {
        Err(StorageError::NotSupported("clone_from_image".into()))
    }
    async fn snapshot(&self, _: &VolumeHandle, _: &str) -> Result<VolumeSnapshotHandle, StorageError> {
        Err(StorageError::NotSupported("snapshot".into()))
    }
    async fn clone_from_snapshot(&self, _: &VolumeSnapshotHandle) -> Result<VolumeHandle, StorageError> {
        Err(StorageError::NotSupported("clone_from_snapshot".into()))
    }
    async fn delete_snapshot(&self, _: VolumeSnapshotHandle) -> Result<(), StorageError> { Ok(()) }
}
```

(Generic iSCSI is intentionally minimal: it documents the wire format for the locator and otherwise refuses to act. Real provisioning lives in TrueNasIscsiControlPlaneBackend.)

- [ ] **Step 8.2: Register in `mod.rs`** — `pub mod iscsi_generic;`.

- [ ] **Step 8.3: Verify + commit** — clean. `feat(storage): IscsiGenericControlPlaneBackend (no-op provisioner)`.

---

## Task 9: `TrueNasIscsiControlPlaneBackend` — REST provisioning

**Files:**
- Create: `apps/manager/src/features/storage/backends/truenas_iscsi.rs`
- Modify: `apps/manager/src/features/storage/backends/mod.rs`

- [ ] **Step 9.1: Implementation skeleton**

```rust
use nexus_storage::{
    BackendInstanceId, BackendKind, Capabilities, ControlPlaneBackend, CreateOpts, StorageError,
    VolumeHandle, VolumeSnapshotHandle,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

#[derive(Deserialize, Clone)]
pub struct TrueNasConfig {
    pub endpoint: String,
    pub api_key_env: String,
    pub pool: String,
    pub target_iqn_prefix: String,
}

pub struct TrueNasIscsiControlPlaneBackend {
    pub id: BackendInstanceId,
    pub config: TrueNasConfig,
    pub api_key: String,  // resolved from env at registry build time
    pub http: Client,
}

impl TrueNasIscsiControlPlaneBackend {
    fn auth_header(&self) -> String { format!("Bearer {}", self.api_key) }

    async fn create_zvol(&self, name: &str, size_bytes: u64) -> Result<String, StorageError> {
        // POST /api/v2.0/pool/dataset
        #[derive(Serialize)] struct Req { name: String, r#type: &'static str, volsize: u64, sparse: bool }
        let url = format!("{}/api/v2.0/pool/dataset", self.config.endpoint);
        let body = Req {
            name: format!("{}/{}", self.config.pool, name),
            r#type: "VOLUME",
            volsize: size_bytes,
            sparse: true,
        };
        let resp = self.http.post(&url).header("Authorization", self.auth_header())
            .json(&body).send().await
            .map_err(|e| StorageError::Backend(Box::new(e)))?;
        if !resp.status().is_success() {
            let s = resp.status();
            let txt = resp.text().await.unwrap_or_default();
            return Err(StorageError::Backend(format!("create_zvol {}: {}", s, txt).into()));
        }
        Ok(format!("{}/{}", self.config.pool, name))
    }

    async fn delete_zvol(&self, dataset: &str) -> Result<(), StorageError> {
        let url = format!("{}/api/v2.0/pool/dataset/id/{}",
            self.config.endpoint, urlencoding::encode(dataset));
        let resp = self.http.delete(&url).header("Authorization", self.auth_header())
            .send().await.map_err(|e| StorageError::Backend(Box::new(e)))?;
        if !resp.status().is_success() && resp.status() != reqwest::StatusCode::NOT_FOUND {
            return Err(StorageError::Backend(format!("delete_zvol: {}", resp.status()).into()));
        }
        Ok(())
    }

    async fn create_lun_extent(&self, dataset: &str) -> Result<u32, StorageError> {
        // POST /api/v2.0/iscsi/extent { name, type:"DISK", disk: dataset }
        // Returns extent id; combine with target via /iscsi/targetextent.
        // (Detailed TrueNAS API; consult TrueNAS docs for exact shape.)
        // Return the LUN number assigned by TrueNAS.
        // For brevity, this method is partially elided in the plan; see TrueNAS REST docs.
        Ok(0) // placeholder; real impl required in this task
    }
}

#[async_trait::async_trait]
impl ControlPlaneBackend for TrueNasIscsiControlPlaneBackend {
    fn kind(&self) -> BackendKind { BackendKind::TrueNasIscsi }
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            supports_native_snapshots: true,
            supports_clone_from_image: false,
            ..Default::default()
        }
    }

    async fn provision(&self, opts: CreateOpts) -> Result<VolumeHandle, StorageError> {
        let vol_id = Uuid::new_v4();
        let zvol_name = format!("v-{vol_id}");
        let dataset = self.create_zvol(&zvol_name, opts.size_bytes).await?;
        let lun = self.create_lun_extent(&dataset).await?;
        let locator = format!("{}|lun={}", self.config.target_iqn_prefix, lun);
        Ok(VolumeHandle {
            volume_id: vol_id,
            backend_id: self.id,
            backend_kind: BackendKind::TrueNasIscsi,
            locator,
            size_bytes: opts.size_bytes,
        })
    }

    async fn destroy(&self, handle: VolumeHandle) -> Result<(), StorageError> {
        // Reverse: delete extent + delete zvol. Locator parsing required.
        // For brevity: extract dataset from locator (need a name→dataset map or store it on the handle).
        // Strategy: stash the dataset name in handle.locator alongside lun number, e.g., "iqn|lun=3|ds=tank/v-uuid".
        Ok(()) // implement properly during this task
    }

    async fn clone_from_image(&self, _: &Path, _: CreateOpts) -> Result<VolumeHandle, StorageError> {
        Err(StorageError::NotSupported("clone_from_image".into()))
    }

    async fn snapshot(&self, volume: &VolumeHandle, name: &str) -> Result<VolumeSnapshotHandle, StorageError> {
        // POST /api/v2.0/zfs/snapshot { dataset, name }
        // Implementation per TrueNAS API.
        Err(StorageError::NotSupported("snapshot impl pending".into()))
    }

    async fn clone_from_snapshot(&self, _: &VolumeSnapshotHandle) -> Result<VolumeHandle, StorageError> {
        Err(StorageError::NotSupported("clone_from_snapshot impl pending".into()))
    }

    async fn delete_snapshot(&self, _: VolumeSnapshotHandle) -> Result<(), StorageError> {
        Ok(())
    }
}
```

NOTE: The snapshot/clone_from_snapshot methods are SHIPPED AS NotSupported in this task to scope the work. A follow-up task can add full snapshot support; the trait still exposes the capability `supports_native_snapshots: true` so future code can invoke them once implemented. Minimum-viable is provision + destroy of a zvol+extent.

- [ ] **Step 9.2: Locator scheme decision**

For `destroy`/`attach` to work cleanly, the locator must carry: (a) target IQN, (b) LUN number, (c) dataset name (for destroy). Use a JSON-encoded string:
```
{"iqn":"iqn.2024-01.com.example:tgt","lun":3,"dataset":"tank/v-uuid"}
```
Encode/decode at the backend boundary. Keep the on-disk `volume.path` as this JSON string (the unique constraint on path still works because each volume has a unique JSON).

- [ ] **Step 9.3: Verify + commit**

```bash
git add apps/manager/src/features/storage/backends/
git commit -m "feat(storage): TrueNasIscsiControlPlaneBackend (provision via REST)"
```

---

## Task 10: `IscsiHostBackend` — agent-side

**Files:**
- Create: `apps/agent/src/features/storage/iscsi.rs`
- Modify: `apps/agent/src/features/storage/mod.rs`
- Modify: `apps/agent/src/main.rs` to register

- [ ] **Step 10.1: Implementation**

```rust
use nexus_storage::{AttachedPath, BackendKind, HostBackend, StorageError, VolumeHandle};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Deserialize, Debug)]
struct LocatorJson {
    iqn: String,
    lun: u32,
    #[serde(default)]
    portal: Option<String>, // optional override; falls back to iqn-derived discovery
}

pub struct IscsiHostBackend;

impl IscsiHostBackend {
    fn parse_locator(s: &str) -> Result<LocatorJson, StorageError> {
        serde_json::from_str(s)
            .map_err(|e| StorageError::InvalidLocator(format!("{s}: {e}")))
    }

    async fn iscsiadm_login(loc: &LocatorJson) -> Result<(), StorageError> {
        // discovery (idempotent)
        let portal = loc.portal.clone().unwrap_or_else(|| {
            // best-effort: try the IQN's hinted portal; default to localhost
            "127.0.0.1".to_string()
        });
        let _ = tokio::process::Command::new("iscsiadm")
            .args(["-m", "discovery", "-t", "sendtargets", "-p", &portal])
            .output().await
            .map_err(|e| StorageError::Backend(Box::new(e)))?;
        let out = tokio::process::Command::new("iscsiadm")
            .args(["-m", "node", "-T", &loc.iqn, "-p", &portal, "--login"])
            .output().await
            .map_err(|e| StorageError::Backend(Box::new(e)))?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            // "already logged in" is fine
            if !stderr.contains("already") {
                return Err(StorageError::Backend(format!("iscsiadm login: {stderr}").into()));
            }
        }
        Ok(())
    }

    async fn iscsiadm_logout(loc: &LocatorJson) -> Result<(), StorageError> {
        let portal = loc.portal.clone().unwrap_or_else(|| "127.0.0.1".to_string());
        let _ = tokio::process::Command::new("iscsiadm")
            .args(["-m", "node", "-T", &loc.iqn, "-p", &portal, "--logout"])
            .output().await;
        Ok(()) // best-effort; aggressive logout per spec recommendation
    }

    fn block_device_path(loc: &LocatorJson) -> PathBuf {
        // udev creates by-path symlinks: /dev/disk/by-path/ip-<portal>:3260-iscsi-<iqn>-lun-<n>
        let portal = loc.portal.clone().unwrap_or_else(|| "127.0.0.1".to_string());
        PathBuf::from(format!(
            "/dev/disk/by-path/ip-{portal}:3260-iscsi-{}-lun-{}",
            loc.iqn, loc.lun
        ))
    }
}

#[async_trait::async_trait]
impl HostBackend for IscsiHostBackend {
    fn kind(&self) -> BackendKind { BackendKind::Iscsi }

    async fn attach(&self, volume: &VolumeHandle) -> Result<AttachedPath, StorageError> {
        let loc = Self::parse_locator(&volume.locator)?;
        Self::iscsiadm_login(&loc).await?;
        // Wait briefly for udev to create the by-path symlink
        let dev = Self::block_device_path(&loc);
        for _ in 0..30 {
            if dev.exists() { return Ok(AttachedPath::BlockDevice(dev)); }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        Err(StorageError::Backend(format!("device {} did not appear after login", dev.display()).into()))
    }

    async fn detach(&self, volume: &VolumeHandle, _attached: AttachedPath) -> Result<(), StorageError> {
        let loc = Self::parse_locator(&volume.locator)?;
        Self::iscsiadm_logout(&loc).await
    }

    async fn populate_streaming(
        &self,
        attached: &AttachedPath,
        source: &Path,
        target_size_bytes: u64,
    ) -> Result<(), StorageError> {
        use tokio::io::AsyncWriteExt;
        let dst_path = attached.path();
        let mut src = tokio::fs::File::open(source).await?;
        // OPEN block device with O_DIRECT not used; rely on kernel buffering for now.
        let mut dst = tokio::fs::OpenOptions::new()
            .write(true).open(dst_path).await?;
        tokio::io::copy(&mut src, &mut dst).await?;
        // For block devices set_len is meaningless but sparse extension is also meaningless.
        // The block device size is fixed by the LUN. target_size_bytes is informational here.
        let _ = target_size_bytes;
        dst.flush().await?;
        Ok(())
    }
}
```

(Note: real-world iSCSI populate has subtleties — the source bytes are typically `dd`-style copied; using `tokio::io::copy` works but won't be the fastest. Optimization for later.)

NOTE: The `IscsiHostBackend` registers itself for `BackendKind::Iscsi`. For `TrueNasIscsi` volumes, the host-side path is identical (it's still iSCSI from the kernel's perspective). Either reuse `IscsiHostBackend` for both kinds (kind() returns Iscsi but it's also called for TrueNasIscsi) OR add a thin wrapper. Cleanest: register the SAME `Arc<IscsiHostBackend>` under both `BackendKind::Iscsi` AND `BackendKind::TrueNasIscsi` in the agent registry. Update `HostBackendRegistry::register` to take an explicit `kind` parameter:

```rust
pub fn register_for(&mut self, kind: BackendKind, backend: Arc<dyn HostBackend>) {
    self.by_kind.insert(kind, backend);
}
```

Then in `main.rs`:
```rust
let iscsi_host = Arc::new(IscsiHostBackend);
storage_registry.register_for(BackendKind::Iscsi, iscsi_host.clone());
storage_registry.register_for(BackendKind::TrueNasIscsi, iscsi_host);
```

`HostBackend::kind` becomes mainly informational; the registry's lookup-by-kind controls which methods route where. (Adjust the supported_kinds() to dedupe — return `[Iscsi, TrueNasIscsi]` since both are advertised.)

- [ ] **Step 10.2: Verify + commit**

```bash
git add apps/agent/src/features/storage/ apps/agent/src/main.rs
git commit -m "feat(storage): IscsiHostBackend (iscsiadm + by-path device discovery)"
```

---

## Task 11: Registry recognizes iscsi/truenas_iscsi

**Files:**
- Modify: `apps/manager/src/features/storage/registry.rs`

- [ ] **Step 11.1: Update `build_backend`**

Replace the `Iscsi`/`TrueNasIscsi` branches that currently return `Err`:

```rust
match kind {
    BackendKind::LocalFile => Ok(Arc::new(LocalFileControlPlaneBackend {
        id: BackendInstanceId(row.id),
    })),
    BackendKind::Iscsi => {
        let cfg: backends::iscsi_generic::IscsiGenericConfig =
            serde_json::from_value(row.config_json.clone())
                .with_context(|| format!("backend '{}' iscsi config", row.name))?;
        Ok(Arc::new(backends::iscsi_generic::IscsiGenericControlPlaneBackend {
            id: BackendInstanceId(row.id),
            config: cfg,
        }))
    }
    BackendKind::TrueNasIscsi => {
        let cfg: backends::truenas_iscsi::TrueNasConfig =
            serde_json::from_value(row.config_json.clone())
                .with_context(|| format!("backend '{}' truenas_iscsi config", row.name))?;
        let api_key = std::env::var(&cfg.api_key_env)
            .with_context(|| format!("env var {} not set for backend '{}'", cfg.api_key_env, row.name))?;
        Ok(Arc::new(backends::truenas_iscsi::TrueNasIscsiControlPlaneBackend {
            id: BackendInstanceId(row.id),
            config: cfg,
            api_key,
            http: reqwest::Client::new(),
        }))
    }
}
```

- [ ] **Step 11.2: Update mod.rs** — append `pub mod iscsi_generic;` and `pub mod truenas_iscsi;` to `apps/manager/src/features/storage/backends/mod.rs`.

- [ ] **Step 11.3: Verify + commit**

```bash
cargo check + clippy
git commit -m "feat(storage): registry instantiates Iscsi + TrueNasIscsi backends"
```

---

## Task 12: VM scheduler enforces host backend-kind compatibility

**Files:**
- Modify: `apps/manager/src/features/vms/service.rs`

- [ ] **Step 12.1: Filter hosts**

Where the manager picks a host (something like `state.hosts.first_healthy()` or a scheduler call), filter to hosts whose `supported_backend_kinds` includes the volume's backend kind.

```rust
// in resolve_vm_spec or wherever host is chosen:
let backend_kind_str = state.registry.get(backend_id)
    .map(|b| b.kind().as_db_str())
    .unwrap_or("local_file");

let candidate_hosts = state.hosts.list_healthy().await?
    .into_iter()
    .filter(|h| h.supported_backend_kinds.iter().any(|k| k == backend_kind_str))
    .collect::<Vec<_>>();
let host = candidate_hosts.first().ok_or_else(|| {
    anyhow!("no host supports backend kind '{backend_kind_str}'")
})?;
```

(Adjust to your existing scheduler shape. If `list_healthy` doesn't exist, modify the existing host-selection to add the filter inline.)

- [ ] **Step 12.2: HostRow extension**

Update `HostRepository` to populate `supported_backend_kinds` on `HostRow`. Read from the JSONB column.

- [ ] **Step 12.3: Verify + commit**

```bash
cargo check + clippy
git commit -m "feat(storage): scheduler refuses incompatible backend/host combinations"
```

---

## Task 13: Tests — TrueNAS REST mock + iscsi locator parsing

**Files:**
- Create: `apps/manager/tests/storage_iscsi.rs`

- [ ] **Step 13.1: Locator parsing test**

Inline test in `apps/agent/src/features/storage/iscsi.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_locator_json() {
        let s = r#"{"iqn":"iqn.x:tgt","lun":3,"portal":"10.0.0.5"}"#;
        let loc = IscsiHostBackend::parse_locator(s).unwrap();
        assert_eq!(loc.lun, 3);
        assert_eq!(loc.portal.as_deref(), Some("10.0.0.5"));
    }
    #[test]
    fn rejects_malformed_locator() {
        let err = IscsiHostBackend::parse_locator("not json").unwrap_err();
        assert!(matches!(err, StorageError::InvalidLocator(_)));
    }
}
```

- [ ] **Step 13.2: TrueNAS REST mock test (best-effort)**

Use `mockito` to stand up a fake TrueNAS endpoint and verify `provision` POSTs to the right URL. This is one of those tests that requires a small lift; add `mockito = "1"` to `apps/manager/Cargo.toml` `[dev-dependencies]`.

```rust
#[tokio::test]
async fn truenas_provision_calls_create_zvol() {
    let mut server = mockito::Server::new_async().await;
    let _m = server.mock("POST", "/api/v2.0/pool/dataset")
        .with_status(200)
        .with_body(r#"{"id":"tank/v-x","volsize":1048576}"#)
        .create_async().await;

    let backend = TrueNasIscsiControlPlaneBackend {
        id: BackendInstanceId(uuid::Uuid::new_v4()),
        config: TrueNasConfig {
            endpoint: server.url(),
            api_key_env: "_unused_".into(),
            pool: "tank".into(),
            target_iqn_prefix: "iqn.x:tgt".into(),
        },
        api_key: "test".into(),
        http: reqwest::Client::new(),
    };
    let h = backend.provision(CreateOpts {
        name: "x".into(), size_bytes: 1024 * 1024, description: None,
    }).await.unwrap();
    assert_eq!(h.size_bytes, 1024 * 1024);
}
```

- [ ] **Step 13.3: Verify + commit**

```bash
cargo test -p manager backends::truenas_iscsi
cargo test -p agent storage::iscsi
git commit -m "test(storage): iscsi locator parsing + truenas mock"
```

---

## Task 14: Final sweep

- [ ] `cargo fmt`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace --exclude installer`
- [ ] Verify Plan 1's tests still pass.
- [ ] Commit any fmt/lint changes.

---

## Plan 2 completion checklist

- [ ] iSCSI generic backend instantiable from TOML
- [ ] TrueNAS iSCSI backend creates zvols via REST
- [ ] Agent host backend logs in and produces a /dev/disk/by-path AttachedPath
- [ ] Slow-path rootfs allocator works end-to-end (provision + populate + resize)
- [ ] Agent reports supported kinds on register
- [ ] Manager refuses to schedule a VM on incompatible host
- [ ] All Plan 1 tests still pass

## Out of scope

- Snapshot/clone for TrueNAS (stub returns NotSupported; trait surface intact)
- CHAP authentication (TOML accepts it but iSCSI host backend ignores)
- Multi-portal failover
- iSCSI session refcount (we use aggressive logout)
