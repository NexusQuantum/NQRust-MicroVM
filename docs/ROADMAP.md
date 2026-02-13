# NexusRust-MicroVM — Next Phases: The Ultimate Platform

## Context

The platform has achieved ~85% of its original 6-month PRD. Core VM lifecycle, containers, functions, snapshots, templates, networking, volumes, images, user auth, and a full Next.js 15 UI are working. What remains is closing critical security/reliability gaps, then building the differentiating features that elevate this from "strong dev project" to "ultimate microVM platform."

**Target**: Solo developer, bare-metal/systemd deployment, sequential phases.

**North star**: "The most capable, secure, and operator-friendly open-source Firecracker orchestrator."

---

## Phase 1: Security Hardening & RBAC Enforcement

> Make multi-user deployment safe. Everything else builds on this.

### 1.1 Wire up RBAC enforcement (M)
`authz.rs` functions exist but are `#[allow(dead_code)]` and never called. `created_by_user_id` is always `None`.

- Remove `#[allow(dead_code)]` from all functions in `apps/manager/src/features/users/authz.rs`
- Set `created_by_user_id` from authenticated user in every service-layer create:
  - `apps/manager/src/features/vms/service.rs` (~line 225)
  - `apps/manager/src/features/functions/service.rs` (~line 44)
  - `apps/manager/src/features/containers/repo.rs` (~line 50)
  - `apps/manager/src/features/networks/repo.rs` (~line 43)
  - `apps/manager/src/features/volumes/repo.rs` (~line 38)
- Add authz checks in route handlers (list/get/update/delete) using existing `can_view_resource`, `can_modify_resource`, `can_delete_resource`
- Upgrade `optional_auth_middleware` to `auth_middleware` for mutating endpoints in `apps/manager/src/features/mod.rs` (lines 58-81)
- Filter list endpoints by ownership for non-admin users

### 1.2 Activate audit logging (S)
Audit schema exists (migration 0023) but functions are dead code.

- Integrate audit writes into all mutating route handlers
- Log: actor, action, resource type, resource ID, before/after state, timestamp
- Wire up existing audit query endpoint

### 1.3 Rate limiting & API security (M)
- Add `tower-governor` rate-limiting middleware in `apps/manager/src/main.rs`
  - Auth endpoints: 10/min per IP
  - Resource creation: 50/min per user
  - Global limits on expensive operations
- Add WebSocket JWT validation on WS upgrade (`/v1/vms/{id}/shell/ws`, `/v1/vms/{id}/metrics/ws`)
- Add CORS configuration and request size limits

### 1.4 API key / service account support (M)
- New migration: `api_tokens` table (id, user_id, name, hashed_token, scopes, expires_at, last_used_at)
- New endpoints: `POST/GET/DELETE /v1/auth/tokens`
- Extend auth middleware to accept both JWT and API key
- Frontend: API keys management tab in `apps/ui/app/(dashboard)/settings/page.tsx`

### 1.5 Reduce panic surface (M)
- Replace ~226 `unwrap()`/`expect()` calls in manager code
- Priority: route handlers and service layer (user-facing paths)
- Target: <50 remaining, all in init-only code

---

## Phase 2: Observability & Reliability

> See everything, recover from anything. Required before scaling.

### 2.1 Prometheus metrics export (M)
- Add `/metrics` endpoint using `metrics-exporter-prometheus`
- Instrument: HTTP request count/duration/status, VM lifecycle ops, active resource gauges, reconciler actions, agent heartbeat latency, DB query latency, WebSocket connection count
- New file: `apps/manager/src/core/metrics.rs`

### 2.2 OpenTelemetry tracing (M)
- Add `tracing-opentelemetry` + OTLP exporter
- Propagate trace context: Manager -> Agent -> Firecracker
- Tag spans: vm_id, user_id, host_id, operation
- Configure via env: `OTEL_EXPORTER_OTLP_ENDPOINT`

### 2.3 Structured logging + request IDs (S)
- Standardize JSON log format: `request_id`, `user_id`, `vm_id`, `duration_ms`, `status`
- Add `request_id` middleware (UUID per request, propagate through tracing)

### 2.4 Health & readiness endpoints (S)
- `GET /health/live` -- always 200
- `GET /health/ready` -- DB connected + healthy agent(s)
- `GET /health/startup` -- migrations complete
- Agent: `/health/ready` (Firecracker available, bridge exists)

