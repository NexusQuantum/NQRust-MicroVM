# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

### Build and Run
- Build entire workspace: `cargo build`
- Build specific package: `cargo build -p manager` or `cargo build -p agent`
- Run manager: `(cd apps/manager && cargo run)`
- Run agent: `sudo -E env AGENT_BIND=127.0.0.1:9090 MANAGER_BASE=http://127.0.0.1:18080 FC_RUN_DIR=/srv/fc FC_BRIDGE=fcbr0 ./target/debug/agent`
- Run tests: `cargo test`
- Run specific package tests: `cargo test -p manager`

### Database Operations (Manager)
- Install SQLx CLI: `cargo install sqlx-cli --no-default-features --features postgres`
- Run migrations: `(cd apps/manager && sqlx migrate run)`
- Create migration: `(cd apps/manager && sqlx migrate add migration_name)`
- Revert migration: `(cd apps/manager && sqlx migrate revert)`
- The manager runs migrations automatically on startup

### Frontend Development
The project has TWO frontends (note: `apps/frontend` is the old Next.js 14 version):
- **Primary UI** (apps/ui): Next.js 15 with React 19, shadcn/ui, Tailwind CSS 4
  - Install: `(cd apps/ui && pnpm install)`
  - Dev mode: `(cd apps/ui && NEXT_PUBLIC_API_BASE_URL=http://127.0.0.1:18080/v1 pnpm dev)`
  - Build: `(cd apps/ui && pnpm build)`
  - Start: `(cd apps/ui && pnpm start)`
  - URL: http://localhost:3000

### Development Setup
1. Start PostgreSQL: `./scripts/dev-up.sh` (starts via Docker)
2. Set up bridge: `sudo ./scripts/fc-bridge-setup.sh fcbr0 <uplink-iface>` (once per host)
3. Configure environment variables (see `.env.example`)
4. Start agent first (needs sudo for KVM), then manager
5. (Optional) Create runtime snapshot for fast containers: `./scripts/create-runtime-snapshot.sh` (reduces container creation from 60-120s to 5-15s)

## Architecture Overview

NQRust-MicroVM is a Rust-based Firecracker microVM management platform with container and function support.

### Manager (`apps/manager`)
- **Central orchestration service** managing VM lifecycle, containers, and functions
- **Technology**: Axum (Rust async web framework), PostgreSQL (SQLx ORM)
- **Port**: 18080 (default)
- **Features**:
  - VM lifecycle management (create, start, stop, pause, resume, delete)
  - Docker container orchestration (container-per-VM architecture)
  - Serverless functions (Node.js/Python/Ruby via microVM isolation)
  - Image registry (kernel, rootfs, container runtime images)
  - Snapshots and templates
  - WebSocket support (shell access, real-time metrics)
  - Reconciler for VM health monitoring
  - Guest agent integration for in-VM metrics

**Key Modules** (`apps/manager/src/features/`):
- `vms/` - VM lifecycle, configuration, shell access, metrics
- `containers/` - Docker container management via Firecracker VMs
- `runtime_snapshots/` - Pre-warmed VM snapshots for fast container creation (60s → 5s)
- `functions/` - Serverless function execution
- `images/` - Image registry (upload, dockerhub, preload)
- `snapshots/` - VM snapshot operations
- `templates/` - VM template management (backend complete, UI needs implementation)
- `hosts/` - Agent registration and heartbeat
- `networks/` - Network registry with VLAN support and auto-registration
- `volumes/` - Volume registry with attachment tracking and auto-registration
- `users/` - User management (future RBAC foundation)
- `reconciler/` - Background VM health checks

### Agent (`apps/agent`)
- **Runs on KVM hosts** to execute VM operations via Firecracker
- **Port**: 9090 (default)
- **Requires**: Root privileges for KVM access
- **Functions**:
  - Registers with manager on startup
  - Sends periodic heartbeats
  - Communicates with Firecracker VMM via Unix domain sockets
  - Handles VM creation, lifecycle management, snapshots
  - Proxies shell access via screen sessions

### Guest Agent (`apps/guest-agent`)
- **Runs inside VMs** to report metrics and status
- **Port**: 9000 (inside VM)
- **Auto-deployed**: Manager installs it during VM creation
- **Functions**:
  - Reports CPU, memory, uptime, load average, process count
  - Auto-reports VM IP address to manager
  - Enables real-time guest metrics via WebSocket

