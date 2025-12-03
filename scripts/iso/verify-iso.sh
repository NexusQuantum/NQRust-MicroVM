#!/bin/bash
# =============================================================================
# Verify NQR-MicroVM ISO Integrity
# =============================================================================
# Verifies the integrity of a built ISO file.
#
# Usage:
#   ./verify-iso.sh <iso-file>
#
# =============================================================================

set -euo pipefail

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[PASS]${NC} $1"
}

log_error() {
    echo -e "${RED}[FAIL]${NC} $1"
}

# Check arguments
if [[ $# -lt 1 ]]; then
    echo "Usage: $(basename "$0") <iso-file>"
    exit 1
fi

ISO_FILE="$1"

if [[ ! -f "${ISO_FILE}" ]]; then
    log_error "ISO file not found: ${ISO_FILE}"
    exit 1
fi

log_info "==================================="
log_info "NQR-MicroVM ISO Verification"
log_info "==================================="
log_info "ISO File: ${ISO_FILE}"
log_info ""

ERRORS=0

# Check file exists and is not empty
log_info "Checking file validity..."
if [[ -s "${ISO_FILE}" ]]; then
    SIZE=$(du -h "${ISO_FILE}" | cut -f1)
    log_success "ISO file exists (${SIZE})"
else
    log_error "ISO file is empty or invalid"
    ERRORS=$((ERRORS + 1))
fi

# Verify checksum if available
log_info "Checking checksums..."
SHA256_FILE="${ISO_FILE}.sha256"
MD5_FILE="${ISO_FILE}.md5"

if [[ -f "${SHA256_FILE}" ]]; then
    if sha256sum -c "${SHA256_FILE}" --status; then
        log_success "SHA256 checksum verified"
    else
        log_error "SHA256 checksum mismatch"
        ERRORS=$((ERRORS + 1))
    fi
else
    log_info "SHA256 checksum file not found (generating...)"
    sha256sum "${ISO_FILE}" | tee "${SHA256_FILE}"
fi

if [[ -f "${MD5_FILE}" ]]; then
    if md5sum -c "${MD5_FILE}" --status; then
        log_success "MD5 checksum verified"
    else
        log_error "MD5 checksum mismatch"
        ERRORS=$((ERRORS + 1))
    fi
else
    log_info "MD5 checksum file not found (generating...)"
    md5sum "${ISO_FILE}" | tee "${MD5_FILE}"
fi

# Check ISO structure
log_info "Checking ISO structure..."

# Mount ISO temporarily
MOUNT_DIR=$(mktemp -d)
cleanup() {
    sudo umount "${MOUNT_DIR}" 2>/dev/null || true
    rm -rf "${MOUNT_DIR}"
}
trap cleanup EXIT

if sudo mount -o loop,ro "${ISO_FILE}" "${MOUNT_DIR}"; then
    log_success "ISO can be mounted"
    
    # Check for required directories/files
    REQUIRED_PATHS=(
        "live"
        "isolinux"
    )
    
    for path in "${REQUIRED_PATHS[@]}"; do
        if [[ -e "${MOUNT_DIR}/${path}" ]]; then
            log_success "Found: ${path}"
        else
            log_error "Missing: ${path}"
            ERRORS=$((ERRORS + 1))
        fi
    done
    
    # Check for live filesystem
    if [[ -f "${MOUNT_DIR}/live/filesystem.squashfs" ]]; then
        SQUASH_SIZE=$(du -h "${MOUNT_DIR}/live/filesystem.squashfs" | cut -f1)
        log_success "Live filesystem found (${SQUASH_SIZE})"
    else
        log_error "Live filesystem not found"
        ERRORS=$((ERRORS + 1))
    fi
    
    # List ISO contents for debugging
    log_info "ISO root contents:"
    ls -la "${MOUNT_DIR}"
    
else
    log_error "Failed to mount ISO"
    ERRORS=$((ERRORS + 1))
fi

# Summary
log_info ""
log_info "==================================="
if [[ ${ERRORS} -eq 0 ]]; then
    log_success "ISO verification passed!"
    log_info "==================================="
    exit 0
else
    log_error "ISO verification failed with ${ERRORS} error(s)"
    log_info "==================================="
    exit 1
fi
