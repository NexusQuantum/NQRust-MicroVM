#!/bin/bash
# =============================================================================
# NQR-MicroVM Air-Gapped ISO Builder
# =============================================================================
# Creates a bootable Debian-based ISO with all NQR-MicroVM components bundled
# for fully offline installation.
#
# Requirements:
#   - Debian/Ubuntu host system
#   - live-build package installed
#   - Docker installed (for image export)
#   - ~10GB free disk space
#
# Usage:
#   ./create-airgap-iso.sh [--release <version>] [--output <dir>]
#
# =============================================================================

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
BUILD_DIR="${SCRIPT_DIR}/build"
OUTPUT_DIR="${SCRIPT_DIR}/output"
RELEASE_VERSION="${RELEASE_VERSION:-latest}"

# Bundle paths
BUNDLE_BASE="/opt/nqrust-bundle"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Parse arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --release)
                RELEASE_VERSION="$2"
                shift 2
                ;;
            --output)
                OUTPUT_DIR="$2"
                shift 2
                ;;
            --help|-h)
                show_help
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
    done
}

show_help() {
    cat << EOF
NQR-MicroVM Air-Gapped ISO Builder

Usage: $(basename "$0") [OPTIONS]

Options:
    --release <version>   Release version to bundle (default: latest)
    --output <dir>        Output directory for ISO (default: ${OUTPUT_DIR})
    --help, -h            Show this help message

Examples:
    $(basename "$0")
    $(basename "$0") --release v0.1.64
    $(basename "$0") --release v0.1.64 --output /tmp/iso

The ISO will contain:
    - NQR-MicroVM binaries (manager, guest-agent, installer)
    - Firecracker v1.13.1
    - Kernel and rootfs images
    - Docker container images (postgres, redis, nginx, alpine)
    - Required Debian packages for offline installation
    - TUI installer with --iso-mode
EOF
}

