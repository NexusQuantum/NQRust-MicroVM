# Nexus Quantum Pre-Installation Requirements Guide

## Table of Contents
- [Hardware Specifications](#hardware-specifications)
- [Software and OS Requirements](#software-and-os-requirements)
- [Networking and Firewall Configuration](#networking-and-firewall-configuration)
- [User and Access Requirements](#user-and-access-requirements)
- [Storage Requirements](#storage-requirements)
- [Pre-Installation Checklist](#pre-installation-checklist)

---

## Hardware Specifications

### CPU Requirements

**Minimum:**
- **Architecture:** x86-64 with hardware virtualization support
- **Virtualization Extensions:** Intel VT-x or AMD-V (required)
- **SLAT Support:** Intel EPT (Extended Page Tables) or AMD RVI (Rapid Virtualization Indexing)
- **Cores:** 4 physical cores
- **Frequency:** 2.0 GHz base clock

**Recommended:**
- **Cores:** 8+ physical cores (16+ threads)
- **Frequency:** 3.0 GHz+ base clock
- **Features:** AES-NI for encryption performance

**Verification Commands:**
```bash
# Check for VT-x/AMD-V support
egrep -c '(vmx|svm)' /proc/cpuinfo

# Check for SLAT support
# Intel EPT
grep ept /proc/cpuinfo
# AMD RVI
grep npt /proc/cpuinfo

# Verify KVM module availability
lsmod | grep kvm
```

### Memory Requirements

**Minimum:**
- **Host RAM:** 8 GB per host machine
- **Available for VMs:** Minimum 4 GB after OS overhead

**Recommended:**
- **Host RAM:** 32 GB+ per host machine
- **Available for VMs:** 24 GB+ for production workloads
- **Configuration:** ECC memory for production environments

**Planning Guidelines:**
- Reserve 2-4 GB for host OS and management services
- Calculate VM capacity: `(Total RAM - Host Overhead) / Average VM Size`
- Example: 32 GB host = ~28 GB available = 28 VMs at 1GB each or 14 VMs at 2GB each

### Storage Requirements

**Required:**
- **Type:** SSD or NVMe storage (HDD not supported for VM storage)
- **Minimum Capacity:** 100 GB per host
- **Filesystem:** ext4 or xfs

**Recommended:**
- **Type:** NVMe SSD for best performance
- **Capacity:** 500 GB+ per host
- **RAID:** RAID 1 or RAID 10 for redundancy
- **Dedicated Volumes:**
  - `/srv/images` - Image storage (50+ GB)
  - `/srv/fc` - VM runtime and storage (flexible, based on workload)

**Storage Protocol Support:**
- **Local:** Direct-attached NVMe/SSD (recommended for best performance)
- **Network:** NFS v4+ (supported for shared image storage)
- **Network:** iSCSI (supported, higher latency than local)

**Performance Requirements:**
- **IOPS:** Minimum 5,000 IOPS for local storage
- **Throughput:** Minimum 500 MB/s sequential read/write
- **Latency:** < 1ms for local NVMe, < 5ms for network storage

### Network Adapter Requirements

**Minimum:**
- **Speed:** 1 Gigabit Ethernet
- **NICs:** 1 physical network interface

**Recommended:**
- **Speed:** 10 Gigabit Ethernet or faster
- **NICs:** 2+ physical network interfaces for traffic separation
  - **NIC 1:** Management traffic (manager API, agent communication, SSH)
  - **NIC 2:** VM traffic (VM networking, container traffic, external access)
  - **Optional NIC 3:** Storage traffic (if using network storage)

**Network Interface Features:**
- SR-IOV support (optional, for advanced VM networking)
- Jumbo frame support (MTU 9000+) for storage networks
- VLAN tagging support (802.1Q)

---

## Software and OS Requirements

### Supported Host Operating Systems

**Supported Distributions:**

| Distribution | Version | Kernel Version | Status |
|-------------|---------|----------------|--------|
| Ubuntu Server | 22.04 LTS | 5.15+ | **Recommended** |
| Ubuntu Server | 24.04 LTS | 6.8+ | **Recommended** |
| Debian | 11 (Bullseye) | 5.10+ | Supported |
| Debian | 12 (Bookworm) | 6.1+ | Supported |
| Rocky Linux | 8.x | 4.18+ | Supported |
| Rocky Linux | 9.x | 5.14+ | Supported |

**Not Supported:**
- Windows (any version)
- macOS (any version)
- BSD variants

### Linux Kernel Requirements

**Minimum Kernel Version:** 5.10+

**Required Kernel Modules:**
```bash
# KVM virtualization
kvm
kvm_intel  # For Intel CPUs
kvm_amd    # For AMD CPUs

# Networking
bridge
veth
vhost_net
vhost_vsock
tun

# Optional but recommended
overlay    # For container storage
nf_nat     # For NAT networking
```

**Verification Commands:**
```bash
# Check kernel version
uname -r

# Verify KVM modules are loaded
lsmod | grep kvm
lsmod | grep vhost

# Load required modules if missing
sudo modprobe kvm
sudo modprobe kvm_intel  # or kvm_amd
sudo modprobe vhost_net
sudo modprobe vhost_vsock
```

### System Dependencies

**Required Packages (Ubuntu/Debian):**
```bash
# Core dependencies
sudo apt-get update
sudo apt-get install -y \
    curl \
    wget \
    git \
    build-essential \
    pkg-config \
    libssl-dev \
    bridge-utils \
    iptables \
    iproute2 \
    screen \
    postgresql-client

# Docker (if using container features)
sudo apt-get install -y \
    docker.io \
    containerd

# Optional monitoring tools
sudo apt-get install -y \
    htop \
    iotop \
    nethogs \
    tcpdump
```

**Required Packages (Rocky Linux/RHEL):**
```bash
# Core dependencies
sudo dnf install -y \
    curl \
    wget \
    git \
    gcc \
    gcc-c++ \
    make \
    openssl-devel \
    bridge-utils \
    iptables \
    iproute \
    screen \
    postgresql

# Docker (if using container features)
sudo dnf install -y \
    docker \
    containerd
```

### Database Requirements

**PostgreSQL:**
- **Version:** 13, 14, 15, or 16
- **Deployment:** Can run on same host or separate database server
- **Configuration:**
  - Minimum 2 GB RAM allocated to PostgreSQL
  - Shared buffers: 25% of allocated RAM
  - Max connections: 100+

**PostgreSQL Installation (Ubuntu):**
```bash
sudo apt-get install -y postgresql-14 postgresql-client-14
sudo systemctl enable postgresql
sudo systemctl start postgresql
```

### Rust Toolchain (for building from source)

**Required for installation:**
```bash
# Install Rust (stable channel)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Verify installation
rustc --version  # Should be 1.70+
cargo --version

# Add musl target for guest-agent
rustup target add x86_64-unknown-linux-musl
```

### Node.js and pnpm (for UI frontend)

**Required versions:**
- **Node.js:** 18.x or 20.x (LTS)
- **pnpm:** 8.x or 9.x

**Installation:**
```bash
# Install Node.js 20.x (Ubuntu)
curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
sudo apt-get install -y nodejs

# Install pnpm
curl -fsSL https://get.pnpm.io/install.sh | sh -

# Verify
node --version
pnpm --version
```

---

## Networking and Firewall Configuration

### Required Network Ports

#### Manager Service (Central Orchestration)

| Port | Protocol | Direction | Purpose | Source/Destination |
|------|----------|-----------|---------|-------------------|
| 18080 | TCP | Inbound | Manager API (REST) | UI, External API clients |
| 18080 | TCP/WS | Inbound | WebSocket (Shell, Metrics) | UI clients |
| 5432 | TCP | Outbound | PostgreSQL database | Database server |

#### Agent Service (Per Host)

| Port | Protocol | Direction | Purpose | Source/Destination |
|------|----------|-----------|---------|-------------------|
| 9090 | TCP | Inbound | Agent API | Manager service |
| 9090 | TCP | Outbound | Manager registration | Manager service (18080) |

#### Guest Agent (Inside VMs)

| Port | Protocol | Direction | Purpose | Source/Destination |
|------|----------|-----------|---------|-------------------|
| 9000 | TCP | Inbound | Guest metrics API | Manager service |

#### Frontend UI

| Port | Protocol | Direction | Purpose | Source/Destination |
|------|----------|-----------|---------|-------------------|
| 3000 | TCP | Inbound | Web UI (dev mode) | End users, browsers |
| 80/443 | TCP | Inbound | Web UI (production) | End users, browsers |

#### VM Traffic (Dynamic Ports)

| Port Range | Protocol | Direction | Purpose | Source/Destination |
|------------|----------|-----------|---------|-------------------|
| 22 | TCP | Inbound | SSH to VMs | Administrators |
| 80, 443 | TCP | Inbound | HTTP/HTTPS from VMs | External clients |
| Custom | TCP/UDP | Bi-directional | Application-specific | Varies by workload |

### Firewall Rules (iptables example)

**Manager Host:**
```bash
# Allow Manager API
sudo iptables -A INPUT -p tcp --dport 18080 -j ACCEPT

# Allow PostgreSQL (if local)
sudo iptables -A INPUT -p tcp --dport 5432 -s <trusted-subnet> -j ACCEPT

# Allow SSH
sudo iptables -A INPUT -p tcp --dport 22 -j ACCEPT

# Allow established connections
sudo iptables -A INPUT -m state --state ESTABLISHED,RELATED -j ACCEPT
```

**Agent Host:**
```bash
# Allow Agent API
sudo iptables -A INPUT -p tcp --dport 9090 -s <manager-ip> -j ACCEPT

# Allow SSH
sudo iptables -A INPUT -p tcp --dport 22 -j ACCEPT

# Allow VM traffic on bridge
sudo iptables -A FORWARD -i fcbr0 -j ACCEPT
sudo iptables -A FORWARD -o fcbr0 -j ACCEPT

# Enable NAT for VMs (if using NAT mode)
sudo iptables -t nat -A POSTROUTING -o <uplink-interface> -j MASQUERADE
```

**Using ufw (Ubuntu):**
```bash
# Manager
sudo ufw allow 18080/tcp
sudo ufw allow 22/tcp
sudo ufw enable

# Agent
sudo ufw allow from <manager-ip> to any port 9090 proto tcp
sudo ufw allow 22/tcp
sudo ufw enable
```

### IP Addressing Requirements

**Management Network:**
- **Manager Service:** Static IP address required
- **Agent Hosts:** Static IP addresses recommended
- **Database Server:** Static IP address required
- **Example:** 10.0.1.0/24 subnet

**VM Network:**
- **DHCP:** Recommended for dynamic VM IP assignment
- **Static:** Supported via VM configuration
- **Subnet:** Separate from management network (e.g., 10.0.100.0/24)
- **Gateway:** Router or NAT gateway for external access

**VLAN/Subnet Segmentation (Recommended for Production):**

| Network | VLAN ID | Subnet | Purpose |
|---------|---------|--------|---------|
| Management | 10 | 10.0.1.0/24 | Manager, Agent API, SSH |
| VM Traffic | 100 | 10.0.100.0/24 | VM networking, containers |
| Storage | 20 | 10.0.2.0/24 | NFS/iSCSI traffic (optional) |
| External | - | Public IPs | External-facing services |

### Network Bridge Configuration

**Required Bridge:** `fcbr0` (default name, configurable)

**Setup Script:** Provided in repository at `scripts/fc-bridge-setup.sh`

**Example Manual Setup:**
```bash
# Create bridge
sudo ip link add fcbr0 type bridge

# Assign IP to bridge
sudo ip addr add 10.0.100.1/24 dev fcbr0

# Bring bridge up
sudo ip link set fcbr0 up

# (Optional) Bridge to physical interface for external access
sudo ip link set <uplink-interface> master fcbr0

# Enable IP forwarding
sudo sysctl -w net.ipv4.ip_forward=1
echo "net.ipv4.ip_forward=1" | sudo tee -a /etc/sysctl.conf
```

**Bridge Networking Modes:**
1. **NAT Mode:** VMs isolated behind NAT (fcbr0 not bridged to physical interface)
2. **Bridged Mode:** VMs visible on network (fcbr0 bridged to physical interface, VMs get DHCP from router)

---

## User and Access Requirements

### Required User Accounts

**Installation User:**
- **Username:** Any non-root user with sudo privileges
- **Sudo Access:** `ALL=(ALL:ALL) ALL` or specific commands:
  ```
  %sudo ALL=(ALL) NOPASSWD: /usr/bin/systemctl
  %sudo ALL=(ALL) NOPASSWD: /usr/sbin/ip
  %sudo ALL=(ALL) NOPASSWD: /usr/sbin/iptables
  %sudo ALL=(ALL) NOPASSWD: /usr/bin/docker
  ```
- **SSH Access:** Key-based authentication required (password auth discouraged)

**Agent Service User:**
- **Requirement:** Must run as `root` (required for KVM access)
- **Alternative:** User in `kvm` group with sudo privileges
- **KVM Device Access:** `/dev/kvm` must be readable/writable

**Verification:**
```bash
# Check user is in sudo group
groups $USER | grep sudo

# Check KVM device permissions
ls -l /dev/kvm
# Should show: crw-rw---- 1 root kvm

# Add user to kvm group if needed
sudo usermod -aG kvm $USER
```

### SSH Access Requirements

**Manager and Agent Hosts:**
- **Protocol:** SSH (port 22 or custom)
- **Authentication:** SSH key-based (required)
- **Key Type:** RSA 2048-bit, ED25519, or ECDSA
- **Access:** From installation workstation to all hosts

**Generate SSH Key (if needed):**
```bash
# On installation workstation
ssh-keygen -t ed25519 -C "nexus-quantum-install"

# Copy to target hosts
ssh-copy-id -i ~/.ssh/id_ed25519.pub user@<manager-host>
ssh-copy-id -i ~/.ssh/id_ed25519.pub user@<agent-host>
```

### Database Access

**PostgreSQL User:**
- **Username:** `nexus_manager` (recommended)
- **Password:** Strong password (16+ characters, mixed case, numbers, symbols)
- **Permissions:** `CREATE DATABASE`, `CREATE TABLE`, `INSERT`, `UPDATE`, `DELETE`, `SELECT`

**Create Database User:**
```bash
sudo -u postgres psql
CREATE USER nexus_manager WITH PASSWORD 'your-secure-password';
CREATE DATABASE nexus_quantum OWNER nexus_manager;
GRANT ALL PRIVILEGES ON DATABASE nexus_quantum TO nexus_manager;
\q
```

---

## Storage Requirements

### Directory Structure

**Required Directories (Agent Hosts):**
```bash
# Create required directories
sudo mkdir -p /srv/fc
sudo mkdir -p /srv/images
sudo mkdir -p /srv/fc/vms

# Set ownership (replace 'user' with actual user)
sudo chown -R user:user /srv/fc
sudo chown -R user:user /srv/images

# Set permissions
sudo chmod 755 /srv/fc
sudo chmod 755 /srv/images
```

**Disk Space Allocation:**
- `/srv/images`: 50-100 GB (kernel, rootfs, container runtime images)
- `/srv/fc/vms`: Flexible, depends on number and size of VMs (recommend 200+ GB)

### Image Storage

**Required Images:**
1. **Kernel Image:** vmlinux (Linux kernel, ~10 MB)
2. **Rootfs Images:** Alpine/Ubuntu ext4 images (~100-500 MB each)
3. **Container Runtime:** Alpine + Docker (~386 MB)

**Preload Script:** `scripts/preload-docker-images.sh`

### Storage Performance Tuning

**Filesystem Mount Options (ext4):**
```bash
# /etc/fstab entry for /srv/fc
/dev/nvme0n1p1 /srv/fc ext4 defaults,noatime,nodiratime 0 2
```

**For NVMe devices:**
```bash
# Enable write cache
sudo hdparm -W1 /dev/nvme0n1
```

---

## Pre-Installation Checklist

Use this checklist to verify all requirements are met before installation:

### Hardware Verification
- [ ] CPU supports VT-x (Intel) or AMD-V (AMD)
- [ ] CPU supports SLAT (EPT/RVI)
- [ ] Minimum 8 GB RAM available (32+ GB recommended)
- [ ] SSD or NVMe storage available (100+ GB minimum)
- [ ] Network adapter is 1 GbE or faster
- [ ] `/dev/kvm` device exists and is accessible

### Software Verification
- [ ] Linux OS version is supported (Ubuntu 22.04/24.04 recommended)
- [ ] Kernel version is 5.10+ (`uname -r`)
- [ ] KVM modules are loaded (`lsmod | grep kvm`)
- [ ] Required system packages are installed
- [ ] PostgreSQL 13+ is installed and running
- [ ] Rust toolchain is installed (1.70+)
- [ ] Node.js 18+ and pnpm are installed (for UI)
- [ ] Docker is installed (if using container features)

### Network Verification
- [ ] Static IP assigned to manager host
- [ ] Firewall rules configured for required ports
- [ ] Network bridge `fcbr0` can be created
- [ ] IP forwarding is enabled (`sysctl net.ipv4.ip_forward`)
- [ ] DNS resolution is working
- [ ] Internet access available for package downloads

### User and Access Verification
- [ ] Installation user has sudo privileges
- [ ] SSH key-based authentication is configured
- [ ] SSH access verified to all hosts
- [ ] PostgreSQL database and user created
- [ ] Database connection string tested

### Storage Verification
- [ ] `/srv/fc` directory created with correct permissions
- [ ] `/srv/images` directory created with correct permissions
- [ ] Sufficient disk space available (300+ GB recommended)
- [ ] Storage performance tested (5000+ IOPS)

### Environment Variables Prepared
- [ ] `DATABASE_URL` prepared (e.g., `postgresql://nexus_manager:password@localhost/nexus_quantum`)
- [ ] `MANAGER_BIND` determined (e.g., `0.0.0.0:18080`)
- [ ] `AGENT_BIND` determined (e.g., `0.0.0.0:9090`)
- [ ] `FC_RUN_DIR` confirmed (`/srv/fc`)
- [ ] `FC_BRIDGE` confirmed (`fcbr0`)
- [ ] `MANAGER_BASE` prepared (e.g., `http://<manager-ip>:18080`)

---

## Next Steps

After verifying all requirements:

1. **Proceed to Installation Guide** - Follow the step-by-step installation runbook
2. **Review Architecture Documentation** - Understand system components and data flow
3. **Prepare Configuration Files** - Set up environment variables and configuration
4. **Schedule Installation Window** - Plan installation with customer technical team
5. **Execute Acceptance Testing** - Validate installation with formal test plan

---

## Support and Questions

For technical questions or clarifications during pre-installation assessment:

- **Documentation:** See `README.md`, `FEATURES.md`, and other docs in repository
- **GitHub Issues:** https://github.com/your-org/nqrust-microvm/issues
- **Email Support:** support@nexus-quantum.example.com (update with actual contact)

---

**Document Version:** 1.0
**Last Updated:** 2025-12-01
**Prepared By:** Nexus Quantum Engineering Team
