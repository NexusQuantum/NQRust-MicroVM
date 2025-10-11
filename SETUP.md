# NQRust-MicroVM Setup Guide

This guide provides step-by-step instructions to set up and run the NQRust-MicroVM project.

## Prerequisites

- Rust and Cargo (installed)
- Docker (installed)
- Linux system with KVM support
- sudo access for network bridge setup

## Quick Setup

### 1. Download and Install Firecracker

```bash
# Download Firecracker v1.13.1
curl -L https://github.com/firecracker-microvm/firecracker/releases/download/v1.13.1/firecracker-v1.13.1-x86_64.tgz -o /tmp/firecracker.tgz

# Extract and install
cd /tmp && tar -xzf firecracker.tgz
sudo cp release-v1.13.1-x86_64/firecracker-v1.13.1-x86_64 /usr/local/bin/firecracker
sudo chmod +x /usr/local/bin/firecracker

# Or install locally in project
cp release-v1.13.1-x86_64/firecracker-v1.13.1-x86_64 /path/to/project/firecracker
chmod +x firecracker
export PATH="$PWD:$PATH"
```

### 2. Start PostgreSQL

```bash
# Fix docker-compose formatting and start PostgreSQL
./scripts/dev-up.sh
```

### 3. Set Up Environment Variables

```bash
# Copy and modify environment file
cp .env.example .env

# Edit .env if needed - default values should work for local development
# For non-sudo setup, use a user-writable directory for FC_RUN_DIR
```

### 4. Set Up Network Bridge (Requires sudo)

```bash
# Set up bridge network (replace eth0 with your network interface)
sudo ./scripts/fc-bridge-setup.sh fcbr0 eth0

# Check available network interfaces with:
ip link show
```

### 5. Create Required Directories

```bash
# Create Firecracker runtime directory
sudo mkdir -p /srv/fc
sudo chown $USER:$USER /srv/fc

# Or use user directory (update .env accordingly)
mkdir -p ~/fc-runtime
# Then edit .env: FC_RUN_DIR=/home/$USER/fc-runtime
```

## Running the Services

### Terminal 1: Start Agent (KVM Host)
```bash
# Load environment variables
source .env

# Ensure Firecracker is in PATH
export PATH="$PWD:$PATH"  # if using local firecracker binary

# Run agent
cd apps/agent && cargo run
```

### Terminal 2: Start Manager
```bash
# Load environment variables
source .env

# Run database migrations and start manager
cd apps/manager && sqlx migrate run && cargo run
```

## Testing the Setup

Create a test VM (requires kernel and rootfs files):

```bash
curl -sS -X POST http://127.0.0.1:8080/v1/vms \
  -H 'content-type: application/json' \
  -d '{
    "name":"test-vm",
    "vcpu":1,
    "mem_mib":256,
    "kernel_path":"/path/to/vmlinux",
    "rootfs_path":"/path/to/rootfs.ext4"
  }'
```

## Troubleshooting

### Permission Issues
- Ensure firecracker binary is executable
- Verify FC_RUN_DIR is writable by the user
- Check that the bridge network was created successfully

### Network Issues
- Verify bridge interface exists: `ip link show fcbr0`
- Check iptables rules are applied
- Ensure IP forwarding is enabled: `sysctl net.ipv4.ip_forward`

### Database Issues
- Verify PostgreSQL is running: `docker ps`
- Check database connection: `psql postgres://nexus:nexus@localhost:5432/nexus`
- Run migrations manually: `cd apps/manager && sqlx migrate run`

## Development Commands

- **Build all**: `cargo build`
- **Run tests**: `cargo test`
- **Clean build**: `cargo clean && cargo build`
- **Check formatting**: `cargo fmt --check`
- **Lint**: `cargo clippy`

## File Locations

- Firecracker binary: `/usr/local/bin/firecracker` or `./firecracker`
- Runtime directory: `/srv/fc` (default) or user-specified
- Database: PostgreSQL running in Docker on port 5432
- Manager API: http://127.0.0.1:8080
- Agent API: http://127.0.0.1:9090