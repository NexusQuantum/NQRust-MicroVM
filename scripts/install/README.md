# NQRust-MicroVM Installer

Production-grade installer for NQRust-MicroVM with comprehensive system setup.

## ✅ Completed Components

### Core Library Files

1. **lib/common.sh** - Foundation utilities
   - Color logging (info, success, warn, error, debug)
   - Progress spinners and confirmations
   - Error handling with automatic sudo keepalive
   - File operations (backup, restore, checksums)
   - Download utilities with progress
   - Network interface detection

2. **lib/preflight.sh** - System validation
   - OS detection (Ubuntu 22.04+, Debian 11+, RHEL 8+)
   - KVM/CPU virtualization checks
   - RAM (2GB+) and disk space (20GB+) validation
   - Port availability (8080, 9090, 3000, 5432)
   - Conflicting software detection
   - Architecture (x86_64) and kernel (4.14+) checks
   - Nested virtualization detection
   - Existing installation detection

3. **lib/kvm.sh** - KVM configuration
   - KVM module loading (kvm_intel/kvm_amd)
   - Persistent module configuration (/etc/modules)
   - KVM group creation and user management
   - /dev/kvm permissions setup (660, root:kvm)
   - udev rules for persistent permissions
   - Nested virtualization support
   - Comprehensive verification

4. **lib/network.sh** - Network setup
   - **NAT mode**: VMs isolated with 10.0.0.0/24
     - Bridge creation (fcbr0)
     - iptables MASQUERADE rules
     - dnsmasq DHCP server (10.0.0.10-250)
     - IP forwarding
   - **Bridged mode**: VMs on physical network
     - Physical interface bridging
     - IP migration to bridge
     - Promiscuous mode
     - Bridge netfilter disabled
   - Netplan persistent configuration
   - Full verification

5. **lib/sudo.sh** - Sudo configuration
   - Manager permissions:
     - mount/umount (rootfs operations)
     - cp/mv/chmod/chown (file operations)
     - cat /etc/shadow (password hashing)
     - tee/dd (write operations)
   - Agent permissions:
     - firecracker (VMM)
     - ip/brctl (networking)
     - systemd-run (scopes)
     - screen (PTY)
   - Syntax validation before install
   - Backup and rollback support

6. **lib/deps.sh** - Dependency installation
   - System packages (build tools, PostgreSQL, networking)
   - Rust toolchain (1.70+)
   - musl target for guest-agent
   - Firecracker v1.13.1 with checksum verification
   - Node.js 20.x + pnpm (optional)
   - SQLx CLI
   - OS-specific package managers (apt/yum/dnf)

7. **lib/database.sh** - PostgreSQL setup
   - PostgreSQL service configuration
   - Database and user creation
   - Password generation
   - Grant all privileges + schema permissions
   - pg_hba.conf configuration for local connections
   - Connection testing
   - Migration support
   - Remote database support

8. **lib/build.sh** - Binary management
   - Build from source:
     - Manager + Agent (release mode)
     - Guest-agent (musl static)
     - UI (Next.js 15)
   - Download pre-built binaries from GitHub releases
   - Binary installation to /opt/nqrust-microvm/bin
   - Version verification
   - Container runtime builder (optional)

9. **lib/config.sh** - Configuration generation
   - System user creation (nqrust)
   - Directory structure:
     - /opt/nqrust-microvm/ (binaries, UI, scripts)
     - /etc/nqrust-microvm/ (configs)
     - /srv/fc/vms/ (VM storage)
     - /srv/images/ (image registry)
   - Configuration files:
     - manager.env (DATABASE_URL, MANAGER_BIND, etc.)
     - agent.env (FC_RUN_DIR, FC_BRIDGE, MANAGER_BASE)
     - ui.env (NEXT_PUBLIC_API_BASE_URL)
     - config.yaml (unified YAML config)
   - Helper script copying
   - Proper permissions (640 for sensitive configs)

### Configuration Files

10. **sudoers.d/nqrust** - Sudoers rules
    - Manager: NOPASSWD for rootfs operations
    - Agent: NOPASSWD for Firecracker/networking
    - Syntax validated before installation

## 📋 Remaining Components

### Still To Implement

1. **lib/services.sh** - Systemd service management
   - Service file installation
   - Enable/start services
   - Service health checks

2. **lib/verify.sh** - Post-install verification
   - Binary verification
   - Service status checks
   - Health endpoint testing
   - Database connection verification
   - Network bridge verification

