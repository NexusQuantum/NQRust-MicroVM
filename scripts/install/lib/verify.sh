#!/usr/bin/env bash
# Post-installation verification for NQRust-MicroVM
# Validates that everything is installed and working correctly

# Verify binaries are installed
verify_binaries() {
    log_info "Verifying binaries..."

    local bin_dir="${INSTALL_DIR}/bin"
    local errors=0

    # Check manager
    if [[ -f "$bin_dir/manager" ]]; then
        if "$bin_dir/manager" --version >/dev/null 2>&1; then
            log_success "Manager binary OK"
        else
            log_error "Manager binary not working"
            errors=$((errors + 1))
        fi
    else
        log_error "Manager binary not found"
        errors=$((errors + 1))
    fi

    # Check agent
    if [[ -f "$bin_dir/agent" ]]; then
        if "$bin_dir/agent" --version >/dev/null 2>&1; then
            log_success "Agent binary OK"
        else
            log_error "Agent binary not working"
            errors=$((errors + 1))
        fi
    else
        log_error "Agent binary not found"
        errors=$((errors + 1))
    fi

    # Check guest-agent
    if [[ -f "$bin_dir/guest-agent" ]]; then
        if file "$bin_dir/guest-agent" | grep -q "statically linked"; then
            log_success "Guest-agent binary OK (static)"
        else
            log_warn "Guest-agent not statically linked"
        fi
    else
        log_error "Guest-agent binary not found"
        errors=$((errors + 1))
    fi

    return $errors
}

# Verify services are running
verify_services() {
    log_info "Verifying services..."

    local errors=0

    # Check agent
    if systemctl list-unit-files | grep -q "nqrust-agent.service"; then
        if systemctl is-active --quiet nqrust-agent; then
            log_success "Agent service running"
        else
            log_error "Agent service not running"
            log_error "Check logs: journalctl -u nqrust-agent -n 50"
            errors=$((errors + 1))
        fi
    fi

    # Check manager
    if systemctl list-unit-files | grep -q "nqrust-manager.service"; then
        if systemctl is-active --quiet nqrust-manager; then
            log_success "Manager service running"
        else
            log_error "Manager service not running"
            log_error "Check logs: journalctl -u nqrust-manager -n 50"
            errors=$((errors + 1))
        fi
    fi

    # Check UI (optional)
    if systemctl list-unit-files | grep -q "nqrust-ui.service"; then
        if systemctl is-active --quiet nqrust-ui; then
            log_success "UI service running"
        else
            log_warn "UI service not running"
            log_error "Check logs: journalctl -u nqrust-ui -n 50"
        fi
    fi

    return $errors
}

# Verify health endpoints
verify_health_endpoints() {
    log_info "Verifying health endpoints..."

    local errors=0
    local max_attempts=10
    local attempt=0

    # Wait for services to be fully ready
    sleep 3

    # Check manager API
    log_info "Checking manager API..."
    attempt=0
    while [[ $attempt -lt $max_attempts ]]; do
        if curl -sf "http://localhost:18080/health" >/dev/null 2>&1; then
            log_success "Manager API responding"
            break
        fi
        attempt=$((attempt + 1))
        sleep 2
    done

    if [[ $attempt -eq $max_attempts ]]; then
        log_error "Manager API not responding"
        log_error "URL: http://localhost:18080/health"
        errors=$((errors + 1))
    fi

    # Check agent API
    log_info "Checking agent API..."
    attempt=0
    while [[ $attempt -lt $max_attempts ]]; do
        if curl -sf "http://localhost:19090/health" >/dev/null 2>&1; then
            log_success "Agent API responding"
            break
        fi
        attempt=$((attempt + 1))
        sleep 2
    done

    if [[ $attempt -eq $max_attempts ]]; then
        log_error "Agent API not responding"
        log_error "URL: http://localhost:19090/health"
        errors=$((errors + 1))
    fi

    # Check UI (optional)
    if [[ "${WITH_UI:-true}" == "true" ]]; then
        log_info "Checking UI..."
        attempt=0
        while [[ $attempt -lt $max_attempts ]]; do
            if curl -sf "http://localhost:3000" >/dev/null 2>&1; then
                log_success "UI responding"
                break
            fi
            attempt=$((attempt + 1))
            sleep 2
        done

        if [[ $attempt -eq $max_attempts ]]; then
            log_warn "UI not responding (may still be starting)"
            log_info "URL: http://localhost:3000"
        fi
    fi

    return $errors
}

# Verify database connection
verify_database() {
    log_info "Verifying database connection..."

    local db_name="${DB_NAME:-nexus}"
    local db_user="${DB_USER:-nexus}"
    local db_host="${DB_HOST:-localhost}"
    local db_port="${DB_PORT:-5432}"
    local db_password="${DB_PASSWORD}"

    if PGPASSWORD="$db_password" psql -h "$db_host" -p "$db_port" -U "$db_user" -d "$db_name" -c "SELECT 1;" >/dev/null 2>&1; then
        log_success "Database connection OK"
        return 0
    else
        log_error "Database connection failed"
        log_error "Connection: postgresql://$db_user@$db_host:$db_port/$db_name"
        return 1
    fi
}

