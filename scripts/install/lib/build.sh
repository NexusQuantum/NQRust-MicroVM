#!/usr/bin/env bash
# Build and binary management for NQRust-MicroVM installer
# Handles building from source or downloading pre-built binaries

# Build from source
build_from_source() {
    log_info "Building from source..."

    # Ensure we're in the project root
    if [[ -z "${PROJECT_ROOT}" ]]; then
        if [[ -f "Cargo.toml" ]] && [[ -d "apps" ]]; then
            PROJECT_ROOT="$(pwd)"
        else
            log_error "Cannot find project root. Please run from project directory."
            exit 1
        fi
    fi

    cd "$PROJECT_ROOT"

    # Build main workspace (manager + agent)
    log_info "Building manager and agent..."
    cargo build --release 2>&1 | tee /tmp/nqrust-build.log | grep -E "(Compiling|Finished)" || true

    if [[ ! -f "target/release/manager" ]] || [[ ! -f "target/release/agent" ]]; then
        log_error "Build failed. Check /tmp/nqrust-build.log"
        exit 1
    fi

    log_success "Manager and agent built"

    # Build guest-agent with musl (static linking)
    log_info "Building guest-agent (static)..."
    cargo build --release --target x86_64-unknown-linux-musl -p guest-agent 2>&1 | grep -E "(Compiling|Finished)" || true

    if [[ ! -f "target/x86_64-unknown-linux-musl/release/guest-agent" ]]; then
        log_error "Guest-agent build failed"
        exit 1
    fi

    log_success "Guest-agent built"

    # Build UI if requested
    if [[ "${WITH_UI:-true}" == "true" ]]; then
        build_ui
    fi

    log_success "Build completed successfully"
}

# Build UI
build_ui() {
    log_info "Building UI..."

    local ui_dir="$PROJECT_ROOT/apps/ui"

    if [[ ! -d "$ui_dir" ]]; then
        log_warn "UI directory not found, skipping"
        return 0
    fi

    cd "$ui_dir"

    # Install dependencies
    log_info "Installing UI dependencies..."
    pnpm install 2>&1 | grep -v "Progress" || true

    # Build
    log_info "Building UI (this may take a few minutes)..."
    pnpm build 2>&1 | grep -E "(Creating|Compiled)" || true

    if [[ ! -d ".next" ]]; then
        log_error "UI build failed"
        exit 1
    fi

    log_success "UI built"

    cd "$PROJECT_ROOT"
}

# Download pre-built binaries
download_binaries() {
    log_info "Downloading pre-built binaries..."

    local version="${RELEASE_VERSION:-latest}"
    local base_url="https://github.com/${GITHUB_REPO:-NexusQuantum/NQRust-MicroVM}/releases"

    if [[ "$version" == "latest" ]]; then
        log_info "Fetching latest release..."
        version=$(curl -s "${base_url}/latest" | grep -oP 'tag/\K.*' | cut -d'"' -f1)

        if [[ -z "$version" ]]; then
            log_error "Could not determine latest version"
            log_error "Please specify version with RELEASE_VERSION=vX.Y.Z"
            exit 1
        fi

        log_info "Latest version: $version"
    fi

    local download_url="${base_url}/download/${version}"

    # Download manager
    log_info "Downloading manager..."
    download_file \
        "${download_url}/nqrust-manager-x86_64-unknown-linux-gnu" \
        "/tmp/manager" \
        "Downloading manager binary"
    chmod +x /tmp/manager

    # Download agent
    log_info "Downloading agent..."
    download_file \
        "${download_url}/nqrust-agent-x86_64-unknown-linux-gnu" \
        "/tmp/agent" \
        "Downloading agent binary"
    chmod +x /tmp/agent

    # Download guest-agent
    log_info "Downloading guest-agent..."
    download_file \
        "${download_url}/nqrust-guest-agent-x86_64-linux-musl" \
        "/tmp/guest-agent" \
        "Downloading guest-agent binary"
    chmod +x /tmp/guest-agent

    # Download UI if requested
    if [[ "${WITH_UI:-true}" == "true" ]]; then
        download_ui "$download_url"
    fi

    # Download base images
    download_base_images "$download_url"

    log_success "Binaries downloaded"
}

