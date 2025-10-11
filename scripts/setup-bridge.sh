#!/usr/bin/env bash
set -euo pipefail

# Setup bridge for NQRust-MicroVM
BRIDGE=${1:-fcbr0}
UPLINK=${2:-ens18}

echo "Setting up bridge $BRIDGE with uplink $UPLINK"

# Create and bring up bridge
sudo ip link show "$BRIDGE" >/dev/null 2>&1 || sudo ip link add "$BRIDGE" type bridge
sudo ip link set "$BRIDGE" up

# Enable IP forwarding
sudo sysctl -w net.ipv4.ip_forward=1

# Setup NAT
sudo iptables -t nat -C POSTROUTING -o "$UPLINK" -j MASQUERADE 2>/dev/null || \
sudo iptables -t nat -A POSTROUTING -o "$UPLINK" -j MASQUERADE

# Add IP to bridge if not present
if ! ip addr show "$BRIDGE" | grep -q "inet "; then
    sudo ip addr add 172.16.0.1/24 dev "$BRIDGE"
fi

echo "Bridge $BRIDGE ready with IP 172.16.0.1/24; NAT via $UPLINK"
echo "Run this command to complete setup:"
echo "sudo ./scripts/setup-bridge.sh"