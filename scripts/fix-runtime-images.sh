#!/bin/bash
set -e

# Fix script for runtime images that are missing proper init configuration
# This patches existing images to add proper /etc/inittab, TERM settings,
# and OpenRC boot sequence

IMAGES_DIR="${1:-/srv/images}"

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

if [ "$EUID" -ne 0 ]; then
    print_error "This script must be run as root (or with sudo)"
    exit 1
fi

MOUNT_POINT="/tmp/fix-runtime-$$"

# Fix function runtime images (Alpine with OpenRC)
fix_runtime_image() {
    local image_path="$1"
    local image_name="$(basename "$image_path")"
    
    if [ ! -f "$image_path" ]; then
        print_step "Skipping $image_name (not found)"
        return 0
    fi
    
    print_step "Fixing $image_name..."
    
    # Create mount point
    mkdir -p "$MOUNT_POINT"
    
    # Mount the image
    mount -o loop "$image_path" "$MOUNT_POINT"
    
    # Ensure cleanup on exit
    trap "umount '$MOUNT_POINT' 2>/dev/null; rm -rf '$MOUNT_POINT'" EXIT
    
    # Create proper /etc/inittab for BusyBox init to start OpenRC
    print_step "  Adding /etc/inittab..."
    cat > "$MOUNT_POINT/etc/inittab" << 'EOF'
# /etc/inittab for Alpine Linux function runtime
::sysinit:/sbin/openrc sysinit
::sysinit:/sbin/openrc boot
::wait:/sbin/openrc default

# Enable serial console for Firecracker with proper terminal type
ttyS0::respawn:/sbin/getty -L -n -l /bin/sh ttyS0 115200 vt100

# Ctrl-Alt-Del -> reboot
::ctrlaltdel:/sbin/reboot

# Shutdown
::shutdown:/sbin/openrc shutdown
EOF

    # Set default TERM for serial console (fixes "Cannot find terminfo entry for 'unknown'")
    mkdir -p "$MOUNT_POINT/etc/profile.d"
    echo 'export TERM=vt100' > "$MOUNT_POINT/etc/profile.d/term.sh"
    chmod +x "$MOUNT_POINT/etc/profile.d/term.sh"
    
    # Create required directories for OpenRC
    mkdir -p "$MOUNT_POINT/run/openrc"
    touch "$MOUNT_POINT/run/openrc/softlevel"
    
    # Check if runtime-server service exists
    if [ -f "$MOUNT_POINT/etc/init.d/runtime-server" ]; then
        print_step "  runtime-server service exists ✓"
        
        # Make sure it's properly configured with logging
        cat > "$MOUNT_POINT/etc/init.d/runtime-server" << 'EOF'
#!/sbin/openrc-run

name="runtime-server"
description="NQRust Lambda Runtime Server"

command_background=true
pidfile="/run/runtime-server.pid"
output_log="/var/log/runtime-server.log"
error_log="/var/log/runtime-server.err"

depend() {
    need net
    after networking
}

start_pre() {
    export FUNCTION_HANDLER="${FUNCTION_HANDLER:-handler}"
    export PORT="${PORT:-3000}"

    # Determine runtime type based on what's installed
    if [ -x /usr/bin/bun ]; then
        command="/usr/bin/bun"
        command_args="/usr/local/bin/runtime-server"
    elif [ -x /usr/bin/python3 ]; then
        command="/usr/bin/python3"
        command_args="/usr/local/bin/runtime-server"
    else
        eerror "No runtime (bun/python) found"
        return 1
    fi
}
EOF
        chmod +x "$MOUNT_POINT/etc/init.d/runtime-server"
        
        # Ensure it's enabled
        if [ ! -L "$MOUNT_POINT/etc/runlevels/default/runtime-server" ]; then
            mkdir -p "$MOUNT_POINT/etc/runlevels/default"
            ln -sf /etc/init.d/runtime-server "$MOUNT_POINT/etc/runlevels/default/runtime-server"
            print_step "  Enabled runtime-server service"
        fi
    else
        print_step "  WARNING: runtime-server service not found!"
    fi
    
    # Ensure networking is enabled
    if [ ! -L "$MOUNT_POINT/etc/runlevels/boot/networking" ]; then
        mkdir -p "$MOUNT_POINT/etc/runlevels/boot"
        [ -f "$MOUNT_POINT/etc/init.d/networking" ] && \
            ln -sf /etc/init.d/networking "$MOUNT_POINT/etc/runlevels/boot/networking"
    fi
    
    # Ensure devfs/procfs/sysfs are enabled for sysinit
    mkdir -p "$MOUNT_POINT/etc/runlevels/sysinit"
    for svc in devfs dmesg mdev; do
        if [ -f "$MOUNT_POINT/etc/init.d/$svc" ] && [ ! -L "$MOUNT_POINT/etc/runlevels/sysinit/$svc" ]; then
            ln -sf "/etc/init.d/$svc" "$MOUNT_POINT/etc/runlevels/sysinit/$svc"
        fi
    done
    
    # Unmount
    umount "$MOUNT_POINT"
    rm -rf "$MOUNT_POINT"
    trap - EXIT
    
    print_step "  $image_name fixed ✓"
}

