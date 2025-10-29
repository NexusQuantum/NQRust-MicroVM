#!/bin/bash
set -e

echo "=============================================="
echo "Migrating /srv/images to btrfs"
echo "=============================================="
echo ""
echo "This script will:"
echo "  1. Backup existing images to /srv/images-backup"
echo "  2. Create a 100GB btrfs filesystem"
echo "  3. Mount it at /srv/images with compression"
echo "  4. Restore your images"
echo "  5. Update /etc/fstab for persistence"
echo ""
echo "⚠️  WARNING: This requires sudo and will modify /srv/images"
echo ""

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo "❌ ERROR: This script must be run as root (or with sudo)"
    exit 1
fi

# Check if /srv/images exists
if [ ! -d "/srv/images" ]; then
    echo "✅ /srv/images doesn't exist, creating fresh..."
    mkdir -p /srv/images
else
    # Check if /srv/images is already btrfs
    current_fs=$(df -T /srv/images | tail -1 | awk '{print $2}')
    if [ "$current_fs" = "btrfs" ]; then
        echo "✅ /srv/images is already btrfs!"
        echo "Filesystem info:"
        btrfs filesystem show /srv/images
        exit 0
    fi

    echo "📦 Current filesystem: $current_fs"
    echo ""

    # Backup existing images
    if [ -n "$(ls -A /srv/images)" ]; then
        echo "📂 Backing up existing images to /srv/images-backup..."
        mkdir -p /srv/images-backup
        rsync -av /srv/images/ /srv/images-backup/
        echo "✅ Backup complete"
    else
        echo "✅ /srv/images is empty, no backup needed"
    fi
fi

# Create btrfs image file
BTRFS_IMAGE="/var/lib/nqrust-images.btrfs"
echo ""
echo "🔧 Creating 100GB btrfs image at $BTRFS_IMAGE..."

if [ -f "$BTRFS_IMAGE" ]; then
    echo "⚠️  Image file already exists at $BTRFS_IMAGE"
    read -p "Remove and recreate? (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        # Unmount if mounted
        if mountpoint -q /srv/images; then
            umount /srv/images || echo "Failed to unmount, may not be mounted"
        fi
        rm -f "$BTRFS_IMAGE"
    else
        echo "❌ Aborting"
        exit 1
    fi
fi

# Create sparse file (instant, doesn't use 100GB upfront)
truncate -s 100G "$BTRFS_IMAGE"

# Format as btrfs with label
mkfs.btrfs -L nqrust-images "$BTRFS_IMAGE"

echo "✅ btrfs filesystem created"
echo ""

# Unmount /srv/images if mounted
if mountpoint -q /srv/images; then
    echo "📤 Unmounting /srv/images..."
    umount /srv/images
fi

# Mount with compression
echo "📥 Mounting btrfs at /srv/images with zstd compression..."
mount -o loop,compress=zstd "$BTRFS_IMAGE" /srv/images

# Restore images if backup exists
if [ -d "/srv/images-backup" ] && [ -n "$(ls -A /srv/images-backup)" ]; then
    echo ""
    echo "♻️  Restoring images from backup..."
    rsync -av /srv/images-backup/ /srv/images/
    echo "✅ Images restored"
fi

# Add to /etc/fstab for persistence
echo ""
echo "📝 Adding entry to /etc/fstab for automatic mounting..."

# Remove old entry if exists
sed -i '\|/srv/images|d' /etc/fstab

# Add new entry
echo "$BTRFS_IMAGE /srv/images btrfs loop,compress=zstd 0 0" >> /etc/fstab

echo "✅ Added to /etc/fstab"
echo ""

# Show filesystem info
echo "=============================================="
echo "✅ Migration Complete!"
echo "=============================================="
echo ""
echo "Filesystem info:"
df -h /srv/images
echo ""
btrfs filesystem show /srv/images
echo ""
echo "🎯 /srv/images is now using btrfs with:"
echo "   - Copy-on-write (COW) support"
echo "   - Instant reflink copies via 'cp --reflink=always'"
echo "   - zstd compression enabled"
echo ""
echo "💡 To test reflink speed:"
echo "   time cp --reflink=always /srv/images/vmlinux-5.10.fc.bin /srv/images/test.bin"
echo ""

if [ -d "/srv/images-backup" ]; then
    echo "🗑️  Backup directory still exists at /srv/images-backup"
    echo "   You can remove it once you've verified everything works:"
    echo "   sudo rm -rf /srv/images-backup"
fi