### Frontend UI (`apps/ui`)
- **Technology**: Next.js 15, React 19, TypeScript, shadcn/ui, Tailwind CSS 4
- **State Management**: TanStack Query (React Query)
- **Real-time**: WebSocket for terminal and metrics
- **Pages**:
  - `/dashboard` - Resource overview and analytics
  - `/vms` - VM list and creation wizard
  - `/vms/[id]` - VM detail (7 tabs: overview, terminal, metrics, storage, network, snapshots, config)
  - `/containers` - Container management
  - `/functions` - Serverless function management with code editor
  - `/registry` - Image registry and DockerHub browser
  - `/hosts` - Host monitoring and management
  - `/networks` - Network registry and VLAN management
  - `/volumes` - Volume registry and attachment management
  - `/templates` - VM templates (page exists, needs component implementation)
  - `/users` - User management (infrastructure exists, UI pending)

### Shared Types (`crates/nexus-types`)
- Common data structures used by manager and agent
- VM creation requests, templates, snapshots, images, host registration
- Ensures type safety across service boundaries

## Key Features

### VM Management
- **Creation**: 5-step wizard (info → credentials → machine config → boot source → network)
- **Lifecycle**: Start, stop, pause, resume operations
- **Configuration**: CPU, memory, drives, NICs, machine config
- **Monitoring**: Real-time metrics (CPU, memory, network, disk)
- **Shell Access**: Browser-based terminal via WebSocket
- **Snapshots**: Full and differential snapshots with instant restore
- **Templates**: Reusable VM configurations

### Container Management
- **Architecture**: Container-per-VM (each Docker container runs in isolated Firecracker VM)
- **Runtime**: Alpine Linux + Docker daemon in microVM
- **API**: Full Docker Remote API compatibility
- **Networking**: Bridge networking for external access
- **Isolation**: Strong kernel-level isolation via Firecracker
- **Status**: Backend fully implemented, frontend UI needs implementation

### Infrastructure Management
- **Networks**: Central network registry with VLAN support (802.1Q)
  - Auto-registration: Networks automatically registered when VMs are created
  - Bridge networking with optional VLAN tagging
  - Full CRUD operations via API and UI
  - See `NETWORKING.md` for details
- **Volumes**: Central volume registry with attachment tracking
  - Auto-registration: Rootfs volumes automatically registered during VM creation
  - Support for ext4, qcow2, raw formats
  - Attach/detach volumes to/from VMs
  - See `VOLUMES.md` for details
- **Hosts**: Agent registration and metrics monitoring
  - Real-time metrics: CPU, memory, network, uptime
  - Heartbeat monitoring for health checks
  - Multi-host support for distributed deployments

### Templates
- **Backend**: Fully implemented (create, list, update, delete, instantiate)
- **Frontend**: Partial implementation - UI components need to be built
- **Status**: Ready for frontend integration
- **See**: `TEMPLATES.md` for complete API reference and implementation guide

### Function Management
- **Runtimes**: Node.js, Python, Ruby
- **Isolation**: Each function runs in dedicated microVM
- **Execution**: HTTP invocation with request/response
- **Logs**: Function execution logs and stdout/stderr capture
- **Editor**: Browser-based Monaco editor in UI

### Image Registry
- **Types**: Kernel, rootfs, container runtime images
- **Sources**: Local file upload, Docker Hub browser, direct paths
- **Preloading**: Script to preload common images
- **Management**: List, import, delete operations

## Key Environment Variables

### Manager
- `DATABASE_URL`: PostgreSQL connection string (required)
- `MANAGER_BIND`: Bind address (default: 127.0.0.1:18080)
- `MANAGER_IMAGE_ROOT`: Image storage path (default: /srv/images)
- `MANAGER_STORAGE_ROOT`: VM storage path (default: /srv/fc/vms)
- `MANAGER_ALLOW_IMAGE_PATHS`: Allow direct file paths for images (default: false)
- `MANAGER_RECONCILER_DISABLED`: Disable VM reconciler (default: false)
- `MANAGER_HOST_ID`: Optional host ID for manager self-registration
- `MANAGER_BRIDGE`: Bridge name for self-registration (default: fcbr0)

### Agent
- `AGENT_BIND`: Bind address (default: 127.0.0.1:9090)
- `FC_RUN_DIR`: Firecracker runtime directory (default: /srv/fc)
- `FC_BRIDGE`: Network bridge name (default: fcbr0)
- `MANAGER_BASE`: Manager API base URL (required)

