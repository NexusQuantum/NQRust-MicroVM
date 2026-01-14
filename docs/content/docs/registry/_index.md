+++
title = "Image Registry"
description = "Complete guide to managing VM images, kernels, and root filesystems through the web interface"
weight = 50
date = 2025-01-13
+++

The Image Registry is your central hub for managing all VM images, including kernels and root filesystems. This guide will show you how to browse, import, and manage images using the web interface.

---

## What is the Image Registry?

The Image Registry stores and manages all images used to create VMs:

**Types of images**:
- **Kernel Images** - Linux kernel files for VM boot
- **Root Filesystems** - Operating system root filesystems
- **Container Runtime** - Specialized images for running containers

**Benefits**:
- Centralized image management
- Reuse images across multiple VMs
- Easy image discovery and selection
- Import from multiple sources

---

## Common Use Cases

### Operating System Images

Store different OS images for various use cases:

```
Ubuntu 22.04 LTS - General purpose servers
Alpine Linux 3.18 - Lightweight containers
Debian 12 - Stable production workloads
Fedora 39 - Latest features and packages
```

Each OS can be reused for multiple VMs without duplication.

---

### Kernel Versioning

Maintain multiple kernel versions:

```
Kernel 5.10 - Long-term support
Kernel 6.1 - Current stable
Kernel 6.6 - Latest features
```

Choose the right kernel for your workload requirements.

---

### Specialized Images

Create purpose-specific images:

```
Database Image - Pre-configured PostgreSQL/MySQL
Web Server Image - Nginx/Apache pre-installed
Dev Environment - Development tools included
Container Runtime - Docker-enabled image
```

---

## Image Types

### Kernel Images

**Purpose**: Boot the VM and provide core OS functionality

**Format**: Uncompressed kernel binary
**Typical size**: 5-20 MB
**Usage**: Required for every VM

**Common kernels**:
- `vmlinux-5.10` - LTS kernel
- `vmlinux-6.1` - Stable kernel
- `vmlinux-6.6` - Latest kernel

---

### Root Filesystem Images

**Purpose**: Provide the complete operating system

**Formats**:
- ext4 (recommended)
- qcow2 (compressed)
- raw (uncompressed)

**Typical size**: 500 MB - 5 GB
**Usage**: Required for every VM

**Common rootfs**:
- Ubuntu 22.04 LTS
- Alpine Linux 3.18
- Debian 12
- Fedora 39

---

### Container Runtime Images

**Purpose**: Enable Docker containers in VMs

**Contents**:
- Base OS (Alpine Linux)
- Docker daemon
- Container tools
- Init system

**Size**: ~400 MB
**Usage**: For container deployments only

---

## Accessing the Registry

### Navigate to Registry Page

Click **"Registry"** in the sidebar to access the Image Registry page.

![Image: Registry navigation](/images/registry/nav-registry.png)

### Registry Page Layout

The registry page displays:
- **Search bar** - Find images quickly
- **Filter dropdown** - Filter by image type
- **Action buttons** - Import new images
- **Image table** - List of all images

![Image: Registry page layout](/images/registry/page-layout.png)

---

## Registry Features

### Browse and Search

- View all available images
- Search by name or description
- Filter by type (kernel, rootfs)
- Sort by name, size, or date

### Import Images

Multiple import methods:
- **Upload** - Upload files from your computer
- **Import from Path** - Use existing files on the server
- **DockerHub** - Pull images from Docker Hub
- **Import from URL** - Download from web URL

### Manage Images

- **Delete** - Remove unused images
- **Rename** - Update image names
- **Copy Path** - Copy file paths for reference
- **View Details** - See image properties

---

## Image Properties

Each image entry displays:

**Basic Information**:
- Image name
- Image type (kernel/rootfs)
- File size
- Storage path

**Usage Information**:
- Number of VMs using this image
- Number of templates using this image
- Last used date
- Creation date

---

## Quick Start

### 1. View Available Images

Navigate to the Registry page to see all available images.

### 2. Search for Images

Use the search bar to find specific images:
- Search by OS name: "ubuntu"
- Search by version: "22.04"
- Search by purpose: "container"

### 3. Import New Images

Click **"Import Image"** to add new images:
- Upload from your computer
- Import from server path
- Pull from DockerHub
- Download from URL

### 4. Use Images in VMs

When creating a VM:
1. Select kernel from registry
2. Select rootfs from registry
3. VM is created using these images

---

## Storage Location

Images are stored on the server:

**Default location**: `/srv/images/`

**Organization**:
```
/srv/images/
├── kernels/
│   ├── vmlinux-5.10
│   └── vmlinux-6.1
├── rootfs/
│   ├── ubuntu-22.04.ext4
│   └── alpine-3.18.ext4
└── container-runtime/
    └── container-runtime.ext4
```

**Note**: Storage location is managed by your system administrator. Contact them for storage capacity and management questions.

---

## Best Practices

### 1. Organize Images by Purpose

Use clear naming conventions:

