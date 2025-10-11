## NexusRust-MicroVM — Capabilities and Gaps (benchmarked to PRD)

Updated: 2025-10-05

### Current Capabilities

- Manager (Rust/Axum + Postgres)
  - VM lifecycle API: 
    - POST/GET /v1/vms, GET/DELETE /v1/vms/:id, POST /v1/vms/:id/stop
    - Orchestration: creates TAP, spawns Firecracker under systemd scope, configures machine/boot/drives/net/logger, starts VM
  - Images registry:
    - POST/GET /v1/images with filtering by kind=kernel|rootfs
    - Path allow-list enforcement under MANAGER_IMAGE_ROOT (default /srv/images); raw paths disabled unless MANAGER_ALLOW_IMAGE_PATHS=1
  - Snapshots and Templates (backend)
    - Snapshots: list/create-for-VM, instantiate snapshot into a new VM
    - Templates: CRUD + instantiate
  - Reconciler wiring: enabled by default; can be disabled via MANAGER_RECONCILER_DISABLED
  - OpenAPI generated and CORS permissive for dev

- Agent (Rust/Axum, root-capable)
  - Systemd scope spawn for Firecracker; fallback direct launch if systemd-run fails
  - UDS HTTP proxy for Firecracker (no socat)
  - Network: ensure bridge, idempotent TAP create/attach/up; stop path tears down TAP and systemd scope
  - Inventory endpoint (scopes/taps/sockets) used by manager for readiness polling
  - Metrics FIFO prepare endpoint (auto mkfifo + chmod) for Firecracker metrics
  - Socket permission handling (chmod 0666 after creation)

- Frontend (Next.js)
  - Next API proxy: /api/proxy/v1 → manager (fixes browser networking issues)
  - Registry UI lists images (kernel/rootfs) from manager
  - VM creation wizard:
    - Uses image IDs from registry by default (paths disabled when ID selected)
    - Quick Create sends CreateVmReq { name, vcpu, mem_mib, kernel_image_id, rootfs_image_id }
  - VMs list and details page show manager state; Stop action wired
  - Basic snapshots UI aligned to new backend endpoints (create/list/restore flows surfaced)

### What Works End-to-End

- Create VM via manager using registry image IDs (no raw paths)
- Agent spawns Firecracker, manager configures machine/boot/drive/net/logger, starts VM
- Optional metrics enabled via MANAGER_ENABLE_METRICS (auto FIFO prepared on agent)
- List/get/stop/delete VMs
- List/create images (with path allow-list)
- UI flows: registry browse → VM create wizard (IDs) → VM shows in lists/details

### Known Constraints

- Raw image paths are blocked by default (PRD-compliant); enable only for dev via MANAGER_ALLOW_IMAGE_PATHS=1 and ensure paths under MANAGER_IMAGE_ROOT
- UI actions limited (Stop only). Pause/Resume/Start exposed in backend façade but not fully wired in FE
- Network/Disk editors in UI are present but not backed by manager endpoints (runtime rate-limit PATCH, extra drives/NIC pre-boot) yet

### Status vs PRD (high-signal)

| PRD Feature | Status | Notes |
| --- | --- | --- |
| F1. Multi-VM lifecycle | Partial | Create/Start/Stop/Delete+List implemented; UI wired for Stop; Start/Pause/Resume minimal FE; bulk ops not yet |
| F2. Native UDS proxy | Done | Agent proxy live; manager uses it for all FC config |
| F3. Bridge/TAP + NAT | Partial | Bridge/TAP lifecycle done; NAT depends on host setup script; no DHCP/IP mgmt |
| F4. Templates & cloning | Partial | Backend endpoints exist incl. instantiate; FE not surfaced yet |
| F5. Snapshots | Partial | Backend list/create/instantiate present; FE surfaces basic flows; full pause/resume orchestration UX minimal |
| F6. Metrics & logs | Partial | Logger configured; metrics optional with auto FIFO; tail/log streaming endpoints minimal; FE metrics charts are placeholder (no live stream) |
| F7. Reconciler & GC | Partial | Reconciler scaffold present; core healing/GC policies not fully validated at scale |
| F8. AuthN & RBAC + tokens | Not started | No token/RBAC middleware yet |
| F9. Audit logging | Not started | No mutate-op audit rows yet |
| F10. Image registry abstraction | Done | DB + API + allow-listed paths; FE wired to IDs |

### Gaps/Backlog (grouped)

- Lifecycle/UX
  - Wire Start/Pause/Resume in FE against manager façade
  - Bulk VM actions (start/stop multiple) per PRD
- Snapshots
  - Full pause→snapshot→resume orchestration from FE, better errors, sizes/metadata
- Templates
  - FE templates browse/create/instantiate; manager hydrate/override UI
- Reconciler/GC
  - Drift healing validation (missing scope/socket), orphan cleanup, backoff metrics
- Observability
  - SSE logs streaming; Prometheus metrics in manager; FE metrics charts backed by API
- Security & Governance
  - API tokens, roles (viewer/operator/admin), per-project scoping
  - Audit log for all mutates with before/after and request_id
- Networking
  - NAT runbook; optional DHCP hook; MAC auto-generation surface; rate limiters runtime PATCH in manager + FE
- Ops & Docs
  - Health/ready endpoints; operator runbook; error mapping polish with actionable suggestions

### Quick Ops Notes

- Prefer registry image IDs (kernel/rootfs) over paths
- Dev toggles:
  - MANAGER_ALLOW_IMAGE_PATHS=1 (dev only, with MANAGER_IMAGE_ROOT)
  - MANAGER_ENABLE_METRICS=1 to configure FC metrics (agent auto-fifo)
  - MANAGER_RECONCILER_DISABLED=1 to disable reconciler during debugging
- FE default base is proxied: /api/proxy/v1 (no browser → manager CORS issues)

### Recommended Next Steps (near-term)

- FE: enable Start/Pause/Resume; expose templates; snapshot create/restore UX
- Manager: add SSE logs + Prom metrics; finalize reconciler GC loop paths
- Security: introduce token auth middleware, role checks, and audit writer
- Docs: add health/ready endpoints and an operator quickstart/runbook per PRD

This captures the working surface and what to ship next to converge with the PRD.