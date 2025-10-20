# Bridged Network Setup Guide

This guide explains how to configure your Firecracker VMs to appear directly on your physical network (e.g., 192.168.18.x), allowing them to get IP addresses from your router via DHCP.

## Overview

**Current Setup (NAT)**:
- VMs behind `fcbr0` bridge with NAT/MASQUERADE
- VMs isolated from external network
- Script: `scripts/fc-bridge-setup.sh`

**New Setup (Bridged)**:
- VMs on same network as host
- VMs get IPs from router via DHCP
- VMs directly accessible from network
- Script: `scripts/fc-bridge-physical.sh`

## Quick Start

### Step 1: Find Your Physical Interface

```bash
ip link show
# Look for your main network interface (e.g., ens18, eth0, enp0s3)
```

### Step 2: Setup Physical Bridge (Temporary)

```bash
# This will bridge fcbr0 to your physical interface
sudo ./scripts/fc-bridge-physical.sh fcbr0 ens18

# Replace 'ens18' with your interface name from Step 1
```

**Warning**: This will temporarily modify your network. Make sure you have console/VNC access in case SSH breaks!

### Step 3: Make It Persistent

Copy and edit the netplan configuration:

```bash
# Copy example config
sudo cp scripts/netplan-bridge-example.yaml /etc/netplan/01-fcbridge.yaml

# Edit with your network details
sudo nano /etc/netplan/01-fcbridge.yaml

# Test the configuration (auto-reverts after 120 seconds if you lose connection)
sudo netplan try

# If it works, press ENTER to keep it
# Or wait 120 seconds for auto-revert if something goes wrong

# Apply permanently
sudo netplan apply
```

### Step 4: Restart Services

```bash
# Restart agent to pick up new bridge
sudo systemctl restart nexus-agent  # or however your agent runs

# Restart manager if needed
sudo systemctl restart nexus-manager
```

### Step 5: Create a VM with Cloud-Init

Create a VM using an image with cloud-init (Ubuntu, Debian, or cloud-enabled Alpine):

1. Go to VM creation wizard
2. Select an Ubuntu/Debian cloud image
3. Configure credentials (username/password)
4. Create VM

### Step 6: Verify Network

SSH into the VM console and check:

```bash
# Check if VM got an IP from DHCP
ip addr show eth0

# You should see something like:
# inet 192.168.18.171/24 brd 192.168.18.255 scope global dynamic eth0

# Test internet connectivity
ping -c 3 8.8.8.8

# Test DNS
ping -c 3 google.com
```

From your home network (another device):

```bash
# Ping the VM directly
ping 192.168.18.171

# SSH into the VM
ssh root@192.168.18.171
```

## How It Works

### 1. Physical Bridge
The `fc-bridge-physical.sh` script:
- Attaches your physical interface (e.g., `ens18`) to the `fcbr0` bridge
- Moves your host's IP from physical interface to the bridge
- Enables promiscuous mode (required for bridging)
- Removes NAT rules (no MASQUERADE needed)

Result: All traffic on `fcbr0` is now forwarded to/from your physical network.

