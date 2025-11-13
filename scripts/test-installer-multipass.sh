#!/usr/bin/env bash
# Quick installer testing script using Multipass
# Usage: bash scripts/test-installer-multipass.sh [vm-name]

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
VM_NAME="${1:-nqrust-test}"
VM_CPUS="${VM_CPUS:-2}"
VM_MEMORY="${VM_MEMORY:-4G}"
VM_DISK="${VM_DISK:-25G}"
UBUNTU_VERSION="${UBUNTU_VERSION:-22.04}"

echo -e "${GREEN}==> NQRust-MicroVM Installer Test${NC}"
echo "VM Name: $VM_NAME"
echo "Config: ${VM_CPUS} CPUs, ${VM_MEMORY} RAM, ${VM_DISK} disk"
echo ""

# Check if multipass is installed
if ! command -v multipass &> /dev/null; then
    echo -e "${RED}Error: Multipass not installed${NC}"
    echo "Install with: sudo snap install multipass"
    exit 1
fi

# Cleanup function
cleanup() {
    if [ "${KEEP_VM:-false}" != "true" ]; then
        echo -e "${YELLOW}==> Cleaning up...${NC}"
        multipass delete "$VM_NAME" --purge 2>/dev/null || true
    fi
}

# Set trap for cleanup on exit
trap cleanup EXIT

# Check if VM already exists
if multipass list | grep -q "^$VM_NAME "; then
    echo -e "${YELLOW}VM $VM_NAME already exists. Delete it? (y/N)${NC}"
    read -r response
    if [[ "$response" =~ ^[Yy]$ ]]; then
        multipass delete "$VM_NAME" --purge
    else
        echo "Using existing VM"
    fi
fi

# Create VM if it doesn't exist
if ! multipass list | grep -q "^$VM_NAME "; then
    echo -e "${GREEN}==> Creating test VM...${NC}"
    multipass launch "$UBUNTU_VERSION" \
        --name "$VM_NAME" \
        --cpus "$VM_CPUS" \
        --memory "$VM_MEMORY" \
        --disk "$VM_DISK"

    # Wait for VM to be ready
    sleep 5
fi

# Copy installer to VM
echo -e "${GREEN}==> Copying installer to VM...${NC}"
multipass transfer -r scripts/install "$VM_NAME:/tmp/"

# Run installer
echo -e "${GREEN}==> Running installer...${NC}"
echo "Command: bash /tmp/install/install.sh --mode production --non-interactive"
echo ""

if multipass exec "$VM_NAME" -- bash /tmp/install/install.sh --mode production --non-interactive; then
    echo ""
    echo -e "${GREEN}✓ Installation completed${NC}"
else
    echo ""
    echo -e "${RED}✗ Installation failed${NC}"
    echo ""
    echo -e "${YELLOW}To debug, shell into VM:${NC}"
    echo "  multipass shell $VM_NAME"
    echo ""
    echo -e "${YELLOW}Check logs:${NC}"
    echo "  multipass exec $VM_NAME -- sudo journalctl -u nqrust-manager -n 50"
    exit 1
fi

# Wait a bit for services to fully start
echo ""
echo -e "${GREEN}==> Waiting for services to start...${NC}"
sleep 5

# Note about KVM group membership
echo -e "${YELLOW}Note: The installer adds user to 'kvm' group, but it requires re-login to take effect.${NC}"
echo -e "${YELLOW}For testing purposes, services run with proper permissions via systemd.${NC}"
echo ""

# Verify installation
echo -e "${GREEN}==> Verifying installation...${NC}"
ERRORS=0

# Check manager service
echo -n "Checking manager service... "
if multipass exec "$VM_NAME" -- systemctl is-active --quiet nqrust-manager; then
    echo -e "${GREEN}✓${NC}"
else
    echo -e "${RED}✗${NC}"
    ERRORS=$((ERRORS + 1))
fi

