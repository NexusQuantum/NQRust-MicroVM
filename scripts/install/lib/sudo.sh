#!/usr/bin/env bash
# Sudo configuration for NQRust-MicroVM installer
# Sets up passwordless sudo for required commands

# Install sudoers configuration
install_sudo_config() {
    log_info "Configuring sudo permissions..."

    local sudoers_src="$SCRIPT_DIR/sudoers.d/nqrust"
    local sudoers_dest="/etc/sudoers.d/nqrust"

    if [[ ! -f "$sudoers_src" ]]; then
        log_error "Sudoers file not found: $sudoers_src"
        exit 1
    fi

    # Validate syntax before installing
    log_info "Validating sudoers syntax..."
    if ! sudo visudo -c -f "$sudoers_src"; then
        log_error "Sudoers file has invalid syntax!"
        log_error "File: $sudoers_src"
        exit 1
    fi

    # Backup existing file if present
    if [[ -f "$sudoers_dest" ]]; then
        backup_file "$sudoers_dest"
    fi

    # Install sudoers file
    sudo cp "$sudoers_src" "$sudoers_dest"
    sudo chmod 440 "$sudoers_dest"
    sudo chown root:root "$sudoers_dest"

    # Verify installation
    if ! sudo visudo -c -f "$sudoers_dest"; then
        log_error "Installed sudoers file is invalid!"
        restore_backup "$sudoers_dest"
        exit 1
    fi

    log_success "Sudoers configuration installed"
    log_info "  File: $sudoers_dest"
}

# Test sudo configuration
test_sudo_config() {
    log_info "Testing sudo configuration..."

    # Test manager commands
    if sudo -n mount --version >/dev/null 2>&1; then
        log_success "Manager sudo commands accessible"
    else
        log_warn "Manager sudo commands may require password"
    fi

    # Test agent commands (if user in group)
    if id -nG | grep -qw nqrust; then
        if sudo -n systemd-run --version >/dev/null 2>&1; then
            log_success "Agent sudo commands accessible"
        else
            log_warn "Agent sudo commands may require password"
        fi
    fi
}

# Show sudo permissions
show_sudo_permissions() {
    log_info "Configured sudo permissions:"
    echo ""
    echo "Manager (user: nqrust):"
    echo "  • mount/umount - Mount/unmount rootfs"
    echo "  • cp/mv/chmod/chown - Modify files in rootfs"
    echo "  • mkdir/rmdir/ln/rm - Directory operations"
    echo "  • cat /etc/shadow - Read shadow file for password hashing"
    echo "  • tee/dd - Write operations"
    echo ""
    echo "Agent (runs as root):"
    echo "  • firecracker - Run Firecracker VMM"
    echo "  • ip/brctl - Network configuration"
    echo "  • systemd-run - Create systemd scopes"
    echo "  • screen - PTY management"
    echo ""
}

# Export functions
export -f install_sudo_config test_sudo_config show_sudo_permissions
