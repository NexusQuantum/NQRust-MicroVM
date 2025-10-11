# NexusRust-MicroVM — 6-Month Product Requirements Document (PRD)

*Last update: 22 Sep 2025 • Owner: you • Scope: Backend + minimal UI hooks*

---

## 1) Executive summary

NexusRust-MicroVM is a control plane and agent system that makes **Firecracker microVMs** feel as easy to operate as containers, while preserving VM isolation and boot speed. In 6 months you’ll ship a production-ready, multi-host manager with a clean API, basic UI, and strong operational guarantees (reconciliation, audit, metrics, snapshots, templates, and guarded passthrough to Firecracker).

**North star**: “Create, run, observe, and retire thousands of microVMs safely and repeatably.”

---

## 2) Goals & success metrics (6-month target)

**Business/UX**

* *<5 min* time-to-first-VM on a fresh host (from agent install to VM running).
* *<3 clicks / 1 API call* to start/stop a VM from a saved template.
* *Zero manual cleanup* after crashes (agent/manager reconciliation cleans taps, sockets, scopes).
* *P95 < 2s* for CRUD calls (excluding image copies/snapshots).

**Reliability**

* Reconciler converges desired vs. actual state within *30s* across hosts.
* *SLO 99.5%* manager API availability.
* *<1%* failed Firecracker calls not auto-retriable (e.g., permanent validation errors).

**Security & governance**

* Audit log coverage *100%* for mutate ops.
* Role-based access control (RBAC) covering read, create, update, delete per project.

---

## 3) In-scope vs. Out-of-scope

**In-scope**

* Multi-VM per host; multi-host fleet (N agents registering to 1 manager).
* Lifecycle: create → configure → start/stop/pause → delete.
* Storage: rootfs + additional drives (pre-boot), runtime rate-limit PATCH.
* Network: TAP on bridge, NAT via host; per-VM MAC auto; optional static IP via DHCP hook later.
* Snapshots (create/pause/resume/load) with local store paths.
* Templates (golden configs) and cloning.
* Metrics/logging wiring (Firecracker logger/metrics) + basic charts (API, later FE).
* Reconciler and GC (orphan scope/tap/socket detection).
* Audit log + RBAC + API tokens.
* Image registry abstraction (host paths allow-listed + named images table).

**Out-of-scope (for this 6 months)**

* Live migration; cross-host snapshot streaming.
* GPU/advanced devices, vsock guest-services.
* Full multi-tenant network overlays (we stick to bridge+NAT).
* Billing/cost engine (export raw metrics; no costing yet).

---

## 4) Personas & primary use cases

* **DevOps “Sarah”**: Needs to run 50-500 ephemeral microVMs for jobs/tests.

  * “Create 20 VMs from template `python-3.12` and tear down in 2 hours.”
  * “See why a VM failed to start; get actionable error and logs.”
* **Platform Eng “Mike”**: Backend integration (CI, functions, worker fleet).

  * “Programmatically clone template → run workload → snapshot or stop.”
  * “Roll out a new kernel image across a pool with canarying.”

---

## 5) System architecture (high-level)

* **Manager (Rust/Axum + Postgres/SQLx)**: Source of truth, REST API, reconciliation, audit, authz, orchestration to agents.
* **Agent (Rust/Axum, root-capable)**: Host-local executor. Creates TAP/bridge, spawns Firecracker under transient `systemd` scope, exposes **native UDS → HTTP proxy** to Firecracker (no socat), performs cleanup.
* **State**: Postgres tracks VMs, hosts, templates, images, snapshots, audit events, API tokens.
* **Traffic**: Manager → Agent over HTTP(S). Agent → Firecracker via unix socket using `http+unix`.

---

## 6) Features (what & why & how) — **detailed spec**

Below are the features you will finish in 6 months, grouped by capability. Each entry includes:

* **What** (user-visible behavior)
* **Why** (value)
* **How** (implementation outline)
* **Acceptance criteria** (tests you’ll run)

