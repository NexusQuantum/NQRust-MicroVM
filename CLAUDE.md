# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

### Build and Run
- Build entire workspace: `cargo build`
- Build specific package: `cargo build -p manager` or `cargo build -p agent`
- Run manager: `(cd apps/manager && cargo run)`
- Run agent: `sudo -E env AGENT_BIND=127.0.0.1:9090 MANAGER_BASE=http://127.0.0.1:18080 FC_RUN_DIR=/srv/fc FC_BRIDGE=fcbr0 ./target/debug/agent`

### Testing
- Run all tests: `cargo test`
- Run package tests: `cargo test -p manager`
- Run single test: `cargo test -p manager test_name`

### Linting and Formatting (CI enforces both)
- Format: `cargo fmt`
- Format check: `cargo fmt -- --check`
- Lint: `cargo clippy --all-targets --all-features -- -D warnings`

### Database Operations (Manager)
- Install SQLx CLI: `cargo install sqlx-cli --no-default-features --features postgres`
- Run migrations: `(cd apps/manager && sqlx migrate run)`
- Create migration: `(cd apps/manager && sqlx migrate add migration_name)`
- Revert migration: `(cd apps/manager && sqlx migrate revert)`
- Migrations run automatically on manager startup

### Frontend (apps/ui)
The project has TWO frontends — `apps/frontend` is the old Next.js 14 version (deprecated). Only work in `apps/ui`.
- Install: `(cd apps/ui && pnpm install)`
- Dev mode: `(cd apps/ui && NEXT_PUBLIC_API_BASE_URL=http://127.0.0.1:18080/v1 pnpm dev)`
- Lint: `(cd apps/ui && pnpm lint)`
- Build: `(cd apps/ui && pnpm build)`
- URL: http://localhost:3000

### Development Setup
1. Start PostgreSQL: `./scripts/dev-up.sh` (starts via Docker)
2. Set up bridge: `sudo ./scripts/fc-bridge-setup.sh fcbr0 <uplink-iface>` (once per host)
3. Configure environment variables (see `.env.example`)
4. Start agent first (needs sudo for KVM), then manager
5. Default admin credentials created on first run: `root` / `root`
6. Swagger UI available at: `http://localhost:18080/swagger-ui/`

## Architecture Overview

NQRust-MicroVM is a Firecracker microVM management platform with three Rust services and a Next.js frontend.

### Manager (`apps/manager`) — Port 18080
Central orchestration service. Axum web framework, PostgreSQL via SQLx with compile-time query checking. Manages VM lifecycle, containers (container-per-VM architecture), serverless functions, image registry, snapshots, templates, networks, volumes, and user auth/RBAC.

### Agent (`apps/agent`) — Port 9090
Runs on KVM hosts with root privileges. Registers with manager on startup, sends heartbeats. Communicates with Firecracker VMM via Unix domain sockets. Handles VM creation, lifecycle, snapshots, and proxies shell access via screen sessions.

### Guest Agent (`apps/guest-agent`) — Port 9000 (inside VM)
Runs inside VMs. Auto-deployed during VM creation. Reports CPU, memory, uptime metrics. Auto-discovers and reports VM IP address. Cross-compiled for target `x86_64-unknown-linux-musl`.

### Frontend UI (`apps/ui`)
Next.js 15, React 19, TypeScript, shadcn/ui, Tailwind CSS 4. TanStack Query for server state, Zustand for client state. WebSocket for terminal (xterm.js) and real-time metrics.

### Shared Types (`crates/nexus-types`)
Common data structures used by manager and agent. Ensures type safety across service boundaries.

## Code Conventions

### Manager Layered Architecture
Each feature in `apps/manager/src/features/` follows a strict layered pattern:

- **`routes.rs`** — Axum HTTP handlers. Extract params via `Extension(AppState)`, `Path(...)`, `Json(...)`. Each handler annotated with `#[utoipa::path(...)]` for OpenAPI generation. Minimal logic, delegates to service layer.
- **`service.rs`** — Business logic and orchestration. Uses `anyhow::Result<T>` with `.context()` / `.with_context()` for error enrichment. Coordinates DB queries, file operations, and agent HTTP calls.
- **`repo.rs`** — Database access. Structs derive `sqlx::FromRow`. Uses `sqlx::query_as` for type-safe queries. Supports test mode via `#[cfg(test)]` with in-memory `Mutex<HashMap>` stores, and `#[cfg(not(test))]` for real Postgres.
- **`mod.rs`** — Exports `pub fn router() -> Router` that constructs the Axum router for the feature.

