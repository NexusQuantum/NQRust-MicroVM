#!/usr/bin/env bash
# Systemd service management for NQRust-MicroVM installer
# Installs, enables, and starts systemd services

# Install systemd service file
install_service_file() {
    local service_name=$1
    local service_src="${SCRIPT_DIR}/systemd/${service_name}.service"
    local service_dest="/etc/systemd/system/${service_name}.service"

    log_info "Installing ${service_name} service..."

    if [[ ! -f "$service_src" ]]; then
        log_error "Service file not found: $service_src"
        exit 1
    fi

    # Backup existing service if present
    if [[ -f "$service_dest" ]]; then
        backup_file "$service_dest"
    fi

    # Copy service file
    sudo cp "$service_src" "$service_dest"
    sudo chmod 644 "$service_dest"

    log_success "Service file installed: ${service_name}"
}

# Reload systemd daemon
reload_systemd() {
    log_info "Reloading systemd daemon..."
    sudo systemctl daemon-reload
    log_success "Systemd reloaded"
}

# Enable service (start on boot)
enable_service() {
    local service_name=$1

    log_info "Enabling ${service_name}..."

    if sudo systemctl enable "${service_name}.service" 2>/dev/null; then
        log_success "${service_name} enabled"
    else
        log_error "Failed to enable ${service_name}"
        return 1
    fi
}

# Start service
start_service() {
    local service_name=$1

    log_info "Starting ${service_name}..."

    if sudo systemctl start "${service_name}.service" 2>&1 | tee /tmp/${service_name}-start.log; then
        sleep 2  # Give service time to start

        if sudo systemctl is-active --quiet "${service_name}.service"; then
            log_success "${service_name} started successfully"
            return 0
        else
            log_error "${service_name} failed to start"
            log_error "Check logs: journalctl -u ${service_name} -n 50"
            cat /tmp/${service_name}-start.log
            return 1
        fi
    else
        log_error "Failed to start ${service_name}"
        log_error "Check logs: journalctl -u ${service_name} -n 50"
        return 1
    fi
}

# Stop service
stop_service() {
    local service_name=$1

    log_info "Stopping ${service_name}..."

    if sudo systemctl stop "${service_name}.service" 2>/dev/null; then
        log_success "${service_name} stopped"
    else
        log_warn "Service ${service_name} was not running"
    fi
}

# Check service status
check_service_status() {
    local service_name=$1

    if sudo systemctl is-active --quiet "${service_name}.service"; then
        echo "active"
        return 0
    elif sudo systemctl is-enabled --quiet "${service_name}.service"; then
        echo "enabled"
        return 0
    else
        echo "inactive"
        return 1
    fi
}

# Install manager service
install_manager_service() {
    log_info "Installing manager service..."

    install_service_file "nqrust-manager"
    reload_systemd
    enable_service "nqrust-manager"

    log_success "Manager service installed"
}

# Install agent service
install_agent_service() {
    log_info "Installing agent service..."

    install_service_file "nqrust-agent"
    reload_systemd
    enable_service "nqrust-agent"

    log_success "Agent service installed"
}

# Install UI service
install_ui_service() {
    if [[ "${WITH_UI:-true}" != "true" ]]; then
        log_info "Skipping UI service installation"
        return 0
    fi

    log_info "Installing UI service..."

    install_service_file "nqrust-ui"
    reload_systemd
    enable_service "nqrust-ui"

    log_success "UI service installed"
}

# Install all services
install_all_services() {
    log_info "Installing systemd services..."

    # Determine which services to install
    local install_manager=false
    local install_agent=false
    local install_ui=false

    case "${INSTALL_MODE:-production}" in
        production)
            install_manager=true
            install_agent=true
            install_ui=true
            ;;
        dev)
            install_manager=true
            install_agent=true
            install_ui=true
            ;;
        manager)
            install_manager=true
            install_ui=true
            ;;
        agent)
            install_agent=true
            ;;
        minimal)
            install_manager=true
            install_agent=true
            ;;
    esac

    # Install services
    if [[ "$install_agent" == "true" ]]; then
        install_agent_service
    fi

    if [[ "$install_manager" == "true" ]]; then
        install_manager_service
    fi

    if [[ "$install_ui" == "true" && "${WITH_UI:-true}" == "true" ]]; then
        install_ui_service
    fi

    log_success "All services installed"
}

