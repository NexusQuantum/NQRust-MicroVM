#!/usr/bin/env bash
# Pre-flight checks for NQRust-MicroVM installer
# Validates system requirements before installation

# Check operating system
check_os() {
    log_info "Checking operating system..."

    get_os_info

    case "$OS_NAME" in
        ubuntu)
            if ! version_ge "$OS_VERSION" "22.04"; then
                log_error "Ubuntu 22.04 or newer required (found: $OS_PRETTY)"
                exit 1
            fi
            ;;
        debian)
            if ! version_ge "$OS_VERSION" "11"; then
                log_error "Debian 11 or newer required (found: $OS_PRETTY)"
                exit 1
            fi
            ;;
        rocky|rhel|centos|almalinux)
            if ! version_ge "$OS_VERSION" "8"; then
                log_error "RHEL/Rocky/AlmaLinux 8 or newer required (found: $OS_PRETTY)"
                exit 1
            fi
            ;;
        *)
            log_warn "Unsupported OS: $OS_PRETTY"
            if ! confirm "Continue anyway? (not recommended)"; then
                exit 1
            fi
            ;;
    esac

    log_success "OS check passed: $OS_PRETTY"
}

# Check CPU for KVM support
check_kvm_support() {
    log_info "Checking CPU virtualization support..."

    if ! grep -E 'vmx|svm' /proc/cpuinfo >/dev/null; then
        log_error "CPU does not support virtualization (no VT-x/AMD-V)"
        log_error "This system cannot run Firecracker VMs"
        exit 1
    fi

    # Check if KVM is available
    if [[ ! -e /dev/kvm ]]; then
        log_warn "/dev/kvm not found. KVM module may not be loaded."
        log_info "Will attempt to load KVM module during installation"
    else
        log_success "KVM support detected"
    fi
}

# Check RAM
check_ram() {
    local min_ram_mb=${1:-2048}  # Default 2GB
    log_info "Checking RAM..."

    local total_ram=$(free -m | awk '/^Mem:/{print $2}')

    if [[ $total_ram -lt $min_ram_mb ]]; then
        log_error "Insufficient RAM: ${total_ram}MB (minimum: ${min_ram_mb}MB)"
        exit 1
    fi

    log_success "RAM check passed: ${total_ram}MB available"
}

# Check disk space
check_disk_space() {
    local min_space_gb=${1:-20}  # Default 20GB
    log_info "Checking disk space..."

    # Check space in /srv (where VMs will be stored)
    local srv_space=$(df -BG /srv 2>/dev/null | awk 'NR==2 {print $4}' | sed 's/G//')
    if [[ -z "$srv_space" ]]; then
        srv_space=$(df -BG / | awk 'NR==2 {print $4}' | sed 's/G//')
    fi

    if [[ $srv_space -lt $min_space_gb ]]; then
        log_error "Insufficient disk space: ${srv_space}GB (minimum: ${min_space_gb}GB)"
        exit 1
    fi

    log_success "Disk space check passed: ${srv_space}GB available"
}

# Check if ports are available
check_ports() {
    log_info "Checking port availability..."

    local ports_to_check=(
        "8080:Manager API"
        "9090:Agent API"
        "3000:UI"
        "5432:PostgreSQL"
    )

    for port_info in "${ports_to_check[@]}"; do
        local port="${port_info%%:*}"
        local service="${port_info##*:}"

        if sudo lsof -Pi :$port -sTCP:LISTEN -t >/dev/null 2>&1; then
            log_warn "Port $port ($service) is already in use"
            local pid=$(sudo lsof -Pi :$port -sTCP:LISTEN -t)
            local process=$(ps -p $pid -o comm=)
            log_warn "  Process: $process (PID: $pid)"

            if [[ "$NON_INTERACTIVE" != "true" ]]; then
                if ! confirm "Continue anyway?"; then
                    exit 1
                fi
            fi
        fi
    done

    log_success "Port availability check passed"
}

# Check for conflicting software
check_conflicts() {
    log_info "Checking for conflicting software..."

    local conflicts_found=false

    # Check for other VM managers
    if command_exists libvirtd && systemctl is-active --quiet libvirtd; then
        log_warn "libvirt is running (may conflict with Firecracker networking)"
        conflicts_found=true
    fi

    # Check for Docker
    if command_exists docker && systemctl is-active --quiet docker; then
        log_warn "Docker is running (may cause bridge conflicts)"
        conflicts_found=true
    fi

    # Check for existing bridge named fcbr0
    if ip link show fcbr0 >/dev/null 2>&1; then
        log_warn "Bridge fcbr0 already exists"
        conflicts_found=true
    fi

    if [[ "$conflicts_found" == "true" && "$NON_INTERACTIVE" != "true" ]]; then
        if ! confirm "Potential conflicts detected. Continue anyway?"; then
            exit 1
        fi
    fi

    log_success "Conflict check completed"
}

