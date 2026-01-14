+++
title = "Networks"
description = "Complete guide to virtual networks for VM connectivity through the web interface"
weight = 60
date = 2025-01-08
+++

Networks provide connectivity for your VMs through bridge networking and VLAN isolation. This guide will show you how to create and manage virtual networks using the web interface, with automatic network creation, VLAN support, and multi-tenant isolation.

---

## What are Networks?

Networks are **virtual network configurations** that connect your VMs to each other and to external networks. Each network is backed by a Linux bridge on the host and can optionally use VLAN tagging for isolation.

### Key Benefits

**1. Automatic Network Creation**
- Networks are created automatically when you create VMs
- No manual setup needed for basic configurations
- All networks tracked in one central location

**2. VLAN Isolation**
- Support for VLAN tagging to isolate networks
- Separate VMs by environment (development/staging/production)
- Multi-tenant network segregation for security
- Keep different projects or customers isolated

**3. Flexible Connectivity**
- Bridge networking for external network access
- NAT mode for internet-only access
- VLAN tagging for multi-tenant isolation
- Easy to manage through web interface

---

## Common Use Cases

### Environment Segregation

Separate development, staging, and production VMs:

```
Network: fcbr0 (VLAN 10) - Development
Network: fcbr0 (VLAN 20) - Staging
Network: fcbr0 (VLAN 30) - Production
```

Each environment is isolated at the network layer.

---

### Multi-Tenant Isolation

Isolate customer or project networks:

```
Network: fcbr0 (VLAN 100) - Customer A
Network: fcbr0 (VLAN 200) - Customer B
Network: fcbr0 (VLAN 300) - Customer C
```

Customers cannot access each other's network traffic.

---

### Hybrid Networking

Mix isolated and shared networks:

```
Network: fcbr0 (no VLAN) - Shared services (DNS, NTP)
Network: fcbr0 (VLAN 10) - Private app tier
Network: fcbr0 (VLAN 20) - Private database tier
```

Shared services accessible to all, sensitive tiers isolated.

---

## Network Types

### Bridge Network (No VLAN)

**Default configuration**:
- Uses host bridge (e.g., `fcbr0`)
- No VLAN tagging
- All VMs on same Layer 2 network
- Simple, no switch configuration needed

**Example**:
```
Bridge: fcbr0
VLAN: None
VMs can communicate freely
```

**Use when**:
- Single environment deployment
- All VMs can trust each other
- Simple home lab or development setup

---

### VLAN-Tagged Network

**Isolated configuration**:
- Uses host bridge with VLAN tag
- 802.1Q VLAN tagging
- VMs isolated by VLAN ID
- Requires VLAN-aware switch

**Example**:
```
Bridge: fcbr0
VLAN: 10
VMs only communicate with VLAN 10
```

**Use when**:
- Multi-tenant environments
- Environment separation (dev/staging/prod)
- Security compliance requires network isolation
- Multiple projects on shared infrastructure

---

## Network Lifecycle

### 1. Automatic Network Creation

Networks are automatically created when you create VMs:

**What happens**:
1. You create a VM with network configuration through the web interface
2. System checks if the network already exists
3. If not found, system automatically creates the network
4. Network appears in the Networks page
5. Your VM connects to this network

**Example**:
```
When creating a VM, you specify:
- Bridge: fcbr0
- VLAN: 10

Result:
- Network "fcbr0-vlan-10" is automatically created
- VM is connected to this network
- Network appears in your Networks page
```

---

### 2. Manual Network Creation

You can also create networks manually before creating VMs:

**When to use**:
- Planning network architecture before deploying VMs
- Documenting your network inventory
- Pre-creating networks for your team
- Setting up multi-tenant environments

**How**:
1. Navigate to Networks page in the sidebar
2. Click "Create Network" button
3. Fill in network name, bridge name, and optional VLAN ID
4. Network is created and ready for VM assignment

---

### 3. Network Usage

Track which VMs use each network:

**Network registry shows**:
- Network name (bridge + VLAN)
- Number of attached VMs
- VM list using the network
- Creation timestamp

---

### 4. Network Deletion

Remove unused networks:

**Safe to delete when**:
- No VMs attached
- Network no longer needed
- Cleaning up inventory

