+++
title = "Manage Networks"
description = "Complete guide to viewing, creating, and managing virtual networks through the web interface"
weight = 61
date = 2025-01-08
+++

This guide will show you how to manage your virtual networks using the web interface. You'll learn how to view the network registry, create new networks, monitor VM attachments, and delete unused networks - all through simple point-and-click operations.

---

## Accessing Networks

### Navigate to Networks Page

![Image: Networks navigation](/images/networks/nav-networks.png)

Click **"Networks"** in the sidebar to access the Networks page.

### Networks Page Layout

![Image: Networks page](/images/networks/page-layout-networks.png)

The page shows:
- **Header** with register button
- **Network count** in card header
- **Network table** with columns:
  - Network Name (bridge + VLAN)
  - Bridge Name
  - VLAN ID
  - Attached VMs count
  - Created date
  - Actions

---

## Network Registry

### Network Table Information

Each network entry displays:

![Image: Network table row](/images/networks/network-row.png)

**Network details**:
- **Network Name** - Auto-generated name (e.g., `fcbr0` or `fcbr0-vlan-10`)
- **Bridge Name** - Linux bridge interface (e.g., `fcbr0`)
- **VLAN ID** - Optional VLAN tag (empty if no VLAN)
- **Attached VMs** - Number of VMs using this network
- **Created** - When network was registered
- **Actions** - View details, delete

**Example rows**:
```
Network Name      Bridge    VLAN ID    Attached VMs    Created         Actions
fcbr0            fcbr0     -          5               Jan 8, 2025    [View] [Delete]
fcbr0-vlan-10    fcbr0     10         3               Jan 8, 2025    [View] [Delete]
fcbr0-vlan-20    fcbr0     20         2               Jan 8, 2025    [View] [Delete]
```

---

### Browsing Networks

**Table features**:
- Networks displayed in table format
- Sorted by creation date (newest first)
- Search and filter (future feature)
- Pagination for large lists (future feature)

**Quick identification**:
- No VLAN ID → Simple bridge network
- With VLAN ID → Isolated VLAN network
- High VM count → Heavily used network
- Zero VMs → Unused, can be deleted

---

## Automatic Network Creation

Networks are automatically created when you create VMs with network configuration.

### How Automatic Creation Works

**Scenario**: Creating a VM with network settings

**Step 1**: Configure VM network in the creation wizard

![Image: VM network configuration](/images/networks/vm-network-config.png)

In the VM creation wizard, you specify:
- Bridge: `fcbr0`
- VLAN: `10`

---

**Step 2**: VM creation triggers automatic network creation

**What happens behind the scenes**:
1. System checks if network `fcbr0` with VLAN 10 already exists
2. If not found, system automatically creates the network
3. Network is saved to the registry
4. VM is linked to this network
5. VM starts and connects to the network

---

**Step 3**: Verify network was created

![Image: Auto-created network](/images/networks/auto-registered.png)

The new network appears in the Networks page:
- Network Name: `fcbr0-vlan-10`
- Bridge: `fcbr0`
- VLAN: `10`
- Attached VMs: `1`

---

### Benefits of Automatic Creation

**No manual work required**:
- No need to pre-create networks
- Simply create VMs with your desired network settings
- Networks are created automatically as needed

**Centralized tracking**:
- All networks are tracked in one place
- Easy to see which networks are in use
- Simplifies network planning and management

**Smart duplicate prevention**:
- Same bridge + VLAN combination → Reuses existing network
- Different VLAN → Creates new network entry automatically

---

## Creating Networks Manually

You can create networks manually before creating VMs.

### When to Create Networks Manually

**Use cases**:
- Planning network architecture in advance
- Pre-creating networks for your team
- Documenting network inventory
- Setting up VLAN scheme before VM deployment

---

### Step 1: Open Create Network Dialog

![Image: Create network button](/images/networks/register-button.png)

Click the **"Create Network"** button in the page header.

The create network dialog opens:

![Image: Create network dialog](/images/networks/register-dialog.png)

---

### Step 2: Enter Bridge Name

![Image: Bridge name field](/images/networks/bridge-field.png)

Enter the Linux bridge name:

**Common bridge names**:
- `fcbr0` - Firecracker default bridge
- `br0` - Standard Linux bridge
- `virbr0` - Libvirt default bridge

