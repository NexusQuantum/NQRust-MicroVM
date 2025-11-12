# NQRust MicroVM - Networking Documentation

**Last Updated:** 2025-11-03

This document provides comprehensive information about networking features in NQRust-MicroVM, including network management, VLAN support, auto-registration, and best practices.

---

## Table of Contents

- [Overview](#overview)
- [Network Architecture](#network-architecture)
- [Bridge Networking](#bridge-networking)
- [VLAN Support](#vlan-support)
- [Network Registry](#network-registry)
- [Auto-Registration](#auto-registration)
- [Network Configuration](#network-configuration)
- [API Reference](#api-reference)
- [Agent Integration](#agent-integration)
- [UI Features](#ui-features)
- [Common Use Cases](#common-use-cases)
- [Troubleshooting](#troubleshooting)

---

## Overview

NQRust-MicroVM provides production-ready networking features with:

- **Bridge Networking**: Connect VMs via Linux bridge devices
- **VLAN Support**: Network isolation using VLAN tagging (802.1Q)
- **Central Registry**: Track all networks across hosts
- **Auto-Registration**: Automatically register networks when VMs are created
- **TAP Devices**: Dedicated TAP interface per VM
- **Network Modes**: NAT (isolated) and Bridged (network-visible)

---

## Network Architecture

### Components

```
┌─────────────────────────────────────────────────────────────┐
│                         Host System                          │
│                                                               │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐    │
│  │  VM 1    │  │  VM 2    │  │  VM 3    │  │  VM 4    │    │
│  │          │  │          │  │          │  │          │    │
│  │  eth0    │  │  eth0    │  │  eth0    │  │  eth0    │    │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘    │
│       │             │             │             │           │
│  ┌────▼─────┐  ┌────▼─────┐  ┌────▼─────┐  ┌────▼─────┐    │
│  │tap-vm-1  │  │tap-vm-2  │  │tap-vm-3  │  │tap-vm-4  │    │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘    │
│       │             │             │             │           │
│       │    VLAN 10  │    VLAN 10  │    VLAN 20  │           │
│       └─────────┬───┴─────────────┴─────────┬───┘           │
│                 │                           │               │
│          ┌──────▼─────────┐         ┌───────▼──────┐        │
│          │  fcbr0 (main)  │         │  fcbr1       │        │
│          │  Bridge        │         │  Bridge      │        │
│          └──────┬─────────┘         └───────┬──────┘        │
│                 │                           │               │
│          ┌──────▼─────────┐         ┌───────▼──────┐        │
│          │  eth0 (uplink) │         │  eth1        │        │
│          │  Physical NIC  │         │  Physical NIC│        │
│          └────────────────┘         └──────────────┘        │
│                 │                           │               │
└─────────────────┼───────────────────────────┼───────────────┘
                  │                           │
                  ▼                           ▼
           External Network             External Network
           (192.168.1.0/24)             (10.0.0.0/24)
```

### Network Flow

1. **VM Network Interface**: Each VM has `eth0` (or multiple NICs)
2. **TAP Device**: Each VM NIC connects to a dedicated TAP interface (`tap-{vm-id}`)
3. **Bridge**: TAP devices connect to Linux bridge (`fcbr0`, `fcbr1`, etc.)
4. **VLAN Tagging**: Optional 802.1Q VLAN tags for network isolation
5. **Physical Uplink**: Bridge connects to physical NIC for external connectivity

---

## Bridge Networking

### Bridge Setup

Create a network bridge using the setup script:

```bash
# NAT mode (isolated network with masquerading)
sudo ./scripts/fc-bridge-setup.sh fcbr0 eth0

# Bridged mode (VMs visible on network)
sudo ./scripts/fc-bridge-setup.sh fcbr0 eth0 bridged
```

### Bridge Configuration

The bridge setup script:
- Creates the bridge device (`fcbr0`)
- Assigns an IP address (default: `192.168.18.240/24`)
- Configures IP forwarding
- Sets up iptables rules for NAT (if NAT mode)
- Optionally bridges to physical interface (if bridged mode)

### Multiple Bridges

You can create multiple bridges for network segmentation:

```bash
# Primary network
sudo ./scripts/fc-bridge-setup.sh fcbr0 eth0

# Development network
sudo ./scripts/fc-bridge-setup.sh fcbr1 eth1

# Production network
sudo ./scripts/fc-bridge-setup.sh fcbr2 eth2
```

---

## VLAN Support

### Overview

VLANs (Virtual LANs) provide network isolation at Layer 2 using 802.1Q tagging:

- **VLAN IDs**: 1-4094 (1-1005 normal, 1006-4094 extended)
- **Isolation**: VMs in different VLANs cannot communicate (unless routed)
- **Flexibility**: Multiple VLANs per bridge
- **Performance**: Hardware VLAN offload support (if NIC supports it)

### Creating VLAN Networks

#### Via Agent API

```bash
# Create a network with VLAN 10
curl -X POST http://localhost:9090/agent/v1/net/create \
  -H "Content-Type: application/json" \
  -d '{
    "bridge": "fcbr0",
    "tap": "tap-vm-1",
    "vlan_id": 10
  }'
```

#### Automatic (during VM creation)

The agent automatically handles VLAN tagging when creating TAP interfaces for VMs if the network has a VLAN ID configured.

### VLAN Use Cases

1. **Multi-Tenancy**: Isolate customer VMs on shared infrastructure
2. **Environment Separation**: Development, staging, production networks
3. **Security Zones**: DMZ, internal, management networks
4. **Service Segmentation**: Database, application, cache networks

### VLAN Best Practices

- **Plan VLAN IDs**: Document your VLAN allocation strategy
- **Physical Switch Config**: Ensure physical switches allow VLAN tags
- **Trunk Ports**: Configure uplink ports as 802.1Q trunk ports
- **Default VLAN**: Reserve VLAN 1 for management/untagged traffic
- **Documentation**: Maintain VLAN-to-purpose mapping

---

## Network Registry

### Overview

The network registry provides centralized tracking of all networks across hosts:

- **Network Discovery**: Automatic registration when VMs are created
- **VLAN Tracking**: Track which VLANs are in use
- **VM Count**: Monitor how many VMs use each network
- **Host Association**: Track which host manages which networks
- **Configuration Storage**: CIDR, gateway, VLAN ID, bridge name

### Database Schema

```sql
CREATE TABLE network (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    type VARCHAR(50) NOT NULL,  -- 'bridge', 'vlan', 'overlay'
    vlan_id INT,  -- 1-4094 for VLAN networks
    bridge_name VARCHAR(255) NOT NULL,
    host_id UUID REFERENCES host(id),
    cidr VARCHAR(50),  -- e.g., '192.168.18.0/24'
    gateway VARCHAR(50),  -- e.g., '192.168.18.1'
    created_at TIMESTAMPTZ DEFAULT now(),
    updated_at TIMESTAMPTZ DEFAULT now()
);

CREATE TABLE vm_network_interface (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    vm_id UUID NOT NULL REFERENCES vm(id) ON DELETE CASCADE,
    network_id UUID REFERENCES network(id) ON DELETE SET NULL,
    iface_id VARCHAR(255) NOT NULL,
    guest_mac VARCHAR(17),
    host_dev_name VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT now()
);
```

### Network Types

- **`bridge`**: Standard Linux bridge network
- **`vlan`**: VLAN-tagged network (future: separate type for VLAN)
- **`overlay`**: Reserved for future overlay networks (VXLAN, etc.)

---

## Auto-Registration

### How It Works

When a VM is created, the manager automatically:

1. **Checks existing networks** for the specified bridge and host
2. **Creates network record** if it doesn't exist
3. **Associates VM** with the network via `vm_network_interface` table
4. **Updates statistics** (VM count per network)

### Auto-Registration Flow

```rust
// apps/manager/src/features/vms/service.rs

async fn ensure_network_registered(
    st: &AppState,
    bridge_name: &str,
    host_id: Uuid
) -> Result<()> {
    // Check if network already exists
    let existing = network_repo.list_by_host(host_id).await?;

    for network in existing {
        if network.bridge_name == bridge_name {
            return Ok(()); // Already registered
        }
    }

    // Create new network record
    let name = format!("{} Network", bridge_name);
    network_repo.create(
        &name,
        Some("Auto-registered from VM creation"),
        "bridge",
        None,  // VLAN ID (if applicable)
        bridge_name,
        host_id,
        None,  // CIDR
        None,  // Gateway
    ).await?;

    Ok(())
}
```

### Benefits

- **Zero Configuration**: Networks appear automatically
- **Consistency**: No manual network creation needed
- **Discoverability**: See which networks are in use
- **Monitoring**: Track VM count per network
- **Cleanup**: Identify unused networks

---

## Network Configuration

### Manager Configuration

```bash
# Environment variables
MANAGER_BRIDGE=fcbr0  # Default bridge name
```

### Agent Configuration

```bash
# Agent environment variables
FC_BRIDGE=fcbr0  # Network bridge name

# Agent registers with manager on startup
MANAGER_BASE=http://192.168.18.240:18080
```

### VM Network Configuration

During VM creation, specify network configuration:

```json
{
  "name": "my-vm",
  "vcpu_count": 2,
  "mem_size_mib": 512,
  "kernel_image_path": "/srv/images/vmlinux-5.10.bin",
  "rootfs_image_id": "550e8400-e29b-41d4-a716-446655440000",
  "network": {
    "bridge": "fcbr0",
    "vlan_id": 10  // Optional VLAN tag
  }
}
```

---

## API Reference

### Manager API

#### List Networks

```http
GET /v1/networks
```

**Response:**
```json
[
  {
    "id": "b1ed8ab4-cf82-4401-8393-6638cf978dd5",
    "name": "fcbr0 Network",
    "description": "Auto-registered from VM creation",
    "type": "bridge",
    "vlan_id": null,
    "bridge_name": "fcbr0",
    "host_id": "bbab8c75-f516-47ec-987a-828422b2ee5a",
    "cidr": "192.168.18.0/24",
    "gateway": "192.168.18.240",
    "created_at": "2025-11-03T08:00:00Z",
    "updated_at": "2025-11-03T08:00:00Z"
  }
]
```

#### Get Network Details

```http
GET /v1/networks/{network_id}
```

**Response:**
```json
{
  "id": "b1ed8ab4-cf82-4401-8393-6638cf978dd5",
  "name": "fcbr0 Network",
  "description": "Auto-registered from VM creation",
  "type": "bridge",
  "vlan_id": null,
  "bridge_name": "fcbr0",
  "host_id": "bbab8c75-f516-47ec-987a-828422b2ee5a",
  "cidr": "192.168.18.0/24",
  "gateway": "192.168.18.240",
  "created_at": "2025-11-03T08:00:00Z",
  "updated_at": "2025-11-03T08:00:00Z"
}
```

#### Create Network

```http
POST /v1/networks
Content-Type: application/json

{
  "name": "Production Network",
  "description": "Production environment network",
  "network_type": "bridge",
  "vlan_id": 100,
  "bridge_name": "fcbr0",
  "host_id": "bbab8c75-f516-47ec-987a-828422b2ee5a",
  "cidr": "10.100.0.0/24",
  "gateway": "10.100.0.1"
}
```

#### Update Network

```http
PATCH /v1/networks/{network_id}
Content-Type: application/json

{
  "name": "Production Network (Updated)",
  "description": "Updated description",
  "cidr": "10.100.0.0/23",
  "gateway": "10.100.0.1"
}
```

#### Delete Network

```http
DELETE /v1/networks/{network_id}
```

**Note**: Cannot delete networks with attached VMs.

#### Get VM Count

```http
GET /v1/networks/{network_id}/vms/count
```

**Response:**
```json
{
  "count": 5
}
```

### Agent API

#### Create TAP with VLAN

```http
POST /agent/v1/net/create
Content-Type: application/json

{
  "bridge": "fcbr0",
  "tap": "tap-vm-12345",
  "vlan_id": 10
}
```

#### Delete TAP

```http
POST /agent/v1/net/delete
Content-Type: application/json

{
  "tap": "tap-vm-12345"
}
```

---

## Agent Integration

### Network Management in Agent

The agent (`apps/agent/src/features/tap/mod.rs`) handles:

1. **TAP Device Creation**: Create TAP interfaces for VMs
2. **Bridge Attachment**: Attach TAP to specified bridge
3. **VLAN Tagging**: Configure VLAN ID if specified
4. **Cleanup**: Remove TAP devices when VMs are deleted

### VLAN Configuration

```rust
// apps/agent/src/features/tap/mod.rs

pub async fn create_tap(
    bridge: &str,
    tap: &str,
    vlan_id: Option<u16>
) -> Result<()> {
    // Create TAP device
    let output = Command::new("ip")
        .args(&["tuntap", "add", "dev", tap, "mode", "tap"])
        .output()?;

    // Set up
    Command::new("ip")
        .args(&["link", "set", tap, "up"])
        .output()?;

    // Attach to bridge with optional VLAN
    if let Some(vlan) = vlan_id {
        // Attach with VLAN tag
        Command::new("bridge")
            .args(&["vlan", "add", "dev", tap, "vid", &vlan.to_string()])
            .output()?;

        Command::new("ip")
            .args(&["link", "set", tap, "master", bridge])
            .output()?;
    } else {
        // Attach without VLAN
        Command::new("ip")
            .args(&["link", "set", tap, "master", bridge])
            .output()?;
    }

    Ok(())
}
```

### Network Interface Configuration

The agent configures VM network interfaces via Firecracker API:

```rust
// Configure NIC in Firecracker
let nic_config = NetworkInterface {
    iface_id: "eth0",
    host_dev_name: "tap-vm-12345",
    guest_mac: Some("AA:FC:00:00:00:01"),
    rx_rate_limiter: None,
    tx_rate_limiter: None,
};
```

---

## UI Features

### Networks Page

Location: `apps/ui/app/(dashboard)/networks/page.tsx`

Features:
- **List all networks** across all hosts
- **Search and filter** by name, type, bridge, host
- **View network details** (CIDR, gateway, VLAN ID)
- **VM count** per network
- **Create new networks** with optional VLAN
- **Edit network** name, description, CIDR, gateway
- **Delete networks** (if no VMs attached)

### Network Table

Component: `apps/ui/components/network/network-table.tsx`

Columns:
- **Name**: Network name
- **Type**: bridge, vlan, overlay
- **Bridge**: Linux bridge device name
- **VLAN**: VLAN ID (if configured)
- **CIDR**: Network address range
- **Gateway**: Gateway IP address
- **VMs**: Number of VMs using this network
- **Host**: Associated host name
- **Actions**: Edit, Delete

### Network Creation Dialog

Component: `apps/ui/components/network/network-create-dialog.tsx`

Fields:
- **Name**: Network name (required)
- **Description**: Network description (optional)
- **Type**: Network type (bridge, vlan, overlay)
- **Bridge Name**: Linux bridge device (required)
- **VLAN ID**: Optional VLAN tag (1-4094)
- **Host**: Associated host (required)
- **CIDR**: Network address range (optional)
- **Gateway**: Gateway IP address (optional)

---

## Common Use Cases

### Use Case 1: Multi-Tenant Environment

**Goal**: Isolate customer VMs on shared infrastructure

**Solution**:
```bash
# Customer A: VLAN 100
curl -X POST http://localhost:18080/v1/networks \
  -d '{
    "name": "Customer A",
    "bridge_name": "fcbr0",
    "vlan_id": 100,
    "cidr": "10.100.0.0/24"
  }'

# Customer B: VLAN 200
curl -X POST http://localhost:18080/v1/networks \
  -d '{
    "name": "Customer B",
    "bridge_name": "fcbr0",
    "vlan_id": 200,
    "cidr": "10.200.0.0/24"
  }'
```

**Result**: VMs in VLAN 100 cannot communicate with VMs in VLAN 200.

### Use Case 2: Environment Segmentation

**Goal**: Separate dev, staging, and production environments

**Solution**:
```bash
# Development: VLAN 10
# Staging: VLAN 20
# Production: VLAN 30
```

Configure VMs with appropriate VLAN during creation.

### Use Case 3: Service-Oriented Networks

**Goal**: Segment by application tier

**Solution**:
- **Web Tier**: VLAN 10 (public-facing)
- **App Tier**: VLAN 20 (internal)
- **DB Tier**: VLAN 30 (restricted)
- **Cache Tier**: VLAN 40 (internal)

Use firewall rules to control inter-VLAN routing.

### Use Case 4: Multiple Physical Networks

**Goal**: Connect VMs to different physical networks

**Solution**:
```bash
# Create multiple bridges
sudo ./scripts/fc-bridge-setup.sh fcbr0 eth0  # Internet-facing
sudo ./scripts/fc-bridge-setup.sh fcbr1 eth1  # Internal network
sudo ./scripts/fc-bridge-setup.sh fcbr2 eth2  # Management network

# Create VMs on different bridges
# Internet-facing VMs use fcbr0
# Internal VMs use fcbr1
# Management VMs use fcbr2
```

---

## Troubleshooting

### Issue: VMs Not Getting IP Addresses

**Symptoms**: VMs boot but don't get IP via DHCP

**Diagnosis**:
```bash
# Check bridge configuration
ip addr show fcbr0

# Check DHCP server (if running dnsmasq)
systemctl status dnsmasq

# Check iptables rules
sudo iptables -t nat -L -n -v
```

**Solution**:
- Ensure bridge has IP address
- Verify DHCP server is running on bridge network
- Check firewall rules allow DHCP (UDP 67/68)

### Issue: VLAN Traffic Not Working

**Symptoms**: VMs in VLAN can't communicate

**Diagnosis**:
```bash
# Check VLAN configuration on bridge
bridge vlan show dev fcbr0

# Check TAP VLAN configuration
bridge vlan show dev tap-vm-12345

# Check physical switch port configuration
```

**Solution**:
- Ensure physical switch port is configured as trunk (802.1Q)
- Verify VLAN IDs match on switch and host
- Check VLAN is allowed on trunk port

### Issue: Network Not Auto-Registering

**Symptoms**: Networks don't appear in registry after VM creation

**Diagnosis**:
```bash
# Check manager logs
grep "auto-register" /var/log/manager.log

# Check database
psql $DATABASE_URL -c "SELECT * FROM network;"
```

**Solution**:
- Verify bridge name matches between agent and VM creation
- Check manager logs for auto-registration errors
- Ensure host_id is valid

### Issue: Cannot Delete Network

**Symptoms**: Network deletion fails with error

**Diagnosis**:
```bash
# Check VMs using network
curl http://localhost:18080/v1/networks/{network_id}/vms/count
```

**Solution**:
- Delete or migrate VMs using the network first
- Network can only be deleted when VM count is 0

### Issue: TAP Device Creation Fails

**Symptoms**: VM creation fails during TAP setup

**Diagnosis**:
```bash
# Check agent logs
journalctl -u agent -f

# Check for existing TAP device
ip link show tap-vm-12345

# Check bridge exists
ip link show fcbr0
```

**Solution**:
- Ensure bridge exists before creating VMs
- Check agent has CAP_NET_ADMIN capability
- Verify no duplicate TAP device names

---

## Performance Considerations

### Bridge Performance

- **Throughput**: Linux bridges can handle 10+ Gbps with modern hardware
- **Latency**: Minimal overhead (< 1ms) for intra-host traffic
- **CPU Usage**: Efficient packet forwarding, low CPU overhead

### VLAN Performance

- **Hardware Offload**: Modern NICs support VLAN offload (no CPU overhead)
- **Software VLAN**: Still performant, minimal CPU usage
- **Scalability**: Support 4094 VLANs per bridge

### Optimization Tips

1. **Enable Hardware Offload**:
   ```bash
   ethtool -K eth0 tx-vlan-offload on
   ethtool -K eth0 rx-vlan-offload on
   ```

2. **Increase TX Queue**:
   ```bash
   ip link set fcbr0 txqueuelen 10000
   ```

3. **Tune Bridge Parameters**:
   ```bash
   echo 0 > /sys/class/net/fcbr0/bridge/multicast_snooping
   ```

4. **Use Multiple Bridges**: Distribute VMs across multiple bridges for load balancing

---

## Security Best Practices

1. **VLAN Isolation**: Use VLANs to segment sensitive workloads
2. **Firewall Rules**: Implement iptables/nftables rules between VLANs
3. **MAC Filtering**: Enable MAC address filtering on bridges
4. **DHCP Snooping**: Prevent rogue DHCP servers
5. **ARP Inspection**: Prevent ARP spoofing attacks
6. **Private VLANs**: Use PVLAN for additional isolation
7. **Rate Limiting**: Apply rate limiters to prevent DoS attacks

---

## Future Enhancements

Planned networking features:

1. **Overlay Networks**: VXLAN, Geneve support for multi-host networking
2. **Network Policies**: Define firewall rules per network
3. **Network Templates**: Pre-configured network setups
4. **Network Monitoring**: Bandwidth usage, packet statistics
5. **Dynamic VLAN Assignment**: Automatic VLAN allocation
6. **IPv6 Support**: Dual-stack networking
7. **Service Discovery**: DNS-based service discovery
8. **Load Balancer Integration**: Direct integration with LB

---

## References

- **Linux Bridge**: https://wiki.linuxfoundation.org/networking/bridge
- **VLAN (802.1Q)**: https://en.wikipedia.org/wiki/IEEE_802.1Q
- **Firecracker Networking**: https://github.com/firecracker-microvm/firecracker/blob/main/docs/network-setup.md
- **ip Command**: https://man7.org/linux/man-pages/man8/ip.8.html
- **bridge Command**: https://man7.org/linux/man-pages/man8/bridge.8.html

---

**End of Networking Documentation**
