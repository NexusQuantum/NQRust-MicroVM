#!/bin/bash
# =============================================================================
# Bundle Docker images for offline installation
# =============================================================================
# Exports Docker images as tarballs for offline loading.
#
# Usage:
#   ./bundle-docker-images.sh [--output <dir>]
#
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT_DIR="${SCRIPT_DIR}/docker"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
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

# Required Docker images for NQR-MicroVM
IMAGES=(
    "postgres:16-alpine"
    "redis:7-alpine"
    "nginx:alpine"
    "alpine:latest"
)

# Check Docker is available
if ! command -v docker &> /dev/null; then
    log_warn "Docker not found. Skipping Docker image bundling."
    exit 0
fi

log_info "Creating output directory: ${OUTPUT_DIR}"
mkdir -p "${OUTPUT_DIR}"

log_info "Bundling Docker images..."

for image in "${IMAGES[@]}"; do
    log_info "Processing ${image}..."
    
    # Pull the image
    docker pull "${image}"
    
    # Create tarball name (replace : and / with -)
    tarball_name=$(echo "${image}" | sed 's/[:\\/]/-/g').tar
    
    # Export image
    docker save "${image}" -o "${OUTPUT_DIR}/${tarball_name}"
    
    # Get size
    size=$(du -h "${OUTPUT_DIR}/${tarball_name}" | cut -f1)
    
    log_success "Exported ${image} -> ${tarball_name} (${size})"
done

log_success "All Docker images bundled"
log_info "Total size: $(du -sh ${OUTPUT_DIR} | cut -f1)"
