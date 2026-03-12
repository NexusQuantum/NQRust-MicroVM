+++
title = "Volumes"
description = "Complete guide to managing storage volumes for VMs through the web interface"
weight = 90
date = 2025-01-13
+++

Volumes are persistent block storage devices that can be attached to VMs for additional storage beyond the root filesystem.

---

## What are Volumes?

A volume is an `.ext4` file allocated on a host that appears as a block device inside a VM. Unlike the root filesystem, volumes persist independently — you can detach a volume from one VM and attach it to another without losing data.

**Common use cases**:

- **Database storage** — Keep PostgreSQL/MySQL data on a separate volume so it survives VM rebuilds
- **Shared storage** — Attach the same volume (read-only) to multiple VMs
- **Development workspaces** — Separate your code volume from the OS volume

---

## Volume Types

| Type | Description |
|---|---|
| **EXT4** | Standard Linux filesystem, recommended for most workloads |
| **Rootfs** | Root filesystem volumes, auto-registered when a VM is created |

---

## Quick Start

1. Go to **Volumes** in the sidebar
2. Click **Create Volume**, fill in the form, and click **Create Volume**
3. Go to a VM detail page → **Storage** tab → **Add Drive** to attach the volume

---

## Next Steps

- **[Browse Volumes](browse-volumes/)** — Search and explore existing volumes
- **[Create Volumes](create-volumes/)** — Add new storage volumes
- **[Manage Volumes](manage-volumes/)** — Attach, detach, and delete volumes
