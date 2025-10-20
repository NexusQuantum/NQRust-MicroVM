#!/usr/bin/env bash
# Setup NOPASSWD sudo rules for nexus manager
# This allows the manager to mount/modify rootfs without password prompts

set -euo pipefail

SUDO_USER=${SUDO_USER:-$USER}

echo "Setting up NOPASSWD sudo rules for user: $SUDO_USER"
echo "This allows nexus-manager to inject credentials into VM rootfs"
echo ""

# Create sudoers file for nexus
cat <<EOF | sudo tee /etc/sudoers.d/nexus-manager
# Nexus Manager - Allow rootfs mounting and modification without password
# Commands needed for credential and network injection

$SUDO_USER ALL=(ALL) NOPASSWD: /usr/bin/mount -o loop * /tmp/nexus-mount-*
$SUDO_USER ALL=(ALL) NOPASSWD: /usr/bin/umount /tmp/nexus-mount-*
$SUDO_USER ALL=(ALL) NOPASSWD: /usr/bin/cat /tmp/nexus-mount-*/etc/shadow
$SUDO_USER ALL=(ALL) NOPASSWD: /usr/bin/tee /tmp/nexus-mount-*/etc/shadow
$SUDO_USER ALL=(ALL) NOPASSWD: /usr/bin/tee /tmp/nexus-mount-*/etc/network/interfaces
$SUDO_USER ALL=(ALL) NOPASSWD: /usr/bin/tee /tmp/nexus-mount-*/etc/udhcpc/default.script
$SUDO_USER ALL=(ALL) NOPASSWD: /usr/bin/chmod 640 /tmp/nexus-mount-*/etc/shadow
$SUDO_USER ALL=(ALL) NOPASSWD: /usr/bin/chmod +x /tmp/nexus-mount-*/etc/udhcpc/default.script
$SUDO_USER ALL=(ALL) NOPASSWD: /usr/bin/ln -sf /etc/init.d/networking /tmp/nexus-mount-*/etc/runlevels/default/networking
$SUDO_USER ALL=(ALL) NOPASSWD: /usr/bin/mkdir -p /tmp/nexus-mount-*/etc/udhcpc
$SUDO_USER ALL=(ALL) NOPASSWD: /usr/bin/rm -f /tmp/nexus-mount-*/etc/network/if-up.d/firecracker-tap
EOF

# Set correct permissions on sudoers file
sudo chmod 440 /etc/sudoers.d/nexus-manager

# Validate sudoers file
sudo visudo -c

echo ""
echo "✓ NOPASSWD sudo rules configured!"
echo ""
echo "The manager can now:"
echo "  - Mount VM rootfs images"
echo "  - Inject credentials into /etc/shadow"
echo "  - Inject network config into /etc/network/interfaces"
echo "  - Remove broken Firecracker scripts"
echo "  - Create directories and inject udhcpc DHCP configuration script"
echo "  - Inject and enable OpenRC networking service for Alpine"
echo "  - Unmount cleanly"
echo ""
echo "All without password prompts during VM creation."