**Cannot delete**:
- Networks with active VMs
- Must detach VMs first

---

## Network Components

### Bridge Name

**Linux bridge interface**:
- Created on host (e.g., `fcbr0`, `br0`)
- Connects VM TAP devices
- Provides Layer 2 switching
- Can be connected to physical NIC for external access

**Common bridges**:
- `fcbr0` - Firecracker default bridge
- `br0` - Standard Linux bridge
- `virbr0` - Libvirt default bridge

---

### VLAN ID

**802.1Q VLAN tag**:
- Optional integer (1-4094)
- Isolates traffic at Layer 2
- Requires VLAN-aware switch for external connectivity
- VMs in different VLANs cannot communicate (without routing)

**VLAN ranges**:
- 1-4094: Valid VLAN IDs
- Null/None: No VLAN tagging
- 1: Often reserved for default VLAN
- 4095: Reserved by standard

---

### TAP Device

**VM network interface**:
- Created per VM NIC
- Named `vmtap<random>` (e.g., `vmtap123456`)
- Connected to bridge
- VLAN-tagged if network has VLAN ID

**TAP device lifecycle**:
1. Created when VM starts
2. Attached to bridge (with optional VLAN tag)
3. VM sees as `eth0` inside guest
4. Destroyed when VM stops

---

## Network Architecture

### Without VLAN

```
[VM1 eth0] ─── [vmtap1] ───┐
                           ├─ [fcbr0] ─── [Physical NIC] ─── [External Network]
[VM2 eth0] ─── [vmtap2] ───┘

VMs can communicate with each other and external network
```

---

### With VLAN Tagging

```
[VM1 eth0] ─── [vmtap1 VLAN 10] ───┐
                                   ├─ [fcbr0] ─── [VLAN-aware Switch]
[VM2 eth0] ─── [vmtap2 VLAN 20] ───┘                │
                                                     ├─ VLAN 10 ─── [Subnet 10.0.10.0/24]
                                                     └─ VLAN 20 ─── [Subnet 10.0.20.0/24]

VM1 and VM2 are isolated, cannot communicate without routing
```

---

## Quick Start

### 1. Navigate to Networks Page

![Image: Networks navigation](/images/networks/nav-networks.png)

Click **"Networks"** in the sidebar to access the Networks page.

---

### 2. View Network Registry

![Image: Networks page](/images/networks/page-layout.png)

See all registered networks:
- Bridge name and VLAN ID
- Number of attached VMs
- Creation date
- Actions (view VMs, delete)

---

### 3. Create VM with Network

![Image: VM network configuration](/images/networks/vm-network-config.png)

When creating a VM:
1. Configure network in VM creation wizard
2. Specify bridge name (e.g., `fcbr0`)
3. Optionally add VLAN ID for isolation
4. Network auto-registers if new
5. VM connects to network on startup

---

### 4. Manual Network Creation

![Image: Create network](/images/networks/register-network.png)

Create networks manually:
1. Click **"Create Network"** button
2. Fill in the network name
3. Select the host
4. Enter bridge name
5. Optionally enter VLAN ID
6. Click **"Create Network"**
7. Network is created and ready for VM assignment

---

## Network Properties

Each network entry tracks:

**Basic Information**:
- Network ID (UUID)
- Bridge name (e.g., `fcbr0`)
- VLAN ID (null or 1-4094)
- Created timestamp

**Usage Information**:
- Attached VM count
- List of VMs using network
- TAP device names
- Network status

---

## VLAN Configuration

### Setting Up VLANs

**System Requirements**:
1. VLAN-aware Linux bridge on the host
2. Physical switch supporting 802.1Q VLAN tagging
3. Proper VLAN configuration on the network switch

**Note**: VLAN setup on the host and switch is managed by your system administrator. If you need VLAN support, contact your administrator to ensure the infrastructure is properly configured.

---

### Assigning VLANs to VMs

**When creating a VM through the web interface**:
- Select or enter bridge name: `fcbr0`
- Enter VLAN ID: `10`
- Your VM traffic will be tagged with VLAN 10
- VM will be isolated from other VLANs

