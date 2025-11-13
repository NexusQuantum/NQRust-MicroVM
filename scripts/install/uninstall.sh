#!/usr/bin/env bash
# NQRust-MicroVM Uninstaller
# Cleanly removes NQRust-MicroVM installation

set -euo pipefail

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# Default paths (from default installation)
INSTALL_DIR="${INSTALL_DIR:-/opt/nqrust-microvm}"
DATA_DIR="${DATA_DIR:-/srv/fc}"
CONFIG_DIR="${CONFIG_DIR:-/etc/nqrust-microvm}"
IMAGE_DIR="${IMAGE_DIR:-/srv/images}"
BRIDGE_NAME="${BRIDGE_NAME:-fcbr0}"

# Options
KEEP_DATA="${KEEP_DATA:-}"
KEEP_DATABASE="${KEEP_DATABASE:-}"
KEEP_CONFIG="${KEEP_CONFIG:-}"
NON_INTERACTIVE="${NON_INTERACTIVE:-false}"
FORCE="${FORCE:-false}"

# Load common utilities
if [[ -f "$SCRIPT_DIR/lib/common.sh" ]]; then
    source "$SCRIPT_DIR/lib/common.sh"
else
    # Minimal fallback if common.sh not available
    log_info() { echo "[INFO] $1"; }
    log_success() { echo "[✓] $1"; }
    log_warn() { echo "[WARN] $1"; }
    log_error() { echo "[ERROR] $1"; }
    confirm() {
        local prompt="${1:-Are you sure?}"
        read -p "$prompt [y/N]: " yn
        case $yn in
            [Yy]* ) return 0;;
            * ) return 1;;
        esac
    }
fi

# Parse arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --keep-data)
                KEEP_DATA=true
                shift
                ;;
            --keep-database)
                KEEP_DATABASE=true
                shift
                ;;
            --keep-config)
                KEEP_CONFIG=true
                shift
                ;;
            --keep-all)
                KEEP_DATA=true
                KEEP_DATABASE=true
                KEEP_CONFIG=true
                shift
                ;;
            --remove-all)
                KEEP_DATA=false
                KEEP_DATABASE=false
                KEEP_CONFIG=false
                shift
                ;;
            --force)
                FORCE=true
                NON_INTERACTIVE=true
                shift
                ;;
            --non-interactive)
                NON_INTERACTIVE=true
                shift
                ;;
            --help|-h)
                show_help
                exit 0
                ;;
            *)
                echo "Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
    done
}

# Show help
show_help() {
    cat <<EOF
NQRust-MicroVM Uninstaller

Usage: $0 [options]

Options:
  --keep-data              Keep VM data (/srv/fc/vms)
  --keep-database          Keep PostgreSQL database
  --keep-config            Keep configuration files
  --keep-all               Keep all data (equivalent to all --keep-* options)
  --remove-all             Remove everything (default if non-interactive)
  --force                  Skip all confirmations
  --non-interactive        No prompts, use defaults
  --help, -h               Show this help

Examples:
  # Interactive uninstall (will ask what to keep)
  sudo $0

  # Remove everything
  sudo $0 --remove-all

  # Keep data and database
  sudo $0 --keep-data --keep-database

  # Force removal of everything
  sudo $0 --force --remove-all

EOF
}

# Interactive prompts
ask_what_to_keep() {
    if [[ "$NON_INTERACTIVE" == "true" ]]; then
        return 0
    fi

    echo ""
    log_warn "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    log_warn "  NQRust-MicroVM Uninstallation"
    log_warn "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""

    # Ask about data
    if [[ -z "$KEEP_DATA" ]]; then
        echo "VM data is stored in: $DATA_DIR/vms/"
        if [[ -d "$DATA_DIR/vms" ]]; then
            local vm_count=$(find "$DATA_DIR/vms" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | wc -l)
            echo "Found $vm_count VM(s)"
        fi
        if confirm "Keep VM data?"; then
            KEEP_DATA=true
        else
            KEEP_DATA=false
        fi
    fi

    # Ask about database
    if [[ -z "$KEEP_DATABASE" ]]; then
        echo ""
        echo "Database contains VM definitions, templates, and configuration"
        if confirm "Keep database?"; then
            KEEP_DATABASE=true
        else
            KEEP_DATABASE=false
        fi
    fi

    # Ask about config
    if [[ -z "$KEEP_CONFIG" ]]; then
        echo ""
        echo "Configuration files contain passwords and settings"
        if confirm "Keep configuration files?"; then
            KEEP_CONFIG=true
        else
            KEEP_CONFIG=false
        fi
    fi

    # Final confirmation
    echo ""
    echo "Uninstall Summary:"
    echo "  Binaries: will be removed"
    echo "  Services: will be removed"
    echo "  VM data: $([ "$KEEP_DATA" == "true" ] && echo "will be kept" || echo "will be DELETED")"
    echo "  Database: $([ "$KEEP_DATABASE" == "true" ] && echo "will be kept" || echo "will be DROPPED")"
    echo "  Config: $([ "$KEEP_CONFIG" == "true" ] && echo "will be kept" || echo "will be removed")"
    echo ""

    if ! confirm "Proceed with uninstallation?"; then
        log_info "Uninstallation cancelled"
        exit 0
    fi
}

