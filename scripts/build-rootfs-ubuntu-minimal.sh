#!/bin/bash
# Build Ubuntu 24.04 LTS Minimal Rootfs for Firecracker MicroVMs
# This script creates a minimal Ubuntu rootfs image suitable for Firecracker with:
# - Cloud-init for credential/network configuration
# - systemd-networkd for networking
# - OpenSSH server
# - Guest agent auto-start support

set -euo pipefail

# Configuration
DISTRO="ubuntu"
VERSION="24.04"
CODENAME="noble"
OUTPUT_IMAGE="${1:-/srv/images/ubuntu-24.04-minimal.ext4}"
IMAGE_SIZE_MB="${2:-800}"  # 800MB to accommodate build process with packages
MOUNT_POINT="/tmp/ubuntu-build-$$"
ARCH="amd64"

# Check available disk space in /tmp
AVAILABLE_SPACE=$(df /tmp | tail -1 | awk '{print $4}')
REQUIRED_SPACE=$((IMAGE_SIZE_MB * 1024 * 2))  # Need 2x image size for build

if [ "$AVAILABLE_SPACE" -lt "$REQUIRED_SPACE" ]; then
    error "Not enough space in /tmp. Available: ${AVAILABLE_SPACE}KB, Required: ${REQUIRED_SPACE}KB"
    error "Try: IMAGE_SIZE_MB=300 $0 or free up space in /tmp"
fi

# Colors for output
RED='\033[0:31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

info() { echo -e "${GREEN}[INFO]${NC} $*"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }
error() { echo -e "${RED}[ERROR]${NC} $*"; exit 1; }

# Check if running as root
[[ $EUID -ne 0 ]] && error "This script must be run as root"

# Check for required tools
for tool in debootstrap mkfs.ext4 chroot; do
    command -v "$tool" &>/dev/null || error "$tool is not installed"
done

info "Building Ubuntu $VERSION ($CODENAME) minimal rootfs..."
info "Output: $OUTPUT_IMAGE ($IMAGE_SIZE_MB MB)"

# Clean up on exit
cleanup() {
    info "Cleaning up..."
    umount "$MOUNT_POINT/dev/pts" 2>/dev/null || true
    umount "$MOUNT_POINT/dev" 2>/dev/null || true
    umount "$MOUNT_POINT/proc" 2>/dev/null || true
    umount "$MOUNT_POINT/sys" 2>/dev/null || true
    umount "$MOUNT_POINT" 2>/dev/null || true
    rm -rf "$MOUNT_POINT"
}
trap cleanup EXIT

# Create image file
info "Creating ${IMAGE_SIZE_MB}MB ext4 image..."
dd if=/dev/zero of="$OUTPUT_IMAGE" bs=1M count="$IMAGE_SIZE_MB" status=progress
mkfs.ext4 -F "$OUTPUT_IMAGE"

# Mount the image
info "Mounting image..."
mkdir -p "$MOUNT_POINT"
mount -o loop "$OUTPUT_IMAGE" "$MOUNT_POINT"

# Install base system with debootstrap
info "Running debootstrap (this may take several minutes)..."
debootstrap \
    --arch="$ARCH" \
    --variant=minbase \
    --include=systemd,systemd-sysv,udev,kmod,apt-utils \
    "$CODENAME" \
    "$MOUNT_POINT" \
    http://archive.ubuntu.com/ubuntu/

# Mount proc/sys/dev for chroot
info "Setting up chroot environment..."
mount -t proc none "$MOUNT_POINT/proc"
mount -t sysfs none "$MOUNT_POINT/sys"
mount --bind /dev "$MOUNT_POINT/dev"
mount --bind /dev/pts "$MOUNT_POINT/dev/pts" 2>/dev/null || true

# Configure APT sources
info "Configuring APT sources..."
cat > "$MOUNT_POINT/etc/apt/sources.list" <<EOF
deb http://archive.ubuntu.com/ubuntu/ $CODENAME main restricted universe multiverse
deb http://archive.ubuntu.com/ubuntu/ $CODENAME-updates main restricted universe multiverse
deb http://security.ubuntu.com/ubuntu $CODENAME-security main restricted universe multiverse
EOF

# Configure basic system
info "Configuring system..."
cat > "$MOUNT_POINT/etc/hostname" <<EOF
ubuntu-microvm
EOF

cat > "$MOUNT_POINT/etc/hosts" <<EOF
127.0.0.1   localhost
127.0.1.1   ubuntu-microvm
::1         localhost ip6-localhost ip6-loopback
EOF

# Configure networking with systemd-networkd
info "Configuring systemd-networkd..."
mkdir -p "$MOUNT_POINT/etc/systemd/network"
cat > "$MOUNT_POINT/etc/systemd/network/20-wired.network" <<EOF
[Match]
Name=e*

[Network]
DHCP=yes
EOF

# Install essential packages
info "Installing essential packages..."
chroot "$MOUNT_POINT" /bin/bash -c "
export DEBIAN_FRONTEND=noninteractive
export APT_LISTCHANGES_FRONTEND=none

# Update package lists
apt-get update