**Example VMs with Different VLANs**:
```
VM: web-server-dev
  Bridge: fcbr0
  VLAN: 10 (Development environment)

VM: web-server-prod
  Bridge: fcbr0
  VLAN: 30 (Production environment)

Result: These VMs cannot communicate with each other
        (they are in different VLANs)
```

---

### Multi-Tenant Example

**Scenario**: Hosting VMs for 3 different customers

**Network setup through web interface**:
```
Customer A VMs: VLAN 100
Customer B VMs: VLAN 200
Customer C VMs: VLAN 300

All use the same bridge (fcbr0)
All customers are isolated from each other
```

**How it works**:
- All three networks use the same physical bridge
- VLAN tagging keeps traffic separated
- Customers cannot access each other's VMs
- Network switch must support VLAN tagging

**To create these networks**:
1. Go to Networks page
2. Create network for Customer A (Bridge: fcbr0, VLAN: 100)
3. Create network for Customer B (Bridge: fcbr0, VLAN: 200)
4. Create network for Customer C (Bridge: fcbr0, VLAN: 300)
5. Assign customer VMs to their respective VLANs

---

## Network vs Bridge

**Network (in the Registry)**:
- Logical representation of network connectivity
- Visible in the Networks page
- Tracks which bridge and VLAN are used
- Shows which VMs are connected
- Can be deleted through the web interface (if no VMs are attached)

**Bridge (on the Host)**:
- Physical network device on the server
- Managed by your system administrator
- Exists on the Linux host system
- Multiple networks can share the same bridge (using different VLANs)
- Contact your administrator for bridge-related setup

---

## Best Practices

### 1. Plan VLAN Scheme

**Use consistent VLAN allocation**:
```
VLAN 10-19: Development environments
VLAN 20-29: Staging environments
VLAN 30-39: Production environments
VLAN 100+:  Customer/tenant isolation
```

**Document your scheme** for team reference.

---

### 2. Naming Convention

**Network names** are auto-generated: `bridge-name` or `bridge-name-vlan-id`

**Examples**:
- `fcbr0` - Bridge without VLAN
- `fcbr0-vlan-10` - Bridge with VLAN 10
- `br0-vlan-100` - Bridge br0 with VLAN 100

---

### 3. Security Considerations

**Isolate sensitive environments**:
- Production VMs in separate VLAN
- Customer data in separate VLANs
- Management network on separate VLAN

**Network firewall rules**:
- Use iptables/nftables on host
- Restrict inter-VLAN routing if needed
- Monitor network traffic

---

### 4. Monitor Network Usage

**Track VM attachments**:
- Check which VMs use each network
- Identify unused networks
- Clean up orphaned networks

**Network registry helps with**:
- Capacity planning
- Security audits
- Troubleshooting connectivity

---

## Troubleshooting

### VM Cannot Get IP Address

**Symptoms**:
- VM starts successfully
- No IP address shown in VM details
- No network connectivity

**Possible causes**:
1. Network bridge not properly configured on host
2. VLAN mismatch with DHCP server
3. Network configuration issue
4. Firewall blocking traffic

**Solution**:
1. Check VM's network configuration in the VM details page
2. Verify the bridge name and VLAN ID are correct
3. Check if other VMs on the same network can get IPs
4. Contact your system administrator if the issue persists

---

### VMs Cannot Communicate

**Symptoms**:
- Both VMs are running
- Both VMs have IP addresses
- VMs cannot ping or connect to each other

**Possible causes**:
1. VMs are in different VLANs (isolated by design)
2. Firewall rules blocking traffic
3. Network configuration issue
4. Switch configuration problem

**Solution**:
1. Check both VMs in the Networks page - verify they use the same network (same bridge + VLAN)
2. If VMs have different VLANs, they cannot communicate (this is by design for isolation)
3. Check VM firewall settings inside the guest OS
4. Contact your system administrator for network troubleshooting

---

### Network Cannot Be Deleted

**Symptoms**:
- Delete button is disabled or shows error
- Error message: "Network has attached VMs"

**Possible causes**:
- One or more VMs are still using this network

**Solution**:
1. Go to the Networks page and click on the network to view details
2. Check the list of VMs currently using this network
3. Stop and delete the VMs using this network, or reconfigure them to use a different network
4. Once no VMs are attached, you can delete the network

---

