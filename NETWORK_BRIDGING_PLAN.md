# Network Bridging Plan - VM Internet Access

## Goal
Make Firecracker VMs accessible on the physical network (e.g., 192.168.18.x) so they get IPs directly from the router, just like the Ubuntu host does.

## Current Setup (NAT Mode)
- VMs are behind `fcbr0` bridge with NAT (MASQUERADE)
- VMs are isolated and not directly accessible from external network
- Script: `scripts/fc-bridge-setup.sh`

## Target Setup (Bridged Mode)
- VMs on same network as host (192.168.18.x)
- VMs get IPs from router via DHCP
- VMs directly accessible from home network
- Example:
  - Proxmox: 192.168.18.1
  - Ubuntu host: 192.168.18.240
  - FC VM 1: 192.168.18.171
  - FC VM 2: 192.168.18.172

## Implementation Plan

### 1. Create Physical Bridge Setup Script
**File**: `scripts/fc-bridge-physical.sh`

**What it does**:
- Creates bridge `fcbr0` (or custom name)
- Attaches physical interface (e.g., `ens18`) to bridge
- Moves host IP from physical interface to bridge
- Enables promiscuous mode on physical interface
- Removes NAT rules (no MASQUERADE needed)
- Makes VMs appear on physical network

**Usage**:
```bash
sudo ./scripts/fc-bridge-physical.sh fcbr0 ens18
```

**Important**: After running, you'll need to make it persistent via netplan/NetworkManager

### 2. Netplan Configuration (For Persistence)
**File**: `/etc/netplan/01-bridge.yaml`

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
      dhcp4: yes  # Host gets IP via DHCP
      dhcp6: no
```

Apply with: `sudo netplan apply`

### 3. VM DHCP Client Configuration
**Challenge**: VMs need DHCP client configured inside the guest OS

**Option A - Cloud-init** (Preferred):
- Use cloud-init to configure network via MMDS
- Already have MMDS infrastructure
- Add network config to cloud-init user-data:
```yaml
#cloud-config
network:
  version: 2
  ethernets:
    eth0:
      dhcp4: true
```

**Option B - Manual/Image-based**:
- Pre-configure images with DHCP enabled
- Most cloud images already have this
- Alpine needs `udhcpc` or `dhclient`

### 4. Backend Changes

#### Agent Configuration
**File**: `apps/agent/src/core/agent.rs`
- Add `FC_BRIDGE_MODE` env var: `nat` or `bridged`
- Report bridge mode in capabilities

#### VM Creation Flow
**File**: `apps/manager/src/features/vms/service.rs`
- Re-enable cloud-init credential injection (currently disabled)
- Add network configuration to cloud-init YAML:
```rust
let cloud_init_yaml = format!(
    r#"#cloud-config
users:
  - name: {username}
    plain_text_passwd: {password}
    lock_passwd: false
    sudo: ALL=(ALL) NOPASSWD:ALL
chpasswd:
  expire: false
network:
  version: 2
  ethernets:
    eth0:
      dhcp4: true
"#
);
```

#### Add Network Config to MMDS
**File**: `apps/manager/src/features/vms/service.rs` (line ~1040)
- Inject network-config alongside user-data:
```rust
put_mmds(
    st,
    vm_id,
    MmdsDataReq {
        data: json!({
            "latest": {
                "user-data": user_data_b64,
                "network-config": network_config_b64  // Add this
            }
        }),
    },
)
```

### 5. Frontend Changes (Optional)

#### Network Mode Selection
**File**: `apps/frontend/components/vm-creation-wizard.tsx`

Add network mode selector in Step 4 (Network):
- **NAT Mode**: VM isolated, host forwards traffic
- **Bridged Mode**: VM on physical network, gets IP from router

Update `CreateVmReq`:
```typescript
interface CreateVmReq {
  // ... existing fields
  network_mode?: 'nat' | 'bridged'
}
```

### 6. Testing Plan

1. **Setup Bridge**:
   ```bash
   sudo ./scripts/fc-bridge-physical.sh fcbr0 ens18
   ```

2. **Create VM with cloud-init enabled**:
   - Use Ubuntu/Debian image with cloud-init
   - Create VM via wizard
   - Check VM gets IP: `ip addr show eth0`

3. **Verify Connectivity**:
   - From host: `ping <vm-ip>`
   - From another device on network: `ping <vm-ip>`
   - From VM: `ping 8.8.8.8` (internet)

4. **Check IP Assignment**:
   - VM should have 192.168.18.x IP
   - Verify with `ip addr` inside VM
   - Check router DHCP leases

## Fallback Plan

If bridging breaks host connectivity:
1. Remove interface from bridge:
   ```bash
   sudo ip link set ens18 nomaster
   ```

2. Restore IP to physical interface:
   ```bash
   sudo ip addr add <old-ip>/24 dev ens18
   sudo ip route add default via <gateway>
   ```

3. Bring interface up:
   ```bash
   sudo ip link set ens18 up
   ```

## Current Status
- ✅ Credentials injection working (rootfs method)
- ⏸️ Cloud-init disabled (was causing errors)
- ❌ VMs not on physical network (using NAT)

## Next Steps Tomorrow
1. Create `fc-bridge-physical.sh` script
2. Test bridge setup on Ubuntu host
3. Re-enable cloud-init with network config
4. Test VM getting IP from router
5. Make setup persistent via netplan

## Notes
- **Risk**: Changing bridge setup can break SSH connection to host
- **Recommendation**: Have console/VNC access to Proxmox VM before testing
- **Alternative**: Test on a separate network interface first
- **Cloud-init requirement**: Guest OS must have cloud-init installed
  - Ubuntu: ✅ Pre-installed
  - Debian: ✅ Pre-installed
  - Alpine: ❌ Need to install `cloud-init` package