### F1. Multi-VM lifecycle (Create/Start/Stop/Delete)

**What**

* Create a VM with: name, vCPU, mem MiB, kernel, rootfs, optional extra drives, network iface.
* Start/Stop/Pause/Delete VM; list and get details (state, host, socket path).
* Bulk: basic multi-select start/stop via API.

**Why**

* Core value: reliably spin up many isolated microVMs fast.

**How**

* Manager creates a VM record (`requested` → `configured` → `running|stopped`).
* Manager calls Agent:

  1. `POST /agent/v1/vms/:id/tap` (ensure bridge; create TAP; idempotent).
  2. `POST /agent/v1/vms/:id/spawn` (systemd transient scope `fc-<id>.scope`, wait for UDS).
  3. `PUT` FC `machine-config`, `boot-source`, `drives`, `network-interfaces`, `logger`, `metrics` via Agent’s UDS proxy.
  4. `PUT` FC `actions {InstanceStart}`.
* Stop: Agent stops scope + deletes TAP + removes UDS; Manager updates state.
* Delete: Calls stop (best-effort) + removes DB row + log cleanup.

**Acceptance**

* Create VM returns `201` with `id`, and `GET /v1/vms/:id` shows `running` within 5s.
* Stop sets state → `stopping` then `stopped` (reconciler completes if agent crashes mid-stop).
* Delete removes VM row; agent confirms TAP and UDS gone.

---

### F2. Native UDS proxy (no socat)

**What**

* Every VM has its own Firecracker socket (e.g., `/srv/fc/vms/<id>/sock/fc.sock`).
* Manager calls Agent `/:id/proxy/*path?sock=<path>` to relay HTTP to that socket.

**Why**

* Avoid per-VM TCP bridges; lower overhead and fewer moving parts.

**How**

* Agent uses an http+unix client to forward method/headers/body; strips `Host`, sets timeouts.
* **Safety**: Agent validates that `sock` path is within its `FC_RUN_DIR` and under `/srv/fc/vms/<id>/sock/`.

**Acceptance**

* PUT to `/proxy/machine-config` succeeds with proper JSON schema, 4xx/5xx mapped through.
* Invalid `sock` (outside run dir) returns `403`.

---

### F3. Networking: bridge/TAP + NAT

**What**

* One bridge per host (default `fcbr0`) with NAT via uplink; per-VM TAP `tap-<uuid>`.
* Auto MAC generation; guest obtains L2 connectivity on the bridge.

**Why**

* Minimal, predictable networking for local workloads and simple clusters.

**How**

* Agent:

  * `ensure_bridge`: create if missing, `ip link set up`, `iptables -t nat MASQUERADE`.
  * `create_tap`: idempotent delete → create → attach to bridge → `up`.
  * `delete_tap` on stop/delete.
* Optional: later hook DHCP server (dnsmasq) for IP assignment (not required in 6 months).

**Acceptance**

* Bridge exists and is `UP`; tap is visible and enslaved to bridge; teardown removes tap.

---

### F4. Templates & cloning

**What**

* Save a template (machine + boot + drives + NIC settings) with a human name.
* “Create from template” with minimal overrides (name, tags).

**Why**

* Reduces friction; standardization for teams.

**How**

* DB table `template {id, name, config_json, owner, created_at}`.
* Manager endpoint `POST /v1/templates`, `POST /v1/templates/:id/instantiate`.
* Manager hydrates template into VM create flow; persists link.

**Acceptance**

* Creating VM from template requires only `name` + optional tags.
* Diff view shows overrides vs. base template (API returns both).

---

### F5. Snapshots (pause/create/load)

**What**

* Pause a running VM → create snapshot (state + drives per FC semantics) → resume or stop.
* Load snapshot into a new VM (not hot-swap in same process).
* Snapshot list per VM; metadata (size, paths, timestamp).