### 2. Cloud-Init DHCP Configuration
When you create a VM:
- Manager injects cloud-init config via MMDS (Firecracker's metadata service)
- Cloud-init user-data contains credentials
- Cloud-init network-config contains DHCP settings:
```yaml
version: 2
ethernets:
  eth0:
    dhcp4: true
```

### 3. VM Boots with DHCP
- VM reads cloud-init config from MMDS at `169.254.169.254`
- Sets up credentials (username/password)
- Configures `eth0` with DHCP
- Sends DHCP request on bridged network
- Router assigns IP from your home network pool

### 4. Dual Credential Injection
For maximum compatibility, the system uses two methods:
1. **Rootfs injection** (lines 103-107): Directly modifies `/etc/shadow` before VM starts
2. **Cloud-init** (lines 155-159): Injects via MMDS for cloud-init enabled images

This ensures credentials work regardless of whether the image has cloud-init.

## Network Diagram

### Before (NAT Mode):
```
Router (192.168.18.1)
  │
  └─ Ubuntu Host (192.168.18.240) - ens18
       │
       └─ fcbr0 (10.0.0.1) - NAT/MASQUERADE
            │
            ├─ VM1 tap (10.0.0.2) - Hidden from network
            └─ VM2 tap (10.0.0.3) - Hidden from network
```

### After (Bridged Mode):
```
Router (192.168.18.1) - DHCP Server
  │
  ├─ Ubuntu Host - fcbr0 (192.168.18.240)
  │    │
  │    └─ ens18 (bridged, no IP)
  │
  ├─ VM1 - eth0 (192.168.18.171) - DHCP from router
  └─ VM2 - eth0 (192.168.18.172) - DHCP from router
```

## Netplan Configuration

Edit `/etc/netplan/01-fcbridge.yaml`:

```yaml
network:
  version: 2
  renderer: networkd

  ethernets:
    ens18:  # Your physical interface
      dhcp4: no
      dhcp6: no

  bridges:
    fcbr0:
      interfaces:
        - ens18

      # Static IP (recommended)
      addresses:
        - 192.168.18.240/24
      routes:
        - to: default
          via: 192.168.18.1
      nameservers:
        addresses:
          - 8.8.8.8

      # Or use DHCP (uncomment):
      # dhcp4: yes
```

## Troubleshooting

### Lost SSH Connection After Bridge Setup

**Recovery**:
1. Access host via Proxmox console/VNC
2. Run recovery commands:
```bash
# Remove interface from bridge
sudo ip link set ens18 nomaster

# Restore IP to physical interface
sudo ip addr add 192.168.18.240/24 dev ens18
sudo ip route add default via 192.168.18.1

# Bring interface up
sudo ip link set ens18 up
```

### Internet/DNS Broken After Bridge Setup

**Symptoms**: Can ping IP addresses but not domain names (e.g., `ping 8.8.8.8` works but `ping google.com` fails)

**Quick Fix**:
```bash
# Fix DNS on the bridge
sudo resolvectl dns fcbr0 192.168.18.1  # Use your router/gateway IP
sudo resolvectl default-route fcbr0 yes

# Or use public DNS
sudo resolvectl dns fcbr0 8.8.8.8 8.8.4.4
sudo resolvectl default-route fcbr0 yes

# Test DNS resolution
ping google.com
```

**Permanent Fix**: Add nameservers to netplan config (see Step 9 in Quick Start)

### VM Not Getting IP

**Check**:
1. Does the image have cloud-init?
   ```bash
   # Inside VM console
   which cloud-init
   systemctl status cloud-init
   ```

2. Check cloud-init logs:
   ```bash
   # Inside VM
   sudo cat /var/log/cloud-init.log
   sudo cloud-init status --long
   ```

3. Manually test DHCP:
   ```bash
   # Inside VM
   sudo dhclient -v eth0
   ```

### Bridge Not Working

**Check**:
1. Is promiscuous mode enabled?
   ```bash
   ip link show ens18 | grep PROMISC
   ```

2. Is forwarding enabled?
   ```bash
   sysctl net.ipv4.ip_forward
   # Should be 1
   ```

3. Check bridge status:
   ```bash
   ip link show fcbr0
   bridge link show
   ```

### VM Can't Access Internet

**Check**:
1. Can VM ping gateway?
   ```bash
   ping 192.168.18.1
   ```

2. Check DNS:
   ```bash
   cat /etc/resolv.conf
   # Should have nameserver entries
   ```

3. Check routes:
   ```bash
   ip route show
   # Should have default route
   ```

## Files Modified

### Backend Changes:
- `apps/manager/src/features/vms/service.rs`:
  - Line 103-107: Rootfs credential injection (fallback)
  - Line 155-159: Cloud-init configuration call
  - Line 1017-1091: `configure_cloud_init_with_network()` function

### Scripts Created:
- `scripts/fc-bridge-physical.sh`: Physical bridge setup
- `scripts/netplan-bridge-example.yaml`: Persistent config example

## Cloud-Init Images

### With Cloud-Init (Recommended):
- ✅ Ubuntu Cloud Images
- ✅ Debian Cloud Images
- ✅ Fedora Cloud Base
- ✅ Rocky Linux Cloud
- ⚠️ Alpine Cloud (needs `cloud-init` package)

### Without Cloud-Init (Fallback):
- ⚠️ Minimal Alpine (rootfs injection only)
- ⚠️ Custom images (rootfs injection only)

For images without cloud-init, the rootfs credential injection will still work, but you'll need to manually configure networking inside the VM.

## Security Notes

1. **Promiscuous Mode**: Required for bridging, allows interface to see all traffic on the bridge
2. **Network Exposure**: VMs are now directly accessible from your network
3. **Firewall**: Consider adding iptables rules if needed
4. **DHCP Pool**: Make sure your router has enough IPs in DHCP pool

## Reverting to NAT Mode

If you need to go back to NAT mode:

```bash
# Remove netplan bridge config
sudo rm /etc/netplan/01-fcbridge.yaml

# Restore original network config
sudo netplan apply

# Run original NAT setup script
sudo ./scripts/fc-bridge-setup.sh fcbr0 ens18
```

## References

- [Firecracker Network Setup](https://github.com/firecracker-microvm/firecracker/blob/main/docs/network-setup.md)
- [Cloud-init Network Config](https://cloudinit.readthedocs.io/en/latest/reference/network-config-format-v2.html)
- [Netplan Documentation](https://netplan.io/reference/)
- [Linux Bridge](https://wiki.linuxfoundation.org/networking/bridge)