# Fix base Alpine images (no OpenRC)
fix_alpine_base() {
    local image_path="$1"
    local image_name="$(basename "$image_path")"
    
    if [ ! -f "$image_path" ]; then
        print_step "Skipping $image_name (not found)"
        return 0
    fi
    
    print_step "Fixing $image_name..."
    
    mkdir -p "$MOUNT_POINT"
    mount -o loop "$image_path" "$MOUNT_POINT"
    trap "umount '$MOUNT_POINT' 2>/dev/null; rm -rf '$MOUNT_POINT'" EXIT
    
    # Create simple inittab for serial console
    cat > "$MOUNT_POINT/etc/inittab" << 'EOF'
::sysinit:/bin/mount -t proc proc /proc
::sysinit:/bin/mount -t sysfs sysfs /sys
::sysinit:/bin/mount -t devtmpfs devtmpfs /dev
ttyS0::respawn:/sbin/getty -L -n -l /bin/sh ttyS0 115200 vt100
::ctrlaltdel:/sbin/reboot
::shutdown:/bin/umount -a -r
EOF
    
    # Set TERM
    mkdir -p "$MOUNT_POINT/etc/profile.d"
    echo 'export TERM=vt100' > "$MOUNT_POINT/etc/profile.d/term.sh"
    chmod +x "$MOUNT_POINT/etc/profile.d/term.sh"
    
    umount "$MOUNT_POINT"
    rm -rf "$MOUNT_POINT"
    trap - EXIT
    
    print_step "  $image_name fixed ✓"
}

# Fix Ubuntu images
fix_ubuntu_image() {
    local image_path="$1"
    local image_name="$(basename "$image_path")"
    
    if [ ! -f "$image_path" ]; then
        print_step "Skipping $image_name (not found)"
        return 0
    fi
    
    print_step "Fixing $image_name..."
    
    mkdir -p "$MOUNT_POINT"
    mount -o loop "$image_path" "$MOUNT_POINT"
    trap "umount '$MOUNT_POINT' 2>/dev/null; rm -rf '$MOUNT_POINT'" EXIT
    
    # Set TERM
    mkdir -p "$MOUNT_POINT/etc/profile.d"
    echo 'export TERM=vt100' > "$MOUNT_POINT/etc/profile.d/term.sh"
    chmod +x "$MOUNT_POINT/etc/profile.d/term.sh"
    
    umount "$MOUNT_POINT"
    rm -rf "$MOUNT_POINT"
    trap - EXIT
    
    print_step "  $image_name fixed ✓"
}

echo "=========================================="
echo "NQRust Runtime Image Fix Script"
echo "=========================================="
echo ""
print_step "Fixing images in $IMAGES_DIR"
echo ""

# Fix function runtime images (these have OpenRC)
fix_runtime_image "$IMAGES_DIR/bun-runtime.ext4"
fix_runtime_image "$IMAGES_DIR/python-runtime.ext4"

# Fix base images
fix_alpine_base "$IMAGES_DIR/alpine-3.18-minimal.ext4"
fix_ubuntu_image "$IMAGES_DIR/ubuntu-24.04-minimal.ext4"

echo ""
echo -e "${GREEN}=========================================="
echo "✅ All images fixed!"
echo "=========================================="
echo -e "${NC}"
echo ""
echo "Now restart any existing VMs or recreate them."
echo "For functions:"
echo "  1. Delete existing functions: DELETE /v1/functions/{id}"
echo "  2. Recreate them: POST /v1/functions"
echo ""
echo "Or restart the manager service:"
echo "  sudo systemctl restart nqrust-manager"
echo ""