**Why**

* Fast warm-starts; stateful recovery; golden images for workloads.

**How**

* Manager orchestrates:

  * `PUT /actions {InstancePause}` → `PUT /snapshot/create` (paths under `/srv/fc/vms/<id>/snapshots/...`) → optional `InstanceResume`.
  * Load: create a fresh VM, configure boot/snapshot load according to FC API, then start.
* Enforce disk path allow-list.

**Acceptance**

* Snapshot create returns metadata (size, files); load yields a running VM with same state.
* Errors surface from Firecracker with actionable `fault_message`.

---

### F6. Metrics & logs (wire-up + basic streaming)

**What**

* Logger → file path; metrics → JSON file per VM.
* API to tail logs and fetch recent metrics; optional SSE stream.

**Why**

* Operators need quick visibility for failures/perf.

**How**

* During create, Manager sets FC `/logger` and `/metrics`.
* API:

  * `GET /v1/logs/tail?path=…` (DEV); later SSE endpoint `GET /v1/logs/stream?id=…`.
  * `GET /v1/metrics/recent?id=…` reads metrics JSON.

**Acceptance**

* Log lines appear as VM boots; metrics JSON updates; endpoints respond within 200ms for small files.

---

### F7. Reconciler (drift correction & GC)

**What**

* Periodic loop compares DB desired state vs. actual host reality.
* Fixes drift: restarts missing scopes if desired `running`, cleans orphan taps/sockets, updates DB when VMs die.

**Why**

* Resilience to crashes, deploys, and manual host changes.

**How**

* Agent “introspect” endpoint: returns live scopes, taps, and sockets.
* Manager cron (every 15s): for each VM in `running`, verify:

  * Agent sees `fc-<id>.scope` active and socket present; if not, try restart; otherwise mark `stopped`.
  * Remove orphan resources under `/srv/fc/vms/*` not present in DB (grace policy).
* Idempotent, rate-limited retries; backoff.

**Acceptance**

* Kill agent/manager or firecracker randomly; within 30s, state converges (no resource leaks).

---

### F8. AuthN & RBAC + API tokens

**What**

* API tokens scoped to a user or service; roles: `viewer`, `operator`, `admin`.
* Permission checks per project (namespace), per resource type.

**Why**

* Safe multi-user operation.

**How**

* Manager issues tokens (PBKDF2/Argon hashed); stores in `api_tokens`.
* Middleware resolves token → user → roles → allows or denies.
* Projects: simple label on VMs/templates for scoping.

**Acceptance**

* Requests without token → 401.
* Viewer cannot mutate; Operator can mutate VMs in their project; Admin full access.

---

### F9. Audit logging

**What**

* Every mutate call logs: who, what, when, before/after summary, result (allow/deny).

**Why**

* Accountability & debugging.

**How**

* Manager writes audit rows on entry (pending) and on exit (success/fail), with request\_id.

**Acceptance**

* Export audit for a VM shows a complete history with timestamps and caller identity.

---

### F10. Image registry abstraction

**What**

* Named images (kernel/rootfs) with validated host paths; prevent arbitrary path injection.

**Why**

* Hygiene & security; reproducibility.

**How**

* Tables: `image {id, kind(kernel|rootfs), name, host_path, sha256(optional), size}`.
* VM create accepts `kernel_image_id`/`rootfs_image_id` or raw path (disabled in prod).

**Acceptance**

* VM create fails if image id not allowed for caller’s project; succeeds if valid.

---

## 7) API surface (manager)

*All JSON; examples abbreviated.*

* `POST /v1/vms` → `{id}`
  body: `{ name, vcpu, mem_mib, kernel_image_id|kernel_path, rootfs_image_id|rootfs_path, extra_drives[], nic? }`
* `GET /v1/vms` → `{ items: [...] }`
* `GET /v1/vms/:id` → `{ item: {...} }`
* `POST /v1/vms/:id/stop` → `{ ok: true }`
* `DELETE /v1/vms/:id` → `{ ok: true }`

