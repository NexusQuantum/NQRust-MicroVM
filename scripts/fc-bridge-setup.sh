#!/usr/bin/env bash
set -euo pipefail
BRIDGE=${1:-fcbr0}
UPLINK=${2:-eth0}


sudo ip link show "$BRIDGE" >/dev/null 2>&1 || sudo ip link add "$BRIDGE" type bridge
sudo ip link set "$BRIDGE" up
sudo sysctl -w net.ipv4.ip_forward=1
sudo iptables -t nat -C POSTROUTING -o "$UPLINK" -j MASQUERADE 2>/dev/null || \
sudo iptables -t nat -A POSTROUTING -o "$UPLINK" -j MASQUERADE


echo "Bridge $BRIDGE ready; NAT via $UPLINK"