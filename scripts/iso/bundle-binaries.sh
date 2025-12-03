#!/bin/bash
# =============================================================================
# Bundle NQR-MicroVM binaries
# =============================================================================
# Downloads or copies NQR-MicroVM binaries for bundling.
#
# Usage:
#   ./bundle-binaries.sh [--release <version>] [--output <dir>]
#
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
OUTPUT_DIR="${SCRIPT_DIR}/bin"
RELEASE_VERSION="latest"

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
        --release)
            RELEASE_VERSION="$2"
            shift 2
            ;;
        --output)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        *)
            shift
            ;;
    esac
done

log_info "Creating output directory: ${OUTPUT_DIR}"
mkdir -p "${OUTPUT_DIR}"

# GitHub release URL
GITHUB_REPO="nexus/nqrust-microvm"
if [[ "${RELEASE_VERSION}" == "latest" ]]; then
    BASE_URL="https://github.com/${GITHUB_REPO}/releases/latest/download"
else
    BASE_URL="https://github.com/${GITHUB_REPO}/releases/download/${RELEASE_VERSION}"
fi

# Binaries to bundle
BINARIES=(
    "nqr-manager"
    "nqr-installer"
    "nqr-guest-agent"
)

log_info "Bundling binaries (version: ${RELEASE_VERSION})..."

for binary in "${BINARIES[@]}"; do
    log_info "Processing ${binary}..."
    
    # Try to download from GitHub releases
    if curl -fsSL "${BASE_URL}/${binary}" -o "${OUTPUT_DIR}/${binary}" 2>/dev/null; then
        log_success "Downloaded ${binary} from GitHub"
    else
        log_warn "GitHub download failed, trying local builds..."
        
        # Try local release build
        if [[ -f "${PROJECT_ROOT}/target/release/${binary}" ]]; then
            cp "${PROJECT_ROOT}/target/release/${binary}" "${OUTPUT_DIR}/"
            log_success "Copied ${binary} from target/release"
        # Try musl build
        elif [[ -f "${PROJECT_ROOT}/target/x86_64-unknown-linux-musl/release/${binary}" ]]; then
            cp "${PROJECT_ROOT}/target/x86_64-unknown-linux-musl/release/${binary}" "${OUTPUT_DIR}/"
            log_success "Copied ${binary} from musl release"
        else
            log_error "Could not find ${binary}"
            exit 1
        fi
    fi
    
    chmod +x "${OUTPUT_DIR}/${binary}"
done

# Bundle Firecracker
log_info "Bundling Firecracker..."

FC_VERSION="v1.13.1"
FC_URL="https://github.com/firecracker-microvm/firecracker/releases/download/${FC_VERSION}/firecracker-${FC_VERSION}-x86_64.tgz"

TEMP_DIR=$(mktemp -d)

if curl -fsSL "${FC_URL}" -o "${TEMP_DIR}/firecracker.tgz"; then
    tar -xzf "${TEMP_DIR}/firecracker.tgz" -C "${TEMP_DIR}"
    
    cp "${TEMP_DIR}/release-${FC_VERSION}-x86_64/firecracker-${FC_VERSION}-x86_64" "${OUTPUT_DIR}/firecracker"
    cp "${TEMP_DIR}/release-${FC_VERSION}-x86_64/jailer-${FC_VERSION}-x86_64" "${OUTPUT_DIR}/jailer"
    
    chmod +x "${OUTPUT_DIR}/firecracker" "${OUTPUT_DIR}/jailer"
    
    log_success "Bundled Firecracker ${FC_VERSION}"
else
    log_error "Failed to download Firecracker"
    exit 1
fi

rm -rf "${TEMP_DIR}"

log_success "All binaries bundled"
log_info "Contents:"
ls -la "${OUTPUT_DIR}"