# Verify network bridge
verify_network_bridge() {
    log_info "Verifying network bridge..."

    local bridge="${BRIDGE_NAME:-fcbr0}"
    local errors=0

    # Check if bridge exists
    if ip link show "$bridge" >/dev/null 2>&1; then
        log_success "Bridge $bridge exists"

        # Check if bridge is up
        if ip link show "$bridge" | grep -q "state UP"; then
            log_success "Bridge $bridge is up"
        else
            log_error "Bridge $bridge is not up"
            errors=$((errors + 1))
        fi

        # Check IP forwarding
        if [[ "$(cat /proc/sys/net/ipv4/ip_forward)" == "1" ]]; then
            log_success "IP forwarding enabled"
        else
            log_error "IP forwarding not enabled"
            errors=$((errors + 1))
        fi

        # Check dnsmasq (NAT mode)
        if systemctl is-active --quiet dnsmasq 2>/dev/null; then
            log_success "DHCP server (dnsmasq) running"
        else
            log_info "DHCP server not running (may be in bridged mode)"
        fi
    else
        log_error "Bridge $bridge not found"
        errors=$((errors + 1))
    fi

    return $errors
}

# Verify KVM setup
verify_kvm() {
    log_info "Verifying KVM setup..."

    local errors=0

    # Check if KVM module is loaded
    if lsmod | grep -q "^kvm "; then
        log_success "KVM module loaded"
    else
        log_error "KVM module not loaded"
        errors=$((errors + 1))
    fi

    # Check /dev/kvm permissions
    if [[ -c /dev/kvm ]]; then
        log_success "/dev/kvm exists"

        local perms=$(stat -c "%a" /dev/kvm)
        if [[ "$perms" == "660" ]] || [[ "$perms" == "666" ]]; then
            log_success "/dev/kvm permissions OK ($perms)"
        else
            log_warn "/dev/kvm permissions: $perms (expected 660)"
        fi
    else
        log_error "/dev/kvm not found"
        errors=$((errors + 1))
    fi

    return $errors
}

# Verify file permissions
verify_permissions() {
    log_info "Verifying file permissions..."

    local errors=0

    # Check data directory
    if [[ -d "${DATA_DIR}" ]]; then
        local owner=$(stat -c "%U:%G" "${DATA_DIR}")
        if [[ "$owner" == "nqrust:nqrust" ]]; then
            log_success "Data directory ownership OK"
        else
            log_warn "Data directory owner: $owner (expected nqrust:nqrust)"
        fi
    else
        log_error "Data directory not found: ${DATA_DIR}"
        errors=$((errors + 1))
    fi

    # Check config files
    if [[ -f "${CONFIG_DIR}/manager.env" ]]; then
        local perms=$(stat -c "%a" "${CONFIG_DIR}/manager.env")
        if [[ "$perms" == "640" ]]; then
            log_success "Config file permissions OK"
        else
            log_warn "Config permissions: $perms (expected 640)"
        fi
    fi

    return $errors
}

# Verify sudo configuration
verify_sudo_config() {
    log_info "Verifying sudo configuration..."

    if [[ -f "/etc/sudoers.d/nqrust" ]]; then
        # Validate syntax
        if sudo visudo -c -f "/etc/sudoers.d/nqrust" >/dev/null 2>&1; then
            log_success "Sudoers file valid"
        else
            log_error "Sudoers file has invalid syntax"
            return 1
        fi

        # Test if nqrust user can run mount
        if sudo -u nqrust sudo -n mount --version >/dev/null 2>&1; then
            log_success "Sudo permissions OK"
        else
            log_warn "Sudo permissions may require password"
        fi
    else
        log_error "Sudoers file not found"
        return 1
    fi

    return 0
}

# Run all verification checks
run_all_verifications() {
    log_phase "11/11" "Verifying Installation"

    local total_errors=0

    verify_binaries || total_errors=$((total_errors + $?))
    verify_kvm || total_errors=$((total_errors + $?))
    verify_network_bridge || total_errors=$((total_errors + $?))
    verify_permissions || total_errors=$((total_errors + $?))
    verify_sudo_config || total_errors=$((total_errors + $?))
    verify_database || total_errors=$((total_errors + $?))
    verify_services || total_errors=$((total_errors + $?))
    verify_health_endpoints || total_errors=$((total_errors + $?))

    echo ""
    if [[ $total_errors -eq 0 ]]; then
        log_success "✓ All verification checks passed!"
        return 0
    else
        log_warn "⚠ $total_errors verification check(s) failed"
        log_warn "Installation may have issues. Check logs above."
        return 1
    fi
}

# Generate verification report
generate_verification_report() {
    local report_file="/var/log/nqrust-install/verification-$(date +%Y%m%d_%H%M%S).txt"

    {
        echo "NQRust-MicroVM Installation Verification Report"
        echo "================================================"
        echo "Date: $(date)"
        echo ""
        echo "System Information:"
        echo "  OS: $(cat /etc/os-release | grep PRETTY_NAME | cut -d'"' -f2)"
        echo "  Kernel: $(uname -r)"
        echo "  Architecture: $(uname -m)"
        echo ""
        echo "Installation Paths:"
        echo "  Install: ${INSTALL_DIR}"
        echo "  Config: ${CONFIG_DIR}"
        echo "  Data: ${DATA_DIR}"
        echo "  Images: ${IMAGE_DIR}"
        echo ""
        echo "Service Status:"
        systemctl status nqrust-agent --no-pager -l || true
        systemctl status nqrust-manager --no-pager -l || true
        systemctl status nqrust-ui --no-pager -l || true
        echo ""
        echo "Network Configuration:"
        ip addr show "${BRIDGE_NAME:-fcbr0}" || true
        echo ""
        echo "Database:"
        PGPASSWORD="${DB_PASSWORD}" psql -h "${DB_HOST:-localhost}" -U "${DB_USER:-nexus}" -d "${DB_NAME:-nexus}" -c "\dt" 2>&1 || true
    } > "$report_file"

    log_info "Verification report saved: $report_file"
}

# Export functions
export -f verify_binaries verify_services verify_health_endpoints
export -f verify_database verify_network_bridge verify_kvm
export -f verify_permissions verify_sudo_config
export -f run_all_verifications generate_verification_report
