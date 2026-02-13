#!/usr/bin/env bash
set -euo pipefail
BRIDGE=${1:-fcbr0}
UPLINK=${2:-eth0}

# Create bridge
sudo ip link show "$BRIDGE" >/dev/null 2>&1 || sudo ip link add "$BRIDGE" type bridge

# Assign bridge IP for NAT mode (10.0.0.1/24)
sudo ip addr show "$BRIDGE" 2>/dev/null | grep -q "10.0.0.1/24" || \
  sudo ip addr add 10.0.0.1/24 dev "$BRIDGE"

sudo ip link set "$BRIDGE" up

# Enable IP forwarding + NAT
sudo sysctl -w net.ipv4.ip_forward=1
sudo iptables -t nat -C POSTROUTING -o "$UPLINK" -j MASQUERADE 2>/dev/null || \
  sudo iptables -t nat -A POSTROUTING -o "$UPLINK" -j MASQUERADE

# Setup dnsmasq DHCP if not already running
if ! systemctl is-active --quiet dnsmasq 2>/dev/null; then
    if command -v dnsmasq &>/dev/null; then
        sudo mkdir -p /etc/dnsmasq.d
        cat <<EOF | sudo tee /etc/dnsmasq.d/nqrust-microvm.conf >/dev/null
# NQRust-MicroVM DHCP Configuration
interface=$BRIDGE
bind-dynamic
port=0
dhcp-range=10.0.0.10,10.0.0.250,12h
dhcp-option=option:router,10.0.0.1
dhcp-option=option:dns-server,8.8.8.8,8.8.4.4,1.1.1.1
EOF
        sudo systemctl enable dnsmasq
        sudo systemctl restart dnsmasq
        echo "DHCP server configured (dnsmasq: 10.0.0.10-250)"
    else
        echo "WARNING: dnsmasq not installed — VMs won't get IPs via DHCP"
        echo "  Install: sudo apt-get install dnsmasq"
    fi
else
    echo "DHCP server already running (dnsmasq)"
fi

echo "Bridge $BRIDGE ready; NAT via $UPLINK"
