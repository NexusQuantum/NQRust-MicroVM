# NQRust-MicroVM

A Rust-based Firecracker microVM management system with a modern web frontend. Manage lightweight VMs with VM templates, snapshots, shell access, and bridged networking.

## Features

- 🚀 **VM Lifecycle Management** - Create, start, stop, pause, and delete VMs
- 🐳 **Docker Container Management** - Run Docker containers in isolated Firecracker VMs
- 📦 **Templates & Snapshots** - Reusable VM templates and snapshot management
- 🖥️ **Web-based Shell** - Browser-based terminal access to VMs via WebSocket
- 🔐 **Credential Injection** - Automatic username/password injection (rootfs + cloud-init)
- 🌐 **Bridged Networking** - VMs can get IPs directly from your network router
- 📊 **Image Registry** - Manage kernel and rootfs images
- 🏥 **Health Monitoring** - Agent heartbeat and VM reconciliation
- 🎯 **Multi-Host Support** - Manage VMs across multiple Firecracker hosts

## Architecture

- **Manager** - Central orchestration service (REST API + PostgreSQL)
- **Agent** - Runs on KVM hosts to manage Firecracker VMs
- **Frontend** - Next.js web UI with real-time terminal
- **Container Runtime** - Alpine Linux + Docker for container isolation

## Requirements

### System Requirements

- **OS**: Linux (Ubuntu 22.04+ recommended)
- **CPU**: x86_64 with KVM support
- **RAM**: 2GB+ (4GB+ recommended)
- **Disk**: 20GB+ free space for VM images

### Software Dependencies

All dependencies are documented here for future installer development.

## Installation

### 1. Install System Dependencies

```bash
# Update package list
sudo apt update

# Install required packages
sudo apt install -y \
  build-essential \
  pkg-config \
  libssl-dev \
  curl \
  git \
  postgresql \
  postgresql-contrib \
  screen \
  openssl \
  iproute2 \
  iptables \
  bridge-utils
```

**Package Purposes**:
- `build-essential`, `pkg-config`, `libssl-dev` - Rust compilation
- `curl`, `git` - Downloading tools and code
- `postgresql`, `postgresql-contrib` - Manager database
- `screen` - VM console access (PTY management)
- `openssl` - Password hashing for credential injection
- `iproute2`, `iptables`, `bridge-utils` - Network management

### 2. Install Rust

```bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Follow prompts and select default installation
# Then reload shell or run:
source "$HOME/.cargo/env"

# Verify installation
rustc --version
cargo --version
```

**Version**: Rust 1.70+

### 3. Install Firecracker

Firecracker is the core microVM hypervisor that powers this system.

```bash
# Set Firecracker version
FIRECRACKER_VERSION="v1.13.1"

# Download Firecracker binary
curl -L -o firecracker.tgz \
  "https://github.com/firecracker-microvm/firecracker/releases/download/${FIRECRACKER_VERSION}/firecracker-${FIRECRACKER_VERSION}-x86_64.tgz"

# Extract
tar xzf firecracker.tgz

# Install to system path
sudo mv release-${FIRECRACKER_VERSION}-x86_64/firecracker-${FIRECRACKER_VERSION}-x86_64 \
  /usr/local/bin/firecracker

# Clean up
rm -rf release-${FIRECRACKER_VERSION}-x86_64 firecracker.tgz

# Verify installation
firecracker --version
# Should output: Firecracker v1.13.1
```

**Important**: This system is tested with Firecracker v1.13.1. Other versions may work but are not guaranteed.

**Why Firecracker?**
- Lightweight: 125ms boot time, <5MB memory overhead
- Secure: Strong isolation using KVM
- Fast: Minimal device emulation

### 4. Install Node.js (for Frontend)

```bash
# Install Node.js 20.x LTS
curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
sudo apt install -y nodejs

# Verify installation
node --version  # Should be v20.x
npm --version   # Should be 10.x
```

### 5. Enable KVM

```bash
# Check if KVM is available
lsmod | grep kvm

# If empty, enable KVM
sudo modprobe kvm
sudo modprobe kvm_intel  # For Intel CPUs
# OR
sudo modprobe kvm_amd    # For AMD CPUs

# Add your user to kvm group
sudo usermod -a -G kvm $USER

# You may need to log out and back in for group changes to take effect
```

### 6. Setup PostgreSQL Database

