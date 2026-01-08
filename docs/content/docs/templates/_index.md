+++
title = "VM Templates"
description = "Save and deploy VM configurations as reusable templates"
weight = 35
date = 2025-01-08
+++

VM Templates allow you to save VM configurations as reusable blueprints. Create templates with pre-configured CPU, memory, kernel, and rootfs settings, then deploy new VMs instantly without repeating the configuration process.

---

## What are Templates?

Templates are **saved VM configurations** that can be reused to quickly create new VMs. Instead of configuring each VM from scratch, you define the configuration once as a template and deploy multiple VMs from it.

### Key Benefits

**1. Time Savings**
- Configure once, deploy many times
- No need to repeat VM creation steps
- Deploy VMs in seconds instead of minutes

**2. Consistency**
- Ensure all VMs have identical configurations
- Reduce human error in manual configuration
- Standardize development/staging/production environments

**3. Organization**
- Group similar VM configurations
- Easy to find and reuse common setups
- Document your infrastructure as code

---

## Common Use Cases

### Development Environments

Create a template for your development environment with specific CPU, RAM, and base image:

```
Template: Dev Environment
- 2 vCPU
- 2048 MiB RAM
- Ubuntu 22.04 rootfs
- Pre-configured kernel
```

Deploy new development VMs for each team member or project instantly.

---

### Testing Infrastructure

Set up templates for different testing scenarios:

```
Template: Performance Test VM
- 4 vCPU
- 4096 MiB RAM
- Alpine Linux (lightweight)

Template: Integration Test VM
- 1 vCPU
- 512 MiB RAM
- Ubuntu 22.04
```

Spin up test environments on-demand, tear them down when done.

---

### Production Standardization

Maintain consistent production configurations:

```
Template: Web Server
- 2 vCPU
- 2048 MiB RAM
- Ubuntu 22.04 + NGINX

Template: Database Server
- 4 vCPU
- 8192 MiB RAM
- Ubuntu 22.04 + PostgreSQL
```

Deploy new instances with guaranteed consistency.

---

## Template Lifecycle

### 1. Create Template

Define your VM configuration:
- Template name (e.g., "Ubuntu 22.04 Base")
- vCPU count (1-32)
- RAM in MiB (128-16384)
- Kernel image path or ID
- Rootfs image path or ID

### 2. Store Template

Templates are saved in the database and can be:
- Listed and browsed
- Updated as needed
- Deleted when no longer needed

### 3. Deploy VMs

Use templates to create VMs:
- Click "Deploy VM" on any template
- Enter a VM name
- VM is created and started automatically
- Deployed VM inherits all template settings

### 4. Manage Template

Update or delete templates:
- Edit to change CPU, RAM, or images
- Delete templates you no longer need
- VMs created from deleted templates continue to work

---

## Template Components

### Resource Configuration

**CPU (vCPU)**
- Number of virtual CPUs
- Range: 1-32
- Determines VM processing power

**Memory (MiB)**
- RAM allocation in mebibytes
- Range: 128-16384 MiB
- Affects VM performance and capacity

---

### Boot Images

**Kernel Image**
- Linux kernel binary for VM boot
- Can specify by path or image registry ID
- Example: `/srv/images/vmlinux-5.10.fc.bin`

**Rootfs Image**
- Root filesystem containing OS
- Can be Ubuntu, Alpine, or custom images
- Example: `/srv/images/ubuntu-22.04.ext4`

---

## Template vs VM

| Feature | Template | VM |
|---------|----------|-----|
| Purpose | Configuration blueprint | Running instance |
| State | Static specification | Dynamic (running/stopped) |
| Resources | Defined but not allocated | Allocated on host |
| Lifecycle | Create, update, delete | Create, start, stop, delete |
| Cost | No resource usage | Uses host CPU/RAM |
| Deploy Time | N/A | ~30 seconds |

**Think of it like**: Template = Class, VM = Object Instance

---

## Quick Start

### 1. Navigate to Templates

![Image: Templates page navigation](/images/templates/nav-templates.png)

Click **"Templates"** in the sidebar.

---

### 2. Create Your First Template

![Image: Create template button](/images/templates/create-button.png)

1. Click **"Create Template"**
2. Enter template name
3. Set CPU and memory
4. Select kernel and rootfs images
5. Click **"Create Template"**