```
Good names:
- ubuntu-22.04-base
- debian-12-webserver
- alpine-3.18-container
- fedora-39-dev

Avoid:
- image1
- test
- my-image
- copy-of-ubuntu
```

---

### 2. Keep Commonly Used Images

Maintain images for:
- **Production OS** - Your standard OS
- **Development OS** - For testing
- **Container runtime** - If using containers
- **Multiple kernel versions** - For compatibility

---

### 3. Clean Up Unused Images

Regularly review and delete:
- Old OS versions no longer used
- Test images
- Duplicate images
- Images with 0 VMs attached

**Check before deleting**:
- Ensure no VMs are using the image
- Check if templates reference it
- Consider keeping one backup

---

### 4. Document Custom Images

Keep external notes for custom images:

```
Image: production-webserver-v2
Base: Ubuntu 22.04
Modifications:
  - Nginx 1.24 installed
  - SSL certificates configured
  - Custom logging setup
Purpose: Production web servers
Created: 2025-01-13
Maintained by: ops-team@company.com
```

---

## Security Considerations

### Image Sources

**Trusted sources**:
- Official OS vendors (Ubuntu, Debian, Alpine)
- Your organization's build system
- Verified DockerHub images

**Verify images** before use:
- Check checksums if available
- Test in development first
- Review what's installed

**Avoid**:
- Unknown third-party images
- Unverified downloads
- Images from untrusted URLs

---

### Access Control

**Image management**:
- Only authorized users should import images
- Document who created each image
- Regular security audits

**VM creation**:
- Users should only use approved images
- Restrict access to registry management
- Monitor image usage

---

## Troubleshooting

### Issue: Image Not Appearing

**Symptoms**:
- Imported image doesn't show in registry
- Image list is empty

**Possible causes**:
1. Page not loading
2. Import failed
3. Filter hiding the image

**Solution**:
1. Refresh the page (press F5)
2. Check all filters are set to "All"
3. Search for the image name
4. Check if import success notification appeared
5. Contact your system administrator if issue persists

---

### Issue: Cannot Delete Image

**Symptoms**:
- Delete button disabled
- Error message appears

**Possible causes**:
- VMs are using this image
- Templates reference this image

**Solution**:
1. Check which VMs use this image
2. Stop and delete those VMs (or migrate to different image)
3. Check if templates reference it
4. Then delete the image

---

### Issue: Import Fails

**Symptoms**:
- Import operation fails
- Error notification appears

**Possible causes**:
1. File format not supported
2. File size too large
3. Insufficient disk space
4. Network issue (for URL/DockerHub imports)

**Solution**:
1. Check file format (kernel: vmlinux, rootfs: ext4/qcow2/raw)
2. Check file size is reasonable
3. Check error message for specific details
4. Contact your system administrator for storage issues

---

## Quick Reference

### Image Types

| Type | Format | Typical Size | Used For |
|------|--------|--------------|----------|
| Kernel | vmlinux | 5-20 MB | VM boot |
| Rootfs | ext4/qcow2/raw | 500MB-5GB | Operating system |
| Container Runtime | ext4 | ~400 MB | Container support |

### Import Methods

| Method | Use Case | Requirements |
|--------|----------|--------------|
| Upload | Local files | File < 10GB |
| Path | Server files | File on server |
| DockerHub | Official images | Internet access |
| URL | Remote downloads | Valid URL |

### Image Actions

| Action | Status | Notes |
|--------|--------|-------|
| Browse images | Available | View all images |
| Search images | Available | Filter by name/type |
| Import image | Available | Multiple methods |
| Delete image | Available | If not in use |
| Rename image | Available | Update display name |
| Copy path | Available | Get file path |
| View details | Available | See properties |

---

## Next Steps

- **[Browse Images](browse-images/)** - Search and explore available images
- **[Import Images](import-images/)** - Add new images to the registry
- **[Manage Images](manage-images/)** - Delete, rename, and organize images
- **[Create VM](/docs/vm/create-vm/)** - Use images to create VMs

---

## FAQ

**Q: How many images can I store?**
A: Limited only by available disk space. Contact your system administrator for storage capacity information.

**Q: Can I use the same image for multiple VMs?**
A: Yes! Images are reused across VMs. Creating multiple VMs from one image doesn't duplicate the image file.

**Q: What format should my images be?**
A: Kernels should be uncompressed vmlinux files. Root filesystems should be ext4 (recommended), qcow2, or raw format.

**Q: How do I update an image?**
A: You cannot modify existing images. Instead, import a new version with a different name (e.g., ubuntu-22.04-v2) and migrate VMs to use it.

**Q: Can I download images from the registry?**
A: You can copy the file path and access it on the server. Browser downloads are not currently supported for large image files.

**Q: What happens if I delete an image that VMs are using?**
A: You cannot delete images that are in use. You must first stop and delete (or reconfigure) all VMs using that image.

**Q: Can I share images between hosts?**
A: Images are host-specific. To share, you'll need to import the same image on each host or set up shared storage (contact your administrator).