```bash
# Switch to postgres user
sudo -u postgres psql

# Inside psql:
CREATE DATABASE nexus;
CREATE USER nexus WITH ENCRYPTED PASSWORD 'your-secure-password';
GRANT ALL PRIVILEGES ON DATABASE nexus TO nexus;

# Grant schema privileges (required for SQLx migrations)
\c nexus
GRANT ALL ON SCHEMA public TO nexus;

# Exit psql
\q
```

### 7. Clone and Build

```bash
# Clone repository
git clone https://github.com/yourusername/nqrust-microvm.git
cd nqrust-microvm

# Build backend (manager + agent)
cargo build --release

# The binaries will be in:
# - target/release/manager
# - target/release/agent
```

### 8. Install SQLx CLI (for migrations)

```bash
# Install sqlx-cli
cargo install sqlx-cli --no-default-features --features postgres

# Run database migrations
cd apps/manager
sqlx migrate run

# Verify migrations
sqlx migrate info
```

### 9. Setup Network Bridge

Choose one of two network modes:

#### Option A: NAT Mode (Simple, Isolated)
VMs hidden behind host IP, good for testing:

```bash
sudo ./scripts/fc-bridge-setup.sh fcbr0 eth0
# Replace eth0 with your interface name
```

#### Option B: Bridged Mode (Network-Visible)
VMs get IPs from your router, accessible from network:

```bash
# Setup bridge
sudo ./scripts/fc-bridge-physical.sh fcbr0 eth0

# Make persistent (edit first!)
sudo cp scripts/netplan-bridge-example.yaml /etc/netplan/01-fcbridge.yaml
sudo nano /etc/netplan/01-fcbridge.yaml  # Edit with your network details
sudo netplan try  # Test before applying

# See BRIDGED_NETWORK_SETUP.md for detailed guide
```

### 10. Create Storage Directories

```bash
# Create directories for VM storage
sudo mkdir -p /srv/fc/vms
sudo mkdir -p /srv/images

# Set permissions
sudo chown -R $USER:$USER /srv/fc
sudo chown -R $USER:$USER /srv/images
```

### 10.1. Build Container Runtime Image (Optional)

For Docker container support:

```bash
# Build container runtime image
sudo scripts/build-container-runtime-v2.sh

# Verify image exists
ls -lh /srv/images/container-runtime.ext4
```

See [CONTAINER.md](CONTAINER.md) for detailed container feature documentation.

### 11. Configure Environment

Create `.env` files for manager and agent:

#### Manager `.env` (apps/manager/.env):
```bash
DATABASE_URL=postgres://nexus:your-secure-password@localhost/nexus
MANAGER_BIND=0.0.0.0:8080
MANAGER_IMAGE_ROOT=/srv/images
MANAGER_ALLOW_IMAGE_PATHS=true
```

#### Agent `.env` (apps/agent/.env):
```bash
AGENT_BIND=0.0.0.0:9090
FC_RUN_DIR=/srv/fc
FC_BRIDGE=fcbr0
MANAGER_BASE=http://localhost:8080
```

### 12. Install and Build Frontend

```bash
cd apps/frontend

# Install dependencies
npm install

# Build for production
npm run build

# Or run in development mode
npm run dev
```

## Running the System

### Start Infrastructure

```bash
# Start PostgreSQL (if not already running)
sudo systemctl start postgresql
sudo systemctl enable postgresql
```

### Start Agent (on VM host)

```bash
cd apps/agent
cargo run --release

# Or run the binary directly
../../target/release/agent
```

**Agent must start before Manager** to register the host.

### Start Manager

```bash
cd apps/manager
cargo run --release

# Or run the binary directly
../../target/release/manager
```

Manager will:
- Connect to PostgreSQL
- Run pending migrations
- Start REST API on port 8080
- Start reconciler for VM health checks

### Start Frontend

```bash
cd apps/frontend

# Production mode
npm run build
npm start

# Development mode (with hot reload)
npm run dev
```

Frontend will be available at:
- **Production**: http://localhost:3000
- **Development**: http://localhost:3000

## Quick Start Guide

### 1. Upload Images

```bash
# Download a sample Ubuntu cloud image
wget https://cloud-images.ubuntu.com/minimal/releases/jammy/release/ubuntu-22.04-minimal-cloudimg-amd64.img

# Upload via API or web UI
# The frontend has an Image Registry where you can add images
```

### 2. Create Your First VM

1. Open web UI: http://localhost:3000
2. Go to "Create VM"
3. Fill in:
   - **Name**: my-first-vm
   - **Username**: root (default)
   - **Password**: changeme
   - **CPU**: 1 vCPU
   - **Memory**: 512 MiB
   - **Kernel**: Select from registry
   - **Rootfs**: Select from registry
4. Click "Create"