# Download base images (kernel, rootfs, runtimes)
download_base_images() {
    local download_url=$1

    log_info "Downloading base images..."

    local image_dir="${IMAGE_DIR:-/srv/images}"
    sudo mkdir -p "$image_dir"

    # Kernel (required)
    log_info "Downloading Firecracker kernel..."
    download_file \
        "${download_url}/vmlinux-5.10.fc.bin" \
        "$image_dir/vmlinux-5.10.fc.bin" \
        "Downloading kernel"

    # Function runtimes
    log_info "Downloading Node.js runtime..."
    download_file \
        "${download_url}/node-runtime.ext4" \
        "$image_dir/node-runtime.ext4" \
        "Downloading Node.js runtime"

    log_info "Downloading Python runtime..."
    download_file \
        "${download_url}/python-runtime.ext4" \
        "$image_dir/python-runtime.ext4" \
        "Downloading Python runtime"

    log_info "Downloading Bun runtime..."
    download_file \
        "${download_url}/bun-runtime.ext4" \
        "$image_dir/bun-runtime.ext4" \
        "Downloading Bun runtime"

    # Base images (optional but useful)
    log_info "Downloading Alpine rootfs..."
    download_file \
        "${download_url}/alpine-3.18-minimal.ext4" \
        "$image_dir/alpine-3.18-minimal.ext4" \
        "Downloading Alpine rootfs"

    log_info "Downloading Ubuntu rootfs..."
    download_file \
        "${download_url}/ubuntu-24.04-minimal.ext4" \
        "$image_dir/ubuntu-24.04-minimal.ext4" \
        "Downloading Ubuntu rootfs"

    # Set permissions
    sudo chown -R nqrust:nqrust "$image_dir" 2>/dev/null || true
    sudo chmod -R 755 "$image_dir"

    log_success "Base images downloaded"
}

# Download UI
download_ui() {
    local download_url=$1

    log_info "Downloading UI..."

    download_file \
        "${download_url}/nqrust-ui.tar.gz" \
        "/tmp/nqrust-ui.tar.gz" \
        "Downloading UI"

    # Extract to temp location
    mkdir -p /tmp/nqrust-ui
    tar xzf /tmp/nqrust-ui.tar.gz -C /tmp/nqrust-ui

    log_success "UI downloaded"
}

# Install binaries to target location
install_binaries() {
    log_info "Installing binaries..."

    local bin_dir="${INSTALL_DIR}/bin"
    create_dir "$bin_dir" "root:root" "755"

    # Determine source location based on build method
    if [[ "$INSTALL_MODE" == "dev" ]]; then
        # Built from source
        local manager_src="$PROJECT_ROOT/target/release/manager"
        local agent_src="$PROJECT_ROOT/target/release/agent"
        local guest_agent_src="$PROJECT_ROOT/target/x86_64-unknown-linux-musl/release/guest-agent"
    else
        # Downloaded pre-built
        local manager_src="/tmp/manager"
        local agent_src="/tmp/agent"
        local guest_agent_src="/tmp/guest-agent"
    fi

    # Install manager
    if [[ -f "$manager_src" ]]; then
        sudo install -m 755 "$manager_src" "$bin_dir/manager"
        log_success "Installed manager"
    else
        log_error "Manager binary not found: $manager_src"
        exit 1
    fi

    # Install agent
    if [[ -f "$agent_src" ]]; then
        sudo install -m 755 "$agent_src" "$bin_dir/agent"
        log_success "Installed agent"
    else
        log_error "Agent binary not found: $agent_src"
        exit 1
    fi

    # Install guest-agent
    if [[ -f "$guest_agent_src" ]]; then
        sudo install -m 755 "$guest_agent_src" "$bin_dir/guest-agent"
        log_success "Installed guest-agent"
    else
        log_error "Guest-agent binary not found: $guest_agent_src"
        exit 1
    fi

    # Verify installations
    if ! "$bin_dir/manager" --version >/dev/null 2>&1; then
        log_error "Manager binary not working"
        exit 1
    fi

    if ! "$bin_dir/agent" --version >/dev/null 2>&1; then
        log_error "Agent binary not working"
        exit 1
    fi

    log_success "All binaries installed and verified"
}