### Frontend UI
- `NEXT_PUBLIC_API_BASE_URL`: Manager API URL (default: /api/proxy/v1)
- `NEXT_PUBLIC_WS_BASE_URL`: WebSocket URL (default: ws://localhost:8000)
- `NEXT_PUBLIC_BRAND_PRESET`: Theme preset (dark/light)

## Important Technical Details

### Database Migrations
- Located in `apps/manager/migrations/`
- Run automatically by manager on startup
- SQLx compile-time query checking enabled
- Migration 10 may need manual reset if issues occur: `psql $DATABASE_URL -c "DELETE FROM _sqlx_migrations WHERE version = 10;"`
- Recent migrations:
  - 0015: Host metrics (CPU, memory, network stats)
  - 0016: Networks table (bridge networks with VLAN support)
  - 0017: Volumes table (central volume registry with attachments)
  - 0018: Users table (authentication and user management)
  - 0019: NIC assigned IP tracking
  - 0020: RBAC permissions (role-based access control)
  - 0021: Resource ownership (link resources to users)

### Network Bridging
- VMs require `fcbr0` bridge for networking
- Two modes: NAT (isolated) or Bridged (network-visible)
- Setup script: `./scripts/fc-bridge-setup.sh fcbr0 <interface>`
- Bridged mode allows VMs to get DHCP IPs from router

### Container Runtime
- Must build container runtime image: `sudo scripts/build-container-runtime-v2.sh`
- Image location: `/srv/images/container-runtime.ext4`
- Alpine Linux 3.18 + Docker 25.0.5 + OpenRC
- Size: ~386MB

### Guest Agent Installation
- Manager auto-installs guest agent during VM creation
- Binary: `target/x86_64-unknown-linux-musl/release/guest-agent`
- Config: `/etc/guest-agent.conf` (inside VM)
- Service auto-starts on VM boot (OpenRC/systemd/sysvinit)

### WebSocket Endpoints
- Shell: `GET /v1/vms/{id}/shell/ws` - xterm.js terminal
- Metrics: `GET /v1/vms/{id}/metrics/ws` - Real-time metrics stream

### Reconciler
- Background task that checks VM health
- Marks stale VMs as failed if agent stops responding
- Can be disabled via `MANAGER_RECONCILER_DISABLED=1`

## Common Development Tasks

### Adding a New Feature Module
1. Create module directory in `apps/manager/src/features/your_feature/`
2. Add `mod.rs`, `routes.rs`, `service.rs`, `repo.rs` as needed
3. Register router in `apps/manager/src/features/mod.rs`
4. Add database migrations if needed (use `sqlx migrate add` in apps/manager)
5. Update OpenAPI docs with utoipa annotations
6. Add corresponding React Query hooks in `apps/ui/lib/queries.ts`
7. Create UI components in `apps/ui/components/your_feature/`
8. Add page route in `apps/ui/app/(dashboard)/your_feature/page.tsx`

### Working with Frontend
- API client: `apps/ui/lib/api/facade.ts`
- React Query hooks: `apps/ui/lib/queries.ts`
- Components: `apps/ui/components/` (organized by feature)
- Types: `apps/ui/lib/types/index.ts`

### Testing VMs Locally
1. Ensure agent is running with proper permissions (sudo)
2. Create VM via API or UI
3. Check VM status: `curl http://localhost:18080/v1/vms`
4. Access shell via WebSocket in UI
5. View logs: Manager logs show VM lifecycle events

## File Structure Reference

```
apps/
├── manager/          # Central orchestration service
│   ├── src/
│   │   ├── features/ # Feature modules (vms, containers, functions, etc.)
│   │   ├── core/     # Shared utilities
│   │   └── main.rs   # Entry point
│   └── migrations/   # Database migrations
├── agent/            # Host agent for Firecracker
│   └── src/
│       ├── core/     # Firecracker interaction
│       └── features/ # VM lifecycle, shell proxy
├── guest-agent/      # In-VM metrics agent
│   └── src/main.rs
├── ui/               # Primary Next.js 15 frontend
│   ├── app/          # App router pages
│   ├── components/   # React components
│   └── lib/          # API client, queries, types
├── frontend/         # Legacy Next.js 14 frontend (deprecated)
└── function-runtime/ # Function execution runtime

crates/
└── nexus-types/      # Shared types between services

scripts/
├── build-container-runtime-v2.sh  # Build container image
├── fc-bridge-setup.sh             # Network bridge setup
└── preload-docker-images.sh       # Preload common images

openapi/
└── manager/
    └── openapi.yaml  # Auto-generated API spec
```

## Documentation Files

- `README.md` - Installation and setup guide
- `RUN.md` - Quick development commands
- `FEATURES.md` - Detailed feature matrix and integration status
- `CONTAINER.md` - Container feature documentation
- `NETWORKING.md` - Network management, VLAN support, and auto-registration
- `TEMPLATES.md` - VM templates feature documentation and implementation guide
- `VOLUMES.md` - Volume management, storage, and attachment tracking
- `PERFORMANCE_OPTIMIZATION.md` - Performance tuning guide
- `QUICK_START_OPTIMIZATION.md` - Quick start improvements
- `EVALUATION.md` - Project evaluation and notes
