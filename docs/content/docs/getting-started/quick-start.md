+++
title = "Quick Start"
description = "Create your first VM in 5 minutes"
weight = 3
date = 2025-12-01

[extra]
toc = true
+++

# Quick Start Guide

This guide will walk you through creating and managing your first microVM in just a few minutes.

**Prerequisites:** Complete the [Installation Guide](/getting-started/installation/) first.

+++

## Start the Services

You need to start all four services in separate terminal sessions.

### Terminal 1: PostgreSQL

If using Docker for PostgreSQL:
```bash
cd /path/to/nqrust-microvm
./scripts/dev-up.sh
```

If using system PostgreSQL, it should already be running from installation.

### Terminal 2: Agent

The agent requires sudo for KVM access:

```bash
cd /path/to/nqrust-microvm

# Build agent (if not already built)
cargo build -p agent

# Start agent with sudo
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

### Terminal 3: Manager

```bash
cd /path/to/nqrust-microvm

# Build manager (if not already built)
cargo build -p manager

# Set database URL
export DATABASE_URL=postgres://nexus:nexus@localhost:5432/nexus

# Optional: Set custom image root
# export MANAGER_IMAGE_ROOT=$HOME/images

# Start manager
export MANAGER_ALLOW_IMAGE_PATHS=true
export RUST_LOG=info
./target/debug/manager
```

**Expected output:**
```
INFO Running migrations...
INFO manager listening bind=127.0.0.1:18080
```

**Note:** If migrations fail with errors about migration 10:
```bash
psql $DATABASE_URL -c "DELETE FROM _sqlx_migrations WHERE version = 10;"
cd apps/manager && sqlx migrate run
```

### Terminal 4: Frontend

```bash
cd /path/to/nqrust-microvm/apps/ui

# Install dependencies (first time only)
pnpm install

# Set API endpoint
export NEXT_PUBLIC_API_BASE_URL=http://127.0.0.1:18080/v1
export NEXT_PUBLIC_WS_BASE_URL=ws://127.0.0.1:18080
export NEXT_PUBLIC_BRAND_PRESET=dark  # Optional: dark or light theme

# Start development server
pnpm dev
```

**Expected output:**
```
▲ Next.js 15.x
- Local: http://localhost:3000
```

+++

## Verify Services are Running

### Check Agent Registration

```bash
curl http://localhost:18080/v1/hosts
```

You should see a host entry with your system information.

### Check VM List (Should be Empty)

```bash
curl http://localhost:18080/v1/vms
```

Should return: `{"items":[]}`

### Open Web UI

Navigate to: **http://localhost:3000**

You should see the NQRust-MicroVM dashboard.

+++

## Get VM Images

Before creating a VM, you need a Linux kernel and root filesystem image.

### Option 1: Download Sample Images (Recommended)

```bash
# Create images directory
mkdir -p /srv/images
cd /srv/images

# Download Alpine Linux kernel (lightweight, ~7MB)
wget https://github.com/firecracker-microvm/firecracker/raw/main/tests/framework/kernels/vmlinux.bin

# Download Alpine Linux rootfs (~50MB)
wget https://github.com/firecracker-microvm/firecracker/raw/main/tests/framework/rootfs/ubuntu-22.04.ext4

