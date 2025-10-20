#!/usr/bin/env bash
# Setup Firecracker bridge to physical network (bridged mode)
# This allows VMs to get IPs directly from your router/DHCP server
# Usage: sudo ./fc-bridge-physical.sh <bridge-name> <physical-interface>
# Example: sudo ./fc-bridge-physical.sh fcbr0 ens18

set -euo pipefail

BRIDGE=${1:-fcbr0}
PHYSICAL=${2:-}

if [ -z "$PHYSICAL" ]; then
    echo "Usage: $0 <bridge-name> <physical-interface>"
    echo "Example: $0 fcbr0 ens18"
    echo ""
    echo "Available interfaces:"
    ip link show | grep -E "^[0-9]+:" | cut -d: -f2 | grep -v lo | sed 's/^ /  /'
    exit 1
fi

# Check if physical interface exists
if ! ip link show "$PHYSICAL" >/dev/null 2>&1; then
    echo "Error: Physical interface $PHYSICAL not found"
    exit 1
fi

echo "=========================================="
echo "Setting up bridged network"
echo "=========================================="
echo "Bridge: $BRIDGE"
echo "Physical: $PHYSICAL"
echo ""
echo "⚠️  WARNING: This will modify your network configuration!"
echo "⚠️  Make sure you have console access in case SSH breaks"
echo "⚠️  Press Ctrl+C to cancel, or wait 5 seconds to continue..."
echo ""
sleep 5

# Save current IP configuration of physical interface
CURRENT_IP=$(ip -4 addr show "$PHYSICAL" | grep "inet " | awk '{print $2}' | head -n1 || echo "")
CURRENT_GW=$(ip route | grep default | grep "$PHYSICAL" | awk '{print $3}' | head -n1 || echo "")
CURRENT_DNS=$(resolvectl dns "$PHYSICAL" 2>/dev/null | awk '{print $2}' || echo "")

echo "Current configuration:"
echo "  Interface: $PHYSICAL"
echo "  IP: ${CURRENT_IP:-DHCP}"
echo "  Gateway: ${CURRENT_GW:-auto}"
echo "  DNS: ${CURRENT_DNS:-auto}"
echo ""

# Create bridge if it doesn't exist
if ! ip link show "$BRIDGE" >/dev/null 2>&1; then
    echo "Creating bridge $BRIDGE..."
    sudo ip link add "$BRIDGE" type bridge
else
    echo "Bridge $BRIDGE already exists"
fi

# Bring bridge up
sudo ip link set "$BRIDGE" up

# Check if physical interface is already part of the bridge
CURRENT_MASTER=$(ip link show "$PHYSICAL" | grep -o "master [^ ]*" | awk '{print $2}' || echo "")
if [ "$CURRENT_MASTER" = "$BRIDGE" ]; then
    echo "$PHYSICAL is already part of $BRIDGE"
else
    # Add physical interface to bridge
    echo "Adding $PHYSICAL to bridge $BRIDGE..."
    sudo ip link set "$PHYSICAL" master "$BRIDGE"
fi

# If physical interface had an IP, move it to the bridge
if [ -n "$CURRENT_IP" ]; then
    echo "Moving IP $CURRENT_IP from $PHYSICAL to $BRIDGE..."
    sudo ip addr flush dev "$PHYSICAL"
    sudo ip addr add "$CURRENT_IP" dev "$BRIDGE"

    # Move default route to bridge if it existed
    if [ -n "$CURRENT_GW" ]; then
        echo "Moving default route via $CURRENT_GW to $BRIDGE..."
        sudo ip route del default via "$CURRENT_GW" dev "$PHYSICAL" 2>/dev/null || true
        sudo ip route add default via "$CURRENT_GW" dev "$BRIDGE" 2>/dev/null || true
    fi
fi

# Bring physical interface up (no IP, just part of bridge)
sudo ip link set "$PHYSICAL" up

# Enable promiscuous mode on physical interface (required for bridging)
sudo ip link set "$PHYSICAL" promisc on

# Enable IP forwarding (needed for bridge to work properly)
echo "Enabling IP forwarding..."
sudo sysctl -w net.ipv4.ip_forward=1 >/dev/null

# Disable bridge netfilter (improves performance and avoids iptables issues)
sudo sysctl -w net.bridge.bridge-nf-call-iptables=0 2>/dev/null || true
sudo sysctl -w net.bridge.bridge-nf-call-ip6tables=0 2>/dev/null || true

# Remove any NAT rules (we don't need MASQUERADE in bridged mode)
echo "Removing NAT rules..."
sudo iptables -t nat -D POSTROUTING -o "$PHYSICAL" -j MASQUERADE 2>/dev/null || true
sudo iptables -t nat -D POSTROUTING -o "$BRIDGE" -j MASQUERADE 2>/dev/null || true

# Configure DNS for the bridge
if [ -n "$CURRENT_DNS" ]; then
    echo "Configuring DNS for bridge..."
    sudo resolvectl dns "$BRIDGE" "$CURRENT_DNS" 2>/dev/null || true
    sudo resolvectl default-route "$BRIDGE" yes 2>/dev/null || true
elif [ -n "$CURRENT_GW" ]; then
    # If we have a gateway but no DNS, use the gateway as DNS (common in home networks)
    echo "Configuring DNS for bridge (using gateway)..."
    sudo resolvectl dns "$BRIDGE" "$CURRENT_GW" 2>/dev/null || true
    sudo resolvectl default-route "$BRIDGE" yes 2>/dev/null || true
fi

echo ""
echo "=========================================="
echo "✓ Bridge setup complete!"
echo "=========================================="
echo ""
echo "Bridge configuration:"
ip addr show "$BRIDGE" | grep -E "inet |link/" | sed 's/^/  /'
echo ""
echo "Physical interface $PHYSICAL is now part of bridge $BRIDGE"
echo "VMs attached to this bridge will appear on the same network as your host"
echo ""
echo "To make this persistent across reboots, create /etc/netplan/01-fcbridge.yaml:"
echo ""
cat <<EOF
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
      dhcp4: ${CURRENT_IP:+no}
      dhcp6: no
      ${CURRENT_IP:+addresses:}
      ${CURRENT_IP:+  - $CURRENT_IP}
      ${CURRENT_GW:+routes:}
      ${CURRENT_GW:+  - to: default}
      ${CURRENT_GW:+    via: $CURRENT_GW}
      nameservers:
        addresses:
          - ${CURRENT_DNS:-$CURRENT_GW}
          - 8.8.8.8
EOF
echo ""
echo "Then run:"
echo "  sudo chmod 600 /etc/netplan/01-fcbridge.yaml  # Fix permissions"
echo "  sudo netplan try  # Test first (auto-reverts in 120s)"
echo "  # Or if confident: sudo netplan apply"
echo ""
echo "If internet/DNS stops working after setup, run:"
echo "  sudo resolvectl dns $BRIDGE ${CURRENT_DNS:-$CURRENT_GW}"
echo "  sudo resolvectl default-route $BRIDGE yes"