3. **systemd/** - Service files
   - nqrust-manager.service
   - nqrust-agent.service
   - nqrust-ui.service (optional)

4. **install.sh** - Main orchestrator
   - Parse command-line arguments
   - Run all phases in order
   - Handle errors and rollback
   - Installation summary

5. **uninstall.sh** - Clean removal
   - Stop services
   - Remove binaries
   - Optional: Remove data
   - Optional: Drop database

## Installation Flow

```
1. Pre-flight Checks      → lib/preflight.sh
2. Install Dependencies    → lib/deps.sh
3. Setup KVM              → lib/kvm.sh
4. Setup Network          → lib/network.sh
5. Setup Database         → lib/database.sh
6. Build/Download         → lib/build.sh
7. Generate Config        → lib/config.sh
8. Configure Sudo         → lib/sudo.sh
9. Install Services       → lib/services.sh
10. Verify Installation   → lib/verify.sh
```

## Usage

```bash
# Interactive mode (recommended)
sudo ./scripts/install/install.sh

# Production mode
sudo ./scripts/install/install.sh --mode production

# Development mode (build from source)
sudo ./scripts/install/install.sh --mode dev

# Manager only (control plane)
sudo ./scripts/install/install.sh --mode manager

# Agent only (worker node)
sudo ./scripts/install/install.sh --mode agent

# Non-interactive with config
sudo ./scripts/install/install.sh --non-interactive --config config.yaml

# With options
sudo ./scripts/install/install.sh \
  --mode production \
  --network-mode bridged \
  --bridge-name fcbr0 \
  --with-ui \
  --with-container-runtime
```

## Environment Variables

```bash
# Installation
INSTALL_DIR=/opt/nqrust-microvm
DATA_DIR=/srv/fc
CONFIG_DIR=/etc/nqrust-microvm
IMAGE_DIR=/srv/images

# Database
DB_TYPE=local           # local or remote
DB_HOST=localhost
DB_PORT=5432
DB_NAME=nexus
DB_USER=nexus
DB_PASSWORD=           # Auto-generated if not set

# Network
NETWORK_MODE=nat        # nat or bridged
BRIDGE_NAME=fcbr0

# Components
WITH_UI=true
WITH_CONTAINER_RUNTIME=false

# Manager
MANAGER_BIND=0.0.0.0:18080
MANAGER_ALLOW_IMAGE_PATHS=true
MANAGER_RECONCILER_DISABLED=false

# Agent
AGENT_BIND=0.0.0.0:19090
MANAGER_BASE=http://localhost:18080
```

## File Locations After Installation

```
/opt/nqrust-microvm/
├── bin/
│   ├── manager
│   ├── agent
│   └── guest-agent
├── ui/                 # If installed
├── scripts/            # Helper scripts
└── backups/

/etc/nqrust-microvm/
├── manager.env
├── agent.env
├── ui.env              # If installed
└── config.yaml

/srv/fc/
└── vms/                # VM storage

/srv/images/            # Image registry

/var/log/nqrust-microvm/  # Logs
```

## Next Steps

To complete the installer:

1. Implement **lib/services.sh** and **lib/verify.sh**
2. Create **systemd service files**
3. Implement **main install.sh orchestrator**
4. Implement **uninstall.sh**
5. Test in clean Ubuntu 22.04 VM
6. Test idempotency (run twice)
7. Test different modes (dev, production, manager-only, agent-only)
8. Document known issues and troubleshooting

## Testing

```bash
# Test in Docker
docker run -it --privileged ubuntu:22.04
cd /path/to/nqrust-microvm
./scripts/install/install.sh --mode production --non-interactive

# Test idempotency
./scripts/install/install.sh --mode production --non-interactive
```

## Architecture Notes

### Why Manager Needs Sudo
- Mount/umount rootfs for credential injection
- Modify files inside rootfs (guest-agent deployment)
- Read /etc/shadow for password hashing
- Write network configs into rootfs

### Why Agent Runs as Root
- Firecracker requires KVM access (/dev/kvm)
- Create TAP network devices
- Manage systemd scopes (systemd-run)
- Screen sessions for PTY management

### Zero-Downtime Update Strategy
VMs run as systemd scopes (fc-{id}.scope), not as child processes:
- Agent restart → VMs keep running
- Manager restart → VMs keep running
- Agent rediscovers VMs via inventory endpoint
- Reconciler heals any inconsistencies

## Security Considerations

1. Configs stored with 640 permissions
2. Database password auto-generated (32 chars)
3. Manager runs as nqrust user (not root)
4. Agent runs as root (required for Firecracker)
5. Sudo limited to specific commands only
6. Network isolation via bridge (NAT mode)

## Troubleshooting

- **Logs**: `/var/log/nqrust-install/*.log`
- **Service logs**: `journalctl -u nqrust-manager -f`
- **Network issues**: `sudo ip link show fcbr0`
- **KVM issues**: `ls -l /dev/kvm` and `groups`
- **Database**: `psql -U nexus -d nexus -h localhost`
