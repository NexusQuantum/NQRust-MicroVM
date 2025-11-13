#!/usr/bin/env bash
# Dependency installation for NQRust-MicroVM installer
# Installs all required system packages and tools

# Install system packages (Ubuntu/Debian)
install_apt_packages() {
    log_info "Installing system packages (apt)..."

    sudo apt-get update -qq

    local packages=(
        build-essential
        pkg-config
        libssl-dev
        curl
        git
        screen
        openssl
        iproute2
        iptables
        bridge-utils
        dnsmasq
        net-tools
        sudo
        lsof
    )

    # Add PostgreSQL if installing locally
    if [[ "${DB_TYPE:-local}" == "local" ]]; then
        packages+=(postgresql postgresql-contrib)
    fi

    sudo DEBIAN_FRONTEND=noninteractive apt-get install -y "${packages[@]}"

    log_success "System packages installed"
}

# Install system packages (RHEL/Rocky/AlmaLinux)
install_yum_packages() {
    log_info "Installing system packages (yum/dnf)..."

    local pkg_manager="yum"
    if command_exists dnf; then
        pkg_manager="dnf"
    fi

    local packages=(
        gcc
        gcc-c++
        make
        pkgconfig
        openssl-devel
        curl
        git
        screen
        openssl
        iproute
        iptables
        bridge-utils
        dnsmasq
        net-tools
        sudo
        lsof
    )

    # Add PostgreSQL if installing locally
    if [[ "${DB_TYPE:-local}" == "local" ]]; then
        packages+=(postgresql postgresql-server postgresql-contrib)
    fi

    sudo $pkg_manager install -y "${packages[@]}"

    log_success "System packages installed"
}

# Install system packages (auto-detect)
install_system_packages() {
    log_info "Installing system dependencies..."

    get_os_info

    case "$OS_NAME" in
        ubuntu|debian)
            install_apt_packages
            ;;
        rocky|rhel|centos|almalinux|fedora)
            install_yum_packages
            ;;
        *)
            log_error "Unsupported OS for automatic package installation: $OS_NAME"
            log_error "Please install dependencies manually"
            exit 1
            ;;
    esac
}

# Install Rust toolchain
install_rust() {
    log_info "Installing Rust toolchain..."

    if command_exists rustc && command_exists cargo; then
        local rust_version=$(rustc --version | awk '{print $2}')
        log_info "Rust already installed: $rust_version"

        # Check if version is recent enough
        if version_ge "$rust_version" "1.70.0"; then
            log_success "Rust version OK"
        else
            log_warn "Rust version is old, updating..."
            rustup update stable
        fi
    else
        log_info "Downloading Rust installer..."

        # Download and run rustup
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal

        # Source cargo environment
        source "$HOME/.cargo/env"

        log_success "Rust installed: $(rustc --version)"
    fi

    # Add musl target for guest-agent (static linking)
    log_info "Adding musl target for guest-agent..."
    rustup target add x86_64-unknown-linux-musl

    log_success "Rust toolchain configured"
}

# Install Firecracker
install_firecracker() {
    log_info "Installing Firecracker..."

    # Check if already installed
    if command_exists firecracker; then
        local fc_version=$(firecracker --version 2>&1 | grep -oP 'v\d+\.\d+\.\d+' | head -1)
        log_info "Firecracker already installed: $fc_version"

        if [[ "$fc_version" == "v1.13.1" ]]; then
            log_success "Firecracker version OK"
            return 0
        else
            log_warn "Different Firecracker version detected, will reinstall v1.13.1"
        fi
    fi

    local FC_VERSION="v1.13.1"
    local FC_ARCH="x86_64"
    local FC_URL="https://github.com/firecracker-microvm/firecracker/releases/download/${FC_VERSION}/firecracker-${FC_VERSION}-${FC_ARCH}.tgz"
    local FC_CHECKSUM="c1c60c7e07ba91eb7aaed5b7d3a09b5ac43a18e9d6f5d3e12a0f8d80ec60e95c"

    log_info "Downloading Firecracker ${FC_VERSION}..."

    download_file "$FC_URL" "/tmp/firecracker.tgz" "Downloading Firecracker"

    # Verify checksum
    log_info "Verifying checksum..."
    if ! verify_checksum "/tmp/firecracker.tgz" "$FC_CHECKSUM"; then
        log_warn "Checksum verification failed (continuing anyway)"
    fi

    # Extract
    log_info "Extracting..."
    tar xzf /tmp/firecracker.tgz -C /tmp/

    # Install
    sudo install -m 755 "/tmp/release-${FC_VERSION}-${FC_ARCH}/firecracker-${FC_VERSION}-${FC_ARCH}" /usr/local/bin/firecracker

    # Cleanup
    rm -rf /tmp/firecracker.tgz "/tmp/release-${FC_VERSION}-${FC_ARCH}"

    # Verify installation
    if firecracker --version >/dev/null 2>&1; then
        log_success "Firecracker installed: $(firecracker --version 2>&1 | head -1)"
    else
        log_error "Firecracker installation failed"
        exit 1
    fi
}

