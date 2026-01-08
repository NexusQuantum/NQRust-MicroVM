# Networks Documentation Images

This directory contains screenshots and images for the Networks documentation.

## Required Images

### Navigation & Page Layout
- `nav-networks.png` - Sidebar navigation showing Networks menu item
- `page-layout.png` - Full Networks page showing header and network table
- `page.png` - Networks page overview

### Network Table
- `network-row.png` - Single network row showing all information
- `network-row-click.png` - Network row showing clickable state
- `network-list.png` - Full network table with multiple entries

### Register Network
- `register-button.png` - Register Network button in page header
- `register-dialog.png` - Register network dialog
- `bridge-field.png` - Bridge name input field
- `vlan-field.png` - VLAN ID input field (optional)
- `register-submit.png` - Register Network button
- `register-success.png` - Success notification after registration

### Network Details
- `detail-page.png` - Network detail page showing information and attached VMs
- `vm-link.png` - VM link in network details
- `attached-vms.png` - List of VMs attached to network

### Delete Network
- `delete-button.png` - Delete button in network actions
- `delete-confirm.png` - Delete confirmation dialog
- `delete-success.png` - Success notification after deletion
- `delete-error.png` - Error when trying to delete network with VMs

### Auto-Registration
- `auto-registered.png` - Network automatically registered during VM creation
- `vm-network-config.png` - VM creation wizard network configuration step

### VLAN Configuration
- `vlan-setup.png` - Host bridge VLAN configuration
- `vlan-tagged-network.png` - Network with VLAN tag
- `vlan-isolation.png` - Multiple VLANs on same bridge

## Image Guidelines

- **Format**: PNG (preferred) or JPG
- **Size**: Maximum 1920px width, optimize for web
- **Quality**: High enough to show UI elements clearly
- **Theme**: Match the UI theme (light/dark as configured)
- **Annotations**: Use red boxes/arrows to highlight important elements
- **Privacy**: Remove any sensitive data (IPs, real names, etc.)

## Taking Screenshots

1. **Full Page**: Use browser's full page screenshot or `Shift+Cmd+4` (Mac)
2. **Specific Element**: Crop to show only relevant UI component
3. **Highlighting**: Use red rectangle/arrow for important buttons/fields
4. **Consistency**: Use same browser zoom level for all screenshots

## Placeholder Images

Until real screenshots are added, the documentation will work but images won't display. Hugo will show broken image links in development mode.

## Adding Images

1. Take screenshot of the UI element
2. Save with appropriate filename (see list above)
3. Place in this directory
4. No code changes needed - documentation already references these images

## VLAN Diagram Examples

For VLAN documentation, consider creating network diagrams showing:
- Bridge with multiple VLAN-tagged TAP devices
- VM connections to different VLANs
- Switch trunk port configuration
- VLAN isolation between VMs

Tools for diagrams:
- draw.io
- Excalidraw
- Mermaid (text-based)
- GraphViz
