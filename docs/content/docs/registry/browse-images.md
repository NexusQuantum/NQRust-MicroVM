+++
title = "Browse Images"
description = "Search, filter, and explore available images in the registry"
weight = 51
date = 2025-01-13
+++

Find and explore available images in the registry.

---

## Accessing the Registry

Navigate to **Image Registry** in the sidebar. The page shows a table of all images with the following columns:

| Column | Description |
|---|---|
| **Name** | Image filename |
| **Type** | Kernel or Rootfs |
| **Size** | File size on disk |
| **VMs** | Number of VMs currently using this image |
| **Created** | Upload/import date |
| **Actions** | Delete, rename, copy path |

---

## Searching Images

Use the search bar at the top of the registry page to filter by name. Search is case-insensitive and matches partial names.

**Examples**:
```
ubuntu        → finds ubuntu-22.04.ext4, ubuntu-20.04.ext4
vmlinux       → finds all kernel images
alpine        → finds alpine-3.18.ext4
```

---

## Filtering by Type

Use the type filter to show only a specific image category:

- **All** — Show every image
- **Kernel** — Show only kernel images
- **Rootfs** — Show only root filesystem images

This is useful when selecting an image during VM creation to narrow down the list quickly.

---

## Image Details

Each row shows:

- **Name** — The filename used when referencing the image
- **Size** — Typical ranges:
  - Kernels: 5–20 MB
  - Root filesystems: 50 MB – 2 GB
- **VMs** — How many VMs are using this image. Images with active VMs cannot be deleted.

---

## Copying an Image Path

Click the **copy path** icon in the Actions column to copy the full server-side path to your clipboard. Useful when writing scripts or referencing images via the API.

---

## Troubleshooting

### No images in the list

The registry is empty — you need to import images before creating VMs. See [Import Images](import-images/).

### No results after searching

Check the spelling, or clear the type filter — the image may be of a different type than currently selected.

### Kernel/Rootfs dropdown empty in VM creation

The dropdown only shows images of the matching type. If it's empty, import the required image type first.

---

## Next Steps

- **[Import Images](import-images/)** — Add new images
- **[Manage Images](manage-images/)** — Delete, rename, organize