**Requirements**:
- Bridge must exist on the host system
- Typically created during system setup
- Common bridges: `fcbr0`, `br0`, `fcbr1`

**Note**: Bridge devices are managed by your system administrator. If you're unsure which bridges are available, contact your administrator or check your system documentation.

---

### Step 3: Enter VLAN ID (Optional)

![Image: VLAN ID field](/images/networks/vlan-field.png)

Optionally enter VLAN ID for isolation:

**VLAN ID range**:
- Leave empty for no VLAN tagging
- 1-4094 for VLAN tagging
- 1 often reserved for default VLAN
- 4095 reserved by 802.1Q standard

**Examples**:
```
No VLAN:     Leave empty
Development: 10
Staging:     20
Production:  30
Customer A:  100
Customer B:  200
```

**VLAN requirements**:
- Host bridge must support VLAN filtering
- Physical switch must support 802.1Q
- Switch port must be configured as trunk
- VLAN must be allowed on trunk port

---

### Step 4: Create the Network

![Image: Create button](/images/networks/register-submit.png)

Click the **"Create Network"** button:

**What happens**:
1. The form validates your input
2. Network is created and saved
3. Success notification appears
4. Network appears in the table
5. Dialog closes automatically

**Success**:
![Image: Success notification](/images/networks/register-success.png)

Your network is now ready to use with your VMs!

---

### Example: Create Development Network

**Scenario**: Create VLAN 10 for development VMs

**Configuration**:
- Bridge Name: `fcbr0`
- VLAN ID: `10`

**Steps**:
1. Click "Create Network" button
2. Enter bridge name: `fcbr0`
3. Enter VLAN ID: `10`
4. Click "Create Network" to save

**Result**:
- Network `fcbr0-vlan-10` is created
- Ready for VM assignment
- Visible in Networks page

---

### Example: Create Multi-Tenant Networks

**Scenario**: Create isolated networks for 3 customers

**Customer A**:
- Bridge: `fcbr0`
- VLAN: `100`

**Customer B**:
- Bridge: `fcbr0`
- VLAN: `200`

**Customer C**:
- Bridge: `fcbr0`
- VLAN: `300`

**Result**: 3 isolated networks, customers cannot intercommunicate.

---


## Delete Network

Remove unused networks from registry.

### When to Delete

**Delete networks when**:
- Network no longer needed
- All VMs removed from network
- Cleaning up network inventory
- Consolidating network architecture

**Cannot delete if**:
- Network has attached VMs
- Must detach VMs first (stop/delete VMs)

---

### Step 1: Select Network

![Image: Network with delete button](/images/networks/delete-button-network.png)

Find network to delete in table:
- Check **Attached VMs** column is `0`
- Click **Delete** button in Actions column

---

### Step 2: Confirm Deletion

![Image: Delete confirmation dialog](/images/networks/delete-confirm-network.png)

Confirmation dialog appears:

```
Delete Network?

Are you sure you want to delete "fcbr0-vlan-10"?

This will:
- Remove network from database
- NOT affect the Linux bridge on host
- NOT affect VMs (if network has no VMs)
- Cannot be undone

[Cancel]  [Delete Network]
```

**Important notes**:
- Only removes database entry
- Bridge on host remains (must delete manually)
- Network can be re-registered later

---

### Step 3: Deletion Complete

![Image: Delete success](/images/networks/delete-success.png)

**Success notification**:
```
Network Deleted
fcbr0-vlan-10 has been removed successfully
```

Network disappears from table.

---

### Error: Cannot Delete Network

![Image: Delete error - VMs attached](/images/networks/delete-error.png)

**Error message**:
```
Cannot Delete Network
Network has 3 attached VMs. Remove VMs first.
```

**Solution**:
1. View network details to see attached VMs
2. Stop and delete VMs using the network
3. Or reconfigure VMs to use different network
4. Then delete network

---

## Network Usage Tracking

### Check Network Usage

**Networks page shows**:
- Number of attached VMs per network
- Helps identify heavily used networks
- Helps find unused networks

**Example usage patterns**:
```
fcbr0              - 10 VMs (default network, heavily used)
fcbr0-vlan-10      - 5 VMs  (development environment)
fcbr0-vlan-20      - 3 VMs  (staging environment)
fcbr0-vlan-30      - 2 VMs  (production environment)
fcbr0-vlan-99      - 0 VMs  (unused, can delete)
```

---

### Network Detail Page

