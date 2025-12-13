#!/bin/bash
# NQRust-MicroVM Full Disk Installation Script (Air-Gapped)
# This script installs a complete system by copying the live environment to disk.
# 
# Usage: install-to-disk.sh <target-disk> [hostname] [root-password]
# Example: install-to-disk.sh /dev/sda nqrust-server mysecurepassword
#
# This is fully air-gapped - no network required!
# The live ISO contains a complete Debian system which is copied to the target disk.
#

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[OK]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Cleanup function for error handling
cleanup_on_error() {
    log_error "Installation failed! Cleaning up..."
    # Unmount in reverse order
    umount -R /mnt/target 2>/dev/null || true
    swapoff -a 2>/dev/null || true
    log_error "Please check the error above and try again."
    exit 1
}

# Set trap for errors
trap cleanup_on_error ERR

# Configuration
TARGET_DISK="${1:-}"
HOSTNAME="${2:-nqrust-node}"
ROOT_PASSWORD="${3:-nqrust}"
TARGET_MOUNT="/mnt/target"
BUNDLE_PATH="${BUNDLE_PATH:-/opt/nqrust-bundle}"

# Validate arguments
if [[ -z "$TARGET_DISK" ]]; then
    echo "Usage: $0 <target-disk> [hostname] [root-password]"
    echo ""
    echo "Available disks:"
    lsblk -d -o NAME,SIZE,MODEL | grep -v "loop\|sr\|fd"
    exit 1
fi

if [[ ! -b "$TARGET_DISK" ]]; then
    log_error "Device $TARGET_DISK does not exist"
    exit 1
fi

# Check if running as root
if [[ $EUID -ne 0 ]]; then
    log_error "This script must be run as root"
    exit 1
fi

# Check for required tools
for tool in parted rsync grub-install; do
    if ! command -v "$tool" &>/dev/null; then
        log_error "Required tool '$tool' not found"
        exit 1
    fi
done

echo ""
echo "========================================="
echo "  NQRust-MicroVM Full Disk Installation"
echo "           (Air-Gapped Mode)"
echo "========================================="
echo ""
echo "Target disk:    $TARGET_DISK"
echo "Hostname:       $HOSTNAME"
echo "Bundle path:    $BUNDLE_PATH"
echo ""
log_warn "WARNING: ALL DATA ON $TARGET_DISK WILL BE DESTROYED!"
echo ""
read -p "Type 'yes' to continue: " CONFIRM
if [[ "$CONFIRM" != "yes" ]]; then
    log_error "Installation cancelled"
    exit 1
fi

# Detect if NVMe disk
IS_NVME=false
if [[ "$TARGET_DISK" == *"nvme"* ]]; then
    IS_NVME=true
    PART_PREFIX="p"
else
    PART_PREFIX=""
fi

# Check if UEFI or BIOS
IS_UEFI=false
if [[ -d /sys/firmware/efi ]]; then
    IS_UEFI=true
    log_info "UEFI mode detected"
else
    log_info "BIOS mode detected"
fi

# ============================================
# Phase 1: Unmount existing partitions
# ============================================
log_info "Unmounting existing partitions..."
umount -R "$TARGET_MOUNT" 2>/dev/null || true
for i in {1..10}; do
    umount "${TARGET_DISK}${PART_PREFIX}${i}" 2>/dev/null || true
done
swapoff -a 2>/dev/null || true

# ============================================
# Phase 2: Partition the disk
# ============================================
log_info "Creating partition table..."
wipefs -a "$TARGET_DISK"
parted -s "$TARGET_DISK" mklabel gpt

if $IS_UEFI; then
    log_info "Creating partitions for UEFI..."
    # EFI System Partition (512MB)
    parted -s "$TARGET_DISK" mkpart "EFI" fat32 1MiB 513MiB
    parted -s "$TARGET_DISK" set 1 esp on
    # Swap (4GB)
    parted -s "$TARGET_DISK" mkpart "swap" linux-swap 513MiB 4609MiB
    # Root (rest)
    parted -s "$TARGET_DISK" mkpart "root" ext4 4609MiB 100%
    
    EFI_PART="${TARGET_DISK}${PART_PREFIX}1"
    SWAP_PART="${TARGET_DISK}${PART_PREFIX}2"
    ROOT_PART="${TARGET_DISK}${PART_PREFIX}3"
