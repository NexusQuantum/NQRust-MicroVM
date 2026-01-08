# Templates Documentation Images

This directory contains screenshots and images for the Templates documentation.

## Required Images

### Navigation & Page Layout
- `nav-templates.png` - Sidebar navigation showing Templates menu item
- `page-layout.png` - Full Templates page showing header and template grid
- `page.png` - Templates page overview

### Template Cards
- `card-layout.png` - Single template card showing all information
- `template-in-list.png` - Template displayed in the grid
- `select-template.png` - Template card with Deploy button highlighted

### Create Template
- `create-button.png` - Create Template button in page header
- `create-name.png` - Template name input field
- `create-vcpu.png` - vCPU input field
- `create-memory.png` - Memory input field
- `create-kernel.png` - Kernel path input field
- `create-rootfs.png` - Rootfs path input field
- `create-review.png` - Configuration review before creation
- `create-submit.png` - Create Template button
- `create-success.png` - Success notification after creation

### Deploy VM
- `deploy-button.png` - Deploy VM button on template card
- `deploy-dialog.png` - Deploy VM dialog with name input
- `deploy-config.png` - Configuration summary in deploy dialog
- `deploy-submit.png` - Deploy VM button in dialog
- `deploy-creating.png` - VM in Creating state
- `deploy-booting.png` - VM in Booting state
- `deploy-running.png` - VM in Running state
- `deployed-vm.png` - VM detail page after deployment

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
