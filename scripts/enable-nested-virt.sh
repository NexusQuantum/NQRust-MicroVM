#!/usr/bin/env bash
# Enable nested virtualization on the host for Multipass testing
# This script must be run on the HOST machine (L0), not inside a VM

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}═══════════════════════════════════════════════════${NC}"
echo -e "${BLUE}  Nested Virtualization Setup for Multipass${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════${NC}"
echo ""

# Check if running as root
if [[ $EUID -eq 0 ]]; then
   echo -e "${RED}Error: This script should not be run as root${NC}"
   echo "Run as: bash $0"
   exit 1
fi

# Step 1: Detect CPU type
echo -e "${GREEN}[1/5] Detecting CPU type...${NC}"

if grep -q "vmx" /proc/cpuinfo; then
    CPU_TYPE="intel"
    KVM_MODULE="kvm_intel"
    echo -e "${GREEN}✓ Intel CPU detected${NC}"
elif grep -q "svm" /proc/cpuinfo; then
    CPU_TYPE="amd"
    KVM_MODULE="kvm_amd"
    echo -e "${GREEN}✓ AMD CPU detected${NC}"
else
    echo -e "${RED}✗ No virtualization support detected${NC}"
    echo "Your CPU does not support hardware virtualization (VT-x/AMD-V)"
    exit 1
fi

echo ""

# Step 2: Check current status
echo -e "${GREEN}[2/5] Checking current nested virtualization status...${NC}"

NESTED_FILE="/sys/module/${KVM_MODULE}/parameters/nested"

if [[ ! -f "$NESTED_FILE" ]]; then
    echo -e "${YELLOW}⚠ KVM module not loaded, will configure it${NC}"
    CURRENT_STATUS="not_loaded"
else
    CURRENT_STATUS=$(cat "$NESTED_FILE")
    if [[ "$CURRENT_STATUS" == "1" ]] || [[ "$CURRENT_STATUS" == "Y" ]]; then
        echo -e "${GREEN}✓ Nested virtualization is already enabled!${NC}"
        echo ""
        echo -e "${GREEN}You can now run:${NC}"
        echo "  bash scripts/test-installer-multipass.sh"
        exit 0
    else
        echo -e "${YELLOW}⚠ Nested virtualization is currently disabled${NC}"
    fi
fi

echo ""

# Step 3: Configure nested virtualization
echo -e "${GREEN}[3/5] Configuring nested virtualization...${NC}"

CONFIG_FILE="/etc/modprobe.d/kvm.conf"

if [[ -f "$CONFIG_FILE" ]]; then
    echo -e "${YELLOW}Configuration file already exists: $CONFIG_FILE${NC}"
    echo "Current contents:"
    cat "$CONFIG_FILE"
    echo ""
    read -p "Overwrite? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Skipping configuration file creation"
    else
        echo "options ${KVM_MODULE} nested=1" | sudo tee "$CONFIG_FILE" > /dev/null
        echo -e "${GREEN}✓ Configuration file updated${NC}"
    fi
else
    echo "options ${KVM_MODULE} nested=1" | sudo tee "$CONFIG_FILE" > /dev/null
    echo -e "${GREEN}✓ Configuration file created: $CONFIG_FILE${NC}"
fi

echo ""

# Step 4: Apply changes
echo -e "${GREEN}[4/5] Applying changes...${NC}"

echo "To enable nested virtualization, you have two options:"
echo ""
echo "  ${GREEN}Option 1 (Recommended):${NC} Reboot your host machine"
echo "  ${YELLOW}Option 2 (Quick):${NC} Reload KVM modules (may fail if VMs are running)"
echo ""

read -p "Choose: [R]eboot / [L]oad modules / [S]kip: " -n 1 -r
echo
echo ""

case $REPLY in
    [Rr])
        echo -e "${YELLOW}Rebooting in 10 seconds...${NC}"
        echo "Press Ctrl+C to cancel"
        sleep 10
        sudo reboot
        ;;
    [Ll])
        echo "Attempting to reload KVM modules..."

        # Stop all Multipass VMs first
        if command -v multipass &> /dev/null; then
            echo "Stopping Multipass VMs..."
            multipass stop --all 2>/dev/null || true
        fi

        # Unload modules
        echo "Unloading KVM modules..."
        sudo modprobe -r ${KVM_MODULE} 2>/dev/null || true
        sudo modprobe -r kvm 2>/dev/null || true

        sleep 2

        # Reload modules
        echo "Reloading KVM modules..."
        sudo modprobe kvm
        sudo modprobe ${KVM_MODULE}

        sleep 1

        # Verify
        NESTED_STATUS=$(cat "$NESTED_FILE" 2>/dev/null || echo "0")
        if [[ "$NESTED_STATUS" == "1" ]] || [[ "$NESTED_STATUS" == "Y" ]]; then
            echo -e "${GREEN}✓ Nested virtualization enabled successfully!${NC}"
        else
            echo -e "${RED}✗ Failed to enable nested virtualization${NC}"
            echo "You may need to reboot your host machine"
            exit 1
        fi
        ;;
    [Ss])
        echo "Skipping module reload. Remember to reboot later!"
        exit 0
        ;;
    *)
        echo "Invalid choice. Exiting."
        exit 1
        ;;
esac

echo ""

# Step 5: Verification
echo -e "${GREEN}[5/5] Verifying nested virtualization...${NC}"

sleep 2

NESTED_STATUS=$(cat "$NESTED_FILE" 2>/dev/null || echo "0")
if [[ "$NESTED_STATUS" == "1" ]] || [[ "$NESTED_STATUS" == "Y" ]]; then
    echo -e "${GREEN}✓ Nested virtualization is ENABLED${NC}"
else
    echo -e "${RED}✗ Nested virtualization is still DISABLED${NC}"
    echo "Please reboot your host machine and run this script again"
    exit 1
fi

echo ""

# Test with Multipass
if command -v multipass &> /dev/null; then
    echo -e "${GREEN}Testing with Multipass...${NC}"

    # Launch a test VM
    TEST_VM="nested-virt-test"

    echo "Launching test VM..."
    multipass launch 22.04 --name "$TEST_VM" --cpus 2 --memory 2G --disk 10G 2>/dev/null || {
        echo -e "${YELLOW}⚠ Using existing VM${NC}"
    }

    echo "Checking if nested virtualization is available inside VM..."
    if multipass exec "$TEST_VM" -- egrep -q '(vmx|svm)' /proc/cpuinfo 2>/dev/null; then
        echo -e "${GREEN}✓ Nested virtualization works inside Multipass VM!${NC}"
        echo ""
        echo -e "${GREEN}You can now run:${NC}"
        echo "  bash scripts/test-installer-multipass.sh"
    else
        echo -e "${RED}✗ Nested virtualization not available inside VM${NC}"
        echo "You may need to:"
        echo "  1. Reboot your host machine"
        echo "  2. Delete and recreate Multipass VMs"
    fi

    # Cleanup
    echo ""
    read -p "Delete test VM? (Y/n): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Nn]$ ]]; then
        multipass delete "$TEST_VM" --purge 2>/dev/null || true
        echo -e "${GREEN}✓ Test VM deleted${NC}"
    fi
else
    echo -e "${YELLOW}⚠ Multipass not installed, skipping VM test${NC}"
    echo "Install with: sudo snap install multipass"
fi

echo ""
echo -e "${GREEN}═══════════════════════════════════════════════════${NC}"
echo -e "${GREEN}  Setup Complete!${NC}"
echo -e "${GREEN}═══════════════════════════════════════════════════${NC}"
