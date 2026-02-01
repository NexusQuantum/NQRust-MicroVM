# Development Setup Guide

Quick guide to set up your development environment in one command.

## 🚀 One-Command Setup

### Full Setup (Includes Network Configuration)

```bash
./scripts/dev-setup-complete.sh
```

This script will automatically:
1. ✅ Check prerequisites (Rust, Docker, KVM)
2. ✅ Download Firecracker v1.13.1
3. ✅ Set up network bridge (NAT or Bridged mode)
4. ✅ Start PostgreSQL in Docker
5. ✅ Create `.env` configuration file
6. ✅ Create required directories (`/srv/fc`, `/srv/images`)
7. ✅ Build the entire project
8. ✅ Run database migrations
9. ✅ Download/build runtime images (~3GB)

### Setup Without Network Configuration

```bash
./scripts/dev-setup-no-network.sh
```

Use this if you've already configured the network bridge or want to do it manually later. This script does everything except step 3 (network bridge setup).

Perfect for:
- Re-running setup after network is already configured
- When you want to configure the network manually
- Running on systems where network is pre-configured

## Prerequisites

Before running the setup script, ensure you have:

- **Rust** (1.70+): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- **Docker**: https://docs.docker.com/engine/install/
- **KVM**: Check with `ls -l /dev/kvm`
  - Enable in BIOS if needed
  - Load modules: `sudo modprobe kvm kvm_intel` (or `kvm_amd`)
  - Add user to kvm group: `sudo usermod -aG kvm $USER && newgrp kvm`
- **musl-tools**: For building static guest-agent binaries
  - Install: `sudo apt install musl-tools`
  - Optional: Debug builds work without this
- **curl**, **tar**, **jq** (optional but recommended)

## Network Modes

The script will ask you to choose a network mode:

### NAT Mode (Recommended for Development)
- VMs get isolated network: 10.0.0.0/24
- No impact on host network
- VMs can access internet via NAT
- Good for testing and development

### Bridged Mode
- VMs get IPs from your router
- VMs visible on physical network
- Required for production-like testing
- May require network reconfiguration

## What Gets Installed

### Binaries
- Firecracker v1.13.1 (`/usr/local/bin/firecracker` or local)
- Manager (`target/debug/manager`)
- Agent (`target/debug/agent`)
- Guest Agent (`target/x86_64-unknown-linux-musl/release/guest-agent`)

### Services
- PostgreSQL (Docker container on port 5432)
  - User: `nexus`
  - Password: `nexus`
  - Database: `nexus`

### Directories
- `/srv/fc/vms` - VM storage
- `/srv/images` - Image registry (kernel, rootfs, container runtime)

### Runtime Images
The script can download these from GitHub releases:
- `vmlinux-5.10.fc.bin` - Firecracker kernel
- `alpine-3.18-minimal.ext4` - Alpine Linux minimal
- `ubuntu-24.04-minimal.ext4` - Ubuntu minimal
- `busybox-1.35.ext4` - BusyBox
- `python-runtime.ext4` - Python function runtime
- `bun-runtime.ext4` - JavaScript/TypeScript function runtime
- `container-runtime.ext4` - Alpine + Docker (~2GB compressed)

## After Setup

Start the services in separate terminals:

### Terminal 1 - Manager
```bash
cd apps/manager && cargo run
```

### Terminal 2 - Agent (requires sudo)
```bash
sudo -E env \
  AGENT_BIND=127.0.0.1:9090 \
  MANAGER_BASE=http://127.0.0.1:18080 \
  FC_RUN_DIR=/srv/fc \
  FC_BRIDGE=fcbr0 \
  ./target/debug/agent
```

### Terminal 3 - Frontend UI (optional)
```bash
cd apps/ui && pnpm install
NEXT_PUBLIC_API_BASE_URL=http://127.0.0.1:18080/v1 pnpm dev
```

## Access Points

- **Manager API**: http://127.0.0.1:18080
- **API Documentation**: http://127.0.0.1:18080/scalar
- **Frontend UI**: http://localhost:3000

## Quick Verification

```bash
# Check manager
curl http://127.0.0.1:18080/v1/vms

# Check agent registration
curl http://127.0.0.1:18080/v1/hosts

# Check images
curl http://127.0.0.1:18080/v1/images

# Check PostgreSQL
docker ps | grep postgres

# Check network bridge
ip link show fcbr0
```

## Troubleshooting

### PostgreSQL Won't Start
```bash
# Check if port 5432 is already in use
sudo netstat -tlnp | grep 5432

# Stop existing PostgreSQL
docker stop $(docker ps -q -f ancestor=postgres:16)

# Restart
./scripts/dev-up.sh
```

### KVM Access Denied
```bash
# Check permissions
ls -l /dev/kvm

# Add user to kvm group
sudo usermod -aG kvm $USER
newgrp kvm

# Or create the group if it doesn't exist
sudo groupadd kvm
sudo chown root:kvm /dev/kvm
sudo chmod 660 /dev/kvm
```

### Bridge Network Issues
```bash
# Check bridge status
ip link show fcbr0
ip addr show fcbr0

# Recreate bridge (NAT mode)
sudo ip link delete fcbr0
sudo ip link add fcbr0 type bridge
sudo ip link set fcbr0 up

# Enable IP forwarding
sudo sysctl -w net.ipv4.ip_forward=1

# Check NAT rule
sudo iptables -t nat -L POSTROUTING -n -v
```

### Build Errors
```bash
# Update Rust
rustup update

# Clean and rebuild
cargo clean
cargo build

# Check for missing dependencies
rustup target list | grep installed
```

### Image Download Fails
```bash
# Run image setup separately
./scripts/dev-setup-images.sh

# Or build container runtime manually
sudo ./scripts/build-container-runtime-v2.sh

# Download kernel manually
curl -L https://github.com/firecracker-microvm/firecracker/releases/download/v1.13.1/vmlinux-5.10-x86_64.bin \
  -o /srv/images/vmlinux-5.10.fc.bin
```

## Manual Setup (Alternative)

If you prefer to set up components individually:

```bash
# 1. PostgreSQL
./scripts/dev-up.sh

# 2. Bridge network
sudo ./scripts/fc-bridge-setup.sh fcbr0 eth0

# 3. Build project
cargo build

# 4. Download images
./scripts/dev-setup-images.sh

# 5. Create directories
sudo mkdir -p /srv/fc/vms /srv/images
sudo chown -R $USER:$USER /srv/fc /srv/images
```

## Cleanup

To clean up the development environment:

```bash
# Stop PostgreSQL
docker stop $(docker ps -q -f ancestor=postgres:16)
docker rm $(docker ps -aq -f ancestor=postgres:16)

# Remove bridge
sudo ip link delete fcbr0

# Remove directories (optional)
sudo rm -rf /srv/fc /srv/images

# Remove binaries (optional)
cargo clean
```

## See Also

- [README.md](README.md) - Full project documentation
- [RUN.md](RUN.md) - Development commands reference
- [CLAUDE.md](CLAUDE.md) - Claude Code integration guide
- [FEATURES.md](FEATURES.md) - Feature matrix