else
    log_info "Creating partitions for BIOS..."
    # BIOS boot partition (1MB)
    parted -s "$TARGET_DISK" mkpart "bios" 1MiB 2MiB
    parted -s "$TARGET_DISK" set 1 bios_grub on
    # Swap (4GB)
    parted -s "$TARGET_DISK" mkpart "swap" linux-swap 2MiB 4098MiB
    # Root (rest)
    parted -s "$TARGET_DISK" mkpart "root" ext4 4098MiB 100%
    
    SWAP_PART="${TARGET_DISK}${PART_PREFIX}2"
    ROOT_PART="${TARGET_DISK}${PART_PREFIX}3"
fi

# Wait for partitions to appear
log_info "Waiting for kernel to recognize partitions..."
sleep 2
partprobe "$TARGET_DISK"
sleep 1

# ============================================
# Phase 3: Format partitions
# ============================================
log_info "Formatting partitions..."

if $IS_UEFI; then
    log_info "Formatting EFI partition..."
    mkfs.fat -F32 -n "EFI" "$EFI_PART"
fi

log_info "Formatting swap partition..."
mkswap -L "swap" "$SWAP_PART"

log_info "Formatting root partition..."
mkfs.ext4 -F -L "nqrust-root" "$ROOT_PART"

log_success "Disk partitioned and formatted"

# ============================================
# Phase 4: Mount partitions
# ============================================
log_info "Mounting partitions..."
mkdir -p "$TARGET_MOUNT"
mount "$ROOT_PART" "$TARGET_MOUNT"

if $IS_UEFI; then
    mkdir -p "$TARGET_MOUNT/boot/efi"
    mount "$EFI_PART" "$TARGET_MOUNT/boot/efi"
fi

swapon "$SWAP_PART"
log_success "Partitions mounted"

# ============================================
# Phase 5: Copy live system to target
# ============================================
log_info "Copying live system to target disk (this may take 5-10 minutes)..."

# Exclude live-specific stuff, mount points, and the target itself
rsync -aAX --info=progress2 \
    --exclude='/dev/*' \
    --exclude='/proc/*' \
    --exclude='/sys/*' \
    --exclude='/tmp/*' \
    --exclude='/run/*' \
    --exclude='/mnt/*' \
    --exclude='/media/*' \
    --exclude='/lost+found' \
    --exclude='/live' \
    --exclude='/lib/live' \
    --exclude='/cdrom' \
    --exclude='/var/lib/nqrust-installed' \
    --exclude="$TARGET_MOUNT" \
    / "$TARGET_MOUNT/"

# Check rsync exit status
RSYNC_EXIT=$?
if [[ $RSYNC_EXIT -ne 0 && $RSYNC_EXIT -ne 24 ]]; then
    # Exit code 24 means "some files vanished before they could be transferred" which is OK
    log_error "rsync failed with exit code $RSYNC_EXIT"
    exit 1
fi

# Create essential directories that were excluded
log_info "Creating essential directories..."
mkdir -p "$TARGET_MOUNT/dev"
mkdir -p "$TARGET_MOUNT/proc"
mkdir -p "$TARGET_MOUNT/sys"
mkdir -p "$TARGET_MOUNT/tmp"
mkdir -p "$TARGET_MOUNT/run"
mkdir -p "$TARGET_MOUNT/mnt"
mkdir -p "$TARGET_MOUNT/media"
chmod 1777 "$TARGET_MOUNT/tmp"

log_success "System copied to target disk"

# ============================================
# Phase 6: Mount virtual filesystems
# ============================================
log_info "Mounting virtual filesystems..."
mount --bind /dev "$TARGET_MOUNT/dev"
mount --bind /dev/pts "$TARGET_MOUNT/dev/pts"
mount -t proc proc "$TARGET_MOUNT/proc"
mount -t sysfs sys "$TARGET_MOUNT/sys"

