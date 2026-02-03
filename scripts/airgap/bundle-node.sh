#!/bin/bash
# =============================================================================
# Bundle Node.js and pnpm for air-gapped installation
# =============================================================================
# Downloads the official Node.js binary distribution and standalone pnpm
# binary for offline installation on Ubuntu servers.
#
# Usage:
#   ./bundle-node.sh [--output <dir>] [--node-version <version>]
# =============================================================================

set -euo pipefail

OUTPUT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/output/node"
NODE_VERSION="20.18.1"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m'

log_info()    { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[OK]${NC} $1"; }
log_error()   { echo -e "${RED}[ERROR]${NC} $1"; }

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --output)       OUTPUT_DIR="$2"; shift 2 ;;
        --node-version) NODE_VERSION="$2"; shift 2 ;;
        --help|-h)
            echo "Usage: $(basename "$0") [--output <dir>] [--node-version <version>]"
            exit 0
            ;;
        *) shift ;;
    esac
done

mkdir -p "${OUTPUT_DIR}"

log_info "==================================="
log_info "NQR-MicroVM Node.js Bundler"
log_info "==================================="
log_info "Node.js version: v${NODE_VERSION}"
log_info "Output: ${OUTPUT_DIR}"
echo ""

# Download Node.js binary distribution
NODE_TARBALL="node-v${NODE_VERSION}-linux-x64.tar.xz"
NODE_URL="https://nodejs.org/dist/v${NODE_VERSION}/${NODE_TARBALL}"

log_info "Downloading Node.js v${NODE_VERSION}..."
if curl -fsSL "${NODE_URL}" -o "${OUTPUT_DIR}/${NODE_TARBALL}"; then
    local_size=$(du -h "${OUTPUT_DIR}/${NODE_TARBALL}" | cut -f1)
    log_success "Node.js downloaded (${local_size})"
else
    log_error "Failed to download Node.js from ${NODE_URL}"
    exit 1
fi

# Verify the download by checking file size (should be ~25MB+)
file_size=$(stat -c%s "${OUTPUT_DIR}/${NODE_TARBALL}" 2>/dev/null || stat -f%z "${OUTPUT_DIR}/${NODE_TARBALL}" 2>/dev/null || echo "0")
if [[ "${file_size}" -lt 10000000 ]]; then
    log_error "Downloaded file is too small (${file_size} bytes) - likely a download error"
    exit 1
fi

# Download pnpm standalone binary
# The UI service uses pnpm to start Next.js
log_info "Downloading pnpm standalone binary..."
PNPM_URL="https://github.com/pnpm/pnpm/releases/latest/download/pnpm-linux-x64"

if curl -fsSL "${PNPM_URL}" -o "${OUTPUT_DIR}/pnpm"; then
    chmod +x "${OUTPUT_DIR}/pnpm"
    log_success "pnpm downloaded"
else
    log_error "Failed to download pnpm"
    exit 1
fi

echo ""
log_success "Node.js bundling complete"
log_info "Contents:"
ls -lh "${OUTPUT_DIR}/"