### AppState (Dependency Injection)
All handlers receive state via `Extension(st): Extension<AppState>`. AppState holds `PgPool`, repositories (`HostRepository`, `ImageRepository`, etc.), `LocalStorage`, and config flags. Constructed in `main.rs`.

### Shared Type Conventions
Types in `crates/nexus-types` derive `#[derive(Serialize, Deserialize, ToSchema)]`. Optional fields use `#[serde(default, skip_serializing_if = "Option::is_none")]`. IDs are `uuid::Uuid`, timestamps are `chrono::DateTime<Utc>`.

### Frontend Conventions
- **API client**: `apps/ui/lib/api/facade.ts` (FacadeApi class) and `apps/ui/lib/api/http.ts` (ApiClient with fetch wrapper)
- **React Query hooks**: `apps/ui/lib/queries.ts` — all API calls go through hooks, components never call API directly
- **Query keys**: Namespaced tuples in `queryKeys` object: `["vms"]`, `["vms", id]`, `["vms", vmId, "drives"]`
- **Types**: `apps/ui/lib/types/index.ts` — TypeScript interfaces mirror Rust types
- **Components**: Organized by feature in `apps/ui/components/` (e.g., `vm/`, `function/`, `container/`)
- **Pages**: App Router in `apps/ui/app/(dashboard)/` with `page.tsx` per route

### Naming Conventions
- Database columns: `snake_case`
- Rust files/modules: `snake_case`
- TypeScript interfaces: `PascalCase`
- TypeScript files: `kebab-case` (e.g., `vm-list.tsx`)
- API routes: `/v1/resource` with `:id` params

### Router Registration
New feature routers are registered in `apps/manager/src/features/mod.rs` via `.nest("/v1/your_feature", your_feature::router())`. Auth middleware applied via `.layer()` as needed.

## Adding a New Feature Module
1. Create directory: `apps/manager/src/features/your_feature/`
2. Add `mod.rs`, `routes.rs`, `service.rs`, `repo.rs`
3. Register router in `apps/manager/src/features/mod.rs`
4. Add database migration: `(cd apps/manager && sqlx migrate add your_feature)`
5. Add shared types to `crates/nexus-types/src/lib.rs` with utoipa annotations
6. Add React Query hooks in `apps/ui/lib/queries.ts`
7. Add TypeScript types in `apps/ui/lib/types/index.ts`
8. Create UI components in `apps/ui/components/your_feature/`
9. Add page route in `apps/ui/app/(dashboard)/your_feature/page.tsx`

## Key Environment Variables

### Manager
- `DATABASE_URL`: PostgreSQL connection string (required)
- `MANAGER_BIND`: Bind address (default: `127.0.0.1:18080`)
- `MANAGER_IMAGE_ROOT`: Image storage path (default: `/srv/images`)
- `MANAGER_STORAGE_ROOT`: VM storage path (default: `/srv/fc/vms`)
- `MANAGER_ALLOW_IMAGE_PATHS`: Allow direct file paths for images (default: false)
- `MANAGER_RECONCILER_DISABLED`: Disable VM reconciler (default: false)
- `MANAGER_METRICS_DISABLED`: Disable metrics collector (default: false)

### Agent
- `AGENT_BIND`: Bind address (default: `127.0.0.1:9090`)
- `FC_RUN_DIR`: Firecracker runtime directory (default: `/srv/fc`)
- `FC_BRIDGE`: Network bridge name (default: `fcbr0`)
- `MANAGER_BASE`: Manager API base URL (required)

### Frontend UI
- `NEXT_PUBLIC_API_BASE_URL`: Manager API URL (default: auto-detected from hostname)
- `NEXT_PUBLIC_WS_BASE_URL`: WebSocket URL (default: `ws://localhost:8000`)

## Important Technical Details

### Database
- Migrations in `apps/manager/migrations/`, auto-run on startup via `sqlx::migrate!()`
- SQLx compile-time query checking enabled — queries are verified at build time
- Migration 10 may need manual reset: `psql $DATABASE_URL -c "DELETE FROM _sqlx_migrations WHERE version = 10;"`

### Network Bridging
- VMs require `fcbr0` bridge. Two modes: NAT (isolated) or Bridged (network-visible)
- Setup: `sudo ./scripts/fc-bridge-setup.sh fcbr0 <interface>`

### WebSocket Endpoints
- Shell: `GET /v1/vms/{id}/shell/ws` (xterm.js terminal)
- Metrics: `GET /v1/vms/{id}/metrics/ws` (real-time stream)

### Container Runtime
- Build image: `sudo scripts/build-container-runtime-v2.sh`
- Alpine Linux 3.18 + Docker 25.0.5 + OpenRC at `/srv/images/container-runtime.ext4`
