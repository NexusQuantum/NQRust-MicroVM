#!/usr/bin/env bash
# NQRust-MicroVM Production Installer
# Main orchestration script

set -euo pipefail

# GitHub repository information
GITHUB_REPO="NexusQuantum/NQRust-MicroVM"
GITHUB_BRANCH="${INSTALLER_BRANCH:-main}"
GITHUB_RAW_BASE="https://raw.githubusercontent.com/${GITHUB_REPO}/${GITHUB_BRANCH}"

# Detect if script is being piped (stdin is not a terminal and no file path)
PIPED_INSTALL=false
if [[ ! -t 0 ]] || [[ "$0" == "bash" ]] || [[ "$0" == "-bash" ]] || [[ "$0" == "/bin/bash" ]]; then
    PIPED_INSTALL=true
fi

# Get script directory or download to temp if piped
if [[ "$PIPED_INSTALL" == "true" ]]; then
    # Create temporary directory for downloaded installer files
    SCRIPT_DIR="$(mktemp -d)"
    CLEANUP_TEMP=true

    echo "Downloading installer files to temporary directory..."

    # Download required library files
    mkdir -p "$SCRIPT_DIR/lib" "$SCRIPT_DIR/systemd" "$SCRIPT_DIR/sudoers.d"

    # Download lib files
    for lib in common.sh preflight.sh deps.sh kvm.sh network.sh database.sh build.sh config.sh sudo.sh services.sh verify.sh; do
        curl -fsSL "${GITHUB_RAW_BASE}/scripts/install/lib/${lib}" -o "$SCRIPT_DIR/lib/${lib}" || {
            echo "Error: Failed to download lib/${lib}"
            rm -rf "$SCRIPT_DIR"
            exit 1
        }
    done

    # Download systemd service files
    for service in nqrust-manager.service nqrust-agent.service nqrust-ui.service; do
        curl -fsSL "${GITHUB_RAW_BASE}/scripts/install/systemd/${service}" -o "$SCRIPT_DIR/systemd/${service}" 2>/dev/null || true
    done

    # Download sudoers file
    curl -fsSL "${GITHUB_RAW_BASE}/scripts/install/sudoers.d/nqrust" -o "$SCRIPT_DIR/sudoers.d/nqrust" 2>/dev/null || true

    echo "Installer files downloaded successfully"

    # Cleanup on exit
    trap "rm -rf '$SCRIPT_DIR'" EXIT INT TERM
else
    # Running from downloaded script
    SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
    CLEANUP_TEMP=false
fi

# Default configuration
INSTALL_MODE="${INSTALL_MODE:-production}"
INSTALL_DIR="${INSTALL_DIR:-/opt/nqrust-microvm}"
DATA_DIR="${DATA_DIR:-/srv/fc}"
CONFIG_DIR="${CONFIG_DIR:-/etc/nqrust-microvm}"
IMAGE_DIR="${IMAGE_DIR:-/srv/images}"

# Components
WITH_UI="${WITH_UI:-true}"
WITH_CONTAINER_RUNTIME="${WITH_CONTAINER_RUNTIME:-false}"

# Network
NETWORK_MODE="${NETWORK_MODE:-nat}"
BRIDGE_NAME="${BRIDGE_NAME:-fcbr0}"

# Database
DB_TYPE="${DB_TYPE:-local}"
DB_HOST="${DB_HOST:-localhost}"
DB_PORT="${DB_PORT:-5432}"
DB_NAME="${DB_NAME:-nexus}"
DB_USER="${DB_USER:-nexus}"
DB_PASSWORD="${DB_PASSWORD:-}"

# Service binds
MANAGER_BIND="${MANAGER_BIND:-0.0.0.0:18080}"
AGENT_BIND="${AGENT_BIND:-0.0.0.0:19090}"
UI_PORT="${UI_PORT:-3000}"

# Options
NON_INTERACTIVE="${NON_INTERACTIVE:-false}"
DEBUG="${DEBUG:-false}"
CONFIG_FILE="${CONFIG_FILE:-}"

# Project root (auto-detect or current directory)
PROJECT_ROOT="${PROJECT_ROOT:-}"

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --mode)
                INSTALL_MODE="$2"
                shift 2
                ;;
            --install-dir)
                INSTALL_DIR="$2"
                shift 2
                ;;
            --data-dir)
                DATA_DIR="$2"
                shift 2
                ;;
            --config-dir)
                CONFIG_DIR="$2"
                shift 2
                ;;
            --network-mode)
                NETWORK_MODE="$2"
                shift 2
                ;;
            --bridge-name)
                BRIDGE_NAME="$2"
                shift 2
                ;;
            --db-host)
                DB_TYPE="remote"
                DB_HOST="$2"
                shift 2
                ;;
            --db-password)
                DB_PASSWORD="$2"
                shift 2
                ;;
            --with-ui)
                WITH_UI=true
                shift
                ;;
            --without-ui)
                WITH_UI=false
                shift
                ;;
            --with-container-runtime)
                WITH_CONTAINER_RUNTIME=true
                shift
                ;;
            --non-interactive)
                NON_INTERACTIVE=true
                shift
                ;;
            --config)
                CONFIG_FILE="$2"
                shift 2
                ;;
            --debug)
                DEBUG=true
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

    # Validate mode
    case "$INSTALL_MODE" in
        production|dev|manager|agent|minimal)
            ;;
        *)
            echo "Invalid mode: $INSTALL_MODE"
            echo "Valid modes: production, dev, manager, agent, minimal"
            exit 1
            ;;
    esac

    # Set install components based on mode
    case "$INSTALL_MODE" in
        production|dev)
            INSTALL_COMPONENTS="manager agent ui"
            ;;
        manager)
            INSTALL_COMPONENTS="manager ui"
            ;;
        agent)
            INSTALL_COMPONENTS="agent"
            ;;
        minimal)
            INSTALL_COMPONENTS="manager agent"
            WITH_UI=false
            ;;
    esac

    export INSTALL_MODE INSTALL_COMPONENTS
}