# Install Node.js and pnpm
install_nodejs() {
    log_info "Installing Node.js..."

    if command_exists node; then
        local node_version=$(node --version | sed 's/v//')
        log_info "Node.js already installed: v$node_version"

        if version_ge "$node_version" "20.0.0"; then
            log_success "Node.js version OK"
        else
            log_warn "Node.js version is old, updating recommended"
        fi
    else
        log_info "Installing Node.js 20.x LTS..."

        if [[ "$OS_NAME" == "ubuntu" || "$OS_NAME" == "debian" ]]; then
            # Add NodeSource repository
            curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
            sudo apt-get install -y nodejs
        elif [[ "$OS_NAME" =~ ^(rocky|rhel|centos|almalinux)$ ]]; then
            # Add NodeSource repository for RHEL
            curl -fsSL https://rpm.nodesource.com/setup_20.x | sudo bash -
            sudo yum install -y nodejs
        else
            log_error "Cannot auto-install Node.js on $OS_NAME"
            log_error "Please install Node.js 20.x manually"
            exit 1
        fi

        log_success "Node.js installed: $(node --version)"
    fi

    # Install pnpm
    if command_exists pnpm; then
        log_info "pnpm already installed: $(pnpm --version)"
    else
        log_info "Installing pnpm..."
        sudo npm install -g pnpm
        log_success "pnpm installed: $(pnpm --version)"
    fi
}

# Install SQLx CLI
install_sqlx_cli() {
    log_info "Installing SQLx CLI..."

    if command_exists sqlx; then
        log_info "SQLx CLI already installed"
        log_success "SQLx CLI: $(sqlx --version)"
        return 0
    fi

    log_info "This may take a few minutes..."

    # Install with only PostgreSQL support (faster compilation)
    cargo install sqlx-cli --no-default-features --features postgres 2>&1 | grep -v "Compiling" | grep -v "Finished" || true

    if command_exists sqlx; then
        log_success "SQLx CLI installed: $(sqlx --version)"
    else
        log_error "SQLx CLI installation failed"
        exit 1
    fi
}

# Install all dependencies
install_all_dependencies() {
    log_phase "2/11" "Installing Dependencies"

    install_system_packages
    install_rust
    install_firecracker

    # Install Node.js only if UI is requested
    if [[ "${WITH_UI:-true}" == "true" ]]; then
        install_nodejs
    else
        log_info "Skipping Node.js installation (UI not requested)"
    fi

    install_sqlx_cli

    log_success "All dependencies installed successfully"
}

# Show installed versions
show_dependency_versions() {
    echo ""
    log_info "Installed dependency versions:"
    echo ""

    if command_exists rustc; then
        echo "  Rust:       $(rustc --version)"
    fi

    if command_exists firecracker; then
        echo "  Firecracker: $(firecracker --version 2>&1 | head -1)"
    fi

    if command_exists node; then
        echo "  Node.js:    $(node --version)"
    fi

    if command_exists pnpm; then
        echo "  pnpm:       v$(pnpm --version)"
    fi

    if command_exists sqlx; then
        echo "  SQLx CLI:   $(sqlx --version)"
    fi

    if command_exists psql; then
        echo "  PostgreSQL: $(psql --version)"
    fi

    echo ""
}

# Export functions
export -f install_system_packages install_apt_packages install_yum_packages
export -f install_rust install_firecracker install_nodejs install_sqlx_cli
export -f install_all_dependencies show_dependency_versions
