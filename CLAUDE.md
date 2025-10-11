# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

### Build and Run
- Build entire workspace: `cargo build`
- Run manager: `(cd apps/manager && cargo run)`
- Run agent: `(cd apps/agent && cargo run)`
- Run tests: `cargo test`

### Database Operations (Manager)
- Run migrations: `(cd apps/manager && sqlx migrate run)`
- The manager requires PostgreSQL and will run migrations on startup

### Development Setup
1. Start infrastructure: `./scripts/dev-up.sh` (starts PostgreSQL via Docker)
2. Set up bridge (once per host): `sudo ./scripts/fc-bridge-setup.sh fcbr0 <uplink-iface>`
3. Configure environment variables (see `.env.example`)
4. Start agent first, then manager

## Architecture Overview

NQRust-MicroVM is a Rust-based microVM management system with two main components:

### Manager (`apps/manager`)
- Central orchestration service that manages VM lifecycle
- Uses PostgreSQL for persistence with SQLx migrations
- Provides REST API for VM, template, snapshot, and image management
- Includes a reconciler component for background tasks
- Manages agent registration and heartbeat monitoring

### Agent (`apps/agent`)
- Runs on KVM hosts to execute VM operations via Firecracker
- Registers with manager and sends periodic heartbeats
- Communicates with Firecracker VMM through Unix domain sockets
- Handles VM creation, lifecycle management, and snapshot operations

### Shared Types (`crates/nexus-types`)
- Common data structures and API types used by both manager and agent
- Includes VM creation requests, templates, snapshots, images, and host registration

## Key Environment Variables

### Manager
- `DATABASE_URL`: PostgreSQL connection string
- `MANAGER_BIND`: Bind address (default: 127.0.0.1:8080)
- `MANAGER_IMAGE_ROOT`: Image storage path (default: /srv/images)
- `MANAGER_ALLOW_IMAGE_PATHS`: Allow direct file paths for images

### Agent
- `AGENT_BIND`: Bind address (default: 127.0.0.1:9090)
- `FC_RUN_DIR`: Firecracker runtime directory (default: /srv/fc)
- `FC_BRIDGE`: Network bridge name (default: fcbr0)
- `MANAGER_BASE`: Manager API base URL

## VM Management Features

The system supports:
- VM templates for standardized deployments
- VM snapshots and restoration
- Image management (kernel and rootfs)
- Host registration and capability reporting
- Network bridge setup for VM connectivity

VMs can be created from templates, direct image references, or restored from snapshots.