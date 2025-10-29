#!/bin/bash
set -e

echo "=========================================="
echo "Building Container Runtime Image (v2)"
echo "=========================================="

# Configuration
ALPINE_VERSION="3.18"
ALPINE_RELEASE="3.18.4"
WORK_DIR="$(pwd)/build-container-runtime"
IMAGE_SIZE="2200M"  # 2.2GB to have space for Docker + guest-agent
OUTPUT_IMAGE="/srv/images/container-runtime.ext4"
CACHE_DIR="/tmp/alpine-cache"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

print_step() {
    echo -e "${BLUE}==>${NC} ${GREEN}$1${NC}"
}

print_error() {
    echo -e "${RED}ERROR:${NC} $1"
}

# Check if running as root (needed for chroot)
if [ "$EUID" -ne 0 ]; then
    print_error "This script must be run as root (or with sudo)"
    exit 1
fi

# Clean up previous build
if [ -d "$WORK_DIR" ]; then
    print_step "Cleaning up previous build..."
    rm -rf "$WORK_DIR"
fi

mkdir -p "$WORK_DIR" "$CACHE_DIR"
cd "$WORK_DIR"

# Download Alpine minirootfs (with caching)
ALPINE_TARBALL="alpine-minirootfs-${ALPINE_RELEASE}-x86_64.tar.gz"
ALPINE_URL="https://dl-cdn.alpinelinux.org/alpine/v${ALPINE_VERSION}/releases/x86_64/${ALPINE_TARBALL}"
CACHED_TARBALL="${CACHE_DIR}/${ALPINE_TARBALL}"

if [ -f "$CACHED_TARBALL" ]; then
    print_step "Using cached Alpine minirootfs from $CACHED_TARBALL"
    cp "$CACHED_TARBALL" alpine-minirootfs.tar.gz
else
    print_step "Downloading Alpine Linux minirootfs..."
    if wget -q --show-progress --timeout=30 --tries=3 "$ALPINE_URL" -O alpine-minirootfs.tar.gz; then
        # Cache it for future use
        cp alpine-minirootfs.tar.gz "$CACHED_TARBALL"
        print_step "Cached Alpine tarball for future builds"
    else
        print_error "Failed to download Alpine minirootfs"
        print_error "Please download manually from:"
        print_error "$ALPINE_URL"
        print_error "And save to: $CACHED_TARBALL"
        exit 1
    fi
fi

# Extract Alpine
print_step "Extracting Alpine Linux..."
mkdir -p rootfs
tar xzf alpine-minirootfs.tar.gz -C rootfs

# Copy /etc/resolv.conf for DNS during build
print_step "Configuring DNS for package installation..."
cp /etc/resolv.conf rootfs/etc/resolv.conf

# Install Docker and dependencies
print_step "Installing Docker and OpenRC..."
cat > rootfs/tmp/setup.sh << 'SETUP_SCRIPT'
#!/bin/sh
set -e

# Update package index
echo "Updating package index..."
apk update || {
    echo "Failed to update package index"
    echo "Trying with alternative mirror..."
    echo "http://dl-4.alpinelinux.org/alpine/v3.18/main" > /etc/apk/repositories
    echo "http://dl-4.alpinelinux.org/alpine/v3.18/community" >> /etc/apk/repositories
    apk update
}

# Install Docker and OpenRC
echo "Installing packages..."
apk add --no-cache \
    docker \
    docker-openrc \
    openrc \
    util-linux \
    coreutils \
    bash \
    curl \
    ca-certificates

# Configure OpenRC
echo "Configuring OpenRC services..."
rc-update add devfs boot
rc-update add procfs boot
rc-update add sysfs boot
rc-update add cgroups boot
rc-update add networking boot
rc-update add docker default

echo "Docker version: $(docker --version)"

# Test Docker service startup (debug)
echo "Testing Docker service configuration..."
echo "OpenRC docker service status:"
rc-status docker || echo "Docker service not yet started (expected)"

echo "Docker configuration files:"
ls -la /etc/conf.d/docker /etc/docker/daemon.json || echo "Config files missing"

echo "Docker binary location:"
which docker

# Enable local services for debug script
rc-update add local default
SETUP_SCRIPT

