#!/bin/bash
# =============================================================================
# Bundle .deb packages for offline installation
# =============================================================================
# Downloads all required .deb packages and their dependencies for offline
# installation on a Debian/Ubuntu system.
#
# Usage:
#   ./bundle-debs.sh [--output <dir>]
#
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT_DIR="${SCRIPT_DIR}/debs"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --output)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        *)
            shift
            ;;
    esac
done

# Required packages for NQR-MicroVM
PACKAGES=(
    # Network
    "bridge-utils"
    "iptables"
    "iproute2"
    "net-tools"
    
    # Database
    "postgresql"
    "postgresql-client"
    "libpq5"
    
    # System
    "sudo"
    "systemd"
    "ca-certificates"
    
    # KVM/Virtualization
    "qemu-utils"
    "libvirt-daemon-system"
    
    # Utilities
    "curl"
    "wget"
    "jq"
)

log_info "Creating output directory: ${OUTPUT_DIR}"
mkdir -p "${OUTPUT_DIR}"

log_info "Downloading packages and dependencies..."

# Create a temporary directory for apt download
TEMP_DIR=$(mktemp -d)
cd "${TEMP_DIR}"

# Download packages with dependencies
for pkg in "${PACKAGES[@]}"; do
    log_info "Downloading ${pkg} and dependencies..."
    apt-get download $(apt-cache depends --recurse --no-recommends --no-suggests \
        --no-conflicts --no-breaks --no-replaces --no-enhances \
        "${pkg}" | grep "^\w" | sort -u) 2>/dev/null || true
done

# Move all downloaded packages to output
mv ./*.deb "${OUTPUT_DIR}/" 2>/dev/null || true

# Cleanup
cd /
rm -rf "${TEMP_DIR}"

# Count packages
PKG_COUNT=$(ls -1 "${OUTPUT_DIR}"/*.deb 2>/dev/null | wc -l)

log_success "Downloaded ${PKG_COUNT} packages to ${OUTPUT_DIR}"
log_info "Total size: $(du -sh ${OUTPUT_DIR} | cut -f1)"