*Snapshots*

* `POST /v1/vms/:id/snapshots` (pause→create) → `{ snapshot_id, size, files }`
* `POST /v1/snapshots/:sid/instantiate` → `{ id }`

*Templates*

* `POST /v1/templates` / `GET /v1/templates` / `POST /v1/templates/:id/instantiate`

*Images*

* `POST /v1/images` / `GET /v1/images` (`kind=kernel|rootfs`)

*Logs & metrics*

* `GET /v1/logs/tail?path=…` (dev)
* `GET /v1/metrics/recent?id=…` (dev)
* (later) `GET /v1/logs/stream?id=…` (SSE)

*Admin/ops*

* `GET /v1/hosts` (agents and their capacities)
* `POST /v1/tokens` (admin only)
* `GET /v1/audit?vm_id=…`

**Error contract**
`{ error, fault_message?, status, suggestion?, request_id }` for non-2xx.

---

## 8) Data model (core tables)

* **vm**: `id, name, state, host_id, host_addr, api_sock, tap, log_path, http_port, fc_unit, template_id?, created_at, updated_at`
* **vm\_event**: `id, vm_id, at, level, message`
* **template**: `id, name, owner, config_json, created_at`
* **snapshot**: `id, vm_id, meta_json, path, size, created_at`
* **image**: `id, kind, name, host_path, sha256, size, created_at`
* **host**: `id, name, addr, capacity_json, last_seen_at`
* **audit**: `id, actor, action, resource, resource_id, request_id, at, before_json?, after_json?, result`
* **api\_token**: `id, owner, hashed, role, project, created_at, last_used_at`

---

## 9) Security requirements

* Validate all host paths against an allow-list root (e.g., `/srv/fc/images`, `/srv/fc/vms`).
* Agent validates `?sock=` to live under its run dir and match VM id prefix.
* CORS: exact allow-list in production.
* Tokens: bearer in `Authorization`; rotation and revocation endpoints.
* Least-privilege `sudoers` for agent: `systemd-run`, `systemctl stop`, `ip`, `iptables`, `sysctl`.

---

## 10) Observability

* Structured JSON logs (manager & agent) with `request_id`, latency, upstream status.
* Prometheus metrics (manager): request counts, durations, reconciler actions, errors.
* Health endpoints:

  * Manager: `/healthz` (liveness), `/readyz` (DB + at least one healthy agent)
  * Agent: `/agent/v1/health`, `/agent/v1/capacity`

---

## 11) Performance & scale targets

* Manager p95 handler < 200 ms (excluding Firecracker operations).
* Reconciler: bounded parallelism per host; backoff on failures.
* Tested to 1,000 VMs across 10 agents (100 each) with basic churn (create/start/stop/delete).

---

## 12) Risks & mitigations

* **Kernel/rootfs drift** → images abstraction + immutability via checksums.
* **iptables vs nftables** → detect and support nftables mode or document requirement.
* **Agent privilege** → constrained sudoers + path allow-lists + audit.
* **API misuse** → façade only forwards *allow-listed* Firecracker endpoints.

---

## 13) Milestones & timeline (6 months)

**M1 (Weeks 1-3):** A1–A3 foundation

* Single-host: multi-VM lifecycle, UDS proxy, logs/metrics wiring, Postgres schema, CLI/API.
* *Demo*: create→start→stop→delete; log tail; recovery after manager restart.

**M2 (Weeks 4-6):** Reconciler & GC + Host registry

* Agent inventory endpoints; manager reconciler loop; orphan cleanup.
* *Demo*: kill firecracker; reconciler heals or marks stopped; no leaks.

**M3 (Weeks 7-9):** Templates & Images

* Template CRUD; image registry (kernel/rootfs) with allow-list paths.
* *Demo*: create from template in 1 request; prevent raw arbitrary paths in prod.