# Show help
show_help() {
    cat <<EOF
NQRust-MicroVM Installer

Usage: $0 [options]

Installation Modes:
  --mode production        Full production install (default)
  --mode dev               Development install (build from source)
  --mode manager           Manager + UI only (control plane)
  --mode agent             Agent only (worker node)
  --mode minimal           Manager + Agent only (no UI)

Directories:
  --install-dir PATH       Installation directory (default: /opt/nqrust-microvm)
  --data-dir PATH          VM storage directory (default: /srv/fc)
  --config-dir PATH        Configuration directory (default: /etc/nqrust-microvm)

Network:
  --network-mode MODE      Network mode: nat or bridged (default: nat)
  --bridge-name NAME       Bridge interface name (default: fcbr0)

Database:
  --db-host HOST           Remote PostgreSQL host (enables remote mode)
  --db-password PASS       Database password

Components:
  --with-ui                Install UI (default)
  --without-ui             Skip UI installation
  --with-container-runtime Build container runtime support

Options:
  --non-interactive        No prompts, use defaults
  --config FILE            Load settings from config file
  --debug                  Enable debug output
  --help, -h               Show this help

Examples:
  # Interactive installation
  sudo $0

  # Production with bridged networking
  sudo $0 --mode production --network-mode bridged

  # Manager only
  sudo $0 --mode manager

  # Agent only (for worker nodes)
  sudo $0 --mode agent --db-host 192.168.1.100 --db-password secret

  # Development mode
  sudo $0 --mode dev --with-container-runtime

EOF
}

# Load configuration from file
load_config_file() {
    if [[ -n "$CONFIG_FILE" && -f "$CONFIG_FILE" ]]; then
        log_info "Loading configuration from $CONFIG_FILE"
        source "$CONFIG_FILE"
    fi
}

# Auto-detect project root
detect_project_root() {
    if [[ -z "$PROJECT_ROOT" ]]; then
        # Check if we're in the project directory
        if [[ -f "Cargo.toml" ]] && [[ -d "apps" ]]; then
            PROJECT_ROOT="$(pwd)"
        elif [[ -f "../../Cargo.toml" ]] && [[ -d "../../apps" ]]; then
            PROJECT_ROOT="$(cd ../.. && pwd)"
        else
            # For production mode, we don't need project root
            if [[ "$INSTALL_MODE" != "dev" ]]; then
                PROJECT_ROOT="/tmp/nqrust-install"
                mkdir -p "$PROJECT_ROOT"
            else
                log_error "Cannot find project root. Please run from project directory or set PROJECT_ROOT"
                exit 1
            fi
        fi
    fi

    export PROJECT_ROOT
    log_debug "Project root: $PROJECT_ROOT"
}