### 2.5 Backup & restore (M)
- New script: `scripts/backup.sh` (pg_dump + image dir tar)
- New script: `scripts/restore.sh`
- Admin endpoint: `POST /v1/admin/backup`
- Document backup strategy, retention, RTO/RPO

### 2.6 Alerting foundations (S)
- Ship Prometheus alert rules as config file in `scripts/monitoring/alerts.yml`
  - VM failed to start (>3 in 5min), agent heartbeat missed (>60s), disk <10%, error rate >5%
- Ship default Grafana dashboard JSON in `scripts/monitoring/dashboards/`

---

## Phase 3: Frontend Completion & UX Polish

> Every backend capability gets a polished UI. Zero dead ends.

### 3.1 Fix broken frontend endpoints (S)
- Implement VM update -- `apps/ui/lib/api/facade.ts` (~line 171)
- Implement VM historical metrics -- `apps/ui/lib/api/facade.ts` (~line 178)
- Implement image file upload -- `apps/ui/lib/api/facade.ts` (~line 374)
- Implement container terminal -- `apps/ui/components/containers/xterm-wrapper.tsx`

### 3.2 RBAC management UI (M)
- New component: `apps/ui/components/users/role-management.tsx`
- Permission indicators (disabled buttons for Viewers, ownership badges)
- "My Resources" filter toggle on list pages
- Update `apps/ui/lib/auth/store.tsx` to expose permission checks

### 3.3 Notification system (M)
- Backend: New `notifications` module in `apps/manager/src/features/notifications/`
  - WebSocket channel for real-time notifications
  - Notification types: VM state change, operation complete, error, alert
  - Per-user preferences, read/unread status
- Frontend: Notification bell in header, notification center panel
- Uncomment notification preferences in `apps/ui/app/(dashboard)/settings/page.tsx` (~line 241)

### 3.4 Dashboard enhancements (M)
- Cluster-wide utilization charts (CPU, memory, network aggregated)
- Recent activity feed (from audit log)
- Quick actions (create from template, deploy container)
- Host health map
- Uncomment metrics retention in settings (~line 242)

### 3.5 Port forwarding UI (S)
- New component: `apps/ui/components/vms/vm-port-forwards.tsx`
- Add to VM detail page, wire to existing backend endpoints

---

## Phase 4: CLI Client & Automation

> The power-user's gateway. Prioritized over SDKs/Terraform.

### 4.1 CLI client (L)
- New crate: `crates/nexus-cli/`
- Commands:
  - `nexus login` / `nexus auth status`
  - `nexus vm list|create|start|stop|delete|ssh|clone`
  - `nexus container list|deploy|logs|exec|stop|delete`
  - `nexus function list|create|invoke|logs|delete`
  - `nexus template list|create|instantiate`
  - `nexus host list|metrics`
  - `nexus image list|import`
  - `nexus snapshot list|create|restore`
- Config: `~/.nexus/config.toml` (API URL, token, default format)
- Output: table (default) and JSON (`--output json`)
- Publish as release artifact alongside manager/agent binaries
- Dependencies: `clap` (args), `reqwest` (HTTP), `tabled` (table output)

### 4.2 Configuration file support (S)
- Add TOML config support: `nexus-manager.toml`, `nexus-agent.toml`
- Priority: env vars > config file > defaults
- `--config` CLI flag, validate on startup with clear errors

### 4.3 Installer improvements (M)
- Improve idempotency (safe to re-run)
- Add rollback on failure
- Add `nexus-installer upgrade` for in-place upgrades
- Post-install health verification

---

## Phase 5: Advanced Platform Features

> Differentiating capabilities that set this apart.

### 5.1 Scheduled operations & TTL (L)
- New module: `apps/manager/src/features/schedules/`
- New migration: `schedules` table (id, resource_type, resource_id, action, cron_expr, enabled, next_run_at)
- Cron-based auto-start/stop, TTL auto-delete, auto-scaling rules (min/max instances)
- Background task: schedule evaluator (check every 30s)
- Frontend: schedule config in VM/container detail pages

### 5.2 VM cloning (M)
- New endpoint: `POST /v1/vms/:id/clone`
- COW rootfs copy (btrfs reflink if available, fallback to cp)
- Preserve or override: name, network, credentials
- Track lineage (`cloned_from_id` column)
- Frontend: "Clone" button on VM detail

