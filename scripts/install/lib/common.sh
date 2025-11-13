#!/usr/bin/env bash
# Common utilities for NQRust-MicroVM installer
# This file is sourced by all other installer scripts

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
CYAN='\033[0;36m'
WHITE='\033[1;37m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}✓${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_debug() {
    if [[ "${DEBUG:-false}" == "true" ]]; then
        echo -e "${CYAN}[DEBUG]${NC} $1"
    fi
}

log_phase() {
    local phase_num=$1
    local phase_name=$2
    echo ""
    echo -e "${BLUE}════════════════════════════════════════════════${NC}"
    echo -e "${WHITE}[$phase_num] $phase_name${NC}"
    echo -e "${BLUE}════════════════════════════════════════════════${NC}"
}

log_banner() {
    local title=$1
    echo ""
    echo -e "${CYAN}╔════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║${NC}  ${WHITE}$title${NC}${CYAN}                          ║${NC}"
    echo -e "${CYAN}╚════════════════════════════════════════════════╝${NC}"
    echo ""
}

# Progress spinner
spinner() {
    local pid=$1
    local message=$2
    local spinstr='⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏'
    while kill -0 $pid 2>/dev/null; do
        for i in $(seq 0 9); do
            echo -ne "\r${CYAN}${spinstr:$i:1}${NC} $message"
            sleep 0.1
        done
    done
    echo -ne "\r"
}

# Confirmation prompt
confirm() {
    local prompt="${1:-Are you sure?}"
    local default="${2:-n}"

    if [[ "$NON_INTERACTIVE" == "true" ]]; then
        [[ "$default" == "y" ]] && return 0 || return 1
    fi

    local yn
    if [[ "$default" == "y" ]]; then
        read -p "$prompt [Y/n]: " yn
        yn=${yn:-y}
    else
        read -p "$prompt [y/N]: " yn
        yn=${yn:-n}
    fi

    case $yn in
        [Yy]* ) return 0;;
        * ) return 1;;
    esac
}

# Error handling
trap_error() {
    local line=$1
    log_error "Installation failed at line $line"
    log_error "Check the log file for details: $LOG_FILE"
    exit 1
}

trap 'trap_error $LINENO' ERR

# Check if running with sudo/root
check_sudo() {
    if [[ $EUID -eq 0 ]]; then
        log_error "This script should not be run as root directly"
        log_error "Please run as a regular user with sudo privileges"
        exit 1
    fi

    if ! sudo -n true 2>/dev/null; then
        log_info "Sudo access required. You may be prompted for your password."
        sudo -v
    fi

    # Keep sudo alive
    (while true; do sudo -n true; sleep 50; done) 2>/dev/null &
    SUDO_KEEPER_PID=$!
}

# Kill sudo keeper on exit
cleanup() {
    if [[ -n "${SUDO_KEEPER_PID:-}" ]]; then
        kill $SUDO_KEEPER_PID 2>/dev/null || true
    fi
}

trap cleanup EXIT

# Check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Check if package is installed (Debian/Ubuntu)
package_installed() {
    dpkg -l "$1" 2>/dev/null | grep -q "^ii"
}

# Get OS information
get_os_info() {
    if [[ -f /etc/os-release ]]; then
        . /etc/os-release
        OS_NAME=$ID
        OS_VERSION=$VERSION_ID
        OS_PRETTY=$PRETTY_NAME
    else
        log_error "Cannot detect OS. /etc/os-release not found"
        exit 1
    fi
}

# Version comparison (returns 0 if v1 >= v2)
version_ge() {
    [ "$(printf '%s\n' "$1" "$2" | sort -V | head -n1)" = "$2" ]
}

# Calculate checksum
calculate_sha256() {
    local file=$1
    sha256sum "$file" | awk '{print $1}'
}