See [Create Template](create-template/) guide for details.

---

### 3. Deploy a VM

![Image: Deploy VM button](/images/templates/deploy-button.png)

1. Find your template in the list
2. Click **"Deploy VM"**
3. Enter VM name
4. Click **"Deploy VM"**

VM is created and started automatically!

See [Manage Templates](manage-templates/) guide for details.

---

## Templates Page Overview

The Templates page shows all your saved templates:

![Image: Templates page layout](/images/templates/page-layout.png)

**Page sections**:
- **Header** - Page title and description
- **Create Button** - Opens create template dialog
- **Template Cards** - Grid of all templates
- **Template Info** - CPU, RAM, creation date for each template
- **Deploy Button** - Quick deploy VM from template

---

## Template Properties

Each template stores:

**Basic Information**
- Template ID (UUID)
- Template name
- Created timestamp
- Updated timestamp

**Resource Spec**
- vCPU count
- Memory (MiB)
- Kernel image path or ID
- Rootfs image path or ID

**Metadata**
- VMs created from this template (tracked)
- Last deployment timestamp

---

## Template Limitations

### Current Limitations

❌ **Network configuration** - Not saved in templates (coming soon)
❌ **Additional drives** - Only rootfs included (coming soon)
❌ **Environment variables** - Not part of template spec
❌ **Tags/categories** - No organization system yet
❌ **Template sharing** - No export/import functionality

✅ **What works**:
- CPU and memory configuration
- Kernel and rootfs image selection
- Unlimited templates
- Deploy multiple VMs from one template
- Update and delete templates

---

## Best Practices

### Naming Templates

✅ **Good naming**:
- "Ubuntu 22.04 Base"
- "Alpine Dev Environment"
- "Production Web Server"
- "Test VM - 1 vCPU"

❌ **Poor naming**:
- "Template 1"
- "test"
- "asdf"
- "new template copy 2"

**Tip**: Include OS name and purpose in template name.

---

### Resource Sizing

**Development templates**:
- 1-2 vCPU
- 512-2048 MiB RAM
- Keep lightweight for faster deployment

**Production templates**:
- 2-4 vCPU
- 2048-8192 MiB RAM
- Size based on actual workload needs

**Testing templates**:
- 1 vCPU
- 512 MiB RAM
- Minimal resources for quick spin-up

---

### Image Management

✅ **Use consistent images**:
- Keep kernel and rootfs versions compatible
- Use tested image combinations
- Document which images work together

✅ **Use image registry**:
- Reference images by registry ID when possible
- Easier to update all templates at once
- Better tracking of image usage

---

## Troubleshooting

### Template won't create

**Check**:
- Template name is not empty
- vCPU is between 1-32
- Memory is between 128-16384 MiB
- Kernel and rootfs paths exist

---

### VM fails after deployment

**Possible causes**:
- Kernel or rootfs file not found on host
- Insufficient host resources
- Image files corrupted

**Solution**:
1. Verify image paths in template
2. Check host has enough CPU/RAM
3. Test images with manual VM creation first

---

### Template not showing in list

**Check**:
- Refresh the page
- Check browser console for errors
- Verify manager API is running
- Check template wasn't deleted

---

## Next Steps

- **[Create Template](create-template/)** - Step-by-step template creation guide
- **[Manage Templates](manage-templates/)** - Deploy, update, and delete templates
- **[Create VM](/docs/vm/create-vm/)** - Learn about manual VM creation
- **[VM Management](/docs/vm/manage-vm/)** - Managing deployed VMs

---

## FAQ

**Q: Can I create a template from an existing VM?**
A: Not yet. This feature is planned for a future release. For now, note the VM's configuration and create a new template manually.

**Q: What happens to VMs when I delete a template?**
A: VMs continue to run normally. They are independent once created.

**Q: Can I update a template after creating it?**
A: Yes! Edit the template and the changes will apply to future deployments (not existing VMs).

**Q: How many VMs can I deploy from one template?**
A: Unlimited, as long as your host has enough resources.

**Q: Can templates include network or storage configuration?**
A: Not yet. Currently templates only save CPU, RAM, kernel, and rootfs. Network and storage templates are planned features.