if $IS_UEFI; then
    mkdir -p "$TARGET_MOUNT/sys/firmware/efi/efivars"
    mount --bind /sys/firmware/efi/efivars "$TARGET_MOUNT/sys/firmware/efi/efivars" 2>/dev/null || true
fi

# ============================================
# Phase 7: Configure the system
# ============================================
log_info "Configuring installed system..."

# Generate fstab
log_info "Generating /etc/fstab..."
ROOT_UUID=$(blkid -s UUID -o value "$ROOT_PART")
SWAP_UUID=$(blkid -s UUID -o value "$SWAP_PART")

cat > "$TARGET_MOUNT/etc/fstab" << EOF
# /etc/fstab - generated by NQRust installer
UUID=$ROOT_UUID  /  ext4  errors=remount-ro  0  1
UUID=$SWAP_UUID  none  swap  sw  0  0
EOF

if $IS_UEFI; then
    EFI_UUID=$(blkid -s UUID -o value "$EFI_PART")
    echo "UUID=$EFI_UUID  /boot/efi  vfat  umask=0077  0  1" >> "$TARGET_MOUNT/etc/fstab"
fi

# Set hostname
log_info "Setting hostname..."
echo "$HOSTNAME" > "$TARGET_MOUNT/etc/hostname"
cat > "$TARGET_MOUNT/etc/hosts" << EOF
127.0.0.1  localhost
127.0.1.1  $HOSTNAME

::1  localhost ip6-localhost ip6-loopback
ff02::1  ip6-allnodes
ff02::2  ip6-allrouters
EOF

# Remove live-boot hooks
log_info "Removing live-boot configuration..."
rm -rf "$TARGET_MOUNT/lib/live" 2>/dev/null || true
rm -f "$TARGET_MOUNT/etc/live"* 2>/dev/null || true

# Configure locale
log_info "Configuring locale..."
echo "en_US.UTF-8 UTF-8" > "$TARGET_MOUNT/etc/locale.gen"
chroot "$TARGET_MOUNT" locale-gen 2>/dev/null || true
echo "LANG=en_US.UTF-8" > "$TARGET_MOUNT/etc/default/locale"

# Set timezone
log_info "Setting timezone..."
rm -f "$TARGET_MOUNT/etc/localtime"
chroot "$TARGET_MOUNT" ln -sf /usr/share/zoneinfo/UTC /etc/localtime

# Set root password
log_info "Setting root password..."
echo "root:$ROOT_PASSWORD" | chroot "$TARGET_MOUNT" chpasswd

# Create nqrust user if not exists
log_info "Creating nqrust system user..."
chroot "$TARGET_MOUNT" useradd --system --no-create-home --shell /usr/sbin/nologin nqrust 2>/dev/null || true

# CRITICAL: Ensure systemd is the init system (live ISO uses sysvinit)
log_info "Configuring systemd as init system..."
# Remove sysvinit if present and ensure systemd-sysv is installed
if chroot "$TARGET_MOUNT" dpkg -l | grep -q "sysvinit-core"; then
    log_info "Removing sysvinit, installing systemd..."
    chroot "$TARGET_MOUNT" apt-get remove --purge -y sysvinit-core 2>/dev/null || true
fi
# Ensure systemd-sysv is installed (makes systemd PID 1)
chroot "$TARGET_MOUNT" apt-get install -y --no-install-recommends systemd-sysv 2>/dev/null || {
    log_warn "Could not install systemd-sysv via apt, trying alternative..."
    # If apt doesn't work (no network), try to fix manually
    # Remove sysvinit from being the default
    rm -f "$TARGET_MOUNT/sbin/init"
    ln -sf /lib/systemd/systemd "$TARGET_MOUNT/sbin/init"
}

# Enable systemd services (target system uses systemd, not sysvinit)
log_info "Enabling services..."
chroot "$TARGET_MOUNT" systemctl enable ssh 2>/dev/null || true
chroot "$TARGET_MOUNT" systemctl enable NetworkManager 2>/dev/null || true
chroot "$TARGET_MOUNT" systemctl enable postgresql 2>/dev/null || true

