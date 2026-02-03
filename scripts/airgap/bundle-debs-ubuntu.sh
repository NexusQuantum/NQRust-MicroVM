#!/bin/bash
# =============================================================================
# Bundle .deb packages for Ubuntu 24.04 (air-gapped installation)
# =============================================================================
# Downloads all required .deb packages and their dependencies for offline
# installation on Ubuntu 24.04 servers. Uses Docker containers to resolve the
# correct dependencies for the target Ubuntu version.
#
# Requirements:
#   - Docker installed and running
#
# Usage:
#   ./bundle-debs-ubuntu.sh [--output <dir>] [--version 24.04]
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT_DIR="${SCRIPT_DIR}/output/debs"
TARGET_VERSION="24.04"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info()    { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[OK]${NC} $1"; }
log_warn()    { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error()   { echo -e "${RED}[ERROR]${NC} $1"; }

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --output)  OUTPUT_DIR="$2"; shift 2 ;;
        --version) TARGET_VERSION="$2"; shift 2 ;;
        --help|-h)
            echo "Usage: $(basename "$0") [--output <dir>] [--version 24.04]"
            exit 0
            ;;
        *) shift ;;
    esac
done

# Check Docker is available
if ! command -v docker &> /dev/null; then
    log_error "Docker is required but not found."
    log_info "Install Docker: curl -fsSL https://get.docker.com | sh"
    exit 1
fi

# Packages required by NQR-MicroVM (from apps/installer/src/installer/deps.rs)
# These are the packages the TUI installer needs on the target Ubuntu server.
PACKAGES=(
    # Network (required for Firecracker VM networking)
    "iproute2"
    "iptables"
    "bridge-utils"
    "dnsmasq"
    "net-tools"
    "lsof"
    # Database (manager requires PostgreSQL)
    "postgresql"
    "postgresql-contrib"
    # System utilities
    "screen"          # Agent uses screen for VM shell sessions
    "ca-certificates"
    "curl"            # Useful for post-install API testing
    "sudo"
    "openssl"
)

PACKAGE_LIST="${PACKAGES[*]}"

# Download debs for a specific Ubuntu version using Docker
download_debs_for_version() {
    local version="$1"
    local output_dir="$2"
    local docker_image="ubuntu:${version}"

    log_info "Downloading .deb packages for Ubuntu ${version}..."
    mkdir -p "${output_dir}"

    # Pull the image first
    docker pull "${docker_image}" >/dev/null 2>&1

    # Write the download script to a temp file to avoid escaping issues
    local script_file
    script_file=$(mktemp /tmp/nqr-deb-download-XXXXXX.sh)
    cat > "${script_file}" << 'DOWNLOAD_SCRIPT'
#!/bin/bash
set -e
export DEBIAN_FRONTEND=noninteractive

# Update package lists
apt-get update -qq >/dev/null 2>&1

# Install prerequisites for adding Docker's APT repository
apt-get install -y -qq ca-certificates curl gnupg >/dev/null 2>&1

# Add Docker's official GPG key and repository
install -m 0755 -d /etc/apt/keyrings
curl -fsSL https://download.docker.com/linux/ubuntu/gpg -o /etc/apt/keyrings/docker.asc
chmod a+r /etc/apt/keyrings/docker.asc
ARCH=$(dpkg --print-architecture)
CODENAME=$(. /etc/os-release && echo "$VERSION_CODENAME")
echo "deb [arch=${ARCH} signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/ubuntu ${CODENAME} stable" > /etc/apt/sources.list.d/docker.list
apt-get update -qq >/dev/null 2>&1

# Resolve all dependencies recursively (system packages + Docker)
DOCKER_PKGS="docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin"
DEPS=$(apt-cache depends --recurse --no-recommends --no-suggests \
    --no-conflicts --no-breaks --no-replaces --no-enhances \
    $SYSTEM_PACKAGES ${DOCKER_PKGS} 2>/dev/null \
    | grep '^\w' \
    | grep -v '^<' \
    | sort -u)

# Download all packages
cd /tmp
apt-get download ${DEPS} 2>/dev/null || true

# Copy to output
cp /tmp/*.deb /output/ 2>/dev/null || true

# Count and report
PKG_COUNT=$(ls -1 /output/*.deb 2>/dev/null | wc -l)
echo "Downloaded ${PKG_COUNT} packages"
DOWNLOAD_SCRIPT

    # Run the script inside a container with the package list as an env var
    docker run --rm \
        -v "${output_dir}:/output" \
        -v "${script_file}:/download.sh:ro" \
        -e "SYSTEM_PACKAGES=${PACKAGE_LIST}" \
        "${docker_image}" \
        bash /download.sh

    rm -f "${script_file}"

    local pkg_count
    pkg_count=$(ls -1 "${output_dir}"/*.deb 2>/dev/null | wc -l)
    local total_size
    total_size=$(du -sh "${output_dir}" 2>/dev/null | cut -f1)

    if [[ "${pkg_count}" -gt 0 ]]; then
        log_success "Ubuntu ${version}: ${pkg_count} packages (${total_size})"
    else
        log_error "Ubuntu ${version}: No packages downloaded!"
        return 1
    fi
}

# Main
log_info "==================================="
log_info "NQR-MicroVM Deb Package Bundler"
log_info "==================================="
log_info "Target: Ubuntu ${TARGET_VERSION}"
log_info "Output: ${OUTPUT_DIR}"
log_info "Packages: ${PACKAGE_LIST}"
echo ""

case "${TARGET_VERSION}" in
    24.04)
        download_debs_for_version "24.04" "${OUTPUT_DIR}/ubuntu-24.04"
        ;;
    *)
        log_error "Unsupported version: ${TARGET_VERSION}"
        log_info "Supported: 24.04"
        exit 1
        ;;
esac

echo ""
log_success "Deb package bundling complete"
log_info "Total size: $(du -sh "${OUTPUT_DIR}" | cut -f1)"