# Stop all services
stop_services() {
    log_info "Stopping services..."

    if [[ -f "$SCRIPT_DIR/lib/services.sh" ]]; then
        source "$SCRIPT_DIR/lib/services.sh"
        stop_all_services
    else
        # Fallback manual stop
        local services=("nqrust-ui" "nqrust-manager" "nqrust-agent")
        for service in "${services[@]}"; do
            if systemctl list-unit-files | grep -q "${service}.service"; then
                sudo systemctl stop "${service}.service" 2>/dev/null || true
                log_info "Stopped ${service}"
            fi
        done
    fi

    log_success "Services stopped"
}

# Disable and remove services
remove_services() {
    log_info "Removing services..."

    if [[ -f "$SCRIPT_DIR/lib/services.sh" ]]; then
        source "$SCRIPT_DIR/lib/services.sh"
        disable_all_services
        remove_service_files
    else
        # Fallback manual removal
        local services=("nqrust-ui" "nqrust-manager" "nqrust-agent")
        for service in "${services[@]}"; do
            local service_file="/etc/systemd/system/${service}.service"
            if [[ -f "$service_file" ]]; then
                sudo systemctl disable "${service}.service" 2>/dev/null || true
                sudo rm "$service_file"
                log_info "Removed ${service}.service"
            fi
        done

        sudo systemctl daemon-reload
    fi

    log_success "Services removed"
}

# Remove binaries
remove_binaries() {
    log_info "Removing binaries..."

    if [[ -d "$INSTALL_DIR" ]]; then
        # Backup before removal (just in case)
        if [[ -d "$INSTALL_DIR/bin" ]]; then
            local backup_dir="/tmp/nqrust-uninstall-backup-$(date +%Y%m%d_%H%M%S)"
            mkdir -p "$backup_dir"
            cp -r "$INSTALL_DIR/bin" "$backup_dir/" 2>/dev/null || true
            log_info "Binaries backed up to: $backup_dir"
        fi

        sudo rm -rf "$INSTALL_DIR"
        log_success "Binaries removed"
    else
        log_info "Installation directory not found, skipping"
    fi
}

# Remove configuration
remove_configuration() {
    if [[ "$KEEP_CONFIG" == "true" ]]; then
        log_info "Keeping configuration files (as requested)"
        return 0
    fi

    log_info "Removing configuration..."

    if [[ -d "$CONFIG_DIR" ]]; then
        # Backup configs
        local backup_dir="/tmp/nqrust-config-backup-$(date +%Y%m%d_%H%M%S)"
        mkdir -p "$backup_dir"
        sudo cp -r "$CONFIG_DIR" "$backup_dir/" 2>/dev/null || true
        log_info "Configuration backed up to: $backup_dir"

        sudo rm -rf "$CONFIG_DIR"
        log_success "Configuration removed"
    else
        log_info "Configuration directory not found, skipping"
    fi
}

# Remove VM data
remove_vm_data() {
    if [[ "$KEEP_DATA" == "true" ]]; then
        log_info "Keeping VM data (as requested)"
        return 0
    fi

    log_warn "Removing VM data..."

    if [[ -d "$DATA_DIR/vms" ]]; then
        local vm_count=$(find "$DATA_DIR/vms" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | wc -l)
        log_warn "Deleting $vm_count VM(s)..."

        sudo rm -rf "$DATA_DIR/vms"
        log_success "VM data removed"
    else
        log_info "VM data directory not found, skipping"
    fi

    # Remove empty parent directory
    if [[ -d "$DATA_DIR" ]] && [[ -z "$(ls -A "$DATA_DIR")" ]]; then
        sudo rmdir "$DATA_DIR" 2>/dev/null || true
    fi
}

# Remove images
remove_images() {
    if [[ "$KEEP_DATA" == "true" ]]; then
        log_info "Keeping images (as requested)"
        return 0
    fi

    log_info "Removing images..."

    if [[ -d "$IMAGE_DIR" ]]; then
        local image_count=$(find "$IMAGE_DIR" -type f 2>/dev/null | wc -l)
        log_info "Removing $image_count image(s)..."

        sudo rm -rf "$IMAGE_DIR"
        log_success "Images removed"
    else
        log_info "Image directory not found, skipping"
    fi
}