**Shows VM list**:
- VM names
- VM states
- VM IDs
- Links to VM pages

**Useful for**:
- Security audits (check which VMs in same network)
- Troubleshooting connectivity
- Capacity planning
- Network migration planning

---

## Common Tasks

### Task: Set Up Environment Networks

**Scenario**: Create isolated networks for dev/staging/prod

**Steps**:
1. Navigate to Networks page
2. Click "Create Network" button
3. Create development network:
   - Bridge: `fcbr0`
   - VLAN: `10`
   - Click "Create Network"
4. Create staging network:
   - Bridge: `fcbr0`
   - VLAN: `20`
   - Click "Create Network"
5. Create production network:
   - Bridge: `fcbr0`
   - VLAN: `30`
   - Click "Create Network"

**Result**: 3 isolated networks ready for VM deployment

**Usage**:
- Assign dev VMs to VLAN 10
- Assign staging VMs to VLAN 20
- Assign prod VMs to VLAN 30
- Each environment is isolated from the others

---

### Task: Audit Network Usage

**Scenario**: Check which VMs are in production network

**Steps**:
1. Navigate to Networks page
2. Find `fcbr0-vlan-30` (production network)
3. Note **Attached VMs** count
4. Click network row to view details
5. Review VM list

**Result**: Complete list of production VMs in isolated network

**Use for**:
- Security compliance
- Change management
- Incident response
- Access control auditing

---

### Task: Clean Up Unused Networks

**Scenario**: Remove networks with no VMs

**Steps**:
1. Navigate to Networks page
2. Sort by **Attached VMs** column
3. Find networks with 0 VMs
4. Check creation date (old and unused?)
5. Click Delete button
6. Confirm deletion

**Result**: Clean network inventory, easier to manage

---

### Task: Multi-Tenant Setup

**Scenario**: Host VMs for 3 customers with complete isolation

**Steps**:
1. Plan VLAN allocation:
   - Customer A: VLAN 100
   - Customer B: VLAN 200
   - Customer C: VLAN 300

2. Create networks through the web interface:
   - Navigate to Networks page
   - Create network for Customer A (Bridge: fcbr0, VLAN: 100)
   - Create network for Customer B (Bridge: fcbr0, VLAN: 200)
   - Create network for Customer C (Bridge: fcbr0, VLAN: 300)

3. Assign VMs to networks:
   - Customer A's VMs → VLAN 100
   - Customer B's VMs → VLAN 200
   - Customer C's VMs → VLAN 300

**Result**: 3 isolated networks where customers cannot access each other's VMs

---

## Troubleshooting

### Issue: Network Not Appearing

**Symptoms**:
- Registered network doesn't show in table
- Network list is empty

**Possible causes**:
1. Page not loading
2. Connection issue with server
3. Registration failed
4. Server issue

**Solution**:
1. Refresh the page (press F5)
2. Check if success notification appeared after registration
3. Try logging out and logging back in
4. Check if other pages are loading properly
5. Contact your system administrator if issue persists

---

### Issue: Cannot Delete Network

**Symptoms**:
- Delete button disabled
- Error: "Network has attached VMs"

**Possible causes**:
- VMs still using the network

**Solution**:
1. Click network row to view details
2. Check attached VMs list
3. Stop and delete VMs, or reconfigure to different network
4. Then delete network

---

### Issue: Duplicate Network Names

**Symptoms**:
- Multiple networks with same name
- Confusion about which network to use

**Possible causes**:
- Auto-registration created duplicates (should not happen)
- Manual registration with same bridge+VLAN

**Solution**:
1. Check VLAN IDs carefully
2. Networks with different VLANs are different networks
3. If true duplicates exist, report as bug
4. Delete duplicates with 0 VMs attached

---

### Issue: Network Creation Fails

**Symptoms**:
- Click "Create Network" button
- Error notification appears
- Network is not created

**Possible causes**:
1. Required fields are empty (Name or Bridge)
2. Invalid VLAN ID (outside 1-4094 range)
3. Network with same configuration already exists
4. Connection issue with server
5. No host selected

**Solution**:
1. Check that Network Name field is filled
2. Check that Bridge Name field is filled
3. Verify Host is selected from dropdown
4. Check VLAN ID is a valid number (1-4094) or leave empty for no VLAN
5. Search the networks table to ensure no duplicate exists
6. Read the error message in the notification for specific details
7. Try refreshing the page and attempting again
8. Contact your system administrator if issue persists

