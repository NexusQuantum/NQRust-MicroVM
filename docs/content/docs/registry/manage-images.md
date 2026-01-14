+++
title = "Manage Images"
description = "Delete, rename, and organize images in the registry"
weight = 53
date = 2025-01-13
+++

Learn how to manage your image registry by deleting unused images, renaming for clarity, copying paths, and keeping your registry organized.

---

## Registry Management Overview

The registry provides tools to keep your images organized:

**Available actions**:
- **Delete** - Remove unused images
- **Rename** - Update display names
- **Copy Path** - Get file system paths
- **View Usage** - See which VMs use images

![Image: Management actions](/images/registry/management-overview.png)

---

## Deleting Images

Remove images that are no longer needed.

### When to Delete

**Safe to delete when**:
- ✅ Zero VMs using the image
- ✅ No templates reference it
- ✅ Obsolete or replaced version
- ✅ Test images no longer needed

**Do NOT delete when**:
- ❌ VMs are using the image
- ❌ Templates reference it
- ❌ Only copy of production image
- ❌ Actively used by team

---

### Step 1: Check Usage

Before deleting, check if image is in use:

![Image: Check usage](/images/registry/check-usage.png)

**VM count indicator**:
- `0 VMs` - Safe to delete
- `1 VM` - In use, check first
- `5+ VMs` - Do not delete

**Click VM count** to see which VMs use it:
- View list of VMs
- Decide if you can migrate
- Delete VMs first if needed

---

### Step 2: Select Delete

![Image: Delete button](/images/registry/delete-button.png)

1. Find image in registry table
2. Click **Delete** button (trash icon)
3. Confirmation dialog appears

---

### Step 3: Confirm Deletion

![Image: Delete confirmation](/images/registry/delete-confirm.png)

**Confirmation dialog shows**:
- Image name
- Warning about permanent deletion
- Confirm/Cancel buttons

**What happens when you confirm**:
1. Image removed from registry
2. Database entry deleted
3. File may remain on server (ask admin if cleanup needed)
4. Action cannot be undone

**Success notification**:
![Image: Delete success](/images/registry/delete-success.png)

```
Image deleted successfully
"ubuntu-22.04-test" has been removed from the registry
```

---

### Cannot Delete In-Use Images

![Image: Cannot delete](/images/registry/cannot-delete.png)

**Error message**:
```
Cannot delete image
This image is currently used by 3 VMs
```

**To delete**:
1. View which VMs use it
2. Stop and delete those VMs
3. Or migrate VMs to different image
4. Then delete the image

---

### Bulk Deletion

Delete multiple unused images:

**Process**:
1. Sort by VM count
2. Filter to "0 VMs"
3. Review each image
4. Delete one by one

**Be careful**:
- Double-check each image
- Ensure not needed for recovery
- Keep at least one production image
- Document what you delete

---

## Renaming Images

Update image names for clarity.

### When to Rename

**Good reasons**:
- Unclear or generic name
- Add version information
- Standardize naming convention
- Add purpose description

**Examples**:
```
Before: image1
After:  ubuntu-22.04-base

Before: test
After:  alpine-3.18-test

Before: my-image
After:  debian-12-webserver
```

---

### Step 1: Open Rename Dialog

![Image: Rename button](/images/registry/rename-button.png)

1. Click **Rename** button (pencil icon)
2. Rename dialog appears

---

### Step 2: Enter New Name

![Image: Rename dialog](/images/registry/rename-dialog.png)

**Current name displayed**
**Enter new name**:

**Good names**:
```
✅ ubuntu-22.04-base
✅ kernel-6.1-production
✅ alpine-3.18-container-v2
✅ debian-12-database
```

**Avoid**:
```
❌ image
❌ test
❌ new
❌ copy-of-ubuntu
```

---

### Step 3: Save Changes

Click **"Rename"** button:

**What happens**:
1. Display name updated
2. File path unchanged (stays same)
3. VMs continue working (reference unchanged)
4. Success notification appears

**Success notification**:
![Image: Rename success](/images/registry/rename-success.png)

```
Image renamed successfully
Renamed from "image1" to "ubuntu-22.04-base"
```

---

### Rename Best Practices

**Include key information**:
```
✅ OS type: ubuntu, alpine, debian
✅ Version: 22.04, 3.18, 12
✅ Purpose: base, webserver, database
✅ Variant: lts, slim, full
```

