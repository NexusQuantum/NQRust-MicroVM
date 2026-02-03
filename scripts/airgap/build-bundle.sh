#!/bin/bash
# =============================================================================
# NQR-MicroVM Air-Gap Bundle Builder
# =============================================================================
# Assembles all components needed for a fully offline installation and
# packages them into a self-extracting binary using makeself.
#
# Run this on an internet-connected machine. The output is a single .run file
# that can be transferred to an air-gapped Ubuntu 22.04/24.04 server.
#
# Requirements:
#   - Internet connection
#   - Docker (for building version-specific .deb packages)
#   - curl
#   - makeself (will be installed automatically if missing)
#
# Usage:
#   ./build-bundle.sh [OPTIONS]
#
# Options:
#   --release <version>          Release version (default: latest)
#   --output <dir>               Output directory (default: ./output)
#   --no-container-runtime       Skip container-runtime.ext4 (~2GB savings)
#   --local                      Use local builds instead of GitHub downloads
#   --no-images                  Skip VM images (for faster testing)
#
# Output:
#   nqr-microvm-airgap-<version>.run  (~3-5GB with container runtime)
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
BUILD_DIR="/tmp/nqr-airgap-build-$$"
OUTPUT_DIR="${SCRIPT_DIR}/output"
RELEASE_VERSION="latest"
INCLUDE_CONTAINER_RUNTIME=true
USE_LOCAL=false
INCLUDE_IMAGES=true

# Firecracker and Node.js versions
FC_VERSION="1.13.1"
NODE_VERSION="20.18.1"

# GitHub repository
GITHUB_REPO="NexusQuantum/NQRust-MicroVM"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info()    { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[OK]${NC} $1"; }
log_warn()    { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error()   { echo -e "${RED}[ERROR]${NC} $1"; }

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --release)                RELEASE_VERSION="$2"; shift 2 ;;
        --output)                 OUTPUT_DIR="$2"; shift 2 ;;
        --no-container-runtime)   INCLUDE_CONTAINER_RUNTIME=false; shift ;;
        --local)                  USE_LOCAL=true; shift ;;
        --no-images)              INCLUDE_IMAGES=false; shift ;;
        --help|-h)
            head -n 30 "$0" | grep -E '^#' | sed 's/^# //' | sed 's/^#//'
            exit 0
            ;;
        *) log_error "Unknown option: $1"; exit 1 ;;
    esac
done

# GitHub download URL
if [[ "${RELEASE_VERSION}" == "latest" ]]; then
    GH_API_URL="https://api.github.com/repos/${GITHUB_REPO}/releases/latest"
    GH_DOWNLOAD_URL="https://github.com/${GITHUB_REPO}/releases/latest/download"
else
    GH_API_URL="https://api.github.com/repos/${GITHUB_REPO}/releases/tags/${RELEASE_VERSION}"
    GH_DOWNLOAD_URL="https://github.com/${GITHUB_REPO}/releases/download/${RELEASE_VERSION}"
fi

# Cleanup on exit
cleanup() {
    if [[ -d "${BUILD_DIR}" ]]; then
        log_info "Cleaning up build directory..."
        rm -rf "${BUILD_DIR}"
    fi
}
trap cleanup EXIT