chmod +x rootfs/tmp/setup.sh
chroot rootfs /tmp/setup.sh
rm rootfs/tmp/setup.sh

# Configure Docker daemon for OpenRC
print_step "Configuring Docker daemon..."
mkdir -p rootfs/etc/docker rootfs/etc/conf.d

# OpenRC configuration for Docker
cat > rootfs/etc/conf.d/docker << 'EOF'
# Docker daemon options for OpenRC
DOCKER_OPTS="-H unix:///var/run/docker.sock -H tcp://0.0.0.0:2375"
EOF

# Docker daemon JSON config (for additional settings)
cat > rootfs/etc/docker/daemon.json << 'EOF'
{
    "storage-driver": "overlay2",
    "log-driver": "json-file",
    "log-opts": {
        "max-size": "10m",
        "max-file": "3"
    }
}
EOF

# Install guest-agent if available
print_step "Installing guest-agent (if available)..."
GUEST_AGENT_BIN="target/x86_64-unknown-linux-musl/release/guest-agent"
if [ -f "$GUEST_AGENT_BIN" ]; then
    print_step "Found guest-agent binary, installing..."

    # Copy guest-agent binary
    cp "$GUEST_AGENT_BIN" rootfs/usr/local/bin/guest-agent
    chmod +x rootfs/usr/local/bin/guest-agent

    # Create OpenRC service for guest-agent
    cat > rootfs/etc/init.d/guest-agent << 'GUEST_AGENT_SERVICE'
#!/sbin/openrc-run

name="guest-agent"
description="Guest metrics agent"
command="/usr/local/bin/guest-agent"
command_background=true
pidfile="/run/guest-agent.pid"
output_log="/var/log/guest-agent.log"
error_log="/var/log/guest-agent.err"

depend() {
    need net
    after networking
}
GUEST_AGENT_SERVICE

    chmod +x rootfs/etc/init.d/guest-agent

    # Enable guest-agent to start on boot
    chroot rootfs rc-update add guest-agent default

    print_step "✅ Guest-agent pre-baked into image"
else
    print_step "⚠️  Guest-agent binary not found at $GUEST_AGENT_BIN, skipping"
    print_step "   Build it with: cargo build --release --target x86_64-unknown-linux-musl -p guest-agent"
fi

# Configure networking (DHCP)
print_step "Configuring networking..."
cat > rootfs/etc/network/interfaces << 'EOF'
auto lo
iface lo inet loopback

auto eth0
iface eth0 inet dhcp
EOF

# Set hostname
echo "container-runtime" > rootfs/etc/hostname

# Enable root login without password (for debugging)
sed -i 's/root:!:/root::/' rootfs/etc/shadow

# Force Docker to start on boot with proper configuration
cat > rootfs/etc/init.d/docker-force << 'EOF'
#!/sbin/openrc-run

description="Force start Docker daemon with TCP support"

depend() {
    need net
    after firewall
}

start() {
    ebegin "Starting Docker daemon with TCP support"
    
    # Ensure Docker daemon directory exists
    mkdir -p /var/run /var/lib/docker
    
    # Start Docker daemon with explicit options
    /usr/bin/dockerd \
        --host=unix:///var/run/docker.sock \
        --host=tcp://0.0.0.0:2375 \
        --storage-driver=overlay2 \
        --log-driver=json-file \
        --log-opt max-size=10m \
        --log-opt max-file=3 \
        > /var/log/docker.log 2>&1 &
    
    DOCKER_PID=$!
    echo $DOCKER_PID > /var/run/docker.pid
    
    # Wait for Docker to be ready
    for i in $(seq 1 30); do
        if /usr/bin/docker info >/dev/null 2>&1; then
            eend 0 "Docker daemon started successfully"
            return 0
        fi
        sleep 1
    done
    
    eend 1 "Docker daemon failed to start"
    return 1
}

stop() {
    ebegin "Stopping Docker daemon"
    if [ -f /var/run/docker.pid ]; then
        kill $(cat /var/run/docker.pid)
        rm -f /var/run/docker.pid
    fi
    eend 0
}
EOF

chmod +x rootfs/etc/init.d/docker-force

# Enable the forced Docker service
chroot rootfs rc-update add docker-force default