### 3. Access VM Shell

1. Go to VM details page
2. Click "Shell" tab
3. Login with credentials you set
4. You're in!

### 4. Check VM Networking

Inside VM shell:
```bash
# Check IP address
ip addr show eth0

# Test internet
ping -c 3 8.8.8.8

# For bridged mode, VM should have IP from your router
# For NAT mode, VM will have 10.x.x.x IP
```

### 5. Create Your First Container (Optional)

If you built the container runtime image:

```bash
# Create hello-world container
curl -X POST http://localhost:18080/v1/containers \
  -H "Content-Type: application/json" \
  -d '{"name": "hello-world", "image": "hello-world:latest"}'

# Check container status (wait 2-3 minutes)
curl http://localhost:18080/v1/containers/{id}

# Get container logs
curl http://localhost:18080/v1/containers/{id}/logs
```

See [CONTAINER.md](CONTAINER.md) for complete container API documentation.

## Project Structure

```
nqrust-microvm/
├── apps/
│   ├── manager/          # Central management service
│   │   ├── src/
│   │   │   ├── features/ # VM, template, snapshot, image management
│   │   │   ├── main.rs
│   │   │   └── ...
│   │   └── migrations/   # Database migrations
│   ├── agent/            # Host agent for Firecracker
│   │   ├── src/
│   │   │   ├── core/     # Firecracker interaction
│   │   │   ├── features/ # VM lifecycle, shell proxy
│   │   │   └── ...
│   │   └── ...
│   └── frontend/         # Next.js web UI
│       ├── app/          # App router pages
│       ├── components/   # React components
│       └── lib/          # API client, queries
├── crates/
│   └── nexus-types/      # Shared types between manager and agent
├── scripts/
│   ├── fc-bridge-setup.sh           # NAT mode bridge setup
│   ├── fc-bridge-physical.sh        # Bridged mode setup
│   ├── netplan-bridge-example.yaml  # Persistent bridge config
│   └── dev-up.sh                    # Start dev infrastructure
├── openapi/              # OpenAPI specs
├── BRIDGED_NETWORK_SETUP.md  # Network bridging guide
├── NETWORK_BRIDGING_PLAN.md  # Implementation plan
└── FEATURES_DOCUMENTATION.md # Feature documentation
```

## Configuration Reference

### Manager Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | Required | PostgreSQL connection string |
| `MANAGER_BIND` | `127.0.0.1:8080` | Manager API bind address |
| `MANAGER_IMAGE_ROOT` | `/srv/images` | Image storage directory |
| `MANAGER_ALLOW_IMAGE_PATHS` | `false` | Allow direct file paths for images |

### Agent Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `AGENT_BIND` | `127.0.0.1:9090` | Agent API bind address |
| `FC_RUN_DIR` | `/srv/fc` | Firecracker runtime directory |
| `FC_BRIDGE` | `fcbr0` | Network bridge name |
| `MANAGER_BASE` | Required | Manager API base URL |

## API Documentation

Once Manager is running, API documentation is available at:
- **Swagger UI**: http://localhost:8080/swagger-ui/

The API includes endpoints for:
- VM management (create, start, stop, delete)
- Templates (create, instantiate)
- Snapshots (create, restore)
- Images (upload, list)
- Host management (register, heartbeat)
- Shell access (WebSocket proxy)

## Advanced Features

### VM Templates

Create reusable VM configurations:

```bash
# Via API
curl -X POST http://localhost:8080/api/v1/templates \
  -H "Content-Type: application/json" \
  -d '{
    "name": "ubuntu-base",
    "spec": {
      "vcpu": 1,
      "mem_mib": 512,
      "kernel_image_id": "kernel-uuid",
      "rootfs_image_id": "rootfs-uuid"
    }
  }'

# Then instantiate
curl -X POST http://localhost:8080/api/v1/templates/{id}/instantiate \
  -H "Content-Type: application/json" \
  -d '{"name": "my-vm-from-template"}'
```

### VM Snapshots

Take snapshots of running VMs:

```bash
# Pause VM first
curl -X PATCH http://localhost:8080/api/v1/vms/{id}/pause

# Create snapshot
curl -X POST http://localhost:8080/api/v1/vms/{id}/snapshots \
  -H "Content-Type: application/json" \
  -d '{
    "name": "before-upgrade",
    "snapshot_type": "Full"
  }'

# Restore from snapshot
curl -X POST http://localhost:8080/api/v1/snapshots/{id}/instantiate \
  -H "Content-Type: application/json" \
  -d '{"name": "restored-vm"}'
```