### 5.3 Webhook / event system (M)
- New module: `apps/manager/src/features/webhooks/`
- New migration: `webhooks` + `webhook_deliveries` tables
- Events: vm.created, vm.started, vm.stopped, vm.failed, container.*, function.*
- Retry with exponential backoff, delivery status log
- Frontend: webhook management page

### 5.4 Resource quotas (M)
- New module: `apps/manager/src/features/quotas/`
- New migration: `quotas` + `resource_usage` tables
- Per-user limits: max VMs, vCPUs, memory, storage
- Real-time usage tracking, enforcement on create
- Frontend: usage dashboard, quota management (admin)

### 5.5 Multi-cluster federation (XL)
- Manager-to-manager registration
- Cross-cluster VM visibility (read-only initially)
- Federated dashboard, cross-cluster template sharing
- New endpoints: `POST/GET /v1/federation/clusters`

---

## Phase 6: Ecosystem & Long-term

> Build when core is rock-solid and demand exists.

### 6.1 Terraform provider (XL)
- Go-based provider using Terraform Plugin Framework
- Resources: `nexus_vm`, `nexus_container`, `nexus_function`, `nexus_template`, `nexus_network`
- Publish to Terraform Registry

### 6.2 SDK libraries (L)
- Python + Go SDKs auto-generated from OpenAPI spec
- Publish to PyPI / Go modules

### 6.3 Documentation site (M)
- Hugo site (builds on existing `scripts/setup-docs-server.sh`)
- Getting started, architecture, API reference, operator runbook, security model
- Auto-deploy in CI

### 6.4 Integration test suite (L)
- `tests/integration/` -- end-to-end tests with mock agent
- VM, container, function lifecycle; auth/RBAC boundaries
- CI job, target 60%+ service layer coverage

### 6.5 Advanced networking (L)
- WireGuard mesh between hosts
- VXLAN overlays for multi-host L2
- Network policies (firewall rules per VM)
- DNS auto-registration

### 6.6 Docker deployment option (M)
- Dockerfiles for each service
- `docker-compose.prod.yml`
- Nice-to-have alongside primary bare-metal deployment

---

## Execution Order & Dependencies

```
Phase 1 (Security) --> Phase 2 (Observability) --> Phase 3 (Frontend)
                                                        |
                                                        v
                                                  Phase 4 (CLI)
                                                        |
                                                        v
                                                  Phase 5 (Advanced)
                                                        |
                                                        v
                                                  Phase 6 (Ecosystem)
```

Sequential for solo developer. Each phase is self-contained and shippable.

## Sizing

| Size | Effort | Solo Duration |
|------|--------|---------------|
| S | 1-3 days | ~1 week |
| M | 3-7 days | ~2 weeks |
| L | 1-3 weeks | ~1 month |
| XL | 3-6 weeks | ~2 months |

| Phase | Effort | Timeline |
|-------|--------|----------|
| Phase 1: Security | ~3 weeks | Weeks 1-3 |
| Phase 2: Observability | ~3 weeks | Weeks 4-6 |
| Phase 3: Frontend | ~3 weeks | Weeks 7-9 |
| Phase 4: CLI | ~3 weeks | Weeks 10-12 |
| Phase 5: Advanced | ~6 weeks | Weeks 13-18 |
| Phase 6: Ecosystem | ~8+ weeks | Weeks 19+ |

**Total to "ultimate" core (Phases 1-5)**: ~18 weeks / ~4.5 months
**Full ecosystem (Phase 6)**: 6-7 months

## Verification

- **Phase 1**: Attempt unauthorized ops -> 403. Audit log complete. `cargo test -p manager`.
- **Phase 2**: Scrape `/metrics` with Prometheus. Traces in OTLP backend. Backup/restore cycle. Alert rules fire.
- **Phase 3**: Manual walkthrough -- zero "not implemented" errors. Notifications deliver.
- **Phase 4**: `nexus vm list` returns data. `nexus vm create --from-template` works. JSON output valid.
- **Phase 5**: Scheduled VM auto-stops. Clone produces independent VM. Webhook delivers. Quota blocks over-limit.
- **Phase 6**: `terraform apply` creates VMs. SDK tests pass. Docs site live.
