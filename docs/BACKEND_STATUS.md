## Backend status (as of 2025-10-04)

### Overview

This backend comprises two Rust services:

- Manager (`apps/manager`): public API (Axum) for VMs, templates, images, hosts, snapshots. Orchestrates VM lifecycle by talking to agents.
- Agent (`apps/agent`): host-side control-plane (Axum). Manages TAP, spawning Firecracker, and proxies Firecracker API via UDS.

Recent work focused on making VM creation reliable end-to-end through the Manager without manual Agent calls. The path now succeeds with images registered in the Manager and a healthy Agent.

### Current capabilities

- VM lifecycle via Manager
  - `POST /v1/vms`: create VM end-to-end (tap → spawn → configure → start)
  - `GET /v1/vms`, `GET /v1/vms/{id}`: list and fetch
  - `POST /v1/vms/{id}/stop`, `DELETE /v1/vms/{id}`: stop/delete

- Images
  - `GET /v1/images` (+ filters), `GET /v1/images/{id}`
  - `POST /v1/images`: register images (kernel/rootfs)
  - `DELETE /v1/images/{id}`

- Templates and Snapshots
  - CRUD and instantiation endpoints exist (see code), not the focus of recent fixes

- Agent features
  - `POST /agent/v1/vms/:id/tap`: ensure bridge (fcbr0) and create TAP
  - `POST /agent/v1/vms/:id/spawn`: idempotent Firecracker spawn (systemd-run or direct fallback)
  - `ANY /agent/v1/vms/:id/proxy/*`: forwards to Firecracker UDS for machine-config, boot-source, drives, network, logger, metrics, actions
  - `POST /agent/v1/vms/:id/stop`: best-effort cleanup (scope, TAP, sock)
  - `GET /agent/v1/inventory`: lists running scopes, TAPs, and UDS sockets (now includes Unix sockets)
  - NEW: `POST /agent/v1/vms/:id/metrics/prepare`: auto-creates a FIFO at the requested path for Firecracker metrics

### What changed (reliability and UX)

- Manager spawn no longer stalls:
  - Fire-and-forget request to Agent, then poll Agent inventory until the UDS appears (socket-ready) before configuring Firecracker
  - Step logs for each config phase; timeouts tuned

- Metrics are optional by default:
  - `MANAGER_ENABLE_METRICS=true` enables metrics
  - When enabled, Manager now calls Agent `metrics/prepare` to create a FIFO before configuring Firecracker metrics

- Agent resiliency
  - Idempotent spawn: handles already-loaded scope, removes stale sockets, verifies connectability, and chmods socket permissions so proxy works
  - Non-interactive sudo (`-n`) for bridge/TAP/iptables; avoids password prompts
  - Inventory now lists Unix sockets so Manager can poll readiness reliably

### Environment & operational notes

- Manager
  - `DATABASE_URL` is required
  - `MANAGER_BIND` (default `127.0.0.1:8080`)
  - `MANAGER_IMAGE_ROOT` (default `/srv/images`)
  - `MANAGER_ALLOW_IMAGE_PATHS=true` for direct host paths in dev only
  - `MANAGER_RECONCILER_DISABLED=1` can be used during bring-up to avoid races
  - `MANAGER_ENABLE_METRICS=true` to configure metrics (FIFO is auto-prepared)

- Agent
  - `AGENT_BIND` (e.g. `127.0.0.1:19090`)
  - `MANAGER_BASE` (e.g. `http://127.0.0.1:18080`)
  - `FC_RUN_DIR` (default `/srv/fc`)
  - `FC_BRIDGE` (default `fcbr0`) — ensure bridge is up (`scripts/fc-bridge-setup.sh fcbr0 <uplink>`)
  - Requires Firecracker in PATH (usually `/usr/local/bin/firecracker`)
  - Host must have KVM (`/dev/kvm`, `kvm_intel`/`kvm_amd` loaded)
  - Run Agent as root (recommended) or configure `sudoers` for `ip`, `systemctl`, `systemd-run`, `iptables`, `mkfifo`

### Known issues to fix next

1) Persistence timing
   - Manager inserts VM row after config/start. Insert earlier (after spawn readiness) with state `starting`, update to `running` after start. Impact: GET `/v1/vms` will reflect active operations and aid reconciliation.

2) Reconciler races
   - With spawn readiness polling, races are reduced, but reconciler can still misinterpret transient states. Action: reconcile should respect `starting` state and allow a grace window; also verify via inventory before cleanup.

3) Security hardening
   - Avoid `chmod 666` on UDS; prefer running Agent as root and gating permissions, or use a dedicated group. Consider systemd service units for Agent with tighter policy.
   - Optionally move to systemd D-Bus for spawn/stop instead of shelling out.

4) Error propagation
   - Manager 500s currently have empty bodies to the client. Map internal errors to structured responses with reason strings and correlation IDs.

5) Networking robustness
   - Auto-detect uplink for bridge setup; validate iptables/NAT. Ensure cleanup of TAP/iptables on delete.

6) Observability & logs
   - Wire metrics to a default sane FIFO path per VM; add log rotation; expose basic health/readiness endpoints and surface recent events in Manager.

7) Tests & CI
   - Add E2E test that registers images, creates a VM via Manager, asserts UDS/connectivity, and tears down. Include bridge/KVM checks in CI runners where possible.

### Future recommendations

- Scheduling and multi-host
  - Improve host selection (capacity-aware: CPU, memory, active VMs). Add pre-flight checks before allocate.

- Templates & profiles
  - Provide curated VM templates (kernel/rootfs) and validate defaults (e.g., always use valid ELF `vmlinux`).

- Secure image handling
  - Optional integrity enforcement (size/hash checks), signed manifests, and project scoping.

- Stronger API contract
  - Return per-step status in create response (or async job with polling). Emit VM events stream for frontend.

- Packaging & service management
  - Provide systemd unit files for Agent & Manager; ensure firecracker binary installation and PATH docs/scripts.

### Quick start (happy path)

1) Prepare images
   - Kernel: `curl -L -o /srv/images/vmlinux-5.10.fc.bin https://s3.amazonaws.com/spec.ccfc.min/img/quickstart_guide/x86_64/kernels/vmlinux.bin`
   - Rootfs: place e.g. `/srv/images/alpine-3.18-minimal.ext4`
   - Register both via `POST /v1/images` and use returned IDs

2) Start services
   - Agent as root, with `FC_BRIDGE=fcbr0` and bridge up; Manager with DB URL and (optionally) `MANAGER_ENABLE_METRICS=true`

3) Create VM via Manager
   - `POST /v1/vms` with kernel/rootfs image IDs
   - Manager logs should show: tap ok → spawn socket ready → machine-config → boot-source → drives → network → logger → (metrics if enabled) → start → 200 response

4) Verify
   - `GET /v1/vms`

---

This document is living; update it as we address the "Known issues" and add features.