# Drop database
drop_database() {
    if [[ "$KEEP_DATABASE" == "true" ]]; then
        log_info "Keeping database (as requested)"
        return 0
    fi

    log_warn "Dropping database..."

    if command -v psql >/dev/null 2>&1; then
        # Backup database before dropping
        local backup_file="/tmp/nqrust-db-backup-$(date +%Y%m%d_%H%M%S).sql"
        if sudo -u postgres pg_dump nexus > "$backup_file" 2>/dev/null; then
            log_info "Database backed up to: $backup_file"
        fi

        # Drop database
        if sudo -u postgres psql -c "DROP DATABASE IF EXISTS nexus;" 2>/dev/null; then
            log_success "Database 'nexus' dropped"
        fi

        # Drop user
        if sudo -u postgres psql -c "DROP USER IF EXISTS nexus;" 2>/dev/null; then
            log_success "Database user 'nexus' dropped"
        fi
    else
        log_info "PostgreSQL not found, skipping database removal"
    fi
}

# Remove network bridge
remove_network_bridge() {
    log_info "Removing network bridge..."

    if ip link show "$BRIDGE_NAME" >/dev/null 2>&1; then
        # Remove dnsmasq config
        if [[ -f "/etc/dnsmasq.d/nqrust-$BRIDGE_NAME.conf" ]]; then
            sudo rm "/etc/dnsmasq.d/nqrust-$BRIDGE_NAME.conf"
            sudo systemctl restart dnsmasq 2>/dev/null || true
        fi

        # Remove netplan config
        if [[ -f "/etc/netplan/99-nqrust-nat.yaml" ]]; then
            sudo rm "/etc/netplan/99-nqrust-nat.yaml"
        fi
        if [[ -f "/etc/netplan/99-nqrust-bridge.yaml" ]]; then
            sudo rm "/etc/netplan/99-nqrust-bridge.yaml"
        fi

        # Bring down bridge
        sudo ip link set "$BRIDGE_NAME" down 2>/dev/null || true

        # Delete bridge
        sudo ip link delete "$BRIDGE_NAME" 2>/dev/null || true

        log_success "Network bridge removed"
    else
        log_info "Network bridge not found, skipping"
    fi

    # Remove iptables rules (best effort)
    sudo iptables -t nat -D POSTROUTING -o eth0 -j MASQUERADE 2>/dev/null || true

    # Remove sysctl config
    if [[ -f "/etc/sysctl.d/99-nqrust.conf" ]]; then
        sudo rm "/etc/sysctl.d/99-nqrust.conf"
    fi
}

# Remove sudo configuration
remove_sudo_config() {
    log_info "Removing sudo configuration..."

    if [[ -f "/etc/sudoers.d/nqrust" ]]; then
        sudo rm "/etc/sudoers.d/nqrust"
        log_success "Sudoers file removed"
    else
        log_info "Sudoers file not found, skipping"
    fi
}

# Remove system user
remove_system_user() {
    log_info "Removing system user..."

    if id "nqrust" >/dev/null 2>&1; then
        sudo userdel nqrust 2>/dev/null || true
        log_success "System user removed"
    else
        log_info "System user not found, skipping"
    fi
}

# Remove logs
remove_logs() {
    log_info "Removing logs..."

    if [[ -d "/var/log/nqrust-microvm" ]]; then
        sudo rm -rf "/var/log/nqrust-microvm"
        log_success "Logs removed"
    fi

    if [[ -d "/var/log/nqrust-install" ]]; then
        sudo rm -rf "/var/log/nqrust-install"
        log_success "Install logs removed"
    fi
}

# Main uninstallation
main() {
    parse_args "$@"

    # Check sudo
    if [[ $EUID -ne 0 ]] && ! sudo -n true 2>/dev/null; then
        log_info "This script requires sudo privileges"
        sudo -v
    fi

    # Ask what to keep (if interactive)
    ask_what_to_keep

    echo ""
    log_info "Starting uninstallation..."
    echo ""

    # Uninstall steps
    stop_services
    remove_services
    remove_binaries
    remove_vm_data
    remove_images
    drop_database
    remove_network_bridge
    remove_configuration
    remove_sudo_config
    remove_system_user
    remove_logs

    # Final message
    echo ""
    log_success "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    log_success "  Uninstallation Complete!"
    log_success "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""

    if [[ "$KEEP_DATA" == "true" ]] || [[ "$KEEP_DATABASE" == "true" ]] || [[ "$KEEP_CONFIG" == "true" ]]; then
        log_info "Data preserved:"
        [[ "$KEEP_DATA" == "true" ]] && echo "  VM data: $DATA_DIR/vms/"
        [[ "$KEEP_DATABASE" == "true" ]] && echo "  Database: nexus (PostgreSQL)"
        [[ "$KEEP_CONFIG" == "true" ]] && echo "  Config: $CONFIG_DIR/"
        echo ""
    fi

    log_info "Backups created in /tmp/nqrust-*-backup-*/"
    echo ""
}

# Run uninstallation
main "$@"