# Add debugging startup script
cat > rootfs/usr/local/bin/debug-docker.sh << 'EOF'
#!/bin/bash
echo "=== Docker Debug Script ===" > /tmp/docker-debug.log
echo "Time: $(date)" >> /tmp/docker-debug.log
echo "Hostname: $(hostname)" >> /tmp/docker-debug.log
echo "IP Address: $(ip addr show eth0 | grep 'inet ' | awk '{print $2}')" >> /tmp/docker-debug.log
echo "" >> /tmp/docker-debug.log
echo "=== Network Configuration ===" >> /tmp/docker-debug.log
cat /etc/network/interfaces >> /tmp/docker-debug.log
echo "" >> /tmp/docker-debug.log
echo "=== OpenRC Services ===" >> /tmp/docker-debug.log
rc-status >> /tmp/docker-debug.log 2>&1
echo "" >> /tmp/docker-debug.log
echo "=== Docker Service Status ===" >> /tmp/docker-debug.log
rc-status docker >> /tmp/docker-debug.log 2>&1
rc-status docker-force >> /tmp/docker-debug.log 2>&1
echo "" >> /tmp/docker-debug.log
echo "=== Docker Configuration ===" >> /tmp/docker-debug.log
echo "OpenRC config:" >> /tmp/docker-debug.log
cat /etc/conf.d/docker >> /tmp/docker-debug.log 2>&1
echo "" >> /tmp/docker-debug.log
echo "Daemon config:" >> /tmp/docker-debug.log
cat /etc/docker/daemon.json >> /tmp/docker-debug.log 2>&1
echo "" >> /tmp/docker-debug.log
echo "=== Docker Process Check ===" >> /tmp/docker-debug.log
ps aux | grep docker >> /tmp/docker-debug.log 2>&1
echo "" >> /tmp/docker-debug.log
echo "=== Docker Socket Check ===" >> /tmp/docker-debug.log
ls -la /var/run/docker.sock >> /tmp/docker-debug.log 2>&1
echo "" >> /tmp/docker-debug.log
echo "=== Docker Log ===" >> /tmp/docker-debug.log
tail -20 /var/log/docker.log >> /tmp/docker-debug.log 2>&1
echo "" >> /tmp/docker-debug.log
echo "=== Testing Docker Daemon ===" >> /tmp/docker-debug.log
docker info >> /tmp/docker-debug.log 2>&1
echo "" >> /tmp/docker-debug.log
echo "=== Testing Docker TCP Port ===" >> /tmp/docker-debug.log
curl -v http://127.0.0.1:2375/_ping >> /tmp/docker-debug.log 2>&1
echo "=== End Debug Script ===" >> /tmp/docker-debug.log
EOF

chmod +x rootfs/usr/local/bin/debug-docker.sh

# Add debug script to run after boot
cat > rootfs/etc/crontab << 'EOF'
# Run debug script 1 minute after boot
* * * * * root sleep 60 && /usr/local/bin/debug-docker.sh
EOF

# Create ext4 filesystem
print_step "Creating ext4 filesystem (${IMAGE_SIZE})..."
mkfs.ext4 -L container-runtime -d rootfs -E lazy_itable_init=0,lazy_journal_init=0 container-runtime.ext4 "$IMAGE_SIZE"

# Copy to output location
print_step "Moving image to $OUTPUT_IMAGE..."
mkdir -p "$(dirname "$OUTPUT_IMAGE")"
mv container-runtime.ext4 "$OUTPUT_IMAGE"

# Cleanup
print_step "Cleaning up..."
cd ..
rm -rf "$WORK_DIR"

echo ""
echo -e "${GREEN}=========================================="
echo "✅ Container runtime image built successfully!"
echo "=========================================="
echo -e "${NC}"
echo "Image location: $OUTPUT_IMAGE"
echo "Image size: $(du -h "$OUTPUT_IMAGE" | cut -f1)"
echo ""
echo "Test the image:"
echo "  1. Create a container with image='hello-world:latest'"
echo "  2. Check if VM boots: curl http://localhost:18080/v1/vms"
echo "  3. Test Docker API: curl http://<guest-ip>:2375/_ping"
echo "  4. For debugging: ssh root@<guest-ip> and cat /tmp/docker-debug.log"
echo ""