**M4 (Weeks 10-12):** Snapshots (local)

* Pause/create/load; snapshot catalog.
* *Demo*: capture snapshot, instantiate new VM from snapshot.

**M5 (Weeks 13-16):** AuthN/RBAC + Audit

* API tokens, roles (viewer/operator/admin), project scoping; full audit trail.
* *Demo*: different tokens show/limit operations; audit export for a VM.

**M6 (Weeks 17-20):** Multi-host scale & perf hardening

* Register multiple agents; host selection strategy (round-robin/resources); perf tests.
* *Demo*: 10 agents, 200 VMs; reconciler stable; metrics dashboards.

**M7 (Weeks 21-24):** Polishing & Ops

* Rate limits/quotas, SSE logs, better errors, docs, upgrade notes; optional FE tidy pages.
* *Release candidate*.

---

## 14) Acceptance checklist (release readiness)

* [ ] Create/start/stop/delete OK across 10 hosts, 1k VMs test load.
* [ ] Reconciler converges; no orphan taps/sockets after fault injection.
* [ ] Snapshot create/load OK; templates OK; images guard paths OK.
* [ ] RBAC enforced; all mutating calls audited.
* [ ] Health/readiness endpoints green; metrics exported.
* [ ] Docs: operator runbook (bridge setup, nftables note), API reference.

---

## 15) Developer quickstart (ops notes)

```
# DB
./scripts/dev-up.sh

# Host network
sudo ./scripts/fc-bridge-setup.sh fcbr0 <uplink>

# Agent
AGENT_BIND=127.0.0.1:9090 FC_RUN_DIR=/srv/fc FC_BRIDGE=fcbr0 \
  (cd apps/agent && cargo run)

# Manager
DATABASE_URL=postgres://nexus:nexus@localhost:5432/nexus \
AGENT_BASE=http://127.0.0.1:9090 \
  (cd apps/manager && sqlx migrate run && cargo run)
```

---

### Final note

This plan lets you “use what Firecracker already has +++”: we orchestrate its official API safely (pre-boot PUTs, runtime PATCH where allowed), add reconcilers, templates, snapshots, and the control-plane concerns Firecracker intentionally leaves to you. If you want, I can translate this PRD into GitHub epics/issues next so you can sprint immediately.


PM Plan:
# Project Management Plan — 6-Month Build

Below is the same project-management phase/board I outlined earlier, cleaned up and ready to drop into your tracker (GitHub Projects/Jira/Linear). It maps exactly to the PRD and the feature slices we designed.

---

## 1) Program structure

**Workstreams**

* **A. Agent** (privileged host service)
* **B. Manager** (control plane API + DB)
* **C. Networking** (bridge/TAP/NAT)
* **D. Orchestration** (reconciler/GC, multi-host)
* **E. Images & Templates**
* **F. Snapshots**
* **G. Security & Governance** (AuthN/RBAC, Audit)
* **H. Observability & Ops** (metrics, logs/SSE, health, docs)

**Cadence**

* 12 × 2-week sprints (Weeks 1–24)
* Demo + retro every sprint; release candidate in Weeks 21–24

---

## 2) Milestones (Epics) & Deliverables

### **M1 — A1–A3 Foundations (Weeks 1–3)**

* **A1** Agent minimal: systemd scope spawn, UDS proxy, health/capacity
* **A2** Manager minimal: create→configure→start, list/get/stop/delete, Postgres migrations
* **A3** Logs & metrics wiring: FC /logger and /metrics, dev log tail endpoint

**Exit criteria**

* Create→start→stop→delete works on one host
* No socat; per-VM unix sockets
* Operator quickstart doc

---

### **M2 — Reconciler & Host Registry (Weeks 4–6)**

* **D1** Agent inventory endpoint (live scopes/taps/sockets)
* **D2** Manager reconciler loop (every 15s): heal or mark stopped; GC orphans
* **B3** Host registry: register agent, heartbeat, basic host selection (single host OK)