# Verify files exist
ls -lh /srv/images/
```

### Option 2: Use the Web UI

1. Navigate to http://localhost:3000/registry
2. Click "Import Image"
3. Select "Kernel" type
4. Upload or browse Docker Hub for images

See [Image Registry Guide](/user-guide/storage/images/) for more options.

+++

## Create Your First VM

### Method 1: Using the Web UI (Recommended)

1. **Navigate to VMs page**
   - Open http://localhost:3000/vms
   - Click "Create VM" button

2. **VM Information (Step 1)**
   - **Name**: `my-first-vm`
   - **Description**: `My first Alpine VM`
   - Click "Next"

3. **Credentials (Step 2)**
   - **Root Password**: Choose a password
   - Credentials are auto-injected via MMDS
   - Click "Next"

4. **Machine Config (Step 3)**
   - **vCPUs**: `1`
   - **Memory**: `512` MB
   - **HT Enabled**: No (unless you know you need it)
   - Click "Next"

5. **Boot Source (Step 4)**
   - **Kernel**: Select your uploaded kernel
   - **Rootfs**: Select your uploaded rootfs
   - **Boot Args** (optional): Leave default or customize
   - Click "Next"

6. **Network (Step 5)**
   - **Bridge**: `fcbr0` (should be auto-detected)
   - **Guest MAC**: Leave blank for auto-generation
   - **Allow MMDS**: Yes (required for credential injection)
   - Click "Create VM"

7. **Start the VM**
   - On the VM detail page, click "Start"
   - Watch the status change to "Running"

8. **Access the VM**
   - Click the "Terminal" tab
   - You'll see a browser-based shell
   - Login with root and your password

**Congratulations!** You just created your first microVM! 🎉

### Method 2: Using the API

First, register your images:

```bash
# Register kernel image
curl -X POST http://127.0.0.1:18080/v1/images \
  -H 'Content-Type: application/json' \
  -d '{
    "name": "alpine-kernel",
    "type": "Kernel",
    "path": "/srv/images/vmlinux.bin"
  }'

# Note the image_id from response (e.g., "59e1c754-2210-4887-858c-f3c5de7d483b")

# Register rootfs image
curl -X POST http://127.0.0.1:18080/v1/images \
  -H 'Content-Type: application/json' \
  -d '{
    "name": "alpine-rootfs",
    "type": "Rootfs",
    "path": "/srv/images/ubuntu-22.04.ext4"
  }'

# Note the image_id from response
```

Then create the VM:

```bash
curl -X POST http://127.0.0.1:18080/v1/vms \
  -H 'Content-Type: application/json' \
  -d '{
    "name": "my-first-vm",
    "vcpu": 1,
    "mem_mib": 512,
    "kernel_image_id": "59e1c754-2210-4887-858c-f3c5de7d483b",
    "rootfs_image_id": "4196a86f-95f4-4609-af23-138ec331b0dc"
  }'

# Note the vm_id from response
```

Start the VM:

```bash
curl -X POST http://127.0.0.1:18080/v1/vms/{vm_id}/start
```

+++

## Manage Your VM

### VM Lifecycle Operations

**Start a stopped VM:**
```bash
curl -X POST http://127.0.0.1:18080/v1/vms/{vm_id}/start
```

**Stop a running VM:**
```bash
curl -X POST http://127.0.0.1:18080/v1/vms/{vm_id}/stop
```

**Pause a running VM:**
```bash
curl -X POST http://127.0.0.1:18080/v1/vms/{vm_id}/pause
```

**Resume a paused VM:**
```bash
curl -X POST http://127.0.0.1:18080/v1/vms/{vm_id}/resume
```

**Delete a VM:**
```bash
curl -X DELETE http://127.0.0.1:18080/v1/vms/{vm_id}
```

### Access the VM Terminal

**Via Web UI:**
1. Go to http://localhost:3000/vms/{vm_id}
2. Click the "Terminal" tab
3. Login with root and your password

**Via API (WebSocket):**
```javascript
// Connect to WebSocket
const ws = new WebSocket('ws://localhost:18080/v1/vms/{vm_id}/shell/ws');

ws.onmessage = (event) => {
  console.log('Received:', event.data);
};

ws.send('ls -la\n');
```

### View VM Metrics

**Via Web UI:**
1. Go to VM detail page
2. Click "Metrics" tab
3. See real-time CPU, memory, network stats

**Via API:**
```bash
curl http://127.0.0.1:18080/v1/vms/{vm_id}/metrics
```

+++

## Create a Snapshot

Snapshots let you save and restore VM state instantly.

**Via Web UI:**
1. Go to VM detail page
2. Click "Snapshots" tab
3. Click "Create Snapshot"
4. Enter snapshot name
5. Select snapshot type (full or differential)
6. Click "Create"

**Via API:**
```bash
curl -X POST http://127.0.0.1:18080/v1/vms/{vm_id}/snapshots \
  -H 'Content-Type: application/json' \
  -d '{
    "snapshot_name": "my-snapshot",
    "snapshot_type": "Full"
  }'
