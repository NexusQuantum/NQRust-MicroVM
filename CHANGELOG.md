# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2024-XX-XX

### Added

#### Core Platform
- **Manager Service**: Central orchestration service for VM lifecycle management
  - REST API (Axum-based) for VM operations
  - PostgreSQL database with SQLx ORM
  - Automatic database migrations on startup
  - OpenAPI/Swagger documentation
  - Health check endpoint

- **Agent Service**: Host-level VM execution via Firecracker
  - KVM-based microVM isolation
  - Firecracker VMM integration
  - Unix domain socket communication
  - Multi-host support with auto-registration
  - Heartbeat monitoring

- **Guest Agent**: In-VM metrics collection
  - CPU, memory, disk, network metrics
  - Automatic IP reporting to manager
  - Static musl binary for portability
  - Auto-start on VM boot (systemd/OpenRC/sysvinit)

#### Virtual Machine Management
- Complete VM lifecycle (create, start, stop, pause, resume, delete)
- 5-step VM creation wizard
- Multiple boot source support (kernel + rootfs)
- Configurable vCPU and memory allocation
- Multiple network interfaces (NICs)
- Multiple block devices (drives)
- Browser-based terminal access via WebSocket
- Real-time metrics streaming via WebSocket
- VM snapshots (full and differential)
- Snapshot restore functionality
- VM templates for reusable configurations

#### Linux Distribution Support
- **Alpine Linux** (minimal, musl-based)
- **BusyBox** (ultra-minimal)
- **Ubuntu 24.04 LTS** (systemd, cloud-init)
- **Debian 12 Bookworm** (systemd, cloud-init)
- Distribution-aware credential injection
- Automatic init system detection
- Cloud-init support for Ubuntu/Debian
- Build scripts for custom rootfs images

#### Container Management
- Container-per-VM architecture for strong isolation
- Docker Remote API compatibility
- Alpine Linux + Docker daemon in microVM
- Bridge networking for external access
- Container lifecycle management
- Image pull from registries

#### Serverless Functions
- Function execution in isolated microVMs
- Runtime support: Node.js, Python, Ruby
- HTTP invocation API
- Function logs and stdout/stderr capture
- Browser-based code editor (Monaco)
- Automatic function packaging

#### Infrastructure Management
- **Networks Registry**
  - Bridge network management
  - VLAN support (802.1Q tagging)
  - Auto-registration on VM creation
  - NAT and bridged modes

- **Volumes Registry**
  - Central volume tracking
  - Support for ext4, qcow2, raw formats
  - Volume attachment/detachment
  - Auto-registration for rootfs

- **Hosts Management**
  - Agent registration and discovery
  - Real-time host metrics
  - Heartbeat monitoring
  - Multi-datacenter support

- **Image Registry**
  - Kernel and rootfs image management
  - DockerHub browser integration
  - Local file upload support
  - Image metadata tracking

#### Web UI (Next.js 15)
- Modern React 19 with TypeScript
- shadcn/ui component library
- Tailwind CSS 4 styling
- TanStack Query for state management
- Dashboard with resource overview
- VM management pages
- Container management UI
- Function editor with syntax highlighting
- Real-time terminal (xterm.js)
- Live metrics charts
- Responsive design

#### Installation & Deployment
- Automated installer script
- Multiple installation modes (production, dev, manager, agent)
- systemd service integration
- Network bridge auto-setup
- Firecracker binary management
- Database setup automation
- Uninstaller with cleanup options

#### CI/CD Pipeline
- GitHub Actions workflows
- Lint checks (rustfmt, clippy)
- Unit and integration tests
- Multi-profile builds (debug, release)
- UI build verification
- Security audit (cargo-audit)
- Shell script validation (shellcheck)
- Self-hosted runner support

### Security
- Firecracker microVM isolation (kernel-level)
- Per-VM network namespacing
- No shared kernel between VMs
- Secure credential injection
- Input validation on all API endpoints

### Performance
- sccache for faster Rust compilation
- Cargo workspace caching
- Optimized LLD linker
- Stripped release binaries
- Static musl builds for guest-agent

### Documentation
- Comprehensive README
- Feature documentation (FEATURES.md)
- Container guide (CONTAINER.md)
- Network management guide (NETWORKING.md)
- Volume management guide (VOLUMES.md)
- Template documentation (TEMPLATES.md)
- Quick start guide (RUN.md)
- Development instructions (CLAUDE.md)

### Known Limitations
- x86_64 architecture only (no ARM support yet)
- Single region deployment
- No built-in authentication/authorization (planned for v0.2.0)
- Container UI needs implementation
- Template UI partially implemented
- No live migration support

### Requirements
- Linux host with KVM support
- Ubuntu 22.04+ / Debian 11+ / RHEL 8+
- 2GB+ RAM minimum
- 20GB+ disk space
- PostgreSQL 14+
- Node.js 20+ (for UI)

---

## Future Plans

### v0.2.0 (Planned)
- User authentication and RBAC
- Multi-tenant support
- Resource quotas
- API rate limiting
- Audit logging

### v0.3.0 (Planned)
- ARM64 support
- GPU passthrough
- Custom kernel support
- Performance monitoring dashboard
- Alerting system

[Unreleased]: https://github.com/user/nqrust-microvm/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/user/nqrust-microvm/releases/tag/v0.1.0