# Disable live-boot auto-login
rm -f "$TARGET_MOUNT/etc/systemd/system/getty@tty1.service.d/autologin.conf" 2>/dev/null || true
rm -rf "$TARGET_MOUNT/etc/systemd/system/getty@tty1.service.d" 2>/dev/null || true

log_success "System configured"

# ============================================
# Phase 8: Install bootloader
# ============================================
log_info "Installing GRUB bootloader..."

if $IS_UEFI; then
    chroot "$TARGET_MOUNT" grub-install --target=x86_64-efi --efi-directory=/boot/efi --bootloader-id=nqrust --recheck "$TARGET_DISK" 2>&1 || {
        log_error "Failed to install GRUB for UEFI"
        log_info "Trying alternative method..."
        chroot "$TARGET_MOUNT" grub-install --target=x86_64-efi --efi-directory=/boot/efi --removable 2>&1 || true
    }
else
    chroot "$TARGET_MOUNT" grub-install --target=i386-pc --recheck "$TARGET_DISK" 2>&1 || {
        log_error "Failed to install GRUB for BIOS"
    }
fi

# Update GRUB config - remove live-boot options
log_info "Updating GRUB configuration..."

# Remove live-boot from default cmdline
sed -i 's/boot=live[[:space:]]*//g' "$TARGET_MOUNT/etc/default/grub" 2>/dev/null || true
sed -i 's/components[[:space:]]*//g' "$TARGET_MOUNT/etc/default/grub" 2>/dev/null || true
sed -i 's/quiet splash//g' "$TARGET_MOUNT/etc/default/grub" 2>/dev/null || true

# Ensure GRUB_CMDLINE_LINUX_DEFAULT is set properly
if ! grep -q "GRUB_CMDLINE_LINUX_DEFAULT" "$TARGET_MOUNT/etc/default/grub"; then
    echo 'GRUB_CMDLINE_LINUX_DEFAULT="quiet"' >> "$TARGET_MOUNT/etc/default/grub"
fi

# Remove live-boot package and hooks if present
log_info "Removing live-boot hooks..."
chroot "$TARGET_MOUNT" apt-get remove --purge -y live-boot live-boot-initramfs-tools 2>/dev/null || true
rm -rf "$TARGET_MOUNT/usr/share/initramfs-tools/scripts/live" 2>/dev/null || true
rm -rf "$TARGET_MOUNT/usr/share/initramfs-tools/hooks/live" 2>/dev/null || true

# Regenerate initramfs without live-boot hooks
log_info "Regenerating initramfs..."
chroot "$TARGET_MOUNT" update-initramfs -u -k all 2>&1 || log_warn "initramfs update returned warning"

# Generate GRUB config
log_info "Generating GRUB menu..."
chroot "$TARGET_MOUNT" update-grub 2>&1

log_success "Bootloader installed"

# ============================================
# Phase 9: Install NQRust components
# ============================================
log_info "Installing NQRust-MicroVM components..."

# Create directories
mkdir -p "$TARGET_MOUNT/opt/nqrust-microvm/bin"
mkdir -p "$TARGET_MOUNT/srv/fc/images"
mkdir -p "$TARGET_MOUNT/srv/fc/kernels"
mkdir -p "$TARGET_MOUNT/etc/nqrust-microvm"
mkdir -p "$TARGET_MOUNT/var/log/nqrust-microvm"

# Copy binaries from bundle (they should already be in the live system)
if [[ -d "$BUNDLE_PATH/bin" ]]; then
    log_info "Copying binaries from bundle..."
    for binary in nqr-manager nqr-agent nqr-guest-agent nqrust-manager nqrust-agent nqrust-guest-agent; do
        if [[ -f "$BUNDLE_PATH/bin/$binary" ]]; then
            cp "$BUNDLE_PATH/bin/$binary" "$TARGET_MOUNT/opt/nqrust-microvm/bin/"
            chmod +x "$TARGET_MOUNT/opt/nqrust-microvm/bin/$binary"
            log_success "Copied $binary"
        fi
    done
    
    # Normalize names - create nqrust-* symlinks pointing to nqr-* binaries
    cd "$TARGET_MOUNT/opt/nqrust-microvm/bin"
    [[ -f nqr-manager && ! -f nqrust-manager ]] && ln -sf nqr-manager nqrust-manager 2>/dev/null || true
    [[ -f nqr-agent && ! -f nqrust-agent ]] && ln -sf nqr-agent nqrust-agent 2>/dev/null || true
    [[ -f nqr-guest-agent && ! -f nqrust-guest-agent ]] && ln -sf nqr-guest-agent nqrust-guest-agent 2>/dev/null || true
    cd - >/dev/null