```

**Restore from snapshot:**
```bash
curl -X POST http://127.0.0.1:18080/v1/vms/{vm_id}/snapshots/{snapshot_id}/restore
```

See [Snapshots Guide](/user-guide/vm-management/snapshots/) for more details.

+++

## Create a Template

Templates let you save VM configurations for reuse.

**From an existing VM:**
```bash
curl -X POST http://127.0.0.1:18080/v1/templates \
  -H 'Content-Type: application/json' \
  -d '{
    "name": "alpine-template",
    "description": "Alpine Linux 512MB template",
    "vm_id": "{vm_id}"
  }'
```

**Create VM from template:**
```bash
curl -X POST http://127.0.0.1:18080/v1/templates/{template_id}/instantiate \
  -H 'Content-Type: application/json' \
  -d '{
    "name": "vm-from-template"
  }'
```

See [Templates Guide](/user-guide/vm-management/templates/) for more details.

+++

## Next Steps

Congratulations on creating your first microVM! Here's what to explore next:

### Learn More Features
- [**VM Management**](/user-guide/vm-management/) - Advanced VM operations
- [**Containers**](/user-guide/containers/) - Run Docker containers in isolated VMs
- [**Serverless Functions**](/user-guide/functions/) - Deploy Node.js, Python, Ruby functions
- [**Networking**](/user-guide/networking/) - Advanced networking with VLANs
- [**Storage**](/user-guide/storage/) - Volume management and image registry

### Production Deployment
- [**Performance Tuning**](/operations/performance-tuning/) - Optimize for production
- [**Bridged Networking**](/operations/bridged-networking/) - Connect VMs to physical network
- [**Deployment Guide**](/deployment/) - Production deployment strategies

### Development
- [**API Reference**](http://localhost:18080/swagger-ui/) - Complete API documentation
- [**Contributing**](/development/contributing/) - Contribute to the project

+++

## Common Issues

### VM Won't Start

**Check logs:**
```bash
# Check manager logs for errors
# Check agent logs for firecracker errors

# Verify images exist
ls -l /srv/images/

# Check VM status
curl http://127.0.0.1:18080/v1/vms/{vm_id}
```

### Can't Connect to VM Terminal

**Verify WebSocket connection:**
1. Check browser console for WebSocket errors
2. Ensure manager is running on port 18080
3. Check VM is in "Running" state

### Bridge Not Found

**Create the bridge:**
```bash
sudo ./scripts/fc-bridge-setup.sh fcbr0 eth0
ip link show fcbr0
```

### Permission Denied (KVM)

**Add user to kvm group:**
```bash
sudo usermod -a -G kvm $USER
newgrp kvm
```

For more troubleshooting, see [Installation Guide](/getting-started/installation/#troubleshooting).

+++

## Development Workflow Reference

### Quick Commands

```bash
# Build entire workspace
cargo build

# Build specific package
cargo build -p manager
cargo build -p agent

# Run manager (from project root)
(cd apps/manager && cargo run)

# Run agent with sudo
sudo -E env AGENT_BIND=127.0.0.1:9090 \
  MANAGER_BASE=http://127.0.0.1:18080 \
  FC_RUN_DIR=/srv/fc FC_BRIDGE=fcbr0 \
  ./target/debug/agent

# Run frontend dev server
(cd apps/ui && pnpm dev)

# Run tests
cargo test

# Run specific package tests
cargo test -p manager
```

### Environment Variables Quick Reference

**Manager:**
- `DATABASE_URL` - PostgreSQL connection string
- `MANAGER_BIND` - Bind address (default: 127.0.0.1:18080)
- `MANAGER_IMAGE_ROOT` - Image storage path (default: /srv/images)
- `MANAGER_ALLOW_IMAGE_PATHS` - Allow direct file paths (default: false)

**Agent:**
- `AGENT_BIND` - Bind address (default: 127.0.0.1:9090)
- `FC_RUN_DIR` - Firecracker runtime directory (default: /srv/fc)
- `FC_BRIDGE` - Network bridge name (default: fcbr0)
- `MANAGER_BASE` - Manager API URL (required)

**Frontend:**
- `NEXT_PUBLIC_API_BASE_URL` - Manager API URL
- `NEXT_PUBLIC_WS_BASE_URL` - WebSocket URL
- `NEXT_PUBLIC_BRAND_PRESET` - Theme (dark/light)

+++

**You're all set!** Start creating VMs and exploring the platform. 🚀