---

### Issue: VMs Not Showing in Network Details

**Symptoms**:
- Network shows "Attached VMs: 0"
- But VMs using this network exist

**Possible causes**:
1. Data synchronization issue
2. VMs created before network registered
3. VMs using different bridge/VLAN combination

**Solution**:
1. Refresh the Networks page
2. Go to VMs page and check VM network configuration
3. Verify bridge name and VLAN ID match exactly between VM and network
4. Check if VM's bridge and VLAN settings match the network entry
5. Contact your system administrator if counts remain incorrect

---

## Best Practices

### 1. Plan VLAN Scheme

**Before creating networks**:
- Document VLAN allocation
- Reserve ranges for different purposes
- Share plan with team

**Example scheme**:
```
VLAN 1-9:    Reserved (don't use)
VLAN 10-19:  Development
VLAN 20-29:  Staging
VLAN 30-39:  Production
VLAN 100+:   Customer/tenant networks
```

---

### 2. Pre-Create Production Networks

**For production environments**:
- Manually create networks before deploying VMs
- Document purpose and intended use in external documentation
- Prevents accidental auto-creation
- Provides better planning and control

**Example Documentation**:
```
Network: fcbr0-vlan-30
Bridge: fcbr0
VLAN: 30
Purpose: Production web servers
Environment: Production
Owner: ops-team@company.com
Created: 2025-01-08
Last Reviewed: 2025-01-08
```

---

### 3. Regular Cleanup

**Periodically review networks**:
- Check for networks with 0 VMs
- Verify old networks still needed
- Delete unused networks
- Keep network registry clean

**Schedule**:
- Monthly: Review network usage
- Quarterly: Delete unused networks
- Annually: Review VLAN scheme

---

### 4. Document Network Purpose

**Keep external documentation**:
- Which network for which environment
- VLAN ID assignments
- IP subnet ranges (if using VLAN routing)
- Contact information

**Example documentation**:
```
Network: fcbr0-vlan-10
Purpose: Development environment
IP Range: 10.0.10.0/24
Gateway: 10.0.10.1
DHCP: 10.0.10.100-200
Contact: dev-team@company.com
Notes: Auto-assigned IPs via DHCP server on VLAN 10
```

---

### 5. Monitor Network Growth

**Track network count**:
- Check total networks regularly
- Identify unused networks quickly
- Plan bridge capacity
- Avoid network sprawl

**Red flags**:
- Many networks with 0 VMs
- Networks with unclear purpose
- Overlapping VLAN usage
- Undocumented networks

---

## Performance Tips

### Minimize Network Count

**Keep network list manageable**:
- Don't create networks unnecessarily
- Reuse networks where possible
- Delete unused networks promptly

**Benefits**:
- Easier to navigate
- Faster page load
- Simpler network management

---

### Use Auto-Creation

**Let the system create networks automatically**:
- Don't pre-create networks unless needed
- Auto-creation is efficient and convenient
- Prevents unused networks
- Reduces manual work

**Manual creation only when**:
- Planning network architecture in advance
- Documentation is required upfront
- Team coordination is needed
- Compliance or audit requirements exist

---

## Quick Reference

### Network Actions

| Action | Steps | Status |
|--------|-------|--------|
| View networks | Navigate to Networks page | Available |
| Create network | Click "Create Network" → Fill form → Create | Available |
| Search networks | Use search box at top of table | Available |
| Filter by type | Use type dropdown (Bridge/VLAN) | Available |
| View network details | Click network row | Available |
| Check attached VMs | View "VMs" column in table | Available |
| Delete network | Click Delete → Confirm (if 0 VMs) | Available |
| Edit network | Not available (delete and recreate) | Not available |

---

### Keyboard Shortcuts

| Action | Shortcut |
|--------|----------|
| Open create dialog | Click "Create Network" button |
| Move between fields | Tab |
| Submit form | Enter |
| Cancel dialog | Esc |

---

## Next Steps

- **[Networks Overview](./)** - Learn about network types and architecture
- **[Volumes](/docs/volumes/)** - Manage VM storage volumes
- **[Users](/docs/users/)** - Manage user accounts and access control
- **[Create VM](/docs/vm/create-vm/)** - Create VMs with network configuration
- **[VM Management](/docs/vm/manage-vm/)** - Manage VM network settings