**Follow patterns**:
```
Pattern: [os]-[version]-[purpose]
Examples:
- ubuntu-22.04-base
- alpine-3.18-container
- debian-12-webserver
- fedora-39-dev
```

**Be consistent across team**:
- Agree on naming convention
- Document the pattern
- Stick to it for all images

---

## Copying Image Paths

Get the server file path for an image.

### When to Copy Path

**Use cases**:
- Documentation
- Scripts and automation
- Manual server operations
- Backup procedures
- Sharing with team

---

### Copy Path Action

![Image: Copy path button](/images/registry/copy-path-button.png)

1. Click **Copy** button (clipboard icon)
2. Path copied to clipboard
3. Confirmation notification appears

**Success notification**:
```
Path copied to clipboard
/srv/images/ubuntu-22.04-base.ext4
```

---

### Using Copied Paths

**Example paths**:
```
Kernel:
/srv/images/kernels/vmlinux-6.1

Rootfs:
/srv/images/rootfs/ubuntu-22.04-base.ext4
/srv/images/rootfs/alpine-3.18.qcow2

Container Runtime:
/srv/images/container-runtime.ext4
```

**Use in documentation**:
```markdown
# Production Images

Kernel: /srv/images/vmlinux-6.1
Rootfs: /srv/images/ubuntu-22.04-base.ext4
Updated: 2025-01-13
```

**Use in scripts** (requires server access):
```bash
# Backup image
cp /srv/images/ubuntu-22.04-base.ext4 /backup/

# Check image size
ls -lh /srv/images/ubuntu-22.04-base.ext4
```

**Note**: Path operations require server access. Contact your system administrator for file operations.

---

## Viewing Image Details

Check image properties and usage.

### Image Information

Each image row displays:

![Image: Image details](/images/registry/image-details.png)

**Visible information**:
- **Name**: Display name
- **Type**: Kernel, Rootfs, or Container Runtime
- **Size**: File size (formatted)
- **VMs**: Number of VMs using it
- **Created**: When added to registry

---

### VM Usage Details

Click the VM count to see details:

![Image: VM usage modal](/images/registry/vm-usage.png)

**Shows**:
- List of VM names
- VM states (Running/Stopped)
- Links to VM detail pages
- Total count

**Use this to**:
- Identify which VMs use image
- Decide if image can be deleted
- Plan VM migrations
- Track image usage

---

## Registry Organization

Keep your registry clean and organized.

### Regular Cleanup Schedule

**Weekly tasks**:
- Review new imports
- Delete obvious test images
- Rename unclear images

**Monthly tasks**:
- Check for unused images (0 VMs)
- Verify image names follow conventions
- Update documentation

**Quarterly tasks**:
- Audit all images
- Delete obsolete versions
- Consolidate duplicate images
- Review storage usage

---

### Naming Standards

Establish and follow team standards:

**Standard pattern**:
```
[os]-[version]-[purpose]-[variant]

Examples:
ubuntu-22.04-base
ubuntu-22.04-base-v2
ubuntu-22.04-webserver
ubuntu-22.04-webserver-ssl
alpine-3.18-container
alpine-3.18-container-slim
```

**Document in wiki**:
```
# Image Naming Convention

Pattern: [os]-[version]-[purpose]

OS names:
- ubuntu
- alpine
- debian
- fedora

Versions:
- Use official version numbers
- Example: 22.04, 3.18, 12, 39

Purpose:
- base (minimal OS)
- webserver (nginx/apache)
- database (postgres/mysql)
- container (docker-enabled)
- dev (development tools)
```

---

### Categorization Strategy

Organize images logically:

**By purpose**:
```
Production:
- ubuntu-22.04-prod
- kernel-6.1-prod

Development:
- ubuntu-22.04-dev
- alpine-3.18-dev-test

Containers:
- container-runtime-v1
- alpine-3.18-container
```

**By environment**:
```
Prod: prod-ubuntu-22.04
Staging: staging-ubuntu-22.04
Dev: dev-ubuntu-22.04
Test: test-ubuntu-22.04
```

---

## Storage Management

Monitor and manage disk usage.

### Check Storage Usage

![Image: Storage info](/images/registry/storage-info.png)