else
    log_warn "Bundle path $BUNDLE_PATH/bin not found - no binaries to copy"
fi

# Copy images
if [[ -d "$BUNDLE_PATH/images" ]]; then
    log_info "Copying images from bundle..."
    cp -r "$BUNDLE_PATH/images/"* "$TARGET_MOUNT/srv/fc/images/" 2>/dev/null || true
fi

# Copy kernels
if [[ -d "$BUNDLE_PATH/kernels" ]]; then
    log_info "Copying kernels from bundle..."
    cp -r "$BUNDLE_PATH/kernels/"* "$TARGET_MOUNT/srv/fc/kernels/" 2>/dev/null || true
fi

# Copy Docker images
if [[ -d "$BUNDLE_PATH/docker" ]]; then
    log_info "Copying Docker images from bundle..."
    mkdir -p "$TARGET_MOUNT/srv/fc/docker"
    cp -r "$BUNDLE_PATH/docker/"* "$TARGET_MOUNT/srv/fc/docker/" 2>/dev/null || true
fi

# Set ownership
chroot "$TARGET_MOUNT" chown -R nqrust:nqrust /opt/nqrust-microvm /srv/fc /etc/nqrust-microvm /var/log/nqrust-microvm 2>/dev/null || true

log_success "NQRust components installed"

# ============================================
# Phase 10: Create systemd services
# ============================================
log_info "Creating systemd services..."

# Manager service
cat > "$TARGET_MOUNT/etc/systemd/system/nqrust-manager.service" << 'EOF'
[Unit]
Description=NQR-MicroVM Manager Service
After=network-online.target postgresql.service
Wants=network-online.target
Requires=postgresql.service

[Service]
Type=simple
User=nqrust
Group=nqrust
WorkingDirectory=/opt/nqrust-microvm
ExecStart=/opt/nqrust-microvm/bin/nqrust-manager
Restart=on-failure
RestartSec=5s
LimitNOFILE=65536
LimitNPROC=4096

# Environment variables
Environment=DATABASE_URL=postgres://nqrust:nqrust@localhost/nqrust
Environment=RUST_LOG=info
Environment=MANAGER_HOST=0.0.0.0
Environment=MANAGER_PORT=18080

[Install]
WantedBy=multi-user.target
EOF

# Agent service
cat > "$TARGET_MOUNT/etc/systemd/system/nqrust-agent.service" << 'EOF'
[Unit]
Description=NQR-MicroVM Agent Service
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=root
Group=root
WorkingDirectory=/opt/nqrust-microvm
ExecStart=/opt/nqrust-microvm/bin/nqrust-agent
Restart=on-failure
RestartSec=5s
LimitNOFILE=65536
LimitNPROC=8192
AmbientCapabilities=CAP_NET_ADMIN CAP_SYS_ADMIN

[Install]
WantedBy=multi-user.target
EOF

# Enable services
chroot "$TARGET_MOUNT" systemctl enable nqrust-manager.service 2>/dev/null || true
chroot "$TARGET_MOUNT" systemctl enable nqrust-agent.service 2>/dev/null || true

log_success "Systemd services created and enabled"

# ============================================
# Phase 11: Create first-boot configuration script
# ============================================
log_info "Creating first-boot configuration..."

cat > "$TARGET_MOUNT/opt/nqrust-microvm/first-boot.sh" << 'EOF'
#!/bin/bash
# First boot configuration for NQRust-MicroVM

echo "Running NQRust first-boot configuration..."

