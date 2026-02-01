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

# Configure inittab to start OpenRC
print_step "Configuring init system..."
cat > rootfs/etc/inittab << 'EOF'
# /etc/inittab

::sysinit:/sbin/openrc sysinit
::sysinit:/sbin/openrc boot
::wait:/sbin/openrc default

# Set up a couple of getty's
ttyS0::respawn:/sbin/getty -L ttyS0 115200 vt100

# Stuff to do for the 3-finger salute
::ctrlaltdel:/sbin/reboot

# Stuff to do before rebooting
::shutdown:/sbin/openrc shutdown
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
echo ""
