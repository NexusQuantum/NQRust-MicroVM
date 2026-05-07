#!/usr/bin/env bash
# In-VM installer for the v0.3.0-alpha.1 release. Run inside the
# iscsi-alpha KubeVirt VM after cloud-init has finished. Idempotent.
#
#   sudo bash iscsi-alpha-install.sh
#
# What it does:
#   1. Fetches the release manifest from GitHub
#   2. Downloads manager + agent + guest-agent + UI artifacts
#   3. Lays them out under /opt/nqrust/ matching the auto-update layout
#   4. Installs systemd units for manager + agent
#   5. Pulls the kernel + alpine rootfs into /srv/images
#   6. Starts the services
#
# Pre-conditions (cloud-init handles these):
#   - open-iscsi, lvm2, qemu-utils, nfs-common installed
#   - postgresql up with nexus/nexus user + db
#   - iscsid + postgresql enabled

set -euo pipefail

VERSION="${VERSION:-v0.3.0-alpha.1}"
REPO="${REPO:-NexusQuantum/NQRust-MicroVM}"
RELEASE_URL="https://github.com/${REPO}/releases/download/${VERSION}"

log() { printf "\n\033[1;36m[install]\033[0m %s\n" "$*"; }

if [[ $EUID -ne 0 ]]; then
  echo "must run as root (sudo)" >&2
  exit 1
fi

log "Installing v0.3.0-alpha.1 from $RELEASE_URL"

mkdir -p /opt/nqrust/bin /srv/images /srv/fc/vms /var/lib/nqrust/nfs /etc/nqrust

# Wait for cloudinit to finish package install before we touch systemd.
if [ ! -f /var/lib/cloud/ready ]; then
  log "Waiting for cloud-init to complete..."
  while [ ! -f /var/lib/cloud/ready ]; do sleep 2; done
fi

# 1. Binaries
download() {
  local name="$1" path="$2"
  log "Downloading $name"
  curl -fsSL --retry 3 -o "$path" "$RELEASE_URL/$name"
  chmod +x "$path"
}

download nqrust-manager-x86_64-linux-musl     /opt/nqrust/bin/manager.${VERSION}
download nqrust-agent-x86_64-linux-musl       /opt/nqrust/bin/agent.${VERSION}
download nqrust-guest-agent-x86_64-linux-musl /opt/nqrust/bin/guest-agent.${VERSION}

ln -sf manager.${VERSION}     /opt/nqrust/bin/manager
ln -sf agent.${VERSION}       /opt/nqrust/bin/agent
ln -sf guest-agent.${VERSION} /opt/nqrust/bin/guest-agent

# 2. UI tarball
log "Downloading UI"
curl -fsSL --retry 3 -o /tmp/nqrust-ui.tar.gz "$RELEASE_URL/nqrust-ui.tar.gz"
mkdir -p /opt/nqrust/ui
tar xzf /tmp/nqrust-ui.tar.gz -C /opt/nqrust/ui --strip-components=1 || tar xzf /tmp/nqrust-ui.tar.gz -C /opt/nqrust/ui

# 3. Base images for VM-create
# Prerelease builds (alpha/beta/rc) don't ship image artifacts since the
# kernel + rootfs haven't changed — fall back to the prior stable release.
IMAGE_FALLBACK="${IMAGE_FALLBACK:-https://github.com/${REPO}/releases/download/v0.2.3}"

fetch_image() {
  local name="$1" dest="$2"
  log "Downloading $name"
  if ! curl -fsSL --retry 3 -o "$dest" "$RELEASE_URL/$name"; then
    log "  $name not in $VERSION (prerelease) — falling back to v0.2.3"
    curl -fsSL --retry 3 -o "$dest" "$IMAGE_FALLBACK/$name"
  fi
}

fetch_image vmlinux-5.10.fc.bin       /srv/images/vmlinux-5.10.fc.bin
fetch_image alpine-3.18-minimal.ext4  /srv/images/alpine-3.18-minimal.ext4

# 4. Firecracker (separate install — the release doesn't bundle it)
if ! command -v firecracker >/dev/null 2>&1; then
  log "Installing Firecracker v1.7.0"
  FC_VER="v1.7.0"
  ARCH=x86_64
  curl -fsSL "https://github.com/firecracker-microvm/firecracker/releases/download/${FC_VER}/firecracker-${FC_VER}-${ARCH}.tgz" \
    | tar -xz -C /tmp
  install -m 0755 /tmp/release-${FC_VER}-${ARCH}/firecracker-${FC_VER}-${ARCH} /usr/local/bin/firecracker
fi

# 5. systemd units
cat > /etc/systemd/system/nqrust-manager.service <<'EOF'
[Unit]
Description=NQRust microVM manager
After=network.target postgresql.service
Wants=postgresql.service

[Service]
Type=simple
Environment=DATABASE_URL=postgres://nexus:nexus@127.0.0.1:5432/nexus
Environment=MANAGER_BIND=0.0.0.0:18080
Environment=MANAGER_IMAGE_ROOT=/srv/images
Environment=MANAGER_STORAGE_ROOT=/srv/fc/vms
Environment=MANAGER_ALLOW_IMAGE_PATHS=true
Environment=LICENSE_DEV_MODE=1
ExecStart=/opt/nqrust/bin/manager
Restart=on-failure
RestartSec=2
RestartForceExitStatus=42

[Install]
WantedBy=multi-user.target
EOF

cat > /etc/systemd/system/nqrust-agent.service <<'EOF'
[Unit]
Description=NQRust microVM agent
After=network.target nqrust-manager.service iscsid.service

[Service]
Type=simple
Environment=AGENT_BIND=127.0.0.1:9090
Environment=MANAGER_BASE=http://127.0.0.1:18080
Environment=FC_RUN_DIR=/srv/fc
Environment=FC_BRIDGE=fcbr0
Environment=AGENT_NFS_MOUNT_BASE=/var/lib/nqrust/nfs
ExecStart=/opt/nqrust/bin/agent
Restart=on-failure
RestartSec=2
RestartForceExitStatus=42

[Install]
WantedBy=multi-user.target
EOF

# 6. Bridge setup (one-time)
if ! ip link show fcbr0 >/dev/null 2>&1; then
  log "Setting up fcbr0 bridge"
  ip link add fcbr0 type bridge
  ip addr add 10.0.0.1/24 dev fcbr0
  ip link set fcbr0 up
  sysctl -w net.ipv4.ip_forward=1
  # NAT out via the VM's primary interface (best-effort detect)
  UPLINK=$(ip -o route get 1.1.1.1 | awk '{print $5; exit}')
  iptables -t nat -A POSTROUTING -o "$UPLINK" -j MASQUERADE
fi

# 7. Start manager + agent
systemctl daemon-reload
systemctl enable --now nqrust-manager.service
sleep 3
systemctl enable --now nqrust-agent.service

log "Done. Manager at http://localhost:18080, agent at http://127.0.0.1:9090"
log "Run the test suite: bash infra/test/iscsi-alpha-runner.sh"
