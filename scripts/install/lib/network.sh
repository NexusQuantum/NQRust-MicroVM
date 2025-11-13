#!/usr/bin/env bash
# Network setup for NQRust-MicroVM installer
# Configures bridge networking (NAT or Bridged mode)

# Setup NAT bridge (VMs isolated behind host IP)
setup_nat_bridge() {
    local BRIDGE=${1:-fcbr0}
    local UPLINK=${2:-}

    log_info "Setting up NAT bridge: $BRIDGE"

    if [[ -z "$UPLINK" ]]; then
        detect_network_interface
        UPLINK="$DETECTED_INTERFACE"
    fi

    log_info "Uplink interface: $UPLINK"

    # Create bridge if it doesn't exist
    if ! ip link show "$BRIDGE" >/dev/null 2>&1; then
        sudo ip link add "$BRIDGE" type bridge
        log_success "Created bridge $BRIDGE"
    else
        log_debug "Bridge $BRIDGE already exists"
    fi

    # Assign IP address to bridge (10.0.0.1/24)
    if ! ip addr show "$BRIDGE" | grep -q "10.0.0.1/24"; then
        sudo ip addr add 10.0.0.1/24 dev "$BRIDGE" 2>/dev/null || true
        log_success "Configured bridge IP: 10.0.0.1/24"
    fi

    # Bring bridge up
    sudo ip link set "$BRIDGE" up

    # Enable IP forwarding
    sudo sysctl -w net.ipv4.ip_forward=1 >/dev/null
    echo "net.ipv4.ip_forward=1" | sudo tee /etc/sysctl.d/99-nqrust.conf >/dev/null
    log_success "IP forwarding enabled"

    # Setup NAT with iptables
    setup_nat_iptables "$BRIDGE" "$UPLINK"

    # Setup DHCP server for VMs
    setup_dnsmasq_nat "$BRIDGE"

    # Make configuration persistent
    create_netplan_nat "$BRIDGE"

    log_success "NAT bridge $BRIDGE configured successfully"
}

# Setup NAT rules with iptables
setup_nat_iptables() {
    local BRIDGE=$1
    local UPLINK=$2

    log_info "Configuring NAT rules..."

    # Enable masquerading for outbound traffic
    if ! sudo iptables -t nat -C POSTROUTING -o "$UPLINK" -j MASQUERADE 2>/dev/null; then
        sudo iptables -t nat -A POSTROUTING -o "$UPLINK" -j MASQUERADE
    fi

    # Allow forwarding from bridge to uplink
    if ! sudo iptables -C FORWARD -i "$BRIDGE" -o "$UPLINK" -j ACCEPT 2>/dev/null; then
        sudo iptables -A FORWARD -i "$BRIDGE" -o "$UPLINK" -j ACCEPT
    fi

    # Allow established connections back
    if ! sudo iptables -C FORWARD -i "$UPLINK" -o "$BRIDGE" -m state --state RELATED,ESTABLISHED -j ACCEPT 2>/dev/null; then
        sudo iptables -A FORWARD -i "$UPLINK" -o "$BRIDGE" -m state --state RELATED,ESTABLISHED -j ACCEPT
    fi

    # Save iptables rules
    save_iptables_rules

    log_success "NAT rules configured"
}

# Save iptables rules (persistent)
save_iptables_rules() {
    log_info "Saving iptables rules..."

    # Install iptables-persistent if not present
    if ! package_installed iptables-persistent && command_exists apt-get; then
        # Pre-seed debconf to avoid interactive prompts
        echo iptables-persistent iptables-persistent/autosave_v4 boolean true | sudo debconf-set-selections
        echo iptables-persistent iptables-persistent/autosave_v6 boolean true | sudo debconf-set-selections
        DEBIAN_FRONTEND=noninteractive sudo -E apt-get install -y iptables-persistent >/dev/null 2>&1 || true
    fi

    # Save rules
    if [[ -d /etc/iptables ]]; then
        sudo iptables-save | sudo tee /etc/iptables/rules.v4 >/dev/null
    elif [[ -d /etc/sysconfig ]]; then
        sudo iptables-save | sudo tee /etc/sysconfig/iptables >/dev/null
    fi

    log_success "iptables rules saved"
}