# Setup PostgreSQL database
echo "Setting up PostgreSQL..."
sudo -u postgres psql -c "CREATE USER nqrust WITH PASSWORD 'nqrust';" 2>/dev/null || true
sudo -u postgres psql -c "CREATE DATABASE nqrust OWNER nqrust;" 2>/dev/null || true
sudo -u postgres psql -c "GRANT ALL PRIVILEGES ON DATABASE nqrust TO nqrust;" 2>/dev/null || true

# Load KVM module
modprobe kvm
modprobe kvm_intel 2>/dev/null || modprobe kvm_amd 2>/dev/null || true

# Create /dev/kvm permissions
chown root:kvm /dev/kvm 2>/dev/null || true
chmod 660 /dev/kvm 2>/dev/null || true

# Setup network bridge
ip link add name fcbr0 type bridge 2>/dev/null || true
ip addr add 10.0.0.1/24 dev fcbr0 2>/dev/null || true
ip link set fcbr0 up 2>/dev/null || true

# Enable IP forwarding
echo 1 > /proc/sys/net/ipv4/ip_forward

# Load bundled Docker images if available
if [[ -d /srv/fc/docker ]] && command -v docker &>/dev/null; then
    for img in /srv/fc/docker/*.tar; do
        [[ -f "$img" ]] && docker load < "$img"
    done
fi

# Disable this script after first run
systemctl disable nqrust-firstboot.service
rm -f /etc/systemd/system/nqrust-firstboot.service
EOF

chmod +x "$TARGET_MOUNT/opt/nqrust-microvm/first-boot.sh"

cat > "$TARGET_MOUNT/etc/systemd/system/nqrust-firstboot.service" << EOF
[Unit]
Description=NQRust-MicroVM First Boot Configuration
After=network-online.target postgresql.service
Wants=network-online.target

[Service]
Type=oneshot
ExecStart=/opt/nqrust-microvm/first-boot.sh
RemainAfterExit=yes

[Install]
WantedBy=multi-user.target
EOF

chroot "$TARGET_MOUNT" systemctl enable nqrust-firstboot.service 2>/dev/null || true

log_success "First-boot configuration created"

# ============================================
# Phase 12: Cleanup
# ============================================
log_info "Cleaning up..."

# Remove live-system marker
rm -f "$TARGET_MOUNT/var/lib/nqrust-installed" 2>/dev/null || true

# Remove installer auto-start from target
rm -f "$TARGET_MOUNT/root/.bash_profile" 2>/dev/null || true
rm -f "$TARGET_MOUNT/root/.profile" 2>/dev/null || true

# Create a simple .bash_profile
cat > "$TARGET_MOUNT/root/.bash_profile" << 'EOF'
# ~/.bash_profile
if [ -f ~/.bashrc ]; then
    . ~/.bashrc
fi
EOF

# Unmount virtual filesystems
sync
umount "$TARGET_MOUNT/sys/firmware/efi/efivars" 2>/dev/null || true
umount "$TARGET_MOUNT/sys" 2>/dev/null || true
umount "$TARGET_MOUNT/proc" 2>/dev/null || true
umount "$TARGET_MOUNT/dev/pts" 2>/dev/null || true
umount "$TARGET_MOUNT/dev" 2>/dev/null || true

if $IS_UEFI; then
    umount "$TARGET_MOUNT/boot/efi" 2>/dev/null || true
fi

umount "$TARGET_MOUNT" 2>/dev/null || true
swapoff "$SWAP_PART" 2>/dev/null || true

log_success "Cleanup complete"

echo ""
echo "========================================="
echo "  Installation Complete!"
echo "========================================="
echo ""
echo "NQRust-MicroVM has been installed to $TARGET_DISK"
echo ""
echo "Next steps:"
echo "  1. Remove the installation media (USB/CD)"
echo "  2. Reboot the system"
echo "  3. Login as root with password: $ROOT_PASSWORD"
echo "  4. The system will automatically configure on first boot"
echo ""
echo "Default ports:"
echo "  - Manager API: 18080"
echo "  - Agent API: 9090"
echo "  - PostgreSQL: 5432"
echo ""
read -p "Press Enter to reboot, or Ctrl+C to exit..."
reboot