**View total storage**:
- Total images count
- Total storage used
- Largest images
- Growth trends

**Contact administrator for**:
- Storage capacity info
- Disk usage reports
- Cleanup recommendations
- Storage expansion

---

### Identify Large Images

Sort by size to find large images:

![Image: Sort by size](/images/registry/sort-by-size.png)

**Typical sizes**:
- Kernel: 10-20 MB (small)
- Alpine rootfs: 100-500 MB (small)
- Ubuntu rootfs: 1-3 GB (medium)
- Full-featured: 5-10 GB (large)

**For large images**:
- Verify they're needed
- Check if used frequently
- Consider compression
- Delete if obsolete

---

### Free Up Space

When storage is low:

**Priority for deletion**:
1. **Test images** - No production value
2. **Old versions** - Replaced by newer
3. **Unused images** - 0 VMs, old dates
4. **Duplicate images** - Same content, different names

**Process**:
1. Sort by VM count (ascending)
2. Filter "0 VMs"
3. Sort by date (oldest first)
4. Review and delete candidates
5. Verify space freed

---

## Best Practices

### Before Deleting

✅ **Checklist**:
- [ ] Check VM count is 0
- [ ] Verify no templates use it
- [ ] Confirm not needed for recovery
- [ ] Have backup if important
- [ ] Team doesn't need it

✅ **Document deletion**:
```
Deleted: ubuntu-20.04-base
Date: 2025-01-13
Reason: Upgraded all VMs to 22.04
Deleted by: admin@company.com
```

---

### Naming Consistency

✅ **Follow team standards**:
- Use agreed-upon pattern
- Include key information
- Be descriptive
- Avoid generic names

✅ **Rename during import**:
- Give good name immediately
- Don't leave generic names
- Plan before importing

---

### Regular Maintenance

✅ **Schedule reviews**:
- Weekly: Quick check
- Monthly: Detailed review
- Quarterly: Deep cleanup

✅ **Keep documentation**:
- List of standard images
- Naming conventions
- Deletion log
- Import history

---

## Troubleshooting

### Issue: Cannot Delete Image

**Symptoms**:
- Delete button disabled
- Error message about VMs

**Solution**:
1. Check VM count
2. View which VMs use it
3. Stop/delete those VMs
4. Then delete image

---

### Issue: Renamed but Old Name Shows

**Symptoms**:
- Renamed image
- Old name still appears somewhere

**Cause**:
- Browser cache
- Page not refreshed

**Solution**:
1. Refresh page (F5)
2. Clear browser cache
3. Log out and back in
4. Contact support if persists

---

### Issue: Copy Path Not Working

**Symptoms**:
- Click copy button
- Nothing happens

**Cause**:
- Browser permission issue
- Clipboard access blocked

**Solution**:
1. Allow clipboard permissions
2. Try different browser
3. Manual copy from notification

---

### Issue: Accidental Deletion

**Symptoms**:
- Deleted wrong image
- Need to recover

**Solution**:
1. Check if file still on server (ask admin)
2. Re-import if you have source
3. Restore from backup
4. Contact administrator immediately

**Prevention**:
- Read confirmation dialog carefully
- Double-check image name
- Keep backups of important images

---

## Quick Reference

### Management Actions

| Action | Requires | Cannot Do If |
|--------|----------|--------------|
| Delete | 0 VMs using | VMs attached |
| Rename | Any time | - |
| Copy Path | Any time | - |
| View Usage | Any time | - |

### Cleanup Priority

| Priority | What to Delete | Check First |
|----------|----------------|-------------|
| 1 | Test images | No production use |
| 2 | Old versions | New version exists |
| 3 | Unused (0 VMs) | Not needed |
| 4 | Duplicates | Keep one copy |

### Storage Guidelines

| Image Type | Expected Size | Notes |
|------------|---------------|-------|
| Kernel | 10-20 MB | Very small |
| Alpine rootfs | 100-500 MB | Lightweight |
| Ubuntu rootfs | 1-3 GB | Standard |
| Full-featured | 5-10 GB | Many packages |

---

## Next Steps

- **[Browse Images](browse-images/)** - Find images in registry
- **[Import Images](import-images/)** - Add new images
- **[Registry Overview](../)** - Learn about image types
- **[Create VM](/docs/vm/create-vm/)** - Use images to create VMs