### Credential Injection

The system uses two methods to inject credentials:

1. **Rootfs Injection** (Fallback)
   - Mounts rootfs before VM starts
   - Modifies `/etc/shadow` directly
   - Works with any Linux image

2. **Cloud-Init** (Preferred)
   - Uses Firecracker MMDS (Metadata Service)
   - Injects credentials + network config
   - Requires cloud-init in guest OS

Credentials are injected automatically during VM creation.

### Bridged Networking

See [BRIDGED_NETWORK_SETUP.md](BRIDGED_NETWORK_SETUP.md) for complete guide.

**Quick summary**:
- VMs can get IPs from your router via DHCP
- VMs appear on same network as host
- Requires cloud-init enabled images
- Network config injected via MMDS

## Development

### Run Tests

```bash
# Run all tests
cargo test

# Run specific package tests
cargo test -p manager
cargo test -p agent
```

### Database Migrations

```bash
# Create new migration
cd apps/manager
sqlx migrate add migration_name

# Run migrations
sqlx migrate run

# Revert last migration
sqlx migrate revert
```

### Frontend Development

```bash
cd apps/frontend

# Install dependencies
npm install

# Run dev server with hot reload
npm run dev

# Type check
npm run type-check

# Lint
npm run lint
```

## Troubleshooting

### Firecracker Permission Denied

```bash
# Ensure you're in kvm group
groups | grep kvm

# If not, add yourself and re-login
sudo usermod -a -G kvm $USER
```

### VM Console Not Working

```bash
# Check screen session
sudo screen -ls

# Attach to VM screen session
sudo screen -x fc-{vm-id}

# If screen not found, VM may not have started
```

### Network Issues

```bash
# Check bridge status
ip link show fcbr0
bridge link show

# Check if interface is in bridge
bridge link show | grep fcbr0

# Fix DNS if internet stopped working after bridge setup
sudo resolvectl dns fcbr0 192.168.18.1  # Your gateway
sudo resolvectl default-route fcbr0 yes

# For bridged mode troubleshooting, see BRIDGED_NETWORK_SETUP.md
```

### Database Connection Failed

```bash
# Check PostgreSQL is running
sudo systemctl status postgresql

# Test connection
psql -h localhost -U nexus -d nexus

# Check DATABASE_URL in .env matches PostgreSQL config
```

## Security Considerations

1. **Network Isolation**: NAT mode provides better isolation than bridged
2. **Credential Storage**: Shell credentials stored in PostgreSQL
3. **API Security**: Currently no authentication (add auth for production)
4. **KVM Isolation**: Firecracker provides strong VM isolation via KVM
5. **Rootfs Mounting**: Requires sudo for credential injection

## Performance Tips

1. **Memory Overcommit**: Enable for higher VM density
   ```bash
   sudo sysctl -w vm.overcommit_memory=1
   ```

2. **Huge Pages**: Improve VM memory performance
   ```bash
   sudo sysctl -w vm.nr_hugepages=512
   ```

3. **CPU Pinning**: Pin VMs to specific CPU cores for consistency

## Production Deployment

For production use, consider:

1. **Systemd Services**: Create service files for manager and agent
2. **Reverse Proxy**: Put nginx/traefik in front of manager API
3. **TLS**: Enable HTTPS for all API and WebSocket traffic
4. **Authentication**: Add JWT or OAuth2 to manager API
5. **Monitoring**: Add Prometheus metrics and Grafana dashboards
6. **Backup**: Regular PostgreSQL backups and VM snapshot rotation
7. **High Availability**: Run multiple manager instances with load balancer

## Dependencies Summary (for Installer)

### APT Packages
```
build-essential pkg-config libssl-dev curl git postgresql postgresql-contrib
screen openssl iproute2 iptables bridge-utils
```

### External Binaries
- Rust (via rustup): https://rustup.rs
- Firecracker v1.13.1: https://github.com/firecracker-microvm/firecracker/releases
- Node.js 20.x: https://nodejs.org
- sqlx-cli: `cargo install sqlx-cli`

### Cargo Dependencies
Managed by Cargo.toml, installed via `cargo build`

### NPM Dependencies
Managed by package.json, installed via `npm install`

### System Configuration
- KVM module loaded
- User in `kvm` group
- PostgreSQL database created
- Network bridge configured
- Storage directories created

## License

[Your License Here]

## Contributing

[Contributing guidelines]

## Support

For issues and questions:
- GitHub Issues: [your-repo-url]/issues
- Documentation: See docs in this repository
- Network Setup: See BRIDGED_NETWORK_SETUP.md
