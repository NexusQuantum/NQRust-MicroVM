+++
title = "Browse Images"
description = "Search, filter, and explore available images in the registry"
weight = 51
date = 2025-01-13
+++

Learn how to browse and search through available images in the registry to find exactly what you need for your VMs.

---

## Accessing the Image Browser

### From Registry Page

Navigate to the Registry page:

![Image: Registry page](/images/registry/browse-main.png)

1. Click **"Registry"** in the sidebar
2. View all available images in the table

---


## Image Table Layout

The image table displays key information:

![Image: Image table](/images/registry/image-table.png)

**Columns**:
- **Name** - Image display name
- **Type** - Kernel, Rootfs, or Container Runtime
- **Size** - File size in MB/GB
- **VMs** - Number of VMs using this image
- **Created** - When image was added
- **Actions** - Available operations

---

## Searching Images

### Basic Search

Use the search bar to find images:

![Image: Search bar](/images/registry/search-bar-registry.png)

**Search by**:
- Image name
- Operating system
- Version number
- Keywords

**Examples**:
```
Search: "ubuntu"
→ Finds: ubuntu-22.04-base, ubuntu-20.04-server

Search: "22.04"
→ Finds: ubuntu-22.04-base, debian-22.04

Search: "container"
→ Finds: container-runtime, alpine-container

Search: "kernel"
→ Finds: vmlinux-5.10, vmlinux-6.1
```

---

### Search Tips

**Be specific**:
```
❌ Too vague: "linux"
✅ Better: "ubuntu 22"
✅ Best: "ubuntu-22.04"
```

**Use version numbers**:
```
❌ Generic: "ubuntu"
✅ Specific: "ubuntu 22.04"
✅ With dash: "ubuntu-22.04"
```

**Search partial names**:
```
"ub" → matches "ubuntu"
"alp" → matches "alpine"
"deb" → matches "debian"
```

---

## Filtering Images

### Filter by Type

Use the type filter dropdown:

![Image: Type filter](/images/registry/filter-type.png)

**Filter options**:
- **All** - Show all image types
- **Kernel** - Show only kernel images
- **Rootfs** - Show only root filesystem images

**Use cases**:
- Filter to "Kernel" when selecting a kernel
- Filter to "Rootfs" when selecting a rootfs
- Use "All" to see everything

---


## Viewing Image Details

### Image Information

Each image row shows key details:

![Image: Image row details](/images/registry/image-row.png)

**Displayed information**:
- **Name**: Display name of the image
- **Type badge**: Color-coded type indicator
  - Blue for Kernel
  - Green for Rootfs
  - Purple for Container Runtime
- **Size**: Human-readable file size
- **VM count**: Number of VMs using this
- **Date**: When image was added

---

### Image Size Display

Sizes are formatted for readability:

```
< 1 MB:     "512 KB"
< 1 GB:     "45 MB"
< 10 GB:    "2.5 GB"
>= 10 GB:   "15 GB"
```

**Typical sizes**:
- Kernel: 10-20 MB
- Alpine rootfs: 100-500 MB
- Ubuntu rootfs: 1-3 GB
- Container runtime: 400-500 MB

---

### Usage Indicators

**VM count badge**:

![Image: VM count badge](/images/registry/vm-count.png)

Shows how many VMs use this image:
- `0 VMs` - Not in use, safe to delete
- `1 VM` - Used by one VM
- `5 VMs` - Actively used, don't delete
- `20+ VMs` - Heavily used, critical image

---

## Selecting Images

### Selecting from Browser Modal

When browsing in VM creation:

![Image: Select image modal](/images/registry/select-image.png)

**Steps**:
1. Search or filter to find image
2. Click image row to select
3. Image name appears in VM creation form
4. Continue with VM creation

**Visual feedback**:
- Selected image is highlighted
- Selection confirmed with checkmark
- Modal closes automatically

---

### Using Image Path

Copy image paths for reference:

![Image: Copy path](/images/registry/copy-path.png)

1. Click **Copy** icon in Actions column
2. Path copied to clipboard
3. Confirmation notification appears

**Use path for**:
- Documentation
- Scripts
- Manual operations
- Backup references

---

## Image Categories

### Official OS Images

Pre-loaded by administrator:

```
Ubuntu:
- ubuntu-22.04-base
- ubuntu-20.04-lts
- ubuntu-24.04-preview

Debian:
- debian-12-bookworm
- debian-11-bullseye

Alpine:
- alpine-3.18
- alpine-3.19
```

---

### Kernel Images

Available kernel versions:

```
LTS Kernels:
- vmlinux-5.10 (Long-term support)
- vmlinux-5.15 (Extended LTS)

Stable Kernels:
- vmlinux-6.1 (Current stable)
- vmlinux-6.6 (Latest stable)
```

**Choose kernel based on**:
- Hardware support needs
- Feature requirements
- Stability vs. latest features
- LTS for production

---

### Specialized Images

Purpose-built images:

