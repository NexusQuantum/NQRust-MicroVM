#!/bin/bash
set -e

echo "=========================================="
echo "Building Bun/TypeScript Function Runtime Image"
echo "=========================================="
echo "Using Alpine Linux + OpenRC + Bun 1.1.35 (native musl)"
echo "=========================================="

# Configuration - Alpine based (like Node runtime)
# Using Alpine 3.19 for newer libstdc++ (Bun needs newer C++ ABI)
ALPINE_VERSION="3.19"
ALPINE_RELEASE="3.19.4"
WORK_DIR="$(pwd)/build-bun-runtime"
IMAGE_SIZE="1G"
# Convert to absolute path in case it's relative (for CI)
OUTPUT_IMAGE="${OUTPUT_IMAGE:-/srv/images/bun-runtime.ext4}"
if [[ "$OUTPUT_IMAGE" != /* ]]; then
    OUTPUT_IMAGE="$(pwd)/$OUTPUT_IMAGE"
fi
CACHE_DIR="/tmp/alpine-cache"

# Bun version - 1.1.35+ has native musl/Alpine support
BUN_VERSION="1.1.35"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m'

print_step() {
    echo -e "${BLUE}==>${NC} ${GREEN}$1${NC}"
}

print_error() {
    echo -e "${RED}ERROR:${NC} $1"
}

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    print_error "This script must be run as root (or with sudo)"
    exit 1
fi

# Check if runtime server exists
RUNTIME_SERVER="apps/function-runtime/bun/server.ts"
if [ ! -f "$RUNTIME_SERVER" ]; then
    print_error "Runtime server not found at $RUNTIME_SERVER"
    print_error "Please ensure you're running this from the project root"
    exit 1
fi
RUNTIME_SERVER_ABS="$(realpath "$RUNTIME_SERVER")"

# Get absolute path to guest-agent if it exists
GUEST_AGENT_REL="target/x86_64-unknown-linux-musl/release/guest-agent"
if [ -f "$GUEST_AGENT_REL" ]; then
    GUEST_AGENT_ABS="$(realpath "$GUEST_AGENT_REL")"
    print_step "Found guest-agent binary at $GUEST_AGENT_ABS"
else
    GUEST_AGENT_ABS=""
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
        cp alpine-minirootfs.tar.gz "$CACHED_TARBALL"
        print_step "Cached Alpine tarball for future builds"
    else
        print_error "Failed to download Alpine minirootfs"
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

# Download Bun musl binary
print_step "Downloading Bun ${BUN_VERSION} (musl/Alpine native binary)..."
BUN_URL="https://github.com/oven-sh/bun/releases/download/bun-v${BUN_VERSION}/bun-linux-x64-musl.zip"
CACHED_BUN="${CACHE_DIR}/bun-${BUN_VERSION}-linux-x64-musl.zip"

if [ -f "$CACHED_BUN" ]; then
    print_step "Using cached Bun binary from $CACHED_BUN"
    cp "$CACHED_BUN" bun.zip
else
    if wget -q --show-progress --timeout=60 --tries=3 "$BUN_URL" -O bun.zip; then
        cp bun.zip "$CACHED_BUN"
        print_step "Cached Bun binary for future builds"
    else
        print_error "Failed to download Bun binary"
        exit 1
    fi
fi

# Extract Bun
print_step "Extracting Bun..."
unzip -q bun.zip
cp bun-linux-x64-musl/bun rootfs/usr/local/bin/bun
chmod +x rootfs/usr/local/bin/bun

# Install minimal dependencies
print_step "Installing OpenRC and dependencies..."
cat > rootfs/tmp/setup.sh << 'SETUP_SCRIPT'
#!/bin/sh
set -e

# Update package index
echo "Updating package index..."
apk update || {
    echo "Failed to update package index, trying alternative mirror..."
    echo "http://dl-4.alpinelinux.org/alpine/v3.19/main" > /etc/apk/repositories
    echo "http://dl-4.alpinelinux.org/alpine/v3.19/community" >> /etc/apk/repositories
    apk update
}

# Install minimal packages - Bun needs libstdc++ for C++ stdlib
echo "Installing packages..."
apk add --no-cache \
    openrc \
    util-linux \
    coreutils \
    libgcc \
    libstdc++

# Configure OpenRC
rc-update add devfs boot
rc-update add procfs boot
rc-update add sysfs boot
rc-update add networking boot

# Verify Bun works
echo "Bun version: $(/usr/local/bin/bun --version)"
SETUP_SCRIPT

chmod +x rootfs/tmp/setup.sh
chroot rootfs /tmp/setup.sh
rm rootfs/tmp/setup.sh

# Copy runtime server
print_step "Installing runtime server..."
mkdir -p rootfs/function
cp "$RUNTIME_SERVER_ABS" rootfs/usr/local/bin/runtime-server.ts
chmod +x rootfs/usr/local/bin/runtime-server.ts

# Create placeholder function code
print_step "Creating placeholder function..."
cat > rootfs/function/code.ts << 'EOF'
// Placeholder TypeScript function
// This will be replaced when a function is deployed

interface Event {
    body?: string;
    headers?: Record<string, string>;
    queryParams?: Record<string, string>;
    path?: string;
    method?: string;
}

interface Response {
    statusCode?: number;
    body?: string;
    headers?: Record<string, string>;
}

export async function handler(event: Event): Promise<Response> {
    return {
        statusCode: 200,
        body: JSON.stringify({
            message: "Hello from NQRust Lambda (Bun)!",
            event: event,
            timestamp: new Date().toISOString()
        }),
        headers: {
            "Content-Type": "application/json"
        }
    };
}
EOF

# Create wrapper script for runtime server (to set environment properly)
print_step "Creating runtime server wrapper..."
cat > rootfs/usr/local/bin/start-runtime-server << 'EOF'
#!/bin/sh
export FUNCTION_HANDLER="${FUNCTION_HANDLER:-handler}"
export PORT="${PORT:-3000}"
export FUNCTION_CODE_PATH="${FUNCTION_CODE_PATH:-/function/code.ts}"
exec /usr/local/bin/bun run /usr/local/bin/runtime-server.ts
EOF
chmod +x rootfs/usr/local/bin/start-runtime-server

# Create OpenRC service for runtime server
print_step "Creating runtime server service..."
cat > rootfs/etc/init.d/runtime-server << 'EOF'
#!/sbin/openrc-run

name="runtime-server"
description="NQRust Lambda Runtime Server (Bun)"

command="/usr/local/bin/start-runtime-server"
command_background=true
pidfile="/run/runtime-server.pid"
output_log="/var/log/runtime-server.log"
error_log="/var/log/runtime-server.err"

depend() {
    need net
    after networking
}
EOF

chmod +x rootfs/etc/init.d/runtime-server

# Enable runtime server to start on boot
print_step "Enabling runtime server..."
chroot rootfs rc-update add runtime-server default

# Install guest-agent if available
print_step "Installing guest-agent (if available)..."
if [ -n "$GUEST_AGENT_ABS" ] && [ -f "$GUEST_AGENT_ABS" ]; then
    print_step "Found guest-agent binary, installing..."

    cp "$GUEST_AGENT_ABS" rootfs/usr/local/bin/guest-agent
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
    chroot rootfs rc-update add guest-agent default

    print_step "✅ Guest-agent pre-baked into image"
else
    print_step "⚠️  Guest-agent binary not found, skipping"
    print_step "   Build it with: cargo build --release --target x86_64-unknown-linux-musl -p guest-agent"
    print_step "   Then run this script from project root"
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
echo "lambda-bun" > rootfs/etc/hostname

# Enable root login without password (for debugging)
sed -i 's/root:!:/root::/' rootfs/etc/shadow

# Create ext4 filesystem
print_step "Creating ext4 filesystem (${IMAGE_SIZE})..."
mkfs.ext4 -L bun-runtime -d rootfs -E lazy_itable_init=0,lazy_journal_init=0 bun-runtime.ext4 "$IMAGE_SIZE"

# Copy to output location
print_step "Moving image to $OUTPUT_IMAGE..."
mkdir -p "$(dirname "$OUTPUT_IMAGE")"
mv bun-runtime.ext4 "$OUTPUT_IMAGE"

# Cleanup
print_step "Cleaning up..."
cd ..
rm -rf "$WORK_DIR"

echo ""
echo -e "${GREEN}=========================================="
echo "✅ Bun runtime image built successfully!"
echo "=========================================="
echo -e "${NC}"
echo "Image location: $OUTPUT_IMAGE"
echo "Image size: $(du -h "$OUTPUT_IMAGE" | cut -f1)"
echo ""
echo "Key improvements:"
echo "  • Alpine Linux (not Debian) - smaller, faster boot"
echo "  • OpenRC (not systemd) - simple, reliable init"
echo "  • Bun 1.1.35 native musl binary - no glibc hack"
echo "  • Should work just like Node functions!"
echo ""
echo "Test the image:"
echo "  1. Create a function with runtime='bun'"
echo "  2. Check if VM boots: curl http://localhost:18080/v1/vms"
echo "  3. Test health: curl http://<guest-ip>:3000/health"
echo ""