# Main installation function
main() {
    # Parse arguments
    parse_args "$@"

    # Load common utilities
    source "$SCRIPT_DIR/lib/common.sh"

    # Initialize logging
    init_logging

    # Show banner
    log_banner "NQRust-MicroVM Installer v1.0.0"

    log_info "Installation mode: $INSTALL_MODE"
    log_info "Components: $INSTALL_COMPONENTS"
    echo ""

    # Load configuration file if specified
    load_config_file

    # Detect project root
    detect_project_root

    # Confirm installation
    if [[ "$NON_INTERACTIVE" != "true" ]]; then
        echo "Installation Summary:"
        echo "  Mode: $INSTALL_MODE"
        echo "  Install to: $INSTALL_DIR"
        echo "  Data directory: $DATA_DIR"
        echo "  Network mode: $NETWORK_MODE"
        echo "  Components: $INSTALL_COMPONENTS"
        [[ "$WITH_UI" == "true" ]] && echo "  UI: Yes" || echo "  UI: No"
        echo ""

        if ! confirm "Continue with installation?"; then
            log_info "Installation cancelled"
            exit 0
        fi
    fi

    # Phase 1: Pre-flight checks
    source "$SCRIPT_DIR/lib/preflight.sh"
    run_preflight_checks

    # Phase 2: Install dependencies
    log_phase "2/11" "Installing Dependencies"
    source "$SCRIPT_DIR/lib/deps.sh"
    install_all_dependencies
    show_dependency_versions

    # Phase 3: Setup KVM
    log_phase "3/11" "Configuring KVM"
    source "$SCRIPT_DIR/lib/kvm.sh"
    setup_kvm

    # Phase 4: Setup network
    log_phase "4/11" "Configuring Network"
    source "$SCRIPT_DIR/lib/network.sh"
    detect_network_interface

    if [[ "$NETWORK_MODE" == "nat" ]]; then
        setup_nat_bridge "$BRIDGE_NAME" "$DETECTED_INTERFACE"
    elif [[ "$NETWORK_MODE" == "bridged" ]]; then
        if [[ "$NON_INTERACTIVE" == "true" ]]; then
            log_warn "Bridged mode requires manual confirmation, using NAT mode"
            setup_nat_bridge "$BRIDGE_NAME" "$DETECTED_INTERFACE"
        else
            setup_bridged_network "$BRIDGE_NAME" "$DETECTED_INTERFACE"
        fi
    fi

    verify_network "$BRIDGE_NAME"

    # Phase 5: Setup database
    log_phase "5/11" "Configuring Database"
    source "$SCRIPT_DIR/lib/database.sh"
    setup_postgresql
    show_database_info

    # Phase 6: Build or download binaries
    log_phase "6/11" "Preparing Binaries"
    source "$SCRIPT_DIR/lib/build.sh"

    if [[ "$INSTALL_MODE" == "dev" ]]; then
        build_from_source
        [[ "$WITH_CONTAINER_RUNTIME" == "true" ]] && build_container_runtime
    else
        download_binaries
    fi

    # Phase 7: Install binaries
    log_phase "7/11" "Installing Binaries"
    install_binaries
    install_ui
    show_binary_versions

    # Phase 8: Generate configuration
    source "$SCRIPT_DIR/lib/config.sh"
    configure_system

    # Phase 9: Configure sudo
    log_phase "9/11" "Configuring Sudo"
    source "$SCRIPT_DIR/lib/sudo.sh"
    install_sudo_config
    show_sudo_permissions

    # Phase 10: Install and start services
    source "$SCRIPT_DIR/lib/services.sh"
    setup_services

    # Phase 11: Verify installation
    source "$SCRIPT_DIR/lib/verify.sh"
    if run_all_verifications; then
        INSTALLATION_SUCCESS=true
    else
        INSTALLATION_SUCCESS=false
    fi

    generate_verification_report

    # Show final summary
    show_installation_summary

    if [[ "$INSTALLATION_SUCCESS" == "true" ]]; then
        exit 0
    else
        exit 1
    fi
}

# Show installation summary
show_installation_summary() {
    echo ""
    echo ""
    log_banner "Installation Complete!"
    echo ""

    if [[ "$INSTALLATION_SUCCESS" == "true" ]]; then
        log_success "NQRust-MicroVM has been installed successfully!"
    else
        log_warn "Installation completed with some warnings"
    fi

    echo ""
    log_info "Services:"
    echo "  Manager API:  http://localhost:18080"
    echo "  Manager Docs: http://localhost:18080/swagger-ui"
    echo "  Agent API:    http://localhost:19090"
    if [[ "$WITH_UI" == "true" ]]; then
        echo "  UI Dashboard: http://localhost:3000"
    fi

    echo ""
    log_info "Service Management:"
    echo "  Status:  systemctl status nqrust-manager"
    echo "  Logs:    journalctl -u nqrust-manager -f"
    echo "  Restart: systemctl restart nqrust-manager"

    echo ""
    log_info "Configuration Files:"
    echo "  Manager: /etc/nqrust-microvm/manager.env"
    echo "  Agent:   /etc/nqrust-microvm/agent.env"
    if [[ "$WITH_UI" == "true" ]]; then
        echo "  UI:      /etc/nqrust-microvm/ui.env"
    fi

    echo ""
    log_info "Data Directories:"
    echo "  VMs:     /srv/fc/vms/"
    echo "  Images:  /srv/images/"
    echo "  Logs:    /var/log/nqrust-microvm/"

    echo ""
    log_info "Next Steps:"
    echo "  1. Upload kernel and rootfs images via the UI or API"
    echo "  2. Create your first VM"
    echo "  3. Access VM shell via the web UI"

    echo ""
    log_info "Documentation:"
    echo "  README:  /opt/nqrust-microvm/README.md"
    echo "  Logs:    /var/log/nqrust-install/"

    echo ""
    log_info "Quick Commands:"
    echo "  # List VMs"
    echo "  curl http://localhost:18080/v1/vms"
    echo ""
    echo "  # Check service status"
    echo "  systemctl status nqrust-*"
    echo ""
    echo "  # View logs"
    echo "  journalctl -u nqrust-manager -f"
    echo ""

    if [[ "${NEW_GID_WARNING:-false}" == "true" ]]; then
        echo ""
        log_warn "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        log_warn "  IMPORTANT: You were added to the 'kvm' group"
        log_warn "  Please log out and back in for changes to take effect"
        log_warn "  Or run: newgrp kvm"
        log_warn "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    fi

    echo ""
}

# Run main installation
main "$@"