# Setup dnsmasq for DHCP (NAT mode)
setup_dnsmasq_nat() {
    local BRIDGE=$1

    log_info "Setting up DHCP server (dnsmasq)..."

    # Install dnsmasq if not present
    if ! command_exists dnsmasq; then
        if command_exists apt-get; then
            sudo apt-get install -y dnsmasq
        elif command_exists yum; then
            sudo yum install -y dnsmasq
        elif command_exists dnf; then
            sudo dnf install -y dnsmasq
        fi
    fi

    # Create dnsmasq config for this bridge
    local dnsmasq_conf="/etc/dnsmasq.d/nqrust-$BRIDGE.conf"

    cat <<EOF | sudo tee "$dnsmasq_conf" >/dev/null
# DHCP server for NQRust-MicroVM bridge $BRIDGE
# VMs will get IPs in the 10.0.0.10-250 range

interface=$BRIDGE
bind-interfaces

# DHCP range
dhcp-range=10.0.0.10,10.0.0.250,255.255.255.0,24h

# Gateway (this host)
dhcp-option=3,10.0.0.1

# DNS servers (Google DNS + Cloudflare)
dhcp-option=6,8.8.8.8,8.8.4.4,1.1.1.1

# Don't read /etc/resolv.conf
no-resolv

# Use these DNS servers for queries
server=8.8.8.8
server=8.8.4.4

# Don't forward plain names
domain-needed

# Log queries (useful for debugging)
# log-queries
# log-dhcp
EOF

    # Restart dnsmasq
    sudo systemctl enable dnsmasq >/dev/null 2>&1
    sudo systemctl restart dnsmasq

    if systemctl is-active --quiet dnsmasq; then
        log_success "DHCP server configured and running"
    else
        log_error "Failed to start dnsmasq"
        log_error "Check: journalctl -u dnsmasq"
        exit 1
    fi
}

