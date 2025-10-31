# Development Environment Setup Guide

This guide will take you from a fresh Linux system to a fully operational NQRust-MicroVM development environment.

**Estimated time:** 30-45 minutes

---

## Table of Contents

1. [System Requirements](#system-requirements)
2. [Install System Dependencies](#install-system-dependencies)
3. [Install Development Tools](#install-development-tools)
4. [Setup Firecracker](#setup-firecracker)
5. [Setup PostgreSQL Database](#setup-postgresql-database)
6. [Configure Network Bridge](#configure-network-bridge)
7. [Create Storage Directories](#create-storage-directories)
8. [Clone and Build Project](#clone-and-build-project)
9. [Configure Environment Variables](#configure-environment-variables)
10. [Start the Stack](#start-the-stack)
11. [Verify Installation](#verify-installation)
12. [Optional: Container Runtime](#optional-container-runtime)
13. [Troubleshooting](#troubleshooting)

---

## System Requirements

### Hardware
- **CPU**: x86_64 with KVM support (Intel VT-x or AMD-V)
- **RAM**: 4GB minimum, 8GB+ recommended
- **Disk**: 20GB+ free space for VM images and build artifacts

### Operating System
- **Linux**: Ubuntu 22.04+ (recommended)
- Other distributions work but this guide uses Ubuntu/Debian commands

### Check KVM Support
```bash
# Check if CPU supports virtualization
egrep -c '(vmx|svm)' /proc/cpuinfo
# Should output > 0

# Check if KVM module is loaded
lsmod | grep kvm
# Should show kvm and kvm_intel (or kvm_amd)
```

---

## Install System Dependencies

### Update System
```bash
sudo apt update
sudo apt upgrade -y
```

### Install Required Packages
```bash
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

**What these are for:**
- `build-essential`, `pkg-config`, `libssl-dev` - Rust compilation
- `postgresql`, `postgresql-contrib` - Database for manager
- `screen` - VM console access (PTY management)
- `openssl` - Password hashing for credential injection
- `iproute2`, `iptables`, `bridge-utils` - Network management
- `curl`, `git` - Downloading tools and source code

---

## Install Development Tools

### 1. Install Rust

```bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Select option 1 (default installation)
# Then reload environment
source "$HOME/.cargo/env"

# Verify installation
rustc --version
cargo --version
```

**Required version:** Rust 1.70+

### 2. Install SQLx CLI

```bash
# Install SQLx CLI for database migrations
cargo install sqlx-cli --no-default-features --features postgres

# Verify installation
sqlx --version
```

### 3. Install Node.js (for Frontend)

```bash
# Install Node.js 20.x LTS
curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
sudo apt install -y nodejs

# Verify installation
node --version  # Should be v20.x
npm --version   # Should be 10.x

# Install pnpm (package manager)
npm install -g pnpm

# Verify pnpm
pnpm --version
```

---

## Setup Firecracker

### 1. Download Firecracker

```bash
# Set version
FIRECRACKER_VERSION="v1.13.1"

# Download binary
curl -L -o /tmp/firecracker.tgz \
  "https://github.com/firecracker-microvm/firecracker/releases/download/${FIRECRACKER_VERSION}/firecracker-${FIRECRACKER_VERSION}-x86_64.tgz"

# Extract
cd /tmp
tar xzf firecracker.tgz

# Install to system
sudo mv release-${FIRECRACKER_VERSION}-x86_64/firecracker-${FIRECRACKER_VERSION}-x86_64 \
  /usr/local/bin/firecracker

# Make executable
sudo chmod +x /usr/local/bin/firecracker

# Clean up
rm -rf /tmp/release-${FIRECRACKER_VERSION}-x86_64 /tmp/firecracker.tgz

# Verify installation
firecracker --version
# Should output: Firecracker v1.13.1
```

**Important:** This system is tested with Firecracker v1.13.1. Other versions may work but are not guaranteed.

### 2. Enable KVM Access

```bash
# Load KVM modules (if not already loaded)
sudo modprobe kvm
sudo modprobe kvm_intel  # For Intel CPUs
# OR
sudo modprobe kvm_amd    # For AMD CPUs

# Add your user to kvm group
sudo usermod -a -G kvm $USER

# Check group membership
groups | grep kvm

# IMPORTANT: Log out and log back in for group changes to take effect
# Or run: newgrp kvm
```

---

## Setup PostgreSQL Database

### 1. Start PostgreSQL Service

```bash
# Start PostgreSQL
sudo systemctl start postgresql
sudo systemctl enable postgresql

# Verify it's running
sudo systemctl status postgresql
```

### 2. Create Database and User

```bash
# Switch to postgres user and create database
sudo -u postgres psql << EOF
CREATE DATABASE nexus;
CREATE USER nexus WITH ENCRYPTED PASSWORD 'nexus';
GRANT ALL PRIVILEGES ON DATABASE nexus TO nexus;

-- Grant schema privileges (required for SQLx migrations)
\c nexus
GRANT ALL ON SCHEMA public TO nexus;
EOF
```

### 3. Test Connection

```bash
# Test connection
psql -h localhost -U nexus -d nexus -c "SELECT version();"
# When prompted, enter password: nexus

# If successful, you'll see PostgreSQL version info
```

**Note:** For production, use a strong password and secure the database properly.

---

## Configure Network Bridge

VMs need a network bridge to communicate. Choose **one** of the following modes:

### Option A: NAT Mode (Recommended for Development)

VMs are isolated behind NAT. Simplest setup.

```bash
cd /path/to/nqrust-microvm

# Find your network interface
ip link show
# Look for your main interface (e.g., eth0, ens18, enp0s3)

# Setup bridge with NAT
sudo ./scripts/fc-bridge-setup.sh fcbr0 eth0
# Replace 'eth0' with your interface name

# Verify bridge exists
ip link show fcbr0
```

### Option B: Bridged Mode (VMs on Physical Network)

VMs get IPs from your router and are accessible on your network.

```bash
cd /path/to/nqrust-microvm

# Find your network interface
ip link show

# Setup physical bridge (TEMPORARY - will survive until reboot)
sudo ./scripts/fc-bridge-physical.sh fcbr0 eth0
# Replace 'eth0' with your interface name

# Make it persistent (optional)
# See BRIDGED_NETWORK_SETUP.md for detailed instructions
```

**⚠️ Warning:** Bridged mode modifies your network configuration. Make sure you have console/VNC access in case SSH breaks!

**For most development, use NAT Mode (Option A).**

---

## Create Storage Directories

```bash
# Create directories for VM storage
sudo mkdir -p /srv/fc/vms
sudo mkdir -p /srv/images

# Set ownership to your user
sudo chown -R $USER:$USER /srv/fc
sudo chown -R $USER:$USER /srv/images

# Verify permissions
ls -ld /srv/fc /srv/images
```

---

## Clone and Build Project

### 1. Clone Repository

```bash
# Clone the repository
git clone <your-repo-url> nqrust-microvm
cd nqrust-microvm
```

### 2. Build Rust Backend

```bash
# Build the entire workspace
cargo build

# This will take several minutes on first build
# Binaries will be in target/debug/
```

**Build artifacts:**
- `target/debug/manager` - Manager service
- `target/debug/agent` - Agent service
- `target/x86_64-unknown-linux-musl/release/guest-agent` - Guest agent (if built)

### 3. Run Database Migrations

```bash
cd apps/manager

# Run migrations
sqlx migrate run

# Verify migrations
sqlx migrate info

cd ../..
```

### 4. Build Frontend

```bash
cd apps/ui

# Install dependencies
pnpm install

# Build for development
pnpm build

cd ../..
```

---

## Configure Environment Variables

### 1. Create Root Environment File

```bash
# Copy example
cp .env.example .env

# Edit if needed
nano .env
```

**Default `.env` contents:**
```bash
# Manager
DATABASE_URL=postgres://nexus:nexus@localhost:5432/nexus
MANAGER_BIND=127.0.0.1:18080
MANAGER_IMAGE_ROOT=/srv/images
MANAGER_ALLOW_IMAGE_PATHS=true

# Agent
AGENT_BIND=127.0.0.1:9090
FC_RUN_DIR=/srv/fc
FC_BRIDGE=fcbr0
MANAGER_BASE=http://127.0.0.1:18080
```

### 2. Create Frontend Environment File

```bash
cd apps/ui

# Copy example
cp .env.example .env.local

# Edit if needed
nano .env.local
```

**Default `apps/ui/.env.local` contents:**
```bash
NEXT_PUBLIC_API_BASE_URL=http://localhost:18080/v1
NEXT_PUBLIC_WS_BASE_URL=ws://localhost:18080
NODE_ENV=development
```

---

## Start the Stack

### Terminal 1: Start PostgreSQL (if using Docker)

```bash
# Start PostgreSQL via Docker Compose
./scripts/dev-up.sh

# This starts PostgreSQL on port 5432
# Credentials: user=nexus, pass=nexus, db=nexus
```

**Or** use system PostgreSQL (already started in previous step).

### Terminal 2: Start Agent

**⚠️ Agent requires sudo for KVM access**

```bash
# Load environment
source .env

# Run agent with sudo
sudo -E env \
  AGENT_BIND=127.0.0.1:9090 \
  MANAGER_BASE=http://127.0.0.1:18080 \
  FC_RUN_DIR=/srv/fc \
  FC_BRIDGE=fcbr0 \
  ./target/debug/agent
```

**Expected output:**
```
INFO agent listening bind=127.0.0.1:9090
INFO registered with manager host_id=<uuid>
```

### Terminal 3: Start Manager

```bash
# Load environment
source .env

# Run manager
cd apps/manager
cargo run

# Or use the binary directly:
# ../target/debug/manager
```

**Expected output:**
```
INFO Running migrations...
INFO manager listening bind=127.0.0.1:18080
```

### Terminal 4: Start Frontend

```bash
cd apps/ui

# Development mode (with hot reload)
pnpm dev

# Frontend will be available at http://localhost:3000
```

**Expected output:**
```
▲ Next.js 15.x
- Local: http://localhost:3000
```

---

## Verify Installation

### 1. Check Services

Open your browser and verify:

- **Frontend UI**: http://localhost:3000
- **Manager API**: http://localhost:18080/v1/vms (should return `{"items":[]}`)
- **Swagger Docs**: http://localhost:18080/swagger-ui/ (if available)

### 2. Check Agent Registration

```bash
# Check if agent registered with manager
curl http://localhost:18080/v1/hosts

# Should return a host entry with your system info
```

### 3. Test VM Creation (Optional)

You'll need VM images first. See README.md for image setup instructions.

```bash
# List available images
curl http://localhost:18080/v1/images

# Create a test VM via UI:
# 1. Go to http://localhost:3000
# 2. Click "Create VM"
# 3. Follow the wizard
```

---

## Optional: Container Runtime

To enable Docker container support, build the container runtime image:

```bash
# Build container runtime image (requires sudo)
sudo ./scripts/build-container-runtime-v2.sh

# This creates /srv/images/container-runtime.ext4
# Size: ~386MB
# Contains: Alpine Linux + Docker + OpenRC

# Verify image exists
ls -lh /srv/images/container-runtime.ext4
```

See [CONTAINER.md](CONTAINER.md) for complete container feature documentation.

---

## Troubleshooting

### KVM Permission Denied

**Problem:** Agent fails with "Permission denied" when accessing `/dev/kvm`

**Solution:**
```bash
# Add user to kvm group
sudo usermod -a -G kvm $USER

# Log out and log back in
# Or run: newgrp kvm

# Verify access
ls -l /dev/kvm
# Should show: crw-rw---- 1 root kvm ...
```

### PostgreSQL Connection Failed

**Problem:** Manager can't connect to database

**Solution:**
```bash
# Check PostgreSQL is running
sudo systemctl status postgresql

# Test connection manually
psql -h localhost -U nexus -d nexus

# Check DATABASE_URL in .env
echo $DATABASE_URL

# Reset database (if needed)
sudo -u postgres psql -c "DROP DATABASE IF EXISTS nexus;"
sudo -u postgres psql -c "CREATE DATABASE nexus;"
sudo -u postgres psql -c "GRANT ALL PRIVILEGES ON DATABASE nexus TO nexus;"
```

### Bridge Not Found

**Problem:** Agent can't find network bridge `fcbr0`

**Solution:**
```bash
# Check if bridge exists
ip link show fcbr0

# If not, create it
sudo ./scripts/fc-bridge-setup.sh fcbr0 eth0

# Verify
ip link show fcbr0
```

### Migration Errors

**Problem:** `sqlx migrate run` fails

**Solution:**
```bash
# Check current migration status
cd apps/manager
sqlx migrate info

# If migration 10 is stuck (common issue):
psql $DATABASE_URL -c "DELETE FROM _sqlx_migrations WHERE version = 10;"

# Re-run migrations
sqlx migrate run
```

### Port Already in Use

**Problem:** "Address already in use" when starting services

**Solution:**
```bash
# Check what's using port 18080 (manager)
sudo lsof -i :18080

# Check what's using port 9090 (agent)
sudo lsof -i :9090

# Kill the process or change port in .env
```

### Frontend Won't Connect

**Problem:** UI can't reach backend API

**Solution:**
```bash
# Check manager is running
curl http://localhost:18080/v1/vms

# Check frontend .env.local
cat apps/ui/.env.local

# Ensure NEXT_PUBLIC_API_BASE_URL matches manager address
# Should be: http://localhost:18080/v1
```

### Firecracker Binary Not Found

**Problem:** Agent can't find `firecracker` binary

**Solution:**
```bash
# Check if firecracker is in PATH
which firecracker

# If not found, install it:
# (see "Setup Firecracker" section above)

# Or set custom path in agent config
export FIRECRACKER_BIN=/path/to/firecracker
```

---

## Next Steps

After successful setup:

1. **Read Documentation**:
   - [README.md](README.md) - Full project overview
   - [FEATURES.md](FEATURES.md) - Feature matrix
   - [CONTAINER.md](CONTAINER.md) - Container support
   - [CLAUDE.md](CLAUDE.md) - Development guidelines

2. **Setup VM Images**:
   - Download kernel and rootfs images
   - See README.md "Quick Start Guide" section

3. **Create Your First VM**:
   - Use the web UI at http://localhost:3000
   - Follow the VM creation wizard

4. **Explore APIs**:
   - Check Swagger UI at http://localhost:18080/swagger-ui/
   - Try API endpoints with curl

5. **Development Workflow**:
   - See [RUN.md](RUN.md) for quick commands
   - Make code changes and test

---

## Quick Reference

### Start All Services (After Initial Setup)

```bash
# Terminal 1: PostgreSQL (if using Docker)
./scripts/dev-up.sh

# Terminal 2: Agent (requires sudo)
sudo -E env AGENT_BIND=127.0.0.1:9090 MANAGER_BASE=http://127.0.0.1:18080 \
  FC_RUN_DIR=/srv/fc FC_BRIDGE=fcbr0 ./target/debug/agent

# Terminal 3: Manager
(cd apps/manager && cargo run)

# Terminal 4: Frontend
(cd apps/ui && pnpm dev)
```

### Rebuild After Code Changes

```bash
# Rebuild Rust backend
cargo build

# Rebuild frontend
(cd apps/ui && pnpm build)

# Restart services
```

### Common Commands

```bash
# Run migrations
(cd apps/manager && sqlx migrate run)

# Run tests
cargo test

# Format code
cargo fmt

# Check for errors
cargo check

# Frontend dev mode
(cd apps/ui && pnpm dev)
```

---

## Getting Help

- **GitHub Issues**: Report bugs or ask questions
- **Documentation**: Check README.md and other .md files
- **Logs**: Check terminal output for error messages
- **Database**: `psql -h localhost -U nexus -d nexus` to inspect data

---

**Setup complete! You should now have a fully functional NQRust-MicroVM development environment.** 🎉