**Exit criteria**

* Kill firecracker or agent; system converges in ≤30s
* No resource leaks after chaos tests

---

### **M3 — Templates & Images (Weeks 7–9)**

* **E1** Templates CRUD; instantiate
* **E2** Image registry (kernel/rootfs), allow-listed host paths; PRD passthrough disabled in prod

**Exit criteria**

* Create VM from template with 1 request
* Unsafe raw paths blocked unless dev mode

---

### **M4 — Snapshots (Weeks 10–12)**

* **F1** Pause→snapshot→resume
* **F2** Load snapshot into new VM; snapshot catalog

**Exit criteria**

* Snapshot create/load works locally, with metadata surfaced via API

---

### **M5 — AuthN/RBAC & Audit (Weeks 13–16)**

* **G1** API tokens, roles (`viewer`, `operator`, `admin`)
* **G2** Project scoping; policy checks on mutate ops
* **G3** Audit log for every mutate (who/what/when/before/after/result)

**Exit criteria**

* Unauthorized actions blocked; audit trail complete for one VM lifecycle

---

### **M6 — Multi-Host & Performance (Weeks 17–20)**

* **D3** Multi-agent scheduling: round-robin or capacity-aware
* **H1** Prometheus metrics exposed; perf tests; quotas/rate-limits

**Exit criteria**

* 10 agents / 200 VMs churn test stable
* Manager p95 < 200 ms (excluding FC work)

---

### **M7 — Polish & Ops (Weeks 21–24)**

* **H2** SSE logs streaming; better errors; docs & runbooks
* **B4** Upgrade notes & migration scripts; RC hardening

**Exit criteria**

* Release candidate tagged; operator docs complete

---

## 3) Sprint plan (high level)

| Sprint | Focus                         | Key Issues (abbrev)                                                 |
| ------ | ----------------------------- | ------------------------------------------------------------------- |
| 1      | Agent & Manager skeletons     | A1-001 agent scaffold, A2-001 manager scaffold, C-001 bridge script |
| 2      | Create/Start/Stop + UDS proxy | A1-010 systemd scope spawn, A1-020 UDS proxy, A2-020 orchestration  |
| 3      | Logs & metrics wiring         | A3-010 logger, A3-020 metrics, B-010 migrations harden              |
| 4      | Host registry                 | D-010 agent register/heartbeat, B-020 host table                    |
| 5      | Reconciler/GC                 | D-020 reconciler, D-030 orphan GC, chaos tests                      |
| 6      | Reliability fixes             | D-040 retries/backoff, A-040 startup reaping, docs v1               |
| 7      | Templates                     | E1-010 CRUD, E1-020 instantiate                                     |
| 8      | Images                        | E2-010 registry, E2-020 path policy, E2-030 prod toggle             |
| 9      | UX & perf                     | E-polish errors, B-060 perf pass (DB indexes)                       |
| 10     | Snapshots create              | F1-010 pause/create/resume, F1-020 metadata                         |
| 11     | Snapshots load                | F2-010 load new VM, F2-020 catalog                                  |
| 12     | Snapshots cleanup             | F-polish GC, F-tests                                                |
| 13     | Auth tokens                   | G1-010 token CRUD, middleware                                       |
| 14     | RBAC                          | G2-010 roles, G2-020 project scope                                  |
| 15     | Audit                         | G3-010 audit writer, G3-020 export                                  |
| 16     | Security pass                 | G-polish threat model, path allow-lists                             |
| 17     | Multi-host schedule           | D3-010 selection policy                                             |
| 18     | Metrics & perf                | H1-010 Prom metrics, load tests                                     |
| 19     | Quotas/limits                 | H1-020 rate limits, guardrails                                      |
| 20     | Stabilization                 | H-bugfixes, scale test 10 agents                                    |
| 21     | SSE logs                      | H2-010 log streaming                                                |
| 22     | Errors/Docs                   | H-docs runbooks, error mapping                                      |
| 23     | RC hardening                  | test matrix, DR drills                                              |
| 24     | Release                       | tag RC, handover                                                    |