# Setup bridged network (VMs visible on network)
setup_bridged_network() {
    local BRIDGE=${1:-fcbr0}
    local PHYSICAL=${2:-}

    if [[ -z "$PHYSICAL" ]]; then
        log_error "Physical interface required for bridged mode"
        echo "Available interfaces:"
        ip link show | grep -E "^[0-9]+:" | cut -d: -f2 | tr -d ' ' | grep -v lo | sed 's/^/  /'
        exit 1
    fi

    log_info "Setting up bridged network: $BRIDGE → $PHYSICAL"
    log_warn "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    log_warn "  WARNING: This will modify network configuration!"
    log_warn "  Ensure you have console access in case SSH breaks"
    log_warn "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    if [[ "$NON_INTERACTIVE" != "true" ]]; then
        if ! confirm "Continue with bridged network setup?"; then
            exit 1
        fi
    fi

    sleep 2

    # Save current network configuration
    local CURRENT_IP=$(ip -4 addr show "$PHYSICAL" | grep "inet " | awk '{print $2}' | head -n1)
    local CURRENT_GW=$(ip route | grep default | grep "$PHYSICAL" | awk '{print $3}' | head -n1)
    local CURRENT_DNS=$(resolvectl dns "$PHYSICAL" 2>/dev/null | awk '{print $2}' | head -n1)

    log_info "Current configuration:"
    log_info "  Interface: $PHYSICAL"
    log_info "  IP: ${CURRENT_IP:-DHCP}"
    log_info "  Gateway: ${CURRENT_GW:-auto}"
    log_info "  DNS: ${CURRENT_DNS:-auto}"

    # Create bridge
    if ! ip link show "$BRIDGE" >/dev/null 2>&1; then
        sudo ip link add "$BRIDGE" type bridge
        log_success "Created bridge $BRIDGE"
    fi

    # Bring bridge up
    sudo ip link set "$BRIDGE" up

    # Check if physical interface is already part of bridge
    local CURRENT_MASTER=$(ip link show "$PHYSICAL" | grep -o "master [^ ]*" | awk '{print $2}' || echo "")

    if [[ "$CURRENT_MASTER" == "$BRIDGE" ]]; then
        log_info "$PHYSICAL already part of $BRIDGE"
    else
        # Add physical interface to bridge
        log_info "Adding $PHYSICAL to bridge..."
        sudo ip link set "$PHYSICAL" master "$BRIDGE"
    fi

    # Move IP address to bridge
    if [[ -n "$CURRENT_IP" ]]; then
        log_info "Moving IP $CURRENT_IP to bridge..."
        sudo ip addr flush dev "$PHYSICAL"
        sudo ip addr add "$CURRENT_IP" dev "$BRIDGE"

        # Move default route
        if [[ -n "$CURRENT_GW" ]]; then
            sudo ip route del default via "$CURRENT_GW" dev "$PHYSICAL" 2>/dev/null || true
            sudo ip route add default via "$CURRENT_GW" dev "$BRIDGE"
        fi
    fi

    # Bring physical interface up (no IP, just part of bridge)
    sudo ip link set "$PHYSICAL" up

    # Enable promiscuous mode (required for bridging)
    sudo ip link set "$PHYSICAL" promisc on

    # Enable IP forwarding
    sudo sysctl -w net.ipv4.ip_forward=1 >/dev/null
    echo "net.ipv4.ip_forward=1" | sudo tee /etc/sysctl.d/99-nqrust.conf >/dev/null

    # Disable bridge netfilter (improves performance)
    sudo sysctl -w net.bridge.bridge-nf-call-iptables=0 2>/dev/null || true
    sudo sysctl -w net.bridge.bridge-nf-call-ip6tables=0 2>/dev/null || true

    echo "net.bridge.bridge-nf-call-iptables=0" | sudo tee -a /etc/sysctl.d/99-nqrust.conf >/dev/null
    echo "net.bridge.bridge-nf-call-ip6tables=0" | sudo tee -a /etc/sysctl.d/99-nqrust.conf >/dev/null

    # Remove NAT rules (not needed in bridged mode)
    sudo iptables -t nat -D POSTROUTING -o "$PHYSICAL" -j MASQUERADE 2>/dev/null || true
    sudo iptables -t nat -D POSTROUTING -o "$BRIDGE" -j MASQUERADE 2>/dev/null || true

    # Configure DNS
    if [[ -n "$CURRENT_DNS" ]]; then
        sudo resolvectl dns "$BRIDGE" "$CURRENT_DNS" 2>/dev/null || true
        sudo resolvectl default-route "$BRIDGE" yes 2>/dev/null || true
    elif [[ -n "$CURRENT_GW" ]]; then
        sudo resolvectl dns "$BRIDGE" "$CURRENT_GW" 2>/dev/null || true
        sudo resolvectl default-route "$BRIDGE" yes 2>/dev/null || true
    fi

    # Make configuration persistent
    create_netplan_bridge "$BRIDGE" "$PHYSICAL" "$CURRENT_IP" "$CURRENT_GW" "$CURRENT_DNS"

    log_success "Bridged network configured"
    log_info "VMs attached to $BRIDGE will appear on the same network"
}

# Create netplan config for NAT mode
create_netplan_nat() {
    local BRIDGE=$1

    if [[ ! -d /etc/netplan ]]; then
        log_debug "Netplan not available, skipping persistent config"
        return
    fi

    log_info "Creating netplan configuration for NAT mode..."

    cat <<EOF | sudo tee /etc/netplan/99-nqrust-nat.yaml >/dev/null
network:
  version: 2
  renderer: networkd
  bridges:
    $BRIDGE:
      dhcp4: no
      dhcp6: no
      addresses:
        - 10.0.0.1/24
EOF

    sudo chmod 600 /etc/netplan/99-nqrust-nat.yaml
    sudo netplan generate || log_warn "netplan generate failed (not critical)"

    log_success "Netplan configuration created"
}