# Install UI
install_ui() {
    if [[ "${WITH_UI:-true}" != "true" ]]; then
        log_info "Skipping UI installation"
        return 0
    fi

    log_info "Installing UI..."

    local ui_dir="${INSTALL_DIR}/ui"
    create_dir "$ui_dir" "nqrust:nqrust" "755"

    if [[ "$INSTALL_MODE" == "dev" ]]; then
        # Copy from source
        local ui_src="$PROJECT_ROOT/apps/ui"

        if [[ -d "$ui_src/.next" ]]; then
            sudo cp -r "$ui_src/.next" "$ui_dir/"
            sudo cp -r "$ui_src/public" "$ui_dir/"
            sudo cp "$ui_src/package.json" "$ui_dir/"
            sudo cp "$ui_src/next.config.js" "$ui_dir/" 2>/dev/null || true

            # Install production dependencies
            cd "$ui_dir"
            sudo -u nqrust pnpm install --prod

            log_success "UI installed from source"
        else
            log_warn "UI not built, skipping installation"
        fi
    else
        # Extract downloaded UI
        if [[ -d "/tmp/nqrust-ui" ]]; then
            sudo cp -r /tmp/nqrust-ui/* "$ui_dir/"

            # Install production dependencies
            cd "$ui_dir"
            sudo -u nqrust pnpm install --prod

            log_success "UI installed from download"
        else
            log_warn "UI not downloaded, skipping installation"
        fi
    fi

    # Set ownership
    sudo chown -R nqrust:nqrust "$ui_dir"
}

# Build container runtime (optional)
build_container_runtime() {
    if [[ "${WITH_CONTAINER_RUNTIME:-false}" != "true" ]]; then
        return 0
    fi

    log_info "Building container runtime..."

    if [[ ! -f "$PROJECT_ROOT/scripts/build-container-runtime-v2.sh" ]]; then
        log_warn "Container runtime build script not found, skipping"
        return 0
    fi

    log_info "This will take 5-10 minutes..."

    cd "$PROJECT_ROOT"
    sudo bash scripts/build-container-runtime-v2.sh 2>&1 | grep -v "Step" || true

    if [[ -f "/srv/images/container-runtime.ext4" ]]; then
        log_success "Container runtime built"
    else
        log_error "Container runtime build failed"
        exit 1
    fi
}

# Show binary versions
show_binary_versions() {
    local bin_dir="${INSTALL_DIR}/bin"

    echo ""
    log_info "Installed binary versions:"
    echo ""

    if [[ -f "$bin_dir/manager" ]]; then
        echo "  Manager:      $("$bin_dir/manager" --version 2>&1 || echo "unknown")"
    fi

    if [[ -f "$bin_dir/agent" ]]; then
        echo "  Agent:        $("$bin_dir/agent" --version 2>&1 || echo "unknown")"
    fi

    if [[ -f "$bin_dir/guest-agent" ]]; then
        echo "  Guest-agent:  $(file "$bin_dir/guest-agent" | grep -o "statically linked" || echo "dynamically linked")"
    fi

    echo ""
}

# Export functions
export -f build_from_source build_ui download_binaries download_ui
export -f install_binaries install_ui build_container_runtime
export -f show_binary_versions