```
Container Runtime:
- container-runtime (Docker-enabled)
- container-runtime-slim (Minimal)

Database:
- postgres-15-base
- mysql-8-base

Web Server:
- nginx-alpine
- apache-ubuntu
```

---

## Sorting Images

Images can be sorted by clicking column headers:

**Sort by Name** (alphabetical):
```
alpine-3.18
debian-12
ubuntu-22.04
vmlinux-6.1
```

**Sort by Size** (smallest first):
```
vmlinux-6.1 (15 MB)
alpine-3.18 (120 MB)
ubuntu-22.04 (2.5 GB)
```

**Sort by Usage** (most used first):
```
ubuntu-22.04 (25 VMs)
vmlinux-6.1 (20 VMs)
alpine-3.18 (5 VMs)
test-image (0 VMs)
```

**Sort by Date** (newest first):
```
ubuntu-24.04 (Yesterday)
vmlinux-6.6 (Last week)
debian-12 (Last month)
```

---

## Empty States

### No Images Found

When no images match your search:

![Image: No results](/images/registry/no-results.png)

**Message**: "No images found"

**Actions**:
1. Clear search query
2. Adjust filters
3. Try different keywords
4. Import new images if needed

---

### Registry Empty

When registry has no images:

![Image: Empty registry](/images/registry/empty-registry.png)

**Message**: "No images in registry"

**Actions**:
1. Import your first image
2. Contact administrator to preload images
3. See Import Images guide

---

## Performance Tips

### Quick Navigation

**Keyboard shortcuts**:
- `Tab` - Move between search and filters
- `Enter` - Select highlighted image
- `Escape` - Close browser modal
- `Arrow keys` - Navigate image list

**Mouse shortcuts**:
- Click image name for quick select
- Double-click for instant selection
- Hover for quick info tooltip

---

### Efficient Searching

**Start broad, then narrow**:
```
Step 1: Filter to type (e.g., "Rootfs")
Step 2: Search for OS (e.g., "ubuntu")
Step 3: Look for version in results
```

**Use prefixes**:
```
"vm" → finds kernels (vmlinux)
"ubuntu" → finds Ubuntu images
"alp" → finds Alpine images
```

**Save common searches**:
Keep notes of frequently used images:
```
Production kernel: vmlinux-6.1
Production rootfs: ubuntu-22.04-base
Dev rootfs: alpine-3.18-dev
```

---

## Best Practices

### Finding the Right Image

✅ **Check VM count**:
- High count = Proven, tested image
- Zero count = Unused, might be obsolete

✅ **Verify version**:
- Match your requirements
- Check if LTS or latest
- Consider support lifecycle

✅ **Review size**:
- Larger = More features installed
- Smaller = Minimal, faster boot
- Balance features vs. efficiency

---

### Before Selecting

✅ **Confirm compatibility**:
- Kernel compatible with rootfs
- Architecture matches (x86_64)
- Version appropriate for workload

✅ **Check freshness**:
- Recently created = Up-to-date packages
- Old images may need updates
- Consider security patches

✅ **Verify purpose**:
- Match image to use case
- Production vs. development
- Specialized vs. general-purpose

---

## Troubleshooting

### Issue: Can't Find Expected Image

**Symptoms**:
- Image not in list
- Search returns no results

**Possible causes**:
1. Image not imported yet
2. Filter hiding the image
3. Typo in search query

**Solution**:
1. Clear all filters (set to "All")
2. Clear search query
3. Check spelling
4. Verify image was imported
5. Ask administrator if it exists

---

### Issue: Too Many Results

**Symptoms**:
- Long list of images
- Hard to find specific image

**Solution**:
1. Use specific search terms
2. Apply type filter
3. Sort by relevant column
4. Add version numbers to search

---

### Issue: Unclear Image Purpose

**Symptoms**:
- Multiple similar images
- Don't know which to choose

**Solution**:
1. Check VM count (popular = tested)
2. Look for descriptive names
3. Ask team about standard images
4. Test in development first
5. Document your choice for next time

---

## Quick Reference

### Search Operators

| Search Term | Matches |
|-------------|---------|
| ubuntu | Any image with "ubuntu" in name |
| 22.04 | Any image with "22.04" in name |
| vm | vmlinux kernels |
| alpine | Alpine Linux images |

### Filter Options

| Filter | Shows |
|--------|-------|
| All | All image types |
| Kernel | Only kernel images (vmlinux) |
| Rootfs | Only root filesystem images |

### Column Sorting

| Column | Sort Order |
|--------|------------|
| Name | Alphabetical (A-Z) |
| Size | Smallest to largest |
| VMs | Most used to least used |
| Created | Newest to oldest |

---

## Next Steps

- **[Import Images](import-images/)** - Add new images to the registry
- **[Manage Images](manage-images/)** - Delete and organize images
- **[Registry Overview](../)** - Learn about image types
- **[Create VM](/docs/vm/create-vm/)** - Use images to create VMs