# Check agent service
echo -n "Checking agent service... "
if multipass exec "$VM_NAME" -- systemctl is-active --quiet nqrust-agent; then
    echo -e "${GREEN}✓${NC}"
else
    echo -e "${RED}✗${NC}"
    ERRORS=$((ERRORS + 1))
fi

# Check UI service
echo -n "Checking UI service... "
if multipass exec "$VM_NAME" -- systemctl is-active --quiet nqrust-ui; then
    echo -e "${GREEN}✓${NC}"
else
    echo -e "${RED}✗${NC}"
    ERRORS=$((ERRORS + 1))
fi

# Check manager health endpoint
echo -n "Checking manager API... "
if multipass exec "$VM_NAME" -- curl -sf http://localhost:18080/health >/dev/null; then
    echo -e "${GREEN}✓${NC}"
else
    echo -e "${RED}✗${NC}"
    ERRORS=$((ERRORS + 1))
fi

# Check agent health endpoint
echo -n "Checking agent API... "
if multipass exec "$VM_NAME" -- curl -sf http://localhost:19090/health >/dev/null; then
    echo -e "${GREEN}✓${NC}"
else
    echo -e "${RED}✗${NC}"
    ERRORS=$((ERRORS + 1))
fi

# Check UI
echo -n "Checking UI... "
if multipass exec "$VM_NAME" -- curl -sf http://localhost:3000 >/dev/null; then
    echo -e "${GREEN}✓${NC}"
else
    echo -e "${YELLOW}⚠${NC} (may still be starting)"
fi

# Show installed versions
echo ""
echo -e "${GREEN}==> Installed versions:${NC}"
multipass exec "$VM_NAME" -- /opt/nqrust-microvm/bin/manager --version || echo "manager: unknown"
multipass exec "$VM_NAME" -- /opt/nqrust-microvm/bin/agent --version || echo "agent: unknown"

# Summary
echo ""
echo "======================================"
if [ $ERRORS -eq 0 ]; then
    echo -e "${GREEN}✓ All checks passed!${NC}"
    echo ""
    echo -e "${GREEN}Access the VM:${NC}"
    echo "  multipass shell $VM_NAME"
    echo ""
    echo -e "${GREEN}Get VM IP:${NC}"
    VM_IP=$(multipass info "$VM_NAME" | grep IPv4 | awk '{print $2}')
    echo "  VM IP: $VM_IP"
    echo ""
    echo -e "${GREEN}Access services:${NC}"
    echo "  Manager API: http://${VM_IP}:18080"
    echo "  Agent API:   http://${VM_IP}:19090"
    echo "  UI:          http://${VM_IP}:3000"
    echo ""
    echo -e "${GREEN}To keep VM after script exits:${NC}"
    echo "  export KEEP_VM=true"
    echo ""
    echo -e "${GREEN}To delete VM:${NC}"
    echo "  multipass delete $VM_NAME --purge"
else
    echo -e "${RED}✗ $ERRORS check(s) failed${NC}"
    echo ""
    echo -e "${YELLOW}Debug commands:${NC}"
    echo "  multipass shell $VM_NAME"
    echo "  multipass exec $VM_NAME -- sudo journalctl -u nqrust-manager -n 50"
    echo "  multipass exec $VM_NAME -- sudo journalctl -u nqrust-agent -n 50"
    echo ""
    echo -e "${YELLOW}Keep VM for debugging:${NC}"
    export KEEP_VM=true
    trap - EXIT  # Remove cleanup trap
    exit 1
fi

# If KEEP_VM is not set, ask user
if [ "${KEEP_VM:-false}" != "true" ]; then
    echo -e "${YELLOW}Keep VM for further testing? (y/N)${NC}"
    read -r response
    if [[ "$response" =~ ^[Yy]$ ]]; then
        export KEEP_VM=true
        trap - EXIT  # Remove cleanup trap
        echo "VM $VM_NAME will be kept"
    fi
fi