# Start all services
start_all_services() {
    log_info "Starting services..."

    local services_to_start=()

    # Agent must start before manager
    if systemctl list-unit-files | grep -q "nqrust-agent.service"; then
        services_to_start+=("nqrust-agent")
    fi

    if systemctl list-unit-files | grep -q "nqrust-manager.service"; then
        services_to_start+=("nqrust-manager")
    fi

    if systemctl list-unit-files | grep -q "nqrust-ui.service"; then
        services_to_start+=("nqrust-ui")
    fi

    # Start services in order
    for service in "${services_to_start[@]}"; do
        if ! start_service "$service"; then
            log_error "Failed to start $service"
            log_error "Check logs with: journalctl -u $service -n 50"
            return 1
        fi
    done

    log_success "All services started"
}

# Show service status
show_service_status() {
    echo ""
    log_info "Service Status:"
    echo ""

    local services=(
        "nqrust-agent"
        "nqrust-manager"
        "nqrust-ui"
    )

    for service in "${services[@]}"; do
        if systemctl list-unit-files | grep -q "${service}.service"; then
            local status=$(systemctl is-active "${service}.service" 2>/dev/null || echo "inactive")
            local enabled=$(systemctl is-enabled "${service}.service" 2>/dev/null || echo "disabled")

            if [[ "$status" == "active" ]]; then
                echo "  ✓ ${service}: ${status} (${enabled})"
            else
                echo "  ✗ ${service}: ${status} (${enabled})"
            fi
        fi
    done

    echo ""
}

# Get service logs
show_service_logs() {
    local service_name=$1
    local lines=${2:-50}

    echo ""
    log_info "Recent logs for ${service_name}:"
    echo ""

    sudo journalctl -u "${service_name}.service" -n "$lines" --no-pager

    echo ""
}

# Stop all services
stop_all_services() {
    log_info "Stopping all services..."

    local services=(
        "nqrust-ui"
        "nqrust-manager"
        "nqrust-agent"
    )

    for service in "${services[@]}"; do
        if systemctl list-unit-files | grep -q "${service}.service"; then
            stop_service "$service"
        fi
    done

    log_success "All services stopped"
}

# Disable all services
disable_all_services() {
    log_info "Disabling all services..."

    local services=(
        "nqrust-ui"
        "nqrust-manager"
        "nqrust-agent"
    )

    for service in "${services[@]}"; do
        if systemctl list-unit-files | grep -q "${service}.service"; then
            sudo systemctl disable "${service}.service" 2>/dev/null || true
            log_info "${service} disabled"
        fi
    done

    log_success "All services disabled"
}

# Remove service files
remove_service_files() {
    log_info "Removing service files..."

    local services=(
        "nqrust-ui"
        "nqrust-manager"
        "nqrust-agent"
    )

    for service in "${services[@]}"; do
        local service_file="/etc/systemd/system/${service}.service"
        if [[ -f "$service_file" ]]; then
            sudo rm "$service_file"
            log_info "Removed ${service}.service"
        fi
    done

    reload_systemd

    log_success "Service files removed"
}

# Main service installation function
setup_services() {
    log_phase "10/11" "Installing Services"

    install_all_services
    start_all_services
    show_service_status

    log_success "Services configured and started"
}

# Export functions
export -f install_service_file reload_systemd
export -f enable_service start_service stop_service check_service_status
export -f install_manager_service install_agent_service install_ui_service
export -f install_all_services start_all_services
export -f show_service_status show_service_logs
export -f stop_all_services disable_all_services remove_service_files
export -f setup_services