### VLAN Tagged Traffic Not Working

**Symptoms**:
- VLAN configured for VM
- VM gets IP address but cannot reach external network
- External hosts cannot reach the VM

**Possible causes**:
1. Network switch port not configured as trunk
2. VLAN not allowed on the switch trunk port
3. Bridge VLAN filtering not enabled on host
4. VLAN ID mismatch between VM, host, and switch

**Solution**:
1. Verify the VLAN ID is correct in your VM's network configuration
2. Check that the same VLAN ID is used in the network entry
3. Contact your system administrator to verify:
   - Switch port is configured as trunk with 802.1Q
   - Correct VLANs are allowed on the trunk port
   - Bridge VLAN filtering is enabled on the host
4. Ensure VLAN IDs match across all components

---

## Performance Tips

### Understanding VLAN Overhead

**VLAN tagging impact**:
- Adds 4 bytes per network packet (802.1Q header)
- Minimal CPU overhead
- Generally no noticeable performance impact
- Modern hardware handles VLAN tagging efficiently

**For maximum performance**:
- Use non-VLAN networks if isolation is not required
- Hardware VLAN offloading can improve performance (ask your administrator)
- Consider network isolation needs vs performance requirements

---

### Network Performance Considerations

**Optimize your network setup**:
- Use VLANs only when you need network isolation
- Keep the number of networks manageable
- Monitor VM network usage through the Metrics tab
- Plan network architecture before deploying many VMs

**Contact your system administrator for**:
- Bridge performance tuning
- Hardware offload configuration
- Network performance optimization
- Advanced networking features

---

## Quick Reference

### Network Actions

| Action | Status | Notes |
|--------|--------|-------|
| Automatic creation | Available | Happens automatically during VM creation |
| Manual creation | Available | Create networks before deploying VMs |
| View network details | Available | See which VMs are using the network |
| Search networks | Available | Use search box in Networks page |
| Filter by type | Available | Filter by Bridge or VLAN type |
| Delete network | Available | Only if no VMs are attached |
| Edit network | Not available | Delete and create new network instead |

---

### VLAN ID Ranges

| Range | Usage |
|-------|-------|
| None | No VLAN tagging |
| 1 | Often default VLAN (check switch) |
| 2-1000 | Typical usage range |
| 1001-4094 | Extended VLAN range |
| 4095 | Reserved (do not use) |

---

## Next Steps

- **[Manage Networks](manage-networks/)** - View, register, and delete networks
- **[Volumes](/docs/volumes/)** - Manage VM storage volumes
- **[Users](/docs/users/)** - Manage user accounts and access control
- **[Create VM](/docs/vm/create-vm/)** - Create VMs with network configuration
- **[VM Management](/docs/vm/manage-vm/)** - Manage VM network settings

---

## FAQ

**Q: How do networks get created?**
A: Networks are created automatically when you create a VM with network configuration through the web interface. You can also manually create networks before deploying VMs by going to the Networks page and clicking "Create Network".

**Q: Can I use multiple bridges?**
A: Yes! You can create networks using different bridge names (e.g., `fcbr0`, `fcbr1`, `br0`). Each bridge provides an independent network. Contact your administrator to see which bridges are available on your system.

**Q: Do I need a VLAN-aware switch?**
A: Only if you want to use VLAN tagging for network isolation. If you're not using VLANs, a standard network switch works fine.

**Q: Can VMs in different VLANs communicate with each other?**
A: Not directly. VMs in different VLANs are isolated from each other by design. They would need a router configured for inter-VLAN routing to communicate. This isolation provides security.

**Q: What happens if I delete a network?**
A: You cannot delete networks that have VMs attached to them. You must first stop and delete those VMs (or move them to a different network), then you can delete the network through the web interface. Deleting a network only removes it from the registry.

**Q: How do I move a VM to a different network?**
A: Currently, you need to create a new VM with the desired network settings. Live network migration while a VM is running is not yet supported. You can stop the VM, delete it, and recreate it with new network configuration.

**Q: Can I have VMs on both VLAN and non-VLAN networks?**
A: Yes! You can mix both types. VMs on a non-VLAN network can communicate with each other freely. VMs with VLAN tags are isolated by their VLAN ID from other networks.
