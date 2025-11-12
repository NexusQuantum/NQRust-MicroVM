#!/bin/bash
# Update guest agent in a VM's rootfs
# Usage: ./update-guest-agent-in-vm.sh <vm-id>

set -e

if [ -z "$1" ]; then
    echo "Usage: $0 <vm-id>"
    echo "Example: $0 f294bb25-a3b7-465e-b74b-02ed786d6bef"
    exit 1
fi

VM_ID=$1
ROOTFS_PATH=$(find /srv/fc/vms/$VM_ID/storage -name "rootfs-*.ext4" | head -1)

if [ -z "$ROOTFS_PATH" ]; then
    echo "Error: Could not find rootfs for VM $VM_ID"
    exit 1
fi

echo "Found rootfs: $ROOTFS_PATH"

# Create mount point
MOUNT_DIR="/tmp/nexus-mount-$VM_ID"
sudo mkdir -p "$MOUNT_DIR"

# Mount rootfs
echo "Mounting rootfs..."
sudo mount -o loop "$ROOTFS_PATH" "$MOUNT_DIR"

# Copy new guest agent
echo "Copying new guest agent..."
sudo cp target/x86_64-unknown-linux-musl/release/guest-agent "$MOUNT_DIR/usr/local/bin/guest-agent"
sudo chmod +x "$MOUNT_DIR/usr/local/bin/guest-agent"

# Verify
echo "Verifying..."
ls -lh "$MOUNT_DIR/usr/local/bin/guest-agent"

# Unmount
echo "Unmounting..."
sudo umount "$MOUNT_DIR"
sudo rmdir "$MOUNT_DIR"

echo "✅ Guest agent updated successfully!"
echo "Now restart the VM to use the new guest agent."
