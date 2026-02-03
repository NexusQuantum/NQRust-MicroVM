#!/bin/bash
# =============================================================================
# NQR-MicroVM Air-Gapped Installer Entry Point
# =============================================================================
# This script is the entry point inside the self-extracting bundle.
# It validates the environment and launches the TUI installer.
#
# This script is called automatically by makeself after extraction.
# =============================================================================

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

BUNDLE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo ""
echo -e "${BLUE}╔════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║  NQR-MicroVM Air-Gapped Installer              ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════╝${NC}"
echo ""

# Check running as root
if [ "$(id -u)" -ne 0 ]; then
    echo -e "${RED}ERROR: This installer must be run as root.${NC}"
    echo "Usage: sudo $0"
    exit 1
fi

# Detect OS
if [ ! -f /etc/os-release ]; then
    echo -e "${RED}ERROR: Cannot detect OS version (/etc/os-release not found)${NC}"
    exit 1
fi

. /etc/os-release

if [ "$ID" != "ubuntu" ]; then
    echo -e "${YELLOW}WARNING: This installer is designed for Ubuntu 24.04${NC}"
    echo -e "Detected: ${PRETTY_NAME}"
    echo ""
    read -p "Continue anyway? [y/N] " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Installation cancelled."
        exit 1
    fi
else
    echo -e "${GREEN}Detected: ${PRETTY_NAME}${NC}"
fi

# Find the installer binary (try canonical name, then suffixed variants)
INSTALLER=""
for name in "nqr-installer" "nqr-installer-x86_64-linux-musl" "nqrust-installer"; do
    if [ -x "${BUNDLE_DIR}/bin/${name}" ]; then
        INSTALLER="${BUNDLE_DIR}/bin/${name}"
        break
    fi
done

if [ -z "${INSTALLER}" ]; then
    echo -e "${RED}ERROR: Installer binary not found in ${BUNDLE_DIR}/bin/${NC}"
    echo "Bundle contents:"
    ls -la "${BUNDLE_DIR}/bin/" 2>/dev/null || echo "(bin/ directory not found)"
    exit 1
fi

echo -e "Bundle path: ${BUNDLE_DIR}"
echo ""

# Detect which flag the installer binary supports (--airgap is new, --iso-mode is legacy)
AIRGAP_FLAG="--iso-mode"
if "${INSTALLER}" install --help 2>&1 | grep -q '\-\-airgap'; then
    AIRGAP_FLAG="--airgap"
fi

# Launch the TUI installer in air-gapped mode
exec "${INSTALLER}" install \
    ${AIRGAP_FLAG} \
    --bundle-path "${BUNDLE_DIR}" \
    "$@"