# Verify checksum
verify_checksum() {
    local file=$1
    local expected=$2
    local actual=$(calculate_sha256 "$file")

    if [[ "$actual" == "$expected" ]]; then
        return 0
    else
        log_error "Checksum mismatch!"
        log_error "Expected: $expected"
        log_error "Actual:   $actual"
        return 1
    fi
}

# Download file with progress
download_file() {
    local url=$1
    local output=$2
    local description="${3:-Downloading file}"

    log_info "$description..."

    if command_exists curl; then
        curl -L --progress-bar -o "$output" "$url"
    elif command_exists wget; then
        wget --show-progress -O "$output" "$url"
    else
        log_error "Neither curl nor wget found. Cannot download files."
        exit 1
    fi
}

# Create backup
backup_file() {
    local file=$1
    if [[ -f "$file" ]]; then
        local backup="${file}.backup.$(date +%Y%m%d_%H%M%S)"
        sudo cp "$file" "$backup"
        log_debug "Backed up $file to $backup"
    fi
}

# Restore backup
restore_backup() {
    local file=$1
    local backup=$(ls -t "${file}.backup."* 2>/dev/null | head -1)
    if [[ -f "$backup" ]]; then
        sudo cp "$backup" "$file"
        log_info "Restored $file from backup"
        return 0
    else
        log_warn "No backup found for $file"
        return 1
    fi
}

# Create directory with proper permissions
create_dir() {
    local dir=$1
    local owner="${2:-$USER:$USER}"
    local perms="${3:-755}"

    if [[ ! -d "$dir" ]]; then
        sudo mkdir -p "$dir"
        sudo chown "$owner" "$dir"
        sudo chmod "$perms" "$dir"
        log_debug "Created directory: $dir"
    fi
}

# Generate random password
generate_password() {
    local length="${1:-32}"
    openssl rand -base64 "$length" | tr -d '=/+' | head -c "$length"
}

# Check internet connectivity
check_internet() {
    log_info "Checking internet connectivity..."
    if ! ping -c 1 -W 2 8.8.8.8 >/dev/null 2>&1; then
        log_error "No internet connection detected"
        exit 1
    fi
    log_success "Internet connection OK"
}

# Detect network interface
detect_network_interface() {
    log_info "Detecting network interface..."

    # Try to find the default route interface
    local iface=$(ip route | grep default | head -1 | awk '{print $5}')

    if [[ -z "$iface" ]]; then
        # Fallback: find first non-loopback interface
        iface=$(ip link show | grep -E "^[0-9]+: (eth|en|wl)" | head -1 | cut -d: -f2 | tr -d ' ')
    fi

    if [[ -z "$iface" ]]; then
        log_warn "Could not auto-detect network interface"
        if [[ "$NON_INTERACTIVE" != "true" ]]; then
            echo "Available interfaces:"
            ip link show | grep -E "^[0-9]+:" | cut -d: -f2 | tr -d ' ' | grep -v lo | sed 's/^/  /'
            read -p "Enter interface name: " iface
        else
            log_error "Cannot continue without network interface"
            exit 1
        fi
    fi

    DETECTED_INTERFACE="$iface"
    log_success "Detected interface: $DETECTED_INTERFACE"
}

# Initialize logging
init_logging() {
    LOG_DIR="${LOG_DIR:-/var/log/nqrust-install}"
    sudo mkdir -p "$LOG_DIR"
    LOG_FILE="$LOG_DIR/install-$(date +%Y%m%d_%H%M%S).log"

    # Redirect all output to log file (in addition to stdout/stderr)
    exec > >(tee -a "$LOG_FILE")
    exec 2>&1

    log_debug "Logging initialized: $LOG_FILE"
}

# Export functions and variables
export -f log_info log_success log_warn log_error log_debug log_phase log_banner
export -f spinner confirm command_exists package_installed
export -f get_os_info version_ge
export -f calculate_sha256 verify_checksum
export -f download_file
export -f backup_file restore_backup
export -f create_dir generate_password
export -f check_internet detect_network_interface