# Check dependencies
check_dependencies() {
    log_info "Checking dependencies..."

    local missing=()

    # Check for live-build
    if ! command -v lb &> /dev/null; then
        missing+=("live-build")
    fi

    # Check for Docker
    if ! command -v docker &> /dev/null; then
        missing+=("docker.io")
    fi

    # Check for debootstrap
    if ! command -v debootstrap &> /dev/null; then
        missing+=("debootstrap")
    fi

    # Check for xorriso
    if ! command -v xorriso &> /dev/null; then
        missing+=("xorriso")
    fi

    # Check for syslinux
    if ! command -v syslinux &> /dev/null; then
        missing+=("syslinux")
    fi

    # Check for isohybrid (from syslinux-utils)
    if ! command -v isohybrid &> /dev/null; then
        missing+=("syslinux-utils")
    fi

    if [[ ${#missing[@]} -gt 0 ]]; then
        log_error "Missing dependencies: ${missing[*]}"
        log_info "Install with: sudo apt-get install ${missing[*]}"
        exit 1
    fi

    log_success "All dependencies found"
}

# Clean previous build
clean_build() {
    log_info "Cleaning previous build..."
    rm -rf "${BUILD_DIR}"
    mkdir -p "${BUILD_DIR}"
    mkdir -p "${OUTPUT_DIR}"
}

# Initialize live-build configuration
init_live_build() {
    log_info "Initializing live-build configuration..."

    cd "${BUILD_DIR}"

    # Initialize live-build with Debian bookworm
    # Note: Use --mode debian to avoid Ubuntu-specific packages like ubuntu-keyring
    # Disable security repos to avoid URL mismatch issues
    # Specify linux-flavours and linux-packages to avoid Contents file lookup
    # Disable firmware-chroot to prevent downloading Contents-amd64.gz (404)
    lb config \
        --mode debian \
        --system live \
        --architecture amd64 \
        --distribution bookworm \
        --binary-images iso-hybrid \
        --bootloader syslinux \
        --debian-installer false \
        --memtest none \
        --bootappend-live "boot=live components quiet splash" \
        --apt-indices false \
        --apt-recommends false \
        --linux-flavours "amd64" \
        --linux-packages "linux-image" \
        --firmware-chroot false \
        --firmware-binary false \
        --archive-areas "main contrib non-free non-free-firmware" \
        --mirror-bootstrap "http://deb.debian.org/debian" \
        --mirror-chroot "http://deb.debian.org/debian" \
        --mirror-binary "http://deb.debian.org/debian" \
        --security false \
        --iso-application "NQR-MicroVM Installer" \
        --iso-preparer "Nexus" \
        --iso-publisher "Nexus" \
        --iso-volume "NQR-MicroVM"

    log_success "Live-build initialized"
}

# Configure package lists
configure_packages() {
    log_info "Configuring package lists..."

    mkdir -p "${BUILD_DIR}/config/package-lists"

    # Base system packages
    cat > "${BUILD_DIR}/config/package-lists/base.list.chroot" << EOF
# Base system
linux-image-amd64
live-boot
systemd
systemd-sysv
dbus
locales
console-setup
kbd

# Bootloader
syslinux
syslinux-common
isolinux

# Network
iproute2
iptables
bridge-utils
net-tools
iputils-ping
openssh-server
openssh-client
curl
wget

# Storage
lvm2
parted
fdisk
gdisk
e2fsprogs
xfsprogs
dosfstools

# Terminal/TUI
ncurses-term
dialog
tmux

# Utilities
sudo
vim-tiny
less
ca-certificates
gnupg
EOF

    # NQR-MicroVM specific packages
    cat > "${BUILD_DIR}/config/package-lists/nqr.list.chroot" << EOF
# Database
postgresql
postgresql-client

# Virtualization support
qemu-utils
libvirt-daemon-system
libguestfs-tools

# Build dependencies (for any runtime needs)
build-essential
libssl-dev
pkg-config
EOF

    log_success "Package lists configured"
}

# Fix bootloader symlinks - the default live-build bootloader config has broken symlinks
configure_bootloader() {
    log_info "Configuring bootloader with correct paths..."

    local bootloader_dir="${BUILD_DIR}/config/bootloaders/isolinux"
    mkdir -p "${bootloader_dir}"

    # Copy the actual binary files (not symlinks) with correct paths
    cp /usr/lib/ISOLINUX/isolinux.bin "${bootloader_dir}/"
    cp /usr/lib/syslinux/modules/bios/menu.c32 "${bootloader_dir}/"
    cp /usr/lib/syslinux/modules/bios/ldlinux.c32 "${bootloader_dir}/"
    cp /usr/lib/syslinux/modules/bios/libcom32.c32 "${bootloader_dir}/"
    cp /usr/lib/syslinux/modules/bios/libutil.c32 "${bootloader_dir}/"

    # Create simple text-based menu config (avoids vesamenu/bootlogo/rsvg issues)
    cat > "${bootloader_dir}/isolinux.cfg" << 'ISOCFG'
ui menu.c32
prompt 0
timeout 50

menu title NQR-MicroVM Installer

label live
  menu label ^Start NQR-MicroVM Installer
  menu default
  linux /live/vmlinuz
  initrd /live/initrd.img
  append boot=live components quiet splash

label live-failsafe
  menu label Start in ^Safe Mode
  linux /live/vmlinuz
  initrd /live/initrd.img
  append boot=live components memtest noapic noapm nodma nomce nolapic nomodeset nosmp nosplash vga=normal
ISOCFG

    # Create live.cfg.in for kernel version substitution
    cat > "${bootloader_dir}/live.cfg.in" << 'LIVECFG'
label live
  menu label ^Start NQR-MicroVM Installer
  menu default
  linux /live/vmlinuz
  initrd /live/initrd.img
  append boot=live components quiet splash
LIVECFG

    # Create empty bootlogo archive - live-build unconditionally reads this file
    # at line 365 of lb_binary_syslinux regardless of config options
    (cd "${bootloader_dir}" && echo "" | cpio -o > bootlogo)

    log_success "Bootloader configured"
}

# Create bundle directory structure in live filesystem
create_bundle_structure() {
    log_info "Creating bundle directory structure..."

    local include_dir="${BUILD_DIR}/config/includes.chroot"
    local bundle_dir="${include_dir}${BUNDLE_BASE}"

    mkdir -p "${bundle_dir}/bin"
    mkdir -p "${bundle_dir}/images/kernel"
    mkdir -p "${bundle_dir}/images/rootfs"
    mkdir -p "${bundle_dir}/images/docker"
    mkdir -p "${bundle_dir}/debs"

    log_success "Bundle structure created"
}

# Download and bundle NQR-MicroVM binaries
bundle_binaries() {
    log_info "Bundling NQR-MicroVM binaries..."

    local include_dir="${BUILD_DIR}/config/includes.chroot"
    local bundle_dir="${include_dir}${BUNDLE_BASE}"
    local bin_dir="${bundle_dir}/bin"

    # GitHub repository info
    local repo="NexusQuantum/NQRust-MicroVM"
    local api_url
    local download_url

    if [[ "${RELEASE_VERSION}" == "latest" ]]; then
        api_url="https://api.github.com/repos/${repo}/releases/latest"
        download_url="https://github.com/${repo}/releases/latest/download"
    else
        api_url="https://api.github.com/repos/${repo}/releases/tags/${RELEASE_VERSION}"
        download_url="https://github.com/${repo}/releases/download/${RELEASE_VERSION}"
    fi

    log_info "Fetching release assets from: ${api_url}"
    
    # Fetch available assets from GitHub API
    local assets_json
    assets_json=$(curl -fsSL "${api_url}" 2>/dev/null) || {
        log_warn "Failed to fetch release info from GitHub API"
        log_info "Falling back to local build..."
        copy_local_binaries "${bin_dir}"
        return
    }
    
    log_info "Available release assets:"
    echo "${assets_json}" | jq -r '.assets[].name' 2>/dev/null || log_warn "Failed to parse assets"
    
    # Find and download manager binary
    local manager_asset
    manager_asset=$(echo "${assets_json}" | jq -r '.assets[].name' | grep -E '^(nqrust-manager|nqr-manager)' | head -n1)
    if [[ -n "${manager_asset}" ]]; then
        log_info "Downloading manager: ${manager_asset}"
        curl -fsSL "${download_url}/${manager_asset}" -o "${bin_dir}/nqr-manager" || {
            log_warn "Failed to download manager, trying local build..."
            cp "${PROJECT_ROOT}/target/release/nqr-manager" "${bin_dir}/" 2>/dev/null || \
            cp "${PROJECT_ROOT}/target/x86_64-unknown-linux-musl/release/nqr-manager" "${bin_dir}/" 2>/dev/null || true
        }
    else
        log_warn "No manager binary found in release, trying local build..."
        cp "${PROJECT_ROOT}/target/release/nqr-manager" "${bin_dir}/" 2>/dev/null || \
        cp "${PROJECT_ROOT}/target/x86_64-unknown-linux-musl/release/nqr-manager" "${bin_dir}/" 2>/dev/null || true
    fi
    
    # Find and download installer binary (exclude .tar.gz archives)
    local installer_asset
    installer_asset=$(echo "${assets_json}" | jq -r '.assets[].name' | grep -E '^(nqr-installer|nqrust-installer)' | grep -v '\.tar\.gz$' | head -n1)
    if [[ -n "${installer_asset}" ]]; then
        log_info "Downloading installer: ${installer_asset}"
        curl -fsSL "${download_url}/${installer_asset}" -o "${bin_dir}/nqr-installer" || {
            log_warn "Failed to download installer, trying local build..."
            cp "${PROJECT_ROOT}/target/release/nqr-installer" "${bin_dir}/" 2>/dev/null || \
            cp "${PROJECT_ROOT}/target/x86_64-unknown-linux-musl/release/nqr-installer" "${bin_dir}/" 2>/dev/null || true
        }
    else
        log_warn "No installer binary found in release, trying local build..."
        cp "${PROJECT_ROOT}/target/release/nqr-installer" "${bin_dir}/" 2>/dev/null || \
        cp "${PROJECT_ROOT}/target/x86_64-unknown-linux-musl/release/nqr-installer" "${bin_dir}/" 2>/dev/null || true
    fi
    
    # Find and download guest-agent binary
    local agent_asset
    agent_asset=$(echo "${assets_json}" | jq -r '.assets[].name' | grep -E '^(nqrust-guest-agent|nqr-guest-agent)' | head -n1)
    if [[ -n "${agent_asset}" ]]; then
        log_info "Downloading guest-agent: ${agent_asset}"
        curl -fsSL "${download_url}/${agent_asset}" -o "${bin_dir}/nqr-guest-agent" || {
            log_warn "Failed to download guest-agent, trying local build..."
            cp "${PROJECT_ROOT}/target/release/nqr-guest-agent" "${bin_dir}/" 2>/dev/null || \
            cp "${PROJECT_ROOT}/target/x86_64-unknown-linux-musl/release/nqr-guest-agent" "${bin_dir}/" 2>/dev/null || true
        }
    else
        log_warn "No guest-agent binary found in release, trying local build..."
        cp "${PROJECT_ROOT}/target/release/nqr-guest-agent" "${bin_dir}/" 2>/dev/null || \
        cp "${PROJECT_ROOT}/target/x86_64-unknown-linux-musl/release/nqr-guest-agent" "${bin_dir}/" 2>/dev/null || true
    fi

    chmod +x "${bin_dir}"/* 2>/dev/null || true
    
    # Verify binaries are present
    log_info "Verifying bundled binaries..."
    ls -la "${bin_dir}/"
    
    if [[ ! -x "${bin_dir}/nqr-installer" ]]; then
        log_error "nqr-installer not found or not executable!"
        log_error "Available assets in release:"
        echo "${assets_json}" | jq -r '.assets[].name' 2>/dev/null || true
        exit 1
    fi

    log_success "Binaries bundled"
}

# Download and bundle Firecracker
bundle_firecracker() {
    log_info "Bundling Firecracker..."

    local include_dir="${BUILD_DIR}/config/includes.chroot"
    local bundle_dir="${include_dir}${BUNDLE_BASE}"
    local bin_dir="${bundle_dir}/bin"

    local fc_version="v1.13.1"
    local fc_url="https://github.com/firecracker-microvm/firecracker/releases/download/${fc_version}/firecracker-${fc_version}-x86_64.tgz"

    log_info "Downloading Firecracker ${fc_version}..."

    curl -fsSL "${fc_url}" -o "/tmp/firecracker.tgz"
    tar -xzf "/tmp/firecracker.tgz" -C /tmp

    # Extract binaries
    cp "/tmp/release-${fc_version}-x86_64/firecracker-${fc_version}-x86_64" "${bin_dir}/firecracker"
    cp "/tmp/release-${fc_version}-x86_64/jailer-${fc_version}-x86_64" "${bin_dir}/jailer"

    chmod +x "${bin_dir}/firecracker" "${bin_dir}/jailer"

    # Cleanup
    rm -rf "/tmp/firecracker.tgz" "/tmp/release-${fc_version}-x86_64"

    log_success "Firecracker bundled"
}

# Download and bundle kernel and rootfs images
bundle_images() {
    log_info "Bundling kernel and rootfs images..."

    local include_dir="${BUILD_DIR}/config/includes.chroot"
    local bundle_dir="${include_dir}${BUNDLE_BASE}"
    local images_dir="${bundle_dir}/images"
    
    local repo="NexusQuantum/NQRust-MicroVM"
    local base_url="https://github.com/${repo}/releases/latest/download"

    # Kernel image (use the one that matches what's in the release)
    log_info "Downloading kernel image..."
    curl -fsSL "${base_url}/vmlinux-5.10.fc.bin" -o "${images_dir}/kernel/vmlinux-5.10.fc.bin" || {
        log_warn "Failed to download kernel from release, checking local..."
        if [[ -f "/srv/images/kernel/vmlinux-5.10.fc.bin" ]]; then
            cp "/srv/images/kernel/vmlinux-5.10.fc.bin" "${images_dir}/kernel/"
        elif [[ -f "/srv/images/vmlinux-5.10.fc.bin" ]]; then
            cp "/srv/images/vmlinux-5.10.fc.bin" "${images_dir}/kernel/"
        else
            log_warn "Kernel image not found - will need to be added manually"
        fi
    }

    # All rootfs images available in the release
    local rootfs_images=(
        "alpine-3.18-minimal.ext4"
        "busybox-1.35.ext4"
        "ubuntu-24.04-minimal.ext4"
        "python-runtime.ext4"
        "bun-runtime.ext4"
        "node-runtime.ext4"
    )
    
    for img in "${rootfs_images[@]}"; do
        log_info "Downloading ${img}..."
        curl -fsSL "${base_url}/${img}" -o "${images_dir}/rootfs/${img}" || {
            log_warn "Failed to download ${img} from release, checking local..."
            if [[ -f "/srv/images/rootfs/${img}" ]]; then
                cp "/srv/images/rootfs/${img}" "${images_dir}/rootfs/"
            elif [[ -f "/srv/images/${img}" ]]; then
                cp "/srv/images/${img}" "${images_dir}/rootfs/"
            fi
        }
    done

    # Container runtime (compressed in release due to GitHub 2GB limit)
    log_info "Downloading container runtime rootfs (compressed)..."
    curl -fsSL "${base_url}/container-runtime.ext4.gz" -o "${images_dir}/rootfs/container-runtime.ext4.gz" || {
        log_warn "Failed to download container-runtime.ext4.gz, checking local..."
        if [[ -f "/srv/images/rootfs/container-runtime.ext4" ]]; then
            cp "/srv/images/rootfs/container-runtime.ext4" "${images_dir}/rootfs/"
        fi
    }
    
    # Decompress if downloaded successfully
    if [[ -f "${images_dir}/rootfs/container-runtime.ext4.gz" ]]; then
        log_info "Decompressing container-runtime.ext4.gz..."
        gunzip -f "${images_dir}/rootfs/container-runtime.ext4.gz" || log_warn "Decompression failed"
    fi
    
    log_info "Bundled images:"
    ls -la "${images_dir}/kernel/" 2>/dev/null || true
    ls -la "${images_dir}/rootfs/" 2>/dev/null || true

    log_success "Images bundled"
}

# Export and bundle Docker images
bundle_docker_images() {
    log_info "Bundling Docker images..."

    local include_dir="${BUILD_DIR}/config/includes.chroot"
    local bundle_dir="${include_dir}${BUNDLE_BASE}"
    local docker_dir="${bundle_dir}/images/docker"

    # List of images to bundle
    local images=(
        "postgres:16-alpine"
        "redis:7-alpine"
        "nginx:alpine"
        "alpine:latest"
    )

    for image in "${images[@]}"; do
        log_info "Exporting ${image}..."

        # Pull the image first
        docker pull "${image}"

        # Create tarball name (replace : and / with -)
        local tarball_name=$(echo "${image}" | sed 's/[:\\/]/-/g').tar

        # Export image
        docker save "${image}" -o "${docker_dir}/${tarball_name}"

        log_success "Exported ${image} -> ${tarball_name}"
    done

    log_success "Docker images bundled"
}

# Create first-boot systemd service
create_firstboot_service() {
    log_info "Creating first-boot service..."

    local include_dir="${BUILD_DIR}/config/includes.chroot"

    # Create systemd service directory
    mkdir -p "${include_dir}/etc/systemd/system"

    # Create the first-boot service
    cat > "${include_dir}/etc/systemd/system/nqrust-firstboot.service" << 'EOF'
[Unit]
Description=NQR-MicroVM First Boot Installer
After=multi-user.target
ConditionPathExists=!/var/lib/nqrust-installed

[Service]
Type=oneshot
ExecStart=/opt/nqrust-bundle/bin/nqr-installer install --iso-mode --bundle-path /opt/nqrust-bundle
ExecStartPost=/bin/touch /var/lib/nqrust-installed
StandardInput=tty
StandardOutput=tty
StandardError=tty
TTYPath=/dev/tty1
TTYReset=yes
TTYVHangup=yes
RemainAfterExit=yes

[Install]
WantedBy=multi-user.target
EOF

    # Create symlink to enable the service
    mkdir -p "${include_dir}/etc/systemd/system/multi-user.target.wants"
    ln -sf "../nqrust-firstboot.service" \
        "${include_dir}/etc/systemd/system/multi-user.target.wants/nqrust-firstboot.service"

    # Create getty override for systemd auto-login on tty1
    mkdir -p "${include_dir}/etc/systemd/system/getty@tty1.service.d"
    cat > "${include_dir}/etc/systemd/system/getty@tty1.service.d/autologin.conf" << 'EOF'
[Service]
ExecStart=
ExecStart=-/sbin/agetty --autologin root --noclear %I $TERM
EOF

    # Create sysvinit inittab override for auto-login (live-boot uses sysvinit)
    # This replaces the default tty1 getty with autologin
    mkdir -p "${include_dir}/etc"
    cat > "${include_dir}/etc/inittab" << 'EOF'
# /etc/inittab: init(8) configuration for NQR-MicroVM Installer
# Auto-login on tty1 for installer

# Default runlevel
id:2:initdefault:

# System initialization
si::sysinit:/etc/init.d/rcS

# Runlevels
l0:0:wait:/etc/init.d/rc 0
l1:1:wait:/etc/init.d/rc 1
l2:2:wait:/etc/init.d/rc 2
l3:3:wait:/etc/init.d/rc 3
l4:4:wait:/etc/init.d/rc 4
l5:5:wait:/etc/init.d/rc 5
l6:6:wait:/etc/init.d/rc 6

# Ctrl-Alt-Del
ca:12345:ctrlaltdel:/sbin/shutdown -t1 -a -r now

# Power management
pf::powerwait:/etc/init.d/powerfail start
pn::powerfailnow:/etc/init.d/powerfail now
po::powerokwait:/etc/init.d/powerfail stop

# TTY1 with AUTO-LOGIN as root, then launch installer
1:2345:respawn:/sbin/agetty --autologin root --noclear tty1 linux

# Other TTYs - normal login
2:23:respawn:/sbin/getty 38400 tty2
3:23:respawn:/sbin/getty 38400 tty3
4:23:respawn:/sbin/getty 38400 tty4
5:23:respawn:/sbin/getty 38400 tty5
6:23:respawn:/sbin/getty 38400 tty6
EOF

    # Create root's .bash_profile to auto-launch installer on tty1
    mkdir -p "${include_dir}/root"
    cat > "${include_dir}/root/.bash_profile" << 'EOF'
# NQR-MicroVM Installer Auto-Launch
# Only run on tty1 and if installer hasn't run yet

if [ "$(tty)" = "/dev/tty1" ] && [ ! -f /var/lib/nqrust-installed ]; then
    echo ""
    echo "Starting NQR-MicroVM Installer..."
    echo ""
    sleep 1
    /opt/nqrust-bundle/bin/nqr-installer install --iso-mode --bundle-path /opt/nqrust-bundle
    touch /var/lib/nqrust-installed
fi
EOF

    # Also create .profile for sh compatibility
    cat > "${include_dir}/root/.profile" << 'EOF'
# NQR-MicroVM Installer Auto-Launch
# Only run on tty1 and if installer hasn't run yet

if [ "$(tty)" = "/dev/tty1" ] && [ ! -f /var/lib/nqrust-installed ]; then
    echo ""
    echo "Starting NQR-MicroVM Installer..."
    echo ""
    sleep 1
    /opt/nqrust-bundle/bin/nqr-installer install --iso-mode --bundle-path /opt/nqrust-bundle
    touch /var/lib/nqrust-installed
fi
EOF

    log_success "First-boot service created"
}

# Configure boot splash and branding
configure_branding() {
    log_info "Configuring branding..."

    local include_dir="${BUILD_DIR}/config/includes.chroot"

    # Create MOTD
    mkdir -p "${include_dir}/etc"
    cat > "${include_dir}/etc/motd" << 'EOF'

    _   _  ___  ____       __  __ _               __     ____  __
   | \ | |/ _ \|  _ \     |  \/  (_) ___ _ __ ___\ \   / /  \/  |
   |  \| | | | | |_) |____| |\/| | |/ __| '__/ _ \\ \ / /| |\/| |
   | |\  | |_| |  _ <_____| |  | | | (__| | | (_) |\ V / | |  | |
   |_| \_|\__\_\_| \_\    |_|  |_|_|\___|_|  \___/  \_/  |_|  |_|

   Air-Gapped Installer v${RELEASE_VERSION:-latest}
   
   The installer will start automatically on first boot.
   For manual installation, run: /opt/nqrust-bundle/bin/nqr-installer install --iso-mode

EOF

    # Configure issue
    cat > "${include_dir}/etc/issue" << 'EOF'
NQR-MicroVM Air-Gapped Installer
=================================
\n \l

EOF

    log_success "Branding configured"
}

# Configure live boot hooks
configure_hooks() {
    log_info "Configuring hooks..."

    mkdir -p "${BUILD_DIR}/config/hooks/live"

    # Create a hook to set up the system
    cat > "${BUILD_DIR}/config/hooks/live/9999-setup.hook.chroot" << 'EOF'
#!/bin/bash
set -e

# Set timezone
ln -sf /usr/share/zoneinfo/UTC /etc/localtime

# Enable SSH
systemctl enable ssh

# Configure keyboard
echo "KEYMAP=us" > /etc/vconsole.conf

# Set root password (temporary, will be changed during install)
echo "root:nqrust" | chpasswd

# Make binaries executable
chmod +x /opt/nqrust-bundle/bin/* 2>/dev/null || true

# Create symlinks for convenience
ln -sf /opt/nqrust-bundle/bin/nqr-installer /usr/local/bin/nqr-installer 2>/dev/null || true
ln -sf /opt/nqrust-bundle/bin/nqr-manager /usr/local/bin/nqr-manager 2>/dev/null || true
EOF

    chmod +x "${BUILD_DIR}/config/hooks/live/9999-setup.hook.chroot"

    log_success "Hooks configured"
}

# Build the ISO
build_iso() {
    log_info "Building ISO..."

    cd "${BUILD_DIR}"

    # Build the ISO
    sudo lb build 2>&1 | tee "${OUTPUT_DIR}/build.log"

    # Move the ISO to output directory
    if [[ -f "${BUILD_DIR}/live-image-amd64.hybrid.iso" ]]; then
        local iso_name="nqr-microvm-${RELEASE_VERSION:-latest}-airgap-amd64.iso"
        mv "${BUILD_DIR}/live-image-amd64.hybrid.iso" "${OUTPUT_DIR}/${iso_name}"
        log_success "ISO created: ${OUTPUT_DIR}/${iso_name}"

        # Generate checksums
        cd "${OUTPUT_DIR}"
        sha256sum "${iso_name}" > "${iso_name}.sha256"
        md5sum "${iso_name}" > "${iso_name}.md5"

        log_success "Checksums generated"

        # Show final info
        local iso_size=$(du -h "${OUTPUT_DIR}/${iso_name}" | cut -f1)
        log_info "==================================="
        log_info "ISO Build Complete!"
        log_info "==================================="
        log_info "Location: ${OUTPUT_DIR}/${iso_name}"
        log_info "Size: ${iso_size}"
        log_info "SHA256: $(cat ${iso_name}.sha256 | cut -d' ' -f1)"
    else
        log_error "ISO build failed - file not found"
        exit 1
    fi
}

# Main execution
main() {
    parse_args "$@"

    log_info "==================================="
    log_info "NQR-MicroVM Air-Gapped ISO Builder"
    log_info "==================================="
    log_info "Release Version: ${RELEASE_VERSION}"
    log_info "Output Directory: ${OUTPUT_DIR}"
    log_info ""

    check_dependencies
    clean_build
    init_live_build
    configure_packages
    configure_bootloader
    create_bundle_structure
    bundle_binaries
    bundle_firecracker
    bundle_images
    bundle_docker_images
    create_firstboot_service
    configure_branding
    configure_hooks
    build_iso

    log_success "==================================="
    log_success "Build completed successfully!"
    log_success "==================================="
}

main "$@"