# Check system architecture
check_architecture() {
    log_info "Checking system architecture..."

    local arch=$(uname -m)

    if [[ "$arch" != "x86_64" ]]; then
        log_error "Unsupported architecture: $arch"
        log_error "NQRust-MicroVM requires x86_64"
        exit 1
    fi

    log_success "Architecture check passed: $arch"
}

# Check kernel version
check_kernel() {
    log_info "Checking kernel version..."

    local kernel_version=$(uname -r | cut -d'-' -f1)
    local min_kernel="4.14"

    if ! version_ge "$kernel_version" "$min_kernel"; then
        log_error "Kernel version too old: $kernel_version (minimum: $min_kernel)"
        exit 1
    fi

    log_success "Kernel check passed: $(uname -r)"
}

# Check if running in a VM (nested virtualization warning)
check_nested_virt() {
    log_info "Checking for nested virtualization..."

    if systemd-detect-virt --quiet; then
        local virt_type=$(systemd-detect-virt)
        log_warn "Running inside a virtual machine: $virt_type"
        log_warn "Nested virtualization must be enabled for Firecracker to work"

        # Check if nested virtualization is enabled
        if [[ -f /sys/module/kvm_intel/parameters/nested ]]; then
            local nested=$(cat /sys/module/kvm_intel/parameters/nested)
            if [[ "$nested" != "Y" && "$nested" != "1" ]]; then
                log_error "Nested virtualization is not enabled"
                log_error "Enable it in your hypervisor settings"
                exit 1
            fi
        fi

        if [[ "$NON_INTERACTIVE" != "true" ]]; then
            if ! confirm "Continue with nested virtualization?"; then
                exit 1
            fi
        fi
    fi
}

# Check systemd
check_systemd() {
    log_info "Checking systemd..."

    if ! command_exists systemctl; then
        log_error "systemd not found (systemctl command missing)"
        log_error "This installer requires systemd"
        exit 1
    fi

    if [[ ! -d /run/systemd/system ]]; then
        log_error "System not running systemd"
        exit 1
    fi

    log_success "systemd check passed"
}

# Check existing installation
check_existing_installation() {
    log_info "Checking for existing installation..."

    if [[ -f "${INSTALL_DIR:-/opt/nqrust-microvm}/bin/manager" ]]; then
        log_warn "Existing installation detected at ${INSTALL_DIR:-/opt/nqrust-microvm}"

        if [[ "$NON_INTERACTIVE" != "true" ]]; then
            echo ""
            echo "Options:"
            echo "  1) Abort (safe option)"
            echo "  2) Backup and reinstall"
            echo "  3) Upgrade existing installation"
            read -p "Choose [1-3]: " choice

            case $choice in
                1) exit 0 ;;
                2)
                    BACKUP_EXISTING=true
                    ;;
                3)
                    UPGRADE_MODE=true
                    ;;
                *)
                    log_error "Invalid choice"
                    exit 1
                    ;;
            esac
        else
            log_warn "Running in non-interactive mode. Will backup existing installation."
            BACKUP_EXISTING=true
        fi
    else
        log_success "No existing installation found"
    fi
}

# Check required commands
check_required_commands() {
    log_info "Checking for required commands..."

    local required_commands=(
        "curl:curl or wget"
        "git:git"
        "sudo:sudo"
        "systemctl:systemd"
        "ip:iproute2"
    )

    local missing=()

    for cmd_info in "${required_commands[@]}"; do
        local cmd="${cmd_info%%:*}"
        local package="${cmd_info##*:}"

        if ! command_exists "$cmd"; then
            missing+=("$package")
        fi
    done

    if [[ ${#missing[@]} -gt 0 ]]; then
        log_error "Missing required commands/packages:"
        for pkg in "${missing[@]}"; do
            log_error "  - $pkg"
        done
        exit 1
    fi

    log_success "All required commands available"
}

# Run all pre-flight checks
run_preflight_checks() {
    log_phase "Pre-flight Checks" ""

    check_architecture
    check_os
    check_kernel
    check_systemd
    check_sudo
    check_kvm_support
    check_ram 2048
    check_disk_space 20
    check_required_commands
    check_ports
    check_conflicts
    check_nested_virt
    check_existing_installation
    check_internet

    log_success "All pre-flight checks passed!"
}

# Export functions
export -f check_os check_kvm_support check_ram check_disk_space
export -f check_ports check_conflicts check_architecture check_kernel
export -f check_nested_virt check_systemd check_existing_installation
export -f check_required_commands run_preflight_checks
