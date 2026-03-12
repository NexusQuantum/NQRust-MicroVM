+++
title = "Image Registry"
description = "Complete guide to managing VM images, kernels, and root filesystems through the web interface"
weight = 70
date = 2025-01-13
+++

The Image Registry is your central hub for managing all VM images, including kernels and root filesystems.

---

## What is the Image Registry?

The Image Registry stores and manages all images used to create VMs:

- **Kernel Images** — Linux kernel files for VM boot
- **Root Filesystems** — Operating system root filesystems
- **Container Runtime** — Specialized images for running containers

Images are reusable across multiple VMs — import once, use everywhere.

---

## Image Types

### Kernel Images

Required for every VM. Provides core OS functionality.

- Format: Uncompressed kernel binary
- Typical size: 5–20 MB
- Examples: `vmlinux-5.10`, `vmlinux-6.1`, `vmlinux-6.6`

### Root Filesystem Images

The operating system your VM runs.

- Format: `.ext4` filesystem image
- Typical size: 50 MB – 2 GB
- Examples: `ubuntu-22.04.ext4`, `alpine-3.18.ext4`

### Container Runtime Images

Specialized images with Docker pre-installed for running containers inside VMs.

- Includes: Alpine Linux + Docker + OpenRC
- Used automatically when deploying containers

---

## Accessing the Registry

Navigate to **Image Registry** in the sidebar. The page lists all available images with their name, type, size, number of VMs using them, and creation date.

---

## Quick Start

1. Go to **Image Registry** in the sidebar
2. Click **Upload** (or use another import method) to add a kernel and a rootfs
3. Go to **Virtual Machines** → **Create VM**
4. Select your kernel and rootfs from the dropdowns

---

## Storage

Images are stored on the manager host at the path configured via `MANAGER_IMAGE_ROOT` (default: `/srv/images`).

---

## Next Steps

- **[Browse Images](browse-images/)** — Search and explore available images
- **[Import Images](import-images/)** — Add new images from file, path, DockerHub, or URL
- **[Manage Images](manage-images/)** — Rename, delete, and organize images