---

## 4) Backlog (issue templates)

**Example: A1-020 — Agent: http+unix UDS proxy**

* *Type*: Feature
* *Desc*: Forward HTTP to Firecracker unix socket without socat
* *Acceptance*:

  * `PUT /proxy/machine-config?sock=<vm sock>` returns FC status
  * Reject paths outside `FC_RUN_DIR` (403)
  * Timeouts mapped to 504
* *Tasks*:

  * Build client; header filtering; path validation; tests
* *Risk*: Path traversal → mitigate with canonicalize + prefix check

**Example: D-020 — Reconciler**

* *Type*: Feature
* *Acceptance*:

  * After SIGKILL firecracker, state converges ≤30s
  * No orphan TAP/UDS after GC
* *Tasks*: Agent inventory; loop; retries/backoff; metrics

---

## 5) Dependencies & critical path

* **Kernel/rootfs availability** → blocks M1 demos
* **Agent sudoers** (systemd, ip, iptables, sysctl) → blocks A1 spawn & C networking
* **DB schema** → blocks Manager features (A2+)
* **Agent inventory** → blocks reconciler (M2)

---

## 6) Definition of Done (DoD)

* API behavior covered by integration tests
* Structured logs include `request_id`, upstream status, duration
* Error contract `{ error, fault_message?, status, suggestion?, request_id }`
* Docs updated (runbook, API)
* Security checks: path allow-lists; auth (once G done)
* Observability: Prom metrics for major operations

---

## 7) Test matrix (selected)

* **Lifecycle**: create→start→stop→delete; repeat 100× (leak check)
* **Reconciler**: kill agent, kill firecracker, delete tap/uds manually; system recovers
* **Snapshots**: stateful app resumes correctly (e.g., in-guest counter)
* **RBAC**: viewer blocked on mutate; operator restricted to project
* **Scale**: 10 agents × 100 VMs churn; manager p95 < 200 ms (excluding FC)
* **Security**: path traversal attempts on `?sock=` and images; blocked

---

## 8) Roles/RACI (can be one person at start)

* **Owner/PM/Tech lead**: you (R/A)
* **Agent**: you or agent sub-lead (R)
* **Manager/API/DB**: you or manager sub-lead (R)
* **Security**: reviewer pass per milestone (C)
* **Ops**: infra scripts & docs (R)
* **QA**: integration + chaos tests (R)

---

## 9) Tracking & metrics

* **Burndown** per sprint
* **Build health**: CI green, test counts, coverage (core paths)
* **Perf dashboards**: request latency histograms, reconciler actions/min
* **Error budget**: non-actionable 5xx < 0.5%/sprint

---

## 10) Acceptance gates per milestone

* **M1**: One-host happy path; operator quickstart
* **M2**: Convergence under faults; no orphans
* **M3**: Template/Images guardrails on; prod mode blocks raw paths
* **M4**: Snapshot create/load; catalog API
* **M5**: Tokens + RBAC + full audit; security review
* **M6**: Multi-host load test passes; metrics exported
* **M7**: SSE logs, docs/runbooks, RC tag

---

## 11) Next steps (this week)

* [ ] Create project board with epics M1–M7 + initial issues (Sprints 1–2)
* [ ] Lock sudoers for dev; verify `firecracker` in PATH
* [ ] Prepare two real images (kernel/rootfs) for M1 demo
* [ ] Wire CI (fmt, clippy, test) + `sqlx` offline schema cache (optional)
* [ ] Start Sprint 1: agent/manager skeletons + bridge script

If you want, I can turn this into a **ready-to-import GitHub Projects CSV** or **Jira epics** with all the issue titles and acceptance criteria prefilled.