# Install packages in batches with cleanup between each batch to save space
info 'Installing batch 1: SSH and networking...'
apt-get install -y --no-install-recommends openssh-server iproute2 iputils-ping
apt-get clean && rm -rf /var/lib/apt/lists/* /usr/share/doc/* /usr/share/man/*

info 'Installing batch 2: cloud-init...'
apt-get update
apt-get install -y --no-install-recommends cloud-init
apt-get clean && rm -rf /var/lib/apt/lists/* /usr/share/doc/* /usr/share/man/*

info 'Installing batch 3: utilities...'
apt-get update
apt-get install -y --no-install-recommends curl wget ca-certificates sudo net-tools vim-tiny
apt-get clean && rm -rf /var/lib/apt/lists/* /usr/share/doc/* /usr/share/man/*

# Final aggressive cleanup
rm -rf /tmp/* /var/tmp/* /var/cache/apt/* /var/log/*
"

# Configure cloud-init
info "Configuring cloud-init..."
cat > "$MOUNT_POINT/etc/cloud/cloud.cfg.d/99-nqrust.cfg" <<EOF
# NQRust-MicroVM cloud-init configuration
datasource_list: [ NoCloud, None ]
datasource:
  NoCloud:
    seedfrom: http://169.254.169.254/

# Disable network config from cloud-init (use systemd-networkd)
network:
  config: disabled

# Basic modules
cloud_init_modules:
  - migrator
  - seed_random
  - bootcmd
  - write-files
  - set_hostname
  - update_hostname
  - users-groups
  - ssh

cloud_config_modules:
  - runcmd
  - ssh-import-id
  - set-passwords

cloud_final_modules:
  - scripts-per-once
  - scripts-per-boot
  - scripts-per-instance
  - scripts-user
  - ssh-authkey-fingerprints
  - final-message
EOF

# Enable systemd services
info "Enabling systemd services..."
chroot "$MOUNT_POINT" /bin/bash -c "
systemctl enable systemd-networkd
systemctl enable systemd-resolved
systemctl enable ssh
systemctl enable cloud-init
systemctl enable cloud-init-local
systemctl enable cloud-config
systemctl enable cloud-final
"

# Configure serial console
info "Configuring serial console..."
chroot "$MOUNT_POINT" /bin/bash -c "
systemctl enable serial-getty@ttyS0.service
"

# Set root password (will be overwritten by cloud-init)
info "Setting default root password..."
chroot "$MOUNT_POINT" /bin/bash -c "
echo 'root:root' | chpasswd
"

# Create /etc/fstab
info "Creating fstab..."
cat > "$MOUNT_POINT/etc/fstab" <<EOF
# <file system> <mount point> <type> <options> <dump> <pass>
/dev/vda        /               ext4   errors=remount-ro 0 1
EOF

# Disable unnecessary services to speed up boot
info "Disabling unnecessary services..."
chroot "$MOUNT_POINT" /bin/bash -c "
systemctl disable apt-daily.timer || true
systemctl disable apt-daily-upgrade.timer || true
systemctl disable unattended-upgrades || true
systemctl mask systemd-resolved || true  # Use simpler DNS
"

# Create symlink for /etc/resolv.conf
info "Configuring DNS..."
rm -f "$MOUNT_POINT/etc/resolv.conf"
cat > "$MOUNT_POINT/etc/resolv.conf" <<EOF
nameserver 8.8.8.8
nameserver 8.8.4.4
EOF

# Clean up
info "Cleaning up rootfs..."
chroot "$MOUNT_POINT" /bin/bash -c "
# Remove unnecessary files
rm -rf /tmp/* /var/tmp/*
rm -rf /var/cache/apt/*
rm -rf /var/lib/apt/lists/*
rm -rf /usr/share/doc/*
rm -rf /usr/share/man/*
rm -rf /usr/share/info/*

# Clear log files
find /var/log -type f -exec truncate -s 0 {} \;
"

# Unmount everything
info "Unmounting..."
umount "$MOUNT_POINT/dev/pts" 2>/dev/null || true
umount "$MOUNT_POINT/dev"
umount "$MOUNT_POINT/proc"
umount "$MOUNT_POINT/sys"
umount "$MOUNT_POINT"
rm -rf "$MOUNT_POINT"

# Show final size
FINAL_SIZE=$(du -h "$OUTPUT_IMAGE" | cut -f1)
info "Build complete!"
info "Image: $OUTPUT_IMAGE ($FINAL_SIZE)"
info ""
info "To use this image with NQRust-MicroVM:"
info "  1. Upload to manager:"
info "     curl -X POST http://localhost:18080/v1/images \\"
info "       -F 'file=@$OUTPUT_IMAGE' \\"
info "       -F 'name=ubuntu-24.04-minimal' \\"
info "       -F 'kind=rootfs'"
info ""
info "  2. The image includes:"
info "     ✓ Cloud-init (credentials via MMDS)"
info "     ✓ systemd-networkd (DHCP networking)"
info "     ✓ OpenSSH server"
info "     ✓ Guest agent support"
info ""
info "  3. Default credentials (before cloud-init):"
info "     Username: root"
info "     Password: root"