# =============================================================================
# Phase 0: Check dependencies
# =============================================================================
check_dependencies() {
    log_info "Checking build dependencies..."

    local missing=()
    command -v curl   &>/dev/null || missing+=("curl")
    command -v docker &>/dev/null || missing+=("docker")
    command -v tar    &>/dev/null || missing+=("tar")
    command -v gzip   &>/dev/null || missing+=("gzip")

    if [[ ${#missing[@]} -gt 0 ]]; then
        log_error "Missing dependencies: ${missing[*]}"
        exit 1
    fi

    log_success "All dependencies found"
}

# =============================================================================
# Phase 1: Prepare workspace
# =============================================================================
prepare_workspace() {
    log_info "Preparing build workspace at ${BUILD_DIR}..."

    rm -rf "${BUILD_DIR}"
    mkdir -p "${BUILD_DIR}/bundle"/{bin,images,debs,ui,node,config,systemd}
    mkdir -p "${OUTPUT_DIR}"

    # Copy install.sh entry point
    cp "${SCRIPT_DIR}/install.sh" "${BUILD_DIR}/bundle/install.sh"
    chmod +x "${BUILD_DIR}/bundle/install.sh"

    log_success "Workspace prepared"
}

# =============================================================================
# Phase 2: Bundle binaries
# =============================================================================
bundle_binaries() {
    log_info "Bundling NQR-MicroVM binaries..."

    local bin_dir="${BUILD_DIR}/bundle/bin"

    if ${USE_LOCAL}; then
        # Copy from local builds
        # Local cargo names differ from installed names:
        #   nqr-installer -> nqr-installer (same)
        #   manager -> nqrust-manager
        #   agent -> nqrust-agent
        #   guest-agent -> nqrust-guest-agent
        log_info "Using local builds..."

        # Associative array: cargo_name -> bundle_name
        declare -A LOCAL_BIN_MAP=(
            ["nqr-installer"]="nqr-installer"
            ["manager"]="nqrust-manager"
            ["agent"]="nqrust-agent"
            ["guest-agent"]="nqrust-guest-agent"
        )

        for cargo_name in "${!LOCAL_BIN_MAP[@]}"; do
            local bundle_name="${LOCAL_BIN_MAP[$cargo_name]}"
            local src="${PROJECT_ROOT}/target/release/${cargo_name}"
            local musl_src="${PROJECT_ROOT}/target/x86_64-unknown-linux-musl/release/${cargo_name}"

            if [[ -f "${musl_src}" ]]; then
                cp "${musl_src}" "${bin_dir}/${bundle_name}"
                log_success "Copied ${cargo_name} -> ${bundle_name} (musl)"
            elif [[ -f "${src}" ]]; then
                cp "${src}" "${bin_dir}/${bundle_name}"
                log_success "Copied ${cargo_name} -> ${bundle_name}"
            else
                log_error "Binary not found: ${cargo_name}"
                log_info "Build with: cargo build --release"
                return 1
            fi
        done
    else
        # Download from GitHub releases
        log_info "Downloading binaries from GitHub (${RELEASE_VERSION})..."

        local assets_json
        assets_json=$(curl -fsSL "${GH_API_URL}" 2>/dev/null) || {
            log_error "Failed to fetch release info from GitHub API"
            log_info "Try --local to use local builds instead"
            return 1
        }

        # Download each binary asset
        local asset_names
        asset_names=$(echo "${assets_json}" | grep -o '"name": "[^"]*"' | cut -d'"' -f4)

        # Map: canonical name -> search patterns (in priority order)
        declare -A BIN_MAP=(
            ["nqr-installer"]="nqr-installer"
            ["nqrust-manager"]="nqrust-manager nqr-manager"
            ["nqrust-agent"]="nqrust-agent nqr-agent"
            ["nqrust-guest-agent"]="nqrust-guest-agent nqr-guest-agent"
        )

        for canonical in "${!BIN_MAP[@]}"; do
            local downloaded=false
            for pattern in ${BIN_MAP[$canonical]}; do
                local asset_name
                asset_name=$(echo "${asset_names}" | grep -E "^${pattern}" | grep -v '\.tar\.gz$' | head -n1) || true

                if [[ -n "${asset_name}" ]]; then
                    log_info "Downloading ${asset_name}..."
                    if curl -fsSL "${GH_DOWNLOAD_URL}/${asset_name}" -o "${bin_dir}/${asset_name}"; then
                        # Rename to canonical name so install.sh and the TUI installer can find it
                        if [[ "${asset_name}" != "${canonical}" ]]; then
                            mv "${bin_dir}/${asset_name}" "${bin_dir}/${canonical}"
                            log_success "Downloaded ${asset_name} -> ${canonical}"
                        else
                            log_success "Downloaded ${canonical}"
                        fi
                        downloaded=true
                        break
                    else
                        log_warn "Failed to download ${asset_name}"
                    fi
                fi
            done
            if ! ${downloaded}; then
                log_warn "No release asset found for ${canonical}"
            fi
        done
    fi

    chmod +x "${bin_dir}"/* 2>/dev/null || true

    # Verify critical binaries exist
    local missing_critical=()
    for name in "nqr-installer" "nqrust-manager" "nqrust-agent"; do
        if [[ ! -x "${bin_dir}/${name}" ]]; then
            missing_critical+=("${name}")
        fi
    done

    if [[ ${#missing_critical[@]} -gt 0 ]]; then
        log_error "Missing critical binaries: ${missing_critical[*]}"
        log_info "Bundle bin/ contents:"
        ls -la "${bin_dir}/"
        return 1
    fi

    log_success "Binaries bundled"
}

# =============================================================================
# Phase 3: Bundle Firecracker
# =============================================================================
bundle_firecracker() {
    log_info "Bundling Firecracker v${FC_VERSION}..."

    local bin_dir="${BUILD_DIR}/bundle/bin"
    local fc_url="https://github.com/firecracker-microvm/firecracker/releases/download/v${FC_VERSION}/firecracker-v${FC_VERSION}-x86_64.tgz"
    local tmp_dir
    tmp_dir=$(mktemp -d)

    curl -fsSL "${fc_url}" -o "${tmp_dir}/firecracker.tgz"
    tar -xzf "${tmp_dir}/firecracker.tgz" -C "${tmp_dir}"

    cp "${tmp_dir}/release-v${FC_VERSION}-x86_64/firecracker-v${FC_VERSION}-x86_64" "${bin_dir}/firecracker"
    cp "${tmp_dir}/release-v${FC_VERSION}-x86_64/jailer-v${FC_VERSION}-x86_64" "${bin_dir}/jailer"
    chmod +x "${bin_dir}/firecracker" "${bin_dir}/jailer"

    rm -rf "${tmp_dir}"

    log_success "Firecracker v${FC_VERSION} bundled"
}

# =============================================================================
# Phase 4: Bundle VM images
# =============================================================================
bundle_images() {
    if ! ${INCLUDE_IMAGES}; then
        log_warn "Skipping VM images (--no-images)"
        return
    fi

    log_info "Bundling VM images..."

    local images_dir="${BUILD_DIR}/bundle/images"

    # Image list (must match BASE_IMAGES in apps/installer/src/installer/build.rs)
    local images=(
        "vmlinux-5.10.fc.bin"
        "alpine-3.18-minimal.ext4"
        "busybox-1.35.ext4"
        "ubuntu-24.04-minimal.ext4"
        "python-runtime.ext4"
        "bun-runtime.ext4"
    )

    # Optionally include container runtime (~2GB)
    if ${INCLUDE_CONTAINER_RUNTIME}; then
        images+=("container-runtime.ext4")
    else
        log_info "Skipping container-runtime.ext4 (--no-container-runtime)"
    fi

    for image in "${images[@]}"; do
        # Try local path first
        local local_path="/srv/images/${image}"
        if [[ -f "${local_path}" ]]; then
            log_info "Copying ${image} from local..."
            cp "${local_path}" "${images_dir}/${image}"
            log_success "Copied ${image}"
            continue
        fi

        # Try GitHub release download
        # container-runtime is compressed on GitHub due to 2GB limit
        if [[ "${image}" == "container-runtime.ext4" ]]; then
            log_info "Downloading ${image}.gz from release..."
            if curl -fsSL "${GH_DOWNLOAD_URL}/${image}.gz" -o "${images_dir}/${image}.gz" 2>/dev/null; then
                log_info "Decompressing ${image}.gz..."
                gunzip -f "${images_dir}/${image}.gz"
                log_success "Downloaded and decompressed ${image}"
            else
                log_warn "Failed to download ${image} - skipping"
            fi
        else
            log_info "Downloading ${image} from release..."
            if curl -fsSL "${GH_DOWNLOAD_URL}/${image}" -o "${images_dir}/${image}" 2>/dev/null; then
                log_success "Downloaded ${image}"
            else
                log_warn "Failed to download ${image} - skipping"
            fi
        fi
    done

    # Show what we got
    log_info "Bundled images:"
    ls -lh "${images_dir}/" 2>/dev/null || true
}

# =============================================================================
# Phase 5: Bundle .deb packages
# =============================================================================
bundle_debs() {
    log_info "Bundling .deb packages for Ubuntu..."

    "${SCRIPT_DIR}/bundle-debs-ubuntu.sh" \
        --output "${BUILD_DIR}/bundle/debs" \
        --version 24.04

    log_success "Deb packages bundled"
}

# =============================================================================
# Phase 6: Bundle Node.js
# =============================================================================
bundle_node() {
    log_info "Bundling Node.js..."

    "${SCRIPT_DIR}/bundle-node.sh" \
        --output "${BUILD_DIR}/bundle/node" \
        --node-version "${NODE_VERSION}"

    log_success "Node.js bundled"
}

# =============================================================================
# Phase 7: Bundle UI
# =============================================================================
bundle_ui() {
    log_info "Bundling Web UI..."

    local ui_dir="${BUILD_DIR}/bundle/ui"

    # Try 1: Download pre-built UI tarball from release
    if ! ${USE_LOCAL}; then
        local tarball_name="nqrust-ui.tar.gz"
        log_info "Trying to download UI bundle from release..."
        if curl -fsSL "${GH_DOWNLOAD_URL}/${tarball_name}" -o "/tmp/nqrust-ui-bundle.tar.gz" 2>/dev/null; then
            tar -xzf "/tmp/nqrust-ui-bundle.tar.gz" -C "${ui_dir}" 2>/dev/null || \
            tar -xzf "/tmp/nqrust-ui-bundle.tar.gz" -C "${ui_dir}" --strip-components=1 2>/dev/null || true
            rm -f "/tmp/nqrust-ui-bundle.tar.gz"

            if [[ -f "${ui_dir}/server.js" ]] || [[ -f "${ui_dir}/package.json" ]]; then
                log_success "UI downloaded from release"
                return
            fi
        fi
    fi

    # Try 2: Build from source using Next.js standalone output
    local source_ui="${PROJECT_ROOT}/apps/ui"
    if [[ -f "${source_ui}/package.json" ]]; then
        log_info "Building UI from source (standalone mode)..."

        # Temporarily hide .env so NEXT_PUBLIC_API_BASE_URL from the dev machine
        # doesn't get baked into the production build. The UI code falls back to
        # window.location.hostname:18080 when the env var is unset.
        local env_backup=""
        if [[ -f "${source_ui}/.env" ]]; then
            env_backup="${source_ui}/.env.airgap-backup"
            mv "${source_ui}/.env" "${env_backup}"
        fi

        local build_ok=true
        if command -v pnpm &>/dev/null; then
            (cd "${source_ui}" && pnpm install && pnpm build) || build_ok=false
        elif command -v npm &>/dev/null; then
            (cd "${source_ui}" && npm install && npm run build) || build_ok=false
        else
            log_warn "Neither pnpm nor npm found - skipping UI build"
            [[ -n "${env_backup}" ]] && mv "${env_backup}" "${source_ui}/.env"
            return
        fi

        # Restore .env
        [[ -n "${env_backup}" ]] && mv "${env_backup}" "${source_ui}/.env"

        if ! ${build_ok}; then
            log_warn "UI build failed"
            return
        fi

        # Remove any .env files from the standalone output (should not be deployed)
        find "${source_ui}/.next/standalone" -name '.env*' -delete 2>/dev/null || true

        # Next.js standalone output produces .next/standalone/ with minimal node_modules
        # In monorepo setups, the app is nested: .next/standalone/apps/ui/
        local standalone_dir="${source_ui}/.next/standalone"
        local standalone_app_dir="${standalone_dir}/apps/ui"

        if [[ -d "${standalone_app_dir}" ]] && [[ -f "${standalone_app_dir}/server.js" ]]; then
            # Monorepo standalone: copy from nested apps/ui/ path
            cp -r "${standalone_app_dir}/." "${ui_dir}/"
            log_info "Using monorepo standalone output from apps/ui/"
        elif [[ -d "${standalone_dir}" ]] && [[ -f "${standalone_dir}/server.js" ]]; then
            # Single-app standalone: copy from root
            cp -r "${standalone_dir}/." "${ui_dir}/"
        else
            log_warn "Standalone output not found at ${standalone_dir}"
            log_warn "Ensure next.config.mjs has output: 'standalone'"
            return
        fi

        # Copy static assets (not included in standalone by default)
        if [[ -d "${source_ui}/.next/static" ]]; then
            mkdir -p "${ui_dir}/.next/static"
            cp -r "${source_ui}/.next/static/." "${ui_dir}/.next/static/"
        fi

        # Copy public directory
        if [[ -d "${source_ui}/public" ]]; then
            mkdir -p "${ui_dir}/public"
            cp -r "${source_ui}/public/." "${ui_dir}/public/"
        fi

        local ui_size
        ui_size=$(du -sh "${ui_dir}" | cut -f1)
        log_success "UI built and bundled from source (standalone: ${ui_size})"
    else
        log_warn "UI source not found at ${source_ui} - skipping"
    fi
}

# =============================================================================
# Phase 8: Create self-extracting archive
# =============================================================================
create_archive() {
    log_info "Creating self-extracting archive..."

    local version_tag="${RELEASE_VERSION}"
    local output_file="${OUTPUT_DIR}/nqr-microvm-airgap-${version_tag}.run"

    # Show bundle summary
    local bundle_size
    bundle_size=$(du -sh "${BUILD_DIR}/bundle" | cut -f1)
    log_info "Bundle size: ${bundle_size}"
    log_info "Bundle contents:"
    echo "  bin/:    $(ls -1 "${BUILD_DIR}/bundle/bin/" 2>/dev/null | wc -l) files"
    echo "  images/: $(ls -1 "${BUILD_DIR}/bundle/images/" 2>/dev/null | wc -l) files"
    echo "  debs/:   $(find "${BUILD_DIR}/bundle/debs/" -name '*.deb' 2>/dev/null | wc -l) packages"
    echo "  node/:   $(ls -1 "${BUILD_DIR}/bundle/node/" 2>/dev/null | wc -l) files"
    echo "  ui/:     $(ls -1 "${BUILD_DIR}/bundle/ui/" 2>/dev/null | wc -l) files"
    echo ""

    # Create self-extracting archive
    # Uses a simple tar.gz + shell header approach (more reliable than makeself
    # with pnpm's symlink-heavy node_modules)
    log_info "Compressing bundle..."
    local tar_file="${BUILD_DIR}/bundle.tar.gz"
    (cd "${BUILD_DIR}/bundle" && tar czf "${tar_file}" .) || {
        log_error "Failed to create tar archive"
        return 1
    }

    # Write self-extracting header
    cat > "${output_file}" << 'SELFEXTRACT'
#!/bin/bash
# NQR-MicroVM Air-Gapped Installer (self-extracting)
# Usage: sudo ./nqr-microvm-airgap-*.run [installer args...]
# Extract only: ./nqr-microvm-airgap-*.run --noexec --target <dir>
set -e

NOEXEC=false
TARGET=""

# Parse self-extractor args
while [[ $# -gt 0 ]]; do
    case "$1" in
        --noexec) NOEXEC=true; shift ;;
        --target) TARGET="$2"; shift 2 ;;
        *) break ;;
    esac
done

if [[ -n "$TARGET" ]]; then
    EXTRACT_DIR="$TARGET"
    mkdir -p "$EXTRACT_DIR"
    CLEANUP=false
else
    EXTRACT_DIR=$(mktemp -d /tmp/nqr-install-XXXXXX)
    CLEANUP=true
fi

if $CLEANUP; then
    trap "rm -rf $EXTRACT_DIR" EXIT
fi

echo "Extracting installer to ${EXTRACT_DIR}..."
ARCHIVE_START=$(awk '/^__ARCHIVE_START__$/{print NR + 1; exit 0;}' "$0")
tail -n+$ARCHIVE_START "$0" | tar xz -C "$EXTRACT_DIR"

if $NOEXEC; then
    echo "Bundle extracted to: ${EXTRACT_DIR}"
    if $CLEANUP; then trap - EXIT; fi
    exit 0
fi

cd "$EXTRACT_DIR"
exec bash ./install.sh "$@"
__ARCHIVE_START__
SELFEXTRACT

    # Append the tar archive
    cat "${tar_file}" >> "${output_file}"
    chmod +x "${output_file}"
    rm -f "${tar_file}"

    # Generate checksums
    (cd "${OUTPUT_DIR}" && sha256sum "$(basename "${output_file}")" > "$(basename "${output_file}").sha256")

    local final_size
    final_size=$(du -h "${output_file}" | cut -f1)

    echo ""
    log_success "==================================="
    log_success "Air-gapped bundle created!"
    log_success "==================================="
    echo ""
    echo "  File:     ${output_file}"
    echo "  Size:     ${final_size}"
    echo "  SHA256:   $(cut -d' ' -f1 "${output_file}.sha256")"
    echo ""
    echo "  Transfer to air-gapped server and run:"
    echo "    sudo ./$(basename "${output_file}")"
    echo ""
    echo "  Or extract without running:"
    echo "    ./$(basename "${output_file}") --noexec --target /opt/nqr-bundle"
    echo ""
}

# =============================================================================
# Main
# =============================================================================
main() {
    echo ""
    log_info "==================================="
    log_info "NQR-MicroVM Air-Gap Bundle Builder"
    log_info "==================================="
    log_info "Release: ${RELEASE_VERSION}"
    log_info "Output:  ${OUTPUT_DIR}"
    log_info "Container runtime: ${INCLUDE_CONTAINER_RUNTIME}"
    log_info "Local builds: ${USE_LOCAL}"
    echo ""

    check_dependencies
    prepare_workspace
    bundle_binaries
    bundle_firecracker
    bundle_images
    bundle_debs
    bundle_node
    bundle_ui
    create_archive
}

main "$@"