# Create netplan config for bridged mode
create_netplan_bridge() {
    local BRIDGE=$1
    local PHYSICAL=$2
    local IP=$3
    local GW=$4
    local DNS=${5:-8.8.8.8}

    if [[ ! -d /etc/netplan ]]; then
        log_debug "Netplan not available, skipping persistent config"
        return
    fi

    log_info "Creating netplan configuration for bridged mode..."

    cat <<EOF | sudo tee /etc/netplan/99-nqrust-bridge.yaml >/dev/null
network:
  version: 2
  renderer: networkd
  ethernets:
    $PHYSICAL:
      dhcp4: no
      dhcp6: no
  bridges:
    $BRIDGE:
      interfaces:
        - $PHYSICAL
EOF

    # Add IP configuration if static IP
    if [[ -n "$IP" ]]; then
        cat <<EOF | sudo tee -a /etc/netplan/99-nqrust-bridge.yaml >/dev/null
      addresses:
        - $IP
EOF
    else
        cat <<EOF | sudo tee -a /etc/netplan/99-nqrust-bridge.yaml >/dev/null
      dhcp4: yes
EOF
    fi

    # Add routes if gateway specified
    if [[ -n "$GW" ]]; then
        cat <<EOF | sudo tee -a /etc/netplan/99-nqrust-bridge.yaml >/dev/null
      routes:
        - to: default
          via: $GW
EOF
    fi

    # Add DNS
    cat <<EOF | sudo tee -a /etc/netplan/99-nqrust-bridge.yaml >/dev/null
      nameservers:
        addresses:
          - $DNS
          - 8.8.8.8
          - 1.1.1.1
EOF

    sudo chmod 600 /etc/netplan/99-nqrust-bridge.yaml
    sudo netplan generate || log_warn "netplan generate failed (not critical)"

    log_success "Netplan configuration created"
    log_info "To apply on next boot: sudo netplan apply"
}

# Verify network setup
verify_network() {
    local BRIDGE=${1:-fcbr0}

    log_info "Verifying network setup..."

    # Check if bridge exists
    if ! ip link show "$BRIDGE" >/dev/null 2>&1; then
        log_error "Bridge $BRIDGE does not exist"
        return 1
    fi

    # Check if bridge is up
    BRIDGE_STATE=$(ip link show "$BRIDGE" | grep -oP 'state \K\w+' || echo "UNKNOWN")
    if ! ip link show "$BRIDGE" | grep -q "state UP"; then
        log_warn "Bridge $BRIDGE is in state: $BRIDGE_STATE (not UP)"
        log_warn "This is normal for a newly created bridge - it will come up when VMs are attached"
    else
        log_success "Bridge $BRIDGE is up"
    fi

    # Check IP address (NAT mode)
    if ip addr show "$BRIDGE" | grep -q "10.0.0.1"; then
        log_success "Bridge has IP: 10.0.0.1/24 (NAT mode)"
    else
        log_info "Bridge using host IP (Bridged mode)"
    fi

    # Check IP forwarding
    if [[ "$(cat /proc/sys/net/ipv4/ip_forward)" == "1" ]]; then
        log_success "IP forwarding enabled"
    else
        log_error "IP forwarding not enabled"
        return 1
    fi

    # Check dnsmasq (NAT mode)
    if systemctl is-active --quiet dnsmasq; then
        log_success "DHCP server (dnsmasq) running"
    fi

    log_success "Network setup verified"
}

# Export functions
export -f setup_nat_bridge setup_bridged_network
export -f setup_nat_iptables save_iptables_rules
export -f setup_dnsmasq_nat
export -f create_netplan_nat create_netplan_bridge
export -f verify_network
