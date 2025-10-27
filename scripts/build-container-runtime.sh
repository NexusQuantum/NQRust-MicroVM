#!/bin/bash
set -e

# Build container-runtime.ext4 for NQRust-MicroVM
# This creates an Alpine Linux image with Docker daemon pre-installed

OUTPUT_IMAGE="/srv/images/container-runtime.ext4"
IMAGE_SIZE_MB=2048
MOUNT_POINT="/tmp/container-runtime-build"

echo "==> Building container-runtime.ext4 image"
echo "    Size: ${IMAGE_SIZE_MB}MB"
echo "    Output: ${OUTPUT_IMAGE}"

# Create empty ext4 image
echo "==> Creating ${IMAGE_SIZE_MB}MB ext4 image..."
dd if=/dev/zero of="${OUTPUT_IMAGE}" bs=1M count=${IMAGE_SIZE_MB}
mkfs.ext4 -F "${OUTPUT_IMAGE}"

# Mount the image
echo "==> Mounting image..."
sudo mkdir -p "${MOUNT_POINT}"
sudo mount -o loop "${OUTPUT_IMAGE}" "${MOUNT_POINT}"

# Ensure cleanup on exit
cleanup() {
    echo "==> Cleaning up..."
    sudo umount "${MOUNT_POINT}" 2>/dev/null || true
    sudo rm -rf "${MOUNT_POINT}"
}
trap cleanup EXIT

# Install Alpine Linux base
echo "==> Installing Alpine Linux base system..."
sudo docker run --rm -v "${MOUNT_POINT}:/target" alpine:3.18 sh -c '
    # Set up basic filesystem structure first
    mkdir -p /target/dev /target/proc /target/sys /target/run /target/tmp
    mkdir -p /target/var/log /target/var/lib/docker
    mkdir -p /target/etc/docker /target/etc/apk /target/etc/network /target/etc/init.d /target/etc/local.d
    mkdir -p /target/etc/runlevels/default /target/etc/runlevels/boot /target/etc/runlevels/sysinit
    mkdir -p /target/lib/apk/db /target/var/cache/apk /target/etc/apk/keys

    # Copy Alpine signing keys from host Alpine container
    cp -a /etc/apk/keys/* /target/etc/apk/keys/

    # Initialize apk database and repositories
    echo "https://dl-cdn.alpinelinux.org/alpine/v3.18/main" > /target/etc/apk/repositories
    echo "https://dl-cdn.alpinelinux.org/alpine/v3.18/community" >> /target/etc/apk/repositories

    # Install packages
    apk add --no-cache --initdb --root /target --repositories-file /target/etc/apk/repositories \
        alpine-base \
        openrc \
        docker \
        docker-openrc \
        util-linux \
        e2fsprogs \
        ca-certificates \
        curl \
        bash

    # Configure Docker daemon to listen on TCP
    cat > /target/etc/docker/daemon.json <<EOF
{
    "hosts": ["unix:///var/run/docker.sock", "tcp://0.0.0.0:2375"],
    "storage-driver": "overlay2",
    "log-driver": "json-file",
    "log-opts": {
        "max-size": "10m",
        "max-file": "3"
    }
}
EOF

    # Configure networking
    cat > /target/etc/network/interfaces <<EOF
auto lo
iface lo inet loopback

auto eth0
iface eth0 inet dhcp
    hostname \$(hostname)
EOF

    # Set hostname
    echo "container-vm" > /target/etc/hostname

    # Create shadow file and set root password
    mkdir -p /target/etc
    touch /target/etc/shadow
    chmod 640 /target/etc/shadow
    # Password hash for "docker" (you should change this)
    echo "root:\$6\$rounds=656000\$YQKGbZbXfJOto.5w\$C0K0qCPLLn.vuw0QP0hR8RJBvDnCn8G7RtBPHHkh2QcJ1LqYjJvTmNhFh6FmfHKBVwfJVzPZJxWZKGCy8bNJe1:19000:0:99999:7:::" > /target/etc/shadow

    # Enable necessary services (create symlinks if init.d services exist)
    [ -f /target/etc/init.d/docker ] && ln -sf /etc/init.d/docker /target/etc/runlevels/default/docker || true
'

# Create init script to start Docker without needing systemd socket
sudo tee "${MOUNT_POINT}/etc/init.d/docker" > /dev/null <<'EOF'
#!/sbin/openrc-run

name="docker"
description="Docker Application Container Engine"

command="/usr/bin/dockerd"
command_args="--host=unix:///var/run/docker.sock --host=tcp://0.0.0.0:2375"
pidfile="/run/docker.pid"
command_background="yes"

depend() {
    need net
    after firewall
}

start_pre() {
    checkpath -d -m 0755 /var/lib/docker
}
EOF

sudo chmod +x "${MOUNT_POINT}/etc/init.d/docker"

# Copy guest-agent if it exists
if [ -f "$(pwd)/target/debug/guest-agent" ]; then
    echo "==> Installing guest-agent..."
    sudo cp "$(pwd)/target/debug/guest-agent" "${MOUNT_POINT}/usr/local/bin/guest-agent"
    sudo chmod +x "${MOUNT_POINT}/usr/local/bin/guest-agent"

    # Create guest-agent init script
    sudo tee "${MOUNT_POINT}/etc/init.d/guest-agent" > /dev/null <<'AGENT_INIT'
#!/sbin/openrc-run

name="guest-agent"
description="NQRust MicroVM Guest Agent"

command="/usr/local/bin/guest-agent"
pidfile="/run/guest-agent.pid"
command_background="yes"

depend() {
    need net
    after networking
}
AGENT_INIT

    sudo chmod +x "${MOUNT_POINT}/etc/init.d/guest-agent"
    sudo ln -sf /etc/init.d/guest-agent "${MOUNT_POINT}/etc/runlevels/default/guest-agent" 2>/dev/null || true
fi

# Create a startup script
sudo tee "${MOUNT_POINT}/etc/local.d/00-init.start" > /dev/null <<'EOF'
#!/bin/sh
# Early initialization script

# Ensure /dev nodes exist
[ -e /dev/null ] || mknod -m 666 /dev/null c 1 3
[ -e /dev/zero ] || mknod -m 666 /dev/zero c 1 5
[ -e /dev/random ] || mknod -m 666 /dev/random c 1 8

# Start Docker daemon
/etc/init.d/docker start

exit 0
EOF

sudo chmod +x "${MOUNT_POINT}/etc/local.d/00-init.start"
sudo ln -sf /etc/init.d/local /etc/runlevels/default/local 2>/dev/null || true

echo "==> Image build complete!"
echo "    Image: ${OUTPUT_IMAGE}"
echo "    Size: $(du -h ${OUTPUT_IMAGE} | cut -f1)"
echo ""
echo "==> To test this image, create a VM with:"
echo "    Kernel: /srv/images/vmlinux-5.10.fc.bin"
echo "    Rootfs: ${OUTPUT_IMAGE}"
echo "    Network: Enabled with DHCP"
echo ""
echo "==> Docker daemon will listen on:"
echo "    - Unix socket: /var/run/docker.sock"
echo "    - TCP: tcp://0.0.0.0:2375"
