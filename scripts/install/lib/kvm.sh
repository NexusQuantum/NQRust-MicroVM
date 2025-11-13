#!/usr/bin/env bash
# KVM setup for NQRust-MicroVM installer
# Configures KVM modules, devices, and permissions

# Load KVM modules
load_kvm_modules() {
    log_info "Loading KVM modules..."

    # Try to load KVM module
    if ! lsmod | grep -q "^kvm "; then
        if ! sudo modprobe kvm; then
            log_error "Failed to load KVM module"
            exit 1
        fi
    fi

    # Load appropriate CPU-specific module
    local cpu_vendor=$(grep -m1 vendor_id /proc/cpuinfo | awk '{print $3}')

    case "$cpu_vendor" in
        GenuineIntel)
            if ! lsmod | grep -q "^kvm_intel "; then
                if sudo modprobe kvm_intel; then
                    log_success "Loaded kvm_intel module"
                else
                    log_error "Failed to load kvm_intel module"
                    log_error "Ensure VT-x is enabled in BIOS"
                    exit 1
                fi
            fi
            KVM_MODULE="kvm_intel"
            ;;
        AuthenticAMD)
            if ! lsmod | grep -q "^kvm_amd "; then
                if sudo modprobe kvm_amd; then
                    log_success "Loaded kvm_amd module"
                else
                    log_error "Failed to load kvm_amd module"
                    log_error "Ensure AMD-V is enabled in BIOS"
                    exit 1
                fi
            fi
            KVM_MODULE="kvm_amd"
            ;;
        *)
            log_error "Unknown CPU vendor: $cpu_vendor"
            exit 1
            ;;
    esac

    log_success "KVM modules loaded"
}

# Make KVM modules load on boot
make_kvm_persistent() {
    log_info "Making KVM modules persistent..."

    # Add to /etc/modules
    if ! grep -q "^kvm$" /etc/modules 2>/dev/null; then
        echo "kvm" | sudo tee -a /etc/modules >/dev/null
    fi

    if ! grep -q "^${KVM_MODULE}$" /etc/modules 2>/dev/null; then
        echo "$KVM_MODULE" | sudo tee -a /etc/modules >/dev/null
    fi

    # Create modprobe config for nested virtualization (if in a VM)
    if systemd-detect-virt --quiet; then
        sudo mkdir -p /etc/modprobe.d
        echo "options $KVM_MODULE nested=1" | sudo tee /etc/modprobe.d/nqrust-kvm.conf >/dev/null
        log_success "Enabled nested virtualization support"
    fi

    log_success "KVM modules will load on boot"
}

# Create KVM group if it doesn't exist
create_kvm_group() {
    log_info "Setting up KVM group..."

    if ! getent group kvm >/dev/null; then
        sudo groupadd kvm
        log_success "Created KVM group"
    else
        log_debug "KVM group already exists"
    fi
}

# Add user to KVM group
add_user_to_kvm() {
    local user="${1:-$USER}"

    log_info "Adding user '$user' to KVM group..."

    if id -nG "$user" | grep -qw kvm; then
        log_debug "User $user already in KVM group"
    else
        sudo usermod -a -G kvm "$user"
        log_success "Added $user to KVM group"
        log_warn "You may need to log out and back in for group changes to take effect"

        # Check if we need to warn about current session
        if [[ "$user" == "$USER" ]]; then
            NEW_GID_WARNING=true
        fi
    fi
}

# Set up /dev/kvm permissions
setup_kvm_device() {
    log_info "Configuring /dev/kvm permissions..."

    if [[ ! -e /dev/kvm ]]; then
        log_error "/dev/kvm does not exist"
        log_error "KVM module may not be loaded properly"
        exit 1
    fi

    # Set ownership and permissions
    sudo chown root:kvm /dev/kvm
    sudo chmod 660 /dev/kvm

    log_success "/dev/kvm permissions configured"
}

# Create udev rule for /dev/kvm
create_kvm_udev_rule() {
    log_info "Creating udev rule for /dev/kvm..."

    local udev_rule="/etc/udev/rules.d/99-kvm.rules"

    cat <<'EOF' | sudo tee "$udev_rule" >/dev/null
# KVM device permissions for NQRust-MicroVM
KERNEL=="kvm", GROUP="kvm", MODE="0660"
EOF

    # Reload udev rules
    sudo udevadm control --reload-rules
    sudo udevadm trigger --name-match=kvm

    log_success "udev rule created and loaded"
}

# Verify KVM is working
verify_kvm() {
    log_info "Verifying KVM setup..."

    # Check if /dev/kvm exists and is accessible
    if [[ ! -r /dev/kvm || ! -w /dev/kvm ]]; then
        log_warn "/dev/kvm is not readable/writable in current session"
        log_warn "Current permissions: $(ls -l /dev/kvm)"
        log_warn "Current user groups: $(groups)"
        log_warn "This is expected if the user was just added to the 'kvm' group"
        log_warn "The services will work correctly after logout/login or reboot"
        log_warn "For now, services will run with proper permissions via systemd"
    else
        log_success "KVM device is accessible"
    fi

    # Try to open /dev/kvm
    if sudo -u "$USER" test -r /dev/kvm && sudo -u "$USER" test -w /dev/kvm; then
        log_success "KVM device is accessible"
    else
        log_warn "KVM device may not be accessible to user $USER"
        log_warn "This may require logging out and back in"
    fi

    # Check KVM capabilities
    if [[ -f /sys/module/kvm/parameters/kvmclock_periodic_sync ]]; then
        log_debug "KVM clock sync: $(cat /sys/module/kvm/parameters/kvmclock_periodic_sync)"
    fi

    # Test basic KVM functionality with a simple check
    if command_exists kvm-ok; then
        if sudo kvm-ok; then
            log_success "KVM functionality verified"
        else
            log_warn "kvm-ok reported issues"
        fi
    fi
}

# Main KVM setup function
setup_kvm() {
    log_info "Setting up KVM..."

    load_kvm_modules
    make_kvm_persistent
    create_kvm_group
    add_user_to_kvm "$USER"
    setup_kvm_device
    create_kvm_udev_rule
    verify_kvm

    log_success "KVM setup complete"

    if [[ "${NEW_GID_WARNING:-false}" == "true" ]]; then
        log_warn "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        log_warn "  Group membership updated for user: $USER"
        log_warn "  You may need to:"
        log_warn "    1. Log out and log back in"
        log_warn "    OR"
        log_warn "    2. Run: newgrp kvm"
        log_warn "  for changes to take effect in current session"
        log_warn "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    fi
}

# Export functions
export -f load_kvm_modules make_kvm_persistent
export -f create_kvm_group add_user_to_kvm
export -f setup_kvm_device create_kvm_udev_rule
export -f verify_kvm setup_kvm
