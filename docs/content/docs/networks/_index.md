+++
title = "Networks"
description = "Complete guide to virtual networks for VM connectivity through the web interface"
weight = 80
date = 2025-01-08
+++

Networks provide connectivity for your VMs. The platform automatically provisions the bridge, DHCP, and firewall rules on the host when you create a network.

---

## Network Types

### NAT
Private subnet with internet access via host NAT. VMs get DHCP addresses and can reach the internet through the host. Best for most workloads where VMs need outbound internet access without being directly reachable on the LAN.

### Isolated
Private subnet with no internet access. VMs can only communicate with each other. Ideal for air-gapped workloads, secure internal services, and environments that must not reach the internet.

### Bridged
Direct LAN access. A physical NIC is attached to a bridge, giving VMs addresses directly on your external network. The external network handles IP assignment — no CIDR, gateway, or DHCP is configured by the platform for bridged networks.

### VXLAN (Overlay)
Multi-host overlay network. VMs on different hosts communicate via VXLAN tunnels. A gateway host runs DHCP and NAT for the overlay. The VNI is auto-assigned and the overlay auto-expands to other hosts when VMs are created.

---

## Networks Page

Navigate to **Networks** in the sidebar to see all networks.

![Networks list page](/images/networks/networks-list.png)

The table shows:

| Column | Description |
|---|---|
| **Name** | Network name (lock icon = default/protected network) |
| **Type** | NAT, Isolated, Bridged, or VXLAN |
| **VLAN/VNI** | VLAN tag or VXLAN VNI (— if not set) |
| **Status** | Active (green) or Inactive |
| **Bridge** | Linux bridge interface on the host (e.g. `fcbr0`) |
| **CIDR** | Subnet range (— for Bridged networks) |
| **Host** | Agent URL managing this network |
| **VMs** | Number of VMs currently on this network |
| **Created** | Relative creation time |
| **Actions** | Edit (pencil) and Delete (trash) |

---

## Common Use Cases

### Environment segregation
```
NAT network — Development (10.0.1.0/24)
NAT network — Staging    (10.0.2.0/24)
NAT network — Production (10.0.3.0/24)
```

### Air-gapped workloads
```
Isolated network — Internal services with no internet access
```

### Direct LAN access
```
Bridged network — VMs get real IP addresses on your office/datacenter LAN
```

### Multi-host VM connectivity
```
VXLAN (Overlay) — VMs on Host A and Host B communicate as if on the same LAN
```

---

## Next Steps

- **[Manage Networks](manage-networks/)** — Create, edit, and delete networks
