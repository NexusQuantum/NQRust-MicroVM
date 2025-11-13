# Installer Testing Guide

This guide explains how to safely test the NQRust-MicroVM installer in isolated environments before releasing it to users.

## Table of Contents

- [Why Test in Isolation?](#why-test-in-isolation)
- [Testing Environment Options](#testing-environment-options)
- [Quick Start: Multipass (Recommended)](#quick-start-multipass-recommended)
- [Option 1: Multipass VMs](#option-1-multipass-vms)
- [Option 2: VirtualBox](#option-2-virtualbox)
- [Option 3: Docker (Limited)](#option-3-docker-limited)
- [Option 4: Cloud VMs](#option-4-cloud-vms)
- [Option 5: LXD Containers](#option-5-lxd-containers)
- [Option 6: Vagrant](#option-6-vagrant)
- [Testing Checklist](#testing-checklist)
- [Common Issues and Solutions](#common-issues-and-solutions)
- [Automated Testing](#automated-testing)

## Why Test in Isolation?

Testing the installer in a separate environment prevents:

- ❌ Breaking your development machine
- ❌ Conflicting with existing installations
- ❌ Polluting system directories
- ❌ Requiring full system reinstall if something goes wrong
- ❌ Accidentally affecting production VMs

Benefits of isolated testing:

- ✅ Safe experimentation
- ✅ Easy reset/rollback
- ✅ Test on clean systems
- ✅ Verify installation on multiple OSes
- ✅ Test upgrade paths

## Testing Environment Options

| Method | Pros | Cons | Best For | KVM Support |
|--------|------|------|----------|-------------|
| **Multipass** | Fast, easy reset | Ubuntu only, limited nested virt | Installer script testing | ⚠️ Limited |
| **VirtualBox** | GUI, snapshots, all OSes | Setup required | Full functional testing | ✅ Yes (with config) |
| **Docker** | Very fast | No systemd, no KVM | Script syntax only | ❌ No |
| **Cloud VMs** | Real environment, multiple OSes | Costs money | Pre-release verification | ✅ Yes (some) |
| **LXD** | Fast, native performance | Linux only, complex | Advanced testing | ✅ Yes |
| **Vagrant** | Reproducible, scriptable | Setup complexity | CI/CD | ✅ Yes (with config) |
| **Bare Metal** | Full functionality | Need physical machine | Development | ✅ Yes |

### What Each Method Can Test

**Multipass** (good for quick iteration):
- ✅ Installer script logic
- ✅ Dependency installation
- ✅ Service configuration
- ✅ Database setup
- ✅ Network bridge setup
- ⚠️ KVM/Firecracker (limited - may not work)

**VirtualBox/Cloud VMs** (recommended for complete testing):
- ✅ Everything Multipass can test
- ✅ Full KVM/Firecracker functionality
- ✅ Actual VM creation and management
- ✅ Guest agent deployment
- ✅ Complete end-to-end workflows

## Quick Start: Multipass (Recommended)

**Multipass** is Ubuntu's lightweight VM manager - perfect for quick installer testing.

**⚠️ Nested Virtualization Required**: Multipass VMs require nested virtualization to be enabled on your host machine for Firecracker to work.

**Quick Setup:**
```bash
# Enable nested virtualization on your host (one-time setup)
bash scripts/enable-nested-virt.sh

# Then test the installer
bash scripts/test-installer-multipass.sh
```

The `enable-nested-virt.sh` script will:
- Detect your CPU type (Intel/AMD)
- Check if nested virt is already enabled
- Configure KVM to allow nested virtualization
- Optionally reload modules or prompt for reboot
- Verify nested virt works inside Multipass VMs

**Alternative if nested virt doesn't work:**
- **VirtualBox** with nested virtualization enabled
- **Bare metal** Linux host
- **Cloud VMs** with nested virt support (AWS bare metal, GCP with nested virt)

### Install Multipass

```bash
# Ubuntu/Debian
sudo snap install multipass

# macOS
brew install multipass

# Windows
# Download from: https://multipass.run/
```

### Test Installer in 2 Minutes

```bash
# Launch Ubuntu VM with nested virtualization
multipass launch 22.04 --name test-installer --cpus 2 --memory 4G --disk 20G

# Enable nested virtualization (IMPORTANT!)
multipass stop test-installer
multipass set local.test-installer.cpus=2
multipass set local.test-installer.memory=4G

# Restart VM
multipass start test-installer

# Copy installer script
multipass exec test-installer -- bash -c "curl -fsSL https://raw.githubusercontent.com/NexusQuantum/NQRust-MicroVM/main/scripts/install/install.sh -o /tmp/install.sh"

# OR copy from local (for testing unreleased changes)
multipass transfer -r scripts/install test-installer:/tmp/

# Run installer (note: don't use sudo, the script will prompt for it)
multipass exec test-installer -- bash /tmp/install/install.sh --mode production --non-interactive

# Check installation
multipass exec test-installer -- systemctl status nqrust-manager
multipass exec test-installer -- curl http://localhost:18080/health

# Access the VM
multipass shell test-installer

# When done, delete VM
multipass delete test-installer --purge
```

### Testing Local Changes (Before Commit)

```bash
# Mount your project directory
multipass mount $(pwd) test-installer:/workspace

# In the VM
multipass shell test-installer
cd /workspace
sudo bash scripts/install/install.sh --mode dev

# Test your changes...

# Unmount when done
multipass umount test-installer
```

## Option 1: Multipass VMs

### Detailed Testing Workflow

#### 1. Create Test VM

```bash
#!/bin/bash
# create-test-vm.sh

VM_NAME="nqrust-test"
OS_VERSION="22.04"  # or 24.04, 20.04

# Create VM
multipass launch $OS_VERSION \
  --name $VM_NAME \
  --cpus 2 \
  --memory 4096M \
  --disk 20G

# Enable nested virtualization for KVM
multipass exec $VM_NAME -- sudo modprobe kvm_intel nested=1
# OR for AMD: sudo modprobe kvm_amd nested=1

echo "VM created: $VM_NAME"
echo "Access with: multipass shell $VM_NAME"
```

#### 2. Test Fresh Install

```bash
#!/bin/bash
# test-fresh-install.sh

VM_NAME="nqrust-test"

# Copy installer (testing local changes)
multipass transfer -r scripts/install $VM_NAME:/tmp/

# Run installer
multipass exec $VM_NAME -- bash -c '
  cd /tmp/install
  bash install.sh --mode production --non-interactive --network-mode nat
'

# Verify installation
multipass exec $VM_NAME -- bash -c '
  # Check services
  systemctl is-active nqrust-manager
  systemctl is-active nqrust-agent
  systemctl is-active nqrust-ui

  # Check health endpoints
  curl -f http://localhost:18080/health
  curl -f http://localhost:19090/health

  # Check binaries
  /opt/nqrust-microvm/bin/manager --version
  /opt/nqrust-microvm/bin/agent --version
'

echo "Installation test completed!"
```

#### 3. Test Upgrade Path

```bash
#!/bin/bash
# test-upgrade.sh

VM_NAME="nqrust-test"

# Install old version
multipass exec $VM_NAME -- sudo bash -c '
  export RELEASE_VERSION=v1.0.0
  curl -fsSL https://raw.githubusercontent.com/NexusQuantum/NQRust-MicroVM/main/scripts/install/install.sh | bash
'

# Create test VM
multipass exec $VM_NAME -- bash -c '
  curl -X POST http://localhost:18080/v1/vms \
    -H "Content-Type: application/json" \
    -d "{\"name\":\"test-vm\",\"vcpu_count\":1,\"mem_size_mib\":512}"
'

# Upgrade to new version
multipass exec $VM_NAME -- sudo bash -c '
  export RELEASE_VERSION=v1.1.0
  curl -fsSL https://raw.githubusercontent.com/NexusQuantum/NQRust-MicroVM/main/scripts/install/install.sh | bash
'

# Verify VM still exists
multipass exec $VM_NAME -- curl http://localhost:18080/v1/vms | jq '.[] | select(.name=="test-vm")'

echo "Upgrade test completed!"
```

#### 4. Test Uninstaller

```bash
#!/bin/bash
# test-uninstall.sh

VM_NAME="nqrust-test"

# Create snapshot before uninstall
multipass snapshot $VM_NAME --name before-uninstall

# Run uninstaller (keep data)
multipass exec $VM_NAME -- sudo bash /tmp/install/uninstall.sh --keep-data

# Verify cleanup
multipass exec $VM_NAME -- bash -c '
  # Services should be gone
  ! systemctl is-active nqrust-manager

  # Binaries should be removed
  ! test -f /opt/nqrust-microvm/bin/manager

  # Data should remain
  test -d /srv/fc
'

# Restore snapshot if needed
multipass restore $VM_NAME --snapshot before-uninstall
```

#### 5. Snapshot Management

```bash
# Create snapshot
multipass snapshot test-installer --name clean-install

# List snapshots
multipass info test-installer

# Restore to snapshot
multipass restore test-installer --snapshot clean-install

# Delete snapshot
multipass delete test-installer.clean-install
```

## Option 2: VirtualBox

### Setup

```bash
# Install VirtualBox
# Ubuntu: sudo apt install virtualbox
# macOS: brew install --cask virtualbox
# Windows: Download from https://www.virtualbox.org/

# Download Ubuntu ISO
wget https://releases.ubuntu.com/22.04/ubuntu-22.04.3-live-server-amd64.iso
```

### Create Test VM

1. **Create new VM**:
   - Name: `nqrust-test`
   - Type: Linux
   - Version: Ubuntu (64-bit)
   - RAM: 4096 MB
   - Disk: 20 GB

2. **Enable nested virtualization**:
   ```bash
   # Stop VM first, then run:
   VBoxManage modifyvm "nqrust-test" --nested-hw-virt on
   VBoxManage modifyvm "nqrust-test" --cpus 2
   ```

3. **Take snapshot** before testing:
   - Right-click VM → Snapshots → Take
   - Name: "Fresh Install"

### Test Workflow

```bash
# SSH into VM (setup port forwarding first)
ssh user@localhost -p 2222

# Copy installer via SCP
scp -P 2222 -r scripts/install user@localhost:/tmp/

# Run installer
sudo bash /tmp/install/install.sh --mode production --non-interactive

# Test...

# Revert to snapshot when done
# VirtualBox GUI: Right-click → Snapshots → Restore
```

### VirtualBox Automation

```bash
#!/bin/bash
# vbox-test.sh

VM_NAME="nqrust-test"

# Start VM
VBoxManage startvm "$VM_NAME" --type headless

# Wait for boot
sleep 30

# SSH and test (requires SSH key setup)
ssh -o StrictHostKeyChecking=no -p 2222 user@localhost << 'EOF'
  # Copy installer
  curl -fsSL https://raw.githubusercontent.com/NexusQuantum/NQRust-MicroVM/main/scripts/install/install.sh -o /tmp/install.sh

  # Run installer
  sudo bash /tmp/install.sh --mode production --non-interactive

  # Verify
  systemctl is-active nqrust-manager
EOF

# Restore snapshot
VBoxManage snapshot "$VM_NAME" restore "Fresh Install"

# Stop VM
VBoxManage controlvm "$VM_NAME" poweroff
```

## Option 3: Docker (Limited)

**Note**: Docker has limitations for testing systemd-based installers, but you can test script logic.

### Test Script Syntax Only

```dockerfile
# Dockerfile.test
FROM ubuntu:22.04

# Install basic dependencies
RUN apt-get update && apt-get install -y \
    bash \
    curl \
    shellcheck

WORKDIR /workspace

# Copy installer
COPY scripts/install /workspace/install

# Test script syntax
RUN bash -n install/install.sh
RUN shellcheck install/install.sh install/lib/*.sh

CMD ["/bin/bash"]
```

```bash
# Build and test
docker build -f Dockerfile.test -t nqrust-installer-test .

# Run syntax checks
docker run --rm nqrust-installer-test

# Test preflight checks only (no actual install)
docker run --rm -it nqrust-installer-test bash -c '
  source install/lib/common.sh
  source install/lib/preflight.sh
  check_os
  check_ram 2048
'
```

### Systemd in Docker (Advanced)

```dockerfile
# Dockerfile.systemd
FROM ubuntu:22.04

# Install systemd
RUN apt-get update && apt-get install -y \
    systemd \
    systemd-sysv \
    dbus \
    && apt-get clean

# Enable systemd
STOPSIGNAL SIGRTMIN+3
CMD ["/lib/systemd/systemd"]
```

```bash
# Run with systemd
docker run -d \
  --name nqrust-test \
  --privileged \
  -v /sys/fs/cgroup:/sys/fs/cgroup:ro \
  nqrust-systemd

# Test installer
docker exec -it nqrust-test bash
# Run installer inside...
```

**Warning**: This approach has many limitations and is not recommended for full testing.

## Option 4: Cloud VMs

### AWS EC2

```bash
#!/bin/bash
# aws-test-vm.sh

# Launch instance
INSTANCE_ID=$(aws ec2 run-instances \
  --image-id ami-0557a15b87f6559cf \
  --instance-type t3.medium \
  --key-name your-key \
  --security-group-ids sg-xxxxx \
  --subnet-id subnet-xxxxx \
  --tag-specifications 'ResourceType=instance,Tags=[{Key=Name,Value=nqrust-test}]' \
  --query 'Instances[0].InstanceId' \
  --output text)

echo "Instance launched: $INSTANCE_ID"

# Wait for running
aws ec2 wait instance-running --instance-ids $INSTANCE_ID

# Get public IP
PUBLIC_IP=$(aws ec2 describe-instances \
  --instance-ids $INSTANCE_ID \
  --query 'Reservations[0].Instances[0].PublicIpAddress' \
  --output text)

echo "Instance IP: $PUBLIC_IP"

# SSH and test
ssh -i your-key.pem ubuntu@$PUBLIC_IP << 'EOF'
  curl -fsSL https://raw.githubusercontent.com/NexusQuantum/NQRust-MicroVM/main/scripts/install/install.sh | sudo bash
EOF

# Terminate when done
aws ec2 terminate-instances --instance-ids $INSTANCE_ID
```

### DigitalOcean

```bash
#!/bin/bash
# do-test-vm.sh

# Create droplet
DROPLET_ID=$(doctl compute droplet create nqrust-test \
  --image ubuntu-22-04-x64 \
  --size s-2vcpu-4gb \
  --region nyc1 \
  --ssh-keys your-ssh-key-id \
  --format ID \
  --no-header)

echo "Droplet created: $DROPLET_ID"

# Wait for active
doctl compute droplet wait $DROPLET_ID

# Get IP
DROPLET_IP=$(doctl compute droplet get $DROPLET_ID --format PublicIPv4 --no-header)

echo "Droplet IP: $DROPLET_IP"

# Test installer
ssh root@$DROPLET_IP << 'EOF'
  curl -fsSL https://raw.githubusercontent.com/NexusQuantum/NQRust-MicroVM/main/scripts/install/install.sh | bash --non-interactive

  # Verify
  systemctl status nqrust-manager
  curl http://localhost:18080/health
EOF

# Destroy when done
doctl compute droplet delete $DROPLET_ID --force
```

## Option 5: LXD Containers

LXD provides system containers with full systemd support.

```bash
# Install LXD
sudo snap install lxd
sudo lxd init --auto

# Launch container
lxc launch ubuntu:22.04 nqrust-test

# Enable nested virtualization
lxc config set nqrust-test security.nesting=true
lxc config set nqrust-test security.privileged=true

# Restart container
lxc restart nqrust-test

# Copy installer
lxc file push -r scripts/install nqrust-test/tmp/

# Run installer
lxc exec nqrust-test -- bash /tmp/install/install.sh --mode production --non-interactive

# Check status
lxc exec nqrust-test -- systemctl status nqrust-manager

# Shell into container
lxc exec nqrust-test -- bash

# Snapshot
lxc snapshot nqrust-test clean-install

# Restore snapshot
lxc restore nqrust-test clean-install

# Delete container
lxc delete nqrust-test --force
```

## Option 6: Vagrant

Create reproducible test environments with Vagrant.

### Vagrantfile

```ruby
# Vagrantfile
Vagrant.configure("2") do |config|
  # Ubuntu 22.04
  config.vm.box = "ubuntu/jammy64"

  config.vm.provider "virtualbox" do |vb|
    vb.cpus = 2
    vb.memory = 4096
    vb.customize ["modifyvm", :id, "--nested-hw-virt", "on"]
  end

  # Port forwarding
  config.vm.network "forwarded_port", guest: 18080, host: 18080
  config.vm.network "forwarded_port", guest: 19090, host: 19090
  config.vm.network "forwarded_port", guest: 3000, host: 3000

  # Sync installer scripts
  config.vm.synced_folder "./scripts/install", "/vagrant/install"

  # Provision - run installer
  config.vm.provision "shell", inline: <<-SHELL
    cd /vagrant/install
    bash install.sh --mode production --non-interactive
  SHELL
end
```

### Test with Vagrant

```bash
# Create and provision VM
vagrant up

# SSH into VM
vagrant ssh

# Test inside VM...

# Destroy VM
vagrant destroy -f

# Test multiple OSes
vagrant up ubuntu22
vagrant up ubuntu24
vagrant up debian11
```

### Multi-OS Vagrantfile

```ruby
# Vagrantfile
Vagrant.configure("2") do |config|
  # Define multiple test environments
  {
    "ubuntu22" => "ubuntu/jammy64",
    "ubuntu24" => "ubuntu/noble64",
    "debian11" => "debian/bullseye64",
    "debian12" => "debian/bookworm64"
  }.each do |name, box|
    config.vm.define name do |vm|
      vm.vm.box = box
      vm.vm.hostname = "nqrust-#{name}"

      vm.vm.provider "virtualbox" do |vb|
        vb.cpus = 2
        vb.memory = 4096
        vb.customize ["modifyvm", :id, "--nested-hw-virt", "on"]
      end

      vm.vm.provision "shell", inline: "bash /vagrant/install/install.sh --mode production --non-interactive"
    end
  end
end
```

```bash
# Test on all OSes
vagrant up

# Test specific OS
vagrant up ubuntu22

# SSH to specific
vagrant ssh ubuntu22
```

## Testing Checklist

### Pre-Installation Tests

- [ ] Script syntax validation (`bash -n install.sh`)
- [ ] Shellcheck passes (`shellcheck install.sh`)
- [ ] Preflight checks work
- [ ] Help text displays correctly

### Installation Tests

- [ ] **Fresh install** on clean system
- [ ] **Re-run installer** (idempotency test)
- [ ] **Install with all modes**:
  - [ ] Production mode
  - [ ] Dev mode (build from source)
  - [ ] Manager-only mode
  - [ ] Agent-only mode
  - [ ] Minimal mode (no UI)
- [ ] **Network modes**:
  - [ ] NAT mode
  - [ ] Bridged mode (if applicable)

### Service Verification

```bash
# Check all services started
systemctl status nqrust-manager
systemctl status nqrust-agent
systemctl status nqrust-ui

# Check health endpoints
curl http://localhost:18080/health
curl http://localhost:19090/health
curl http://localhost:3000

# Check logs for errors
journalctl -u nqrust-manager -n 50 --no-pager
journalctl -u nqrust-agent -n 50 --no-pager
```

### Functional Tests

```bash
# Create a test VM
curl -X POST http://localhost:18080/v1/vms \
  -H "Content-Type: application/json" \
  -d '{
    "name": "test-vm",
    "vcpu_count": 1,
    "mem_size_mib": 512,
    "kernel_image": "path/to/kernel",
    "rootfs_image": "path/to/rootfs"
  }'

# List VMs
curl http://localhost:18080/v1/vms | jq

# Delete VM
curl -X DELETE http://localhost:18080/v1/vms/test-vm
```

### Upgrade Tests

- [ ] Upgrade from previous version
- [ ] VMs survive upgrade
- [ ] Database migrations work
- [ ] Config files preserved

### Uninstallation Tests

- [ ] Uninstall with `--keep-data`
- [ ] Uninstall with `--remove-all`
- [ ] Re-install after uninstall
- [ ] Services properly removed

### Multi-OS Tests

Test on:
- [ ] Ubuntu 22.04 LTS
- [ ] Ubuntu 24.04 LTS
- [ ] Ubuntu 20.04 LTS
- [ ] Debian 11
- [ ] Debian 12
- [ ] RHEL 8 / Rocky Linux 8 (if supported)

## Common Issues and Solutions

### Nested Virtualization Not Working

```bash
# Check if nested virt is enabled
cat /sys/module/kvm_intel/parameters/nested
# Should output: Y

# Enable on host (requires reboot)
echo "options kvm_intel nested=1" | sudo tee /etc/modprobe.d/kvm.conf

# For AMD:
echo "options kvm_amd nested=1" | sudo tee /etc/modprobe.d/kvm.conf
```

### VM Not Getting Internet Access

```bash
# Check network in VM
ip addr
ip route
ping 8.8.8.8

# For Multipass - check bridge
multipass exec test-installer -- ip addr show
```

### Port Already in Use

```bash
# Check what's using ports
sudo lsof -i :18080
sudo lsof -i :19090
sudo lsof -i :3000

# Kill existing processes
sudo systemctl stop nqrust-manager
```

### Installer Hangs

```bash
# Run with debug output
sudo DEBUG=true bash install.sh --mode production

# Check logs
tail -f /var/log/nqrust-install/install-*.log
```

## Automated Testing

### Full Test Script

```bash
#!/bin/bash
# full-test.sh - Complete installer test suite

set -e

VM_NAME="nqrust-test-$(date +%s)"
ERRORS=0

cleanup() {
    echo "Cleaning up..."
    multipass delete $VM_NAME --purge 2>/dev/null || true
}

trap cleanup EXIT

# Create VM
echo "==> Creating test VM..."
multipass launch 22.04 --name $VM_NAME --cpus 2 --memory 4G --disk 20G

# Copy installer
echo "==> Copying installer..."
multipass transfer -r scripts/install $VM_NAME:/tmp/

# Test 1: Fresh install
echo "==> Test 1: Fresh installation"
if multipass exec $VM_NAME -- sudo bash /tmp/install/install.sh --mode production --non-interactive; then
    echo "✓ Fresh install passed"
else
    echo "✗ Fresh install failed"
    ERRORS=$((ERRORS + 1))
fi

# Test 2: Services running
echo "==> Test 2: Service health"
if multipass exec $VM_NAME -- systemctl is-active nqrust-manager && \
   multipass exec $VM_NAME -- curl -sf http://localhost:18080/health >/dev/null; then
    echo "✓ Services healthy"
else
    echo "✗ Services not healthy"
    ERRORS=$((ERRORS + 1))
fi

# Test 3: Idempotency
echo "==> Test 3: Re-run installer (idempotency)"
if multipass exec $VM_NAME -- sudo bash /tmp/install/install.sh --mode production --non-interactive; then
    echo "✓ Idempotency test passed"
else
    echo "✗ Idempotency test failed"
    ERRORS=$((ERRORS + 1))
fi

# Test 4: Uninstaller
echo "==> Test 4: Uninstaller"
if multipass exec $VM_NAME -- sudo bash /tmp/install/uninstall.sh --force --remove-all; then
    echo "✓ Uninstaller passed"
else
    echo "✗ Uninstaller failed"
    ERRORS=$((ERRORS + 1))
fi

# Results
echo ""
echo "================================"
if [ $ERRORS -eq 0 ]; then
    echo "✓ All tests passed!"
    exit 0
else
    echo "✗ $ERRORS test(s) failed"
    exit 1
fi
```

```bash
# Run full test suite
bash full-test.sh
```

### CI Integration Test Script

```bash
#!/bin/bash
# ci-installer-test.sh - For GitHub Actions

set -euo pipefail

echo "==> Testing installer in CI environment"

# Test preflight checks
echo "Testing preflight checks..."
bash -n scripts/install/install.sh
shellcheck scripts/install/install.sh scripts/install/lib/*.sh

# Test in Docker (syntax only)
echo "Testing script syntax..."
docker run --rm -v $(pwd):/workspace ubuntu:22.04 bash -c '
    apt-get update -qq
    apt-get install -y -qq bash shellcheck
    cd /workspace
    bash -n scripts/install/install.sh
    shellcheck scripts/install/install.sh
'

echo "✓ CI installer tests passed"
```

## Summary

**Recommended approach for most developers**:

1. **Quick tests**: Use **Multipass** for rapid iteration
2. **Manual testing**: Use **VirtualBox** with snapshots
3. **CI testing**: Use **GitHub Actions** workflows (already configured)
4. **Pre-release**: Test on **cloud VMs** (multiple OSes)

**Before every release**:
```bash
# 1. Test locally
multipass launch 22.04 --name test
multipass transfer -r scripts/install test:/tmp/
multipass exec test -- bash /tmp/install/install.sh --mode production --non-interactive

# 2. Push and let GitHub Actions test
git push origin main

# 3. Create release only if CI passes
git tag v1.0.0
git push origin v1.0.0
```

This ensures your installer is thoroughly tested before users download it! 🛡️
