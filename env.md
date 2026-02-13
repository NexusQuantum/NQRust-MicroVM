# Environment Variables

## Manager

```bash
export DATABASE_URL=postgresql://nexus:nexus@localhost:5432/nexus
export MANAGER_BIND=0.0.0.0:18080
export MANAGER_IMAGE_ROOT=/srv/images
export MANAGER_STORAGE_ROOT=/srv/fc/vms
export MANAGER_ALLOW_IMAGE_PATHS=true
```

## Agent

```bash
export AGENT_BIND=127.0.0.1:19090
export MANAGER_BASE=http://127.0.0.1:18080
export FC_RUN_DIR=/srv/fc
export FC_BRIDGE=fcbr0
```

## Container Runtime

```bash
export CONTAINER_RUNTIME_KERNEL=/srv/images/vmlinux-5.10.fc.bin
export CONTAINER_RUNTIME_ROOTFS=/srv/images/container-runtime.ext4
```

## Quick Start

### Run Manager
```bash
source env.md  # or copy/paste the exports above
(cd apps/manager && cargo run)
```

### Run Agent (requires sudo)
```bash
sudo -E env \
  AGENT_BIND=127.0.0.1:19090 \
  MANAGER_BASE=http://127.0.0.1:18080 \
  FC_RUN_DIR=/srv/fc \
  FC_BRIDGE=fcbr0 \
  ./target/debug/agent
```
