+++
title = "Manage Images"
description = "Delete, rename, and organize images in the registry"
weight = 53
date = 2025-01-13
+++

Manage your image registry by deleting unused images, renaming for clarity, and keeping storage organized.

---

## Available Actions

Each image row has an **Actions** menu with:

| Action | Description |
|---|---|
| **Rename** | Change the image name |
| **Copy Path** | Copy the server-side file path |
| **Delete** | Permanently remove the image |

---

## Deleting Images

### Before deleting

Check the **VMs** column — it shows how many VMs are currently using the image. Images in use by one or more VMs cannot be deleted. Stop or delete those VMs first.

### Steps

1. Click **Delete** in the Actions column
2. Confirm the deletion in the dialog
3. The image is permanently removed from the registry and disk

**This cannot be undone.** Make sure no VMs depend on the image before deleting.

### Bulk cleanup

To free up space efficiently:
1. Filter by **Type** to focus on one category at a time
2. Sort by **VMs** column to find images with 0 VMs
3. Delete unused images one by one

---

## Renaming Images

Rename images to follow a consistent naming convention or to clarify what an image contains.

### Steps

1. Click **Rename** in the Actions column
2. Enter the new name in the dialog
3. Click **Save**

### Naming conventions

```
<os>-<version>[-variant]
  ubuntu-22.04
  alpine-3.18-minimal
  debian-12

vmlinux-<version>[-variant]
  vmlinux-6.1
  vmlinux-5.10-lts
```

**Note**: Renaming does not affect VMs already using the image — they reference the image by ID internally, not by name.

---

## Copying Image Paths

Click **Copy Path** to copy the full server-side file path to your clipboard.

**Use cases**:
- Referencing images in API calls
- Writing provisioning scripts
- Documentation and runbooks

**Example path**:
```
/srv/images/ubuntu-22.04.ext4
/srv/images/vmlinux-6.1
```

---

## Storage Management

### Finding large images

Sort the table by **Size** to identify the largest images. Root filesystem images are typically the biggest — kernels are small.

### Checking what's in use

The **VMs** column shows active usage. Images with `0` VMs are safe to delete if no longer needed.

### Recommended cleanup schedule

| Frequency | Task |
|---|---|
| Weekly | Delete images with 0 VMs that are no longer needed |
| Monthly | Review and rename images with unclear names |
| After upgrades | Remove old kernel/rootfs versions once VMs are migrated |

---

## Troubleshooting

### Cannot delete — image is in use

The **VMs** column shows a count greater than 0. Navigate to those VMs, then either stop and delete them, or update them to use a different image before retrying the deletion.

### Rename doesn't reflect immediately

Refresh the browser page — the registry table may be cached.

### Accidentally deleted an image

There is no recycle bin — deleted images are gone. You will need to re-import the image using one of the [import methods](import-images/).

---

## Next Steps

- **[Browse Images](browse-images/)** — Search and filter images
- **[Import Images](import-images/)** — Add new images to replace deleted ones
