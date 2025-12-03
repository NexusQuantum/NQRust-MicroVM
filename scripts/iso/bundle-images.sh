#!/bin/bash
# =============================================================================
# Bundle kernel and rootfs images
# =============================================================================
# Downloads or copies kernel and rootfs images for bundling.
#
# Usage:
#   ./bundle-images.sh [--output <dir>]
#
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT_DIR="${SCRIPT_DIR}/images"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

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

log_info "Creating output directories..."
mkdir -p "${OUTPUT_DIR}/kernel"
mkdir -p "${OUTPUT_DIR}/rootfs"
mkdir -p "${OUTPUT_DIR}/docker"

# GitHub release URL
GITHUB_REPO="nexus/nqrust-microvm"
BASE_URL="https://github.com/${GITHUB_REPO}/releases/latest/download"

# Local paths to check
LOCAL_IMAGE_DIR="/srv/images"

# Kernel images
log_info "Bundling kernel images..."

KERNEL_IMAGES=(
    "vmlinux-6.1"
)

for kernel in "${KERNEL_IMAGES[@]}"; do
    log_info "Processing ${kernel}..."
    
    # Try GitHub download
    if curl -fsSL "${BASE_URL}/${kernel}" -o "${OUTPUT_DIR}/kernel/${kernel}" 2>/dev/null; then
        log_success "Downloaded ${kernel} from GitHub"
    # Try local path
    elif [[ -f "${LOCAL_IMAGE_DIR}/kernel/${kernel}" ]]; then
        cp "${LOCAL_IMAGE_DIR}/kernel/${kernel}" "${OUTPUT_DIR}/kernel/"
        log_success "Copied ${kernel} from local"
    else
        log_error "Could not find kernel: ${kernel}"
        exit 1
    fi
done

# Rootfs images
log_info "Bundling rootfs images..."

ROOTFS_IMAGES=(
    "debian-minimal.ext4"
    "ubuntu-minimal.ext4"
    "container-runtime.ext4"
)

for rootfs in "${ROOTFS_IMAGES[@]}"; do
    log_info "Processing ${rootfs}..."
    
    # Try GitHub download
    if curl -fsSL "${BASE_URL}/${rootfs}" -o "${OUTPUT_DIR}/rootfs/${rootfs}" 2>/dev/null; then
        log_success "Downloaded ${rootfs} from GitHub"
    # Try local path
    elif [[ -f "${LOCAL_IMAGE_DIR}/rootfs/${rootfs}" ]]; then
        cp "${LOCAL_IMAGE_DIR}/rootfs/${rootfs}" "${OUTPUT_DIR}/rootfs/"
        log_success "Copied ${rootfs} from local"
    else
        log_warn "Could not find rootfs: ${rootfs} (skipping)"
    fi
done

log_success "Images bundled"
log_info "Total size: $(du -sh ${OUTPUT_DIR} | cut -f1)"
log_info "Contents:"
find "${OUTPUT_DIR}" -type f -exec ls -lh {} \;
