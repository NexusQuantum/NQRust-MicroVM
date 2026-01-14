+++
title = "Import Images"
description = "Add new kernel and rootfs images to the registry from multiple sources"
weight = 52
date = 2025-01-13
+++

Learn how to import new images into the registry from various sources including file uploads, server paths, DockerHub, and web URLs.

---

## Import Methods Overview

The registry supports four import methods:

![Image: Import methods](/images/registry/import-methods.png)

| Method | Use Case | Best For |
|--------|----------|----------|
| **Upload File** | Local files on your computer | Small images, testing |
| **Import from Path** | Files already on server | Large images, pre-staged files |
| **DockerHub** | Official container images | Standard OS images |
| **Import from URL** | Remote web downloads | Public image repositories |

---

## Import from Upload

Upload image files directly from your computer.

### When to Use

**Best for**:
- Custom-built images
- Downloaded images on your computer
- Small to medium files (< 2 GB)
- Quick testing

**Not ideal for**:
- Very large files (> 5 GB)
- Slow internet connections
- Server-side files (use Path instead)

---

### Step 1: Open Upload Dialog

![Image: Upload button](/images/registry/upload-button.png)

1. Go to Registry page
2. Click **"Upload File"** button
3. Upload dialog appears

---

### Step 2: Select Image Type

![Image: Select type](/images/registry/upload-type.png)

Choose the image type:
- **Kernel** - For kernel images (vmlinux files)
- **Rootfs** - For root filesystem images

**Type affects**:
- How the image is categorized
- Where it appears in browser
- Which VMs can use it

---

### Step 3: Choose File

![Image: File chooser](/images/registry/upload-choose.png)

Click **"Choose File"** or drag and drop:

**Supported formats**:
- **Kernel**: vmlinux, vmlinuz
- **Rootfs**: .ext4, .qcow2, .raw, .img

**File size limits**:
- Recommended: < 2 GB
- Maximum: Depends on server configuration
- Larger files: Use "Import from Path" instead

---

### Step 4: Enter Image Name

![Image: Image name](/images/registry/upload-name.png)

Provide a descriptive name:

**Good names**:
```
✅ ubuntu-22.04-base
✅ alpine-3.18-container
✅ debian-12-webserver
✅ vmlinux-6.1-lts
```

**Avoid**:
```
❌ image1
❌ test
❌ my-file
❌ download
```

**Naming tips**:
- Include OS name
- Include version
- Add purpose if specialized
- Use hyphens, not spaces

---

### Step 5: Upload

![Image: Upload progress](/images/registry/upload-progress.png)

Click **"Upload"** button:

**What happens**:
1. File uploads to server
2. Progress bar shows upload status
3. Server processes the image
4. Success notification appears
5. Image appears in registry

**Upload time depends on**:
- File size
- Internet speed
- Server processing

**Example times**:
- 10 MB kernel: 5-10 seconds
- 500 MB rootfs: 1-2 minutes
- 2 GB rootfs: 5-10 minutes

---

### Upload Tips

**Prepare files before uploading**:
- Compress if possible (qcow2 format)
- Verify file integrity
- Check file size

**For large files**:
- Use wired connection, not WiFi
- Upload during off-peak hours
- Consider "Import from Path" instead

**Drag and drop**:
- Drag file onto upload area
- Automatic file selection
- Faster than clicking

---

## Import from Path

Import images that already exist on the server.

### When to Use

**Best for**:
- Very large images (> 5 GB)
- Files pre-staged on server
- Shared storage locations
- No network upload needed

**Requirements**:
- File must exist on server
- You need the exact file path
- Admin may need to copy file first

---

### Step 1: Open Import Dialog

![Image: Import from path button](/images/registry/import-path-button.png)

1. Click **"Import from Path"** button
2. Import dialog opens

---

### Step 2: Enter File Path

![Image: Path input](/images/registry/import-path-input.png)

Enter the full server path:

**Path examples**:
```
/srv/images/ubuntu-22.04.ext4
/tmp/alpine-3.18.qcow2
/mnt/storage/debian-12.raw
/home/admin/kernels/vmlinux-6.1
```

**Path requirements**:
- Must be absolute path (starts with /)
- File must exist at that location
- Server must have read access
- Correct permissions required

**Getting the path**:
- Ask your system administrator
- Check file transfer logs
- Use server file browser (if available)

---

### Step 3: Enter Image Name

![Image: Import name](/images/registry/import-name.png)

Provide a name for the registry:

**Note**: This is the display name, not the filename

**Example**:
```
File path: /srv/images/ubuntu_22_04_base.ext4
Image name: ubuntu-22.04-base
```

---

### Step 4: Select Type

Choose image type (Kernel or Rootfs):

**Auto-detection**:
- System tries to detect from filename
- Verify the selection is correct
- Change if needed

---

### Step 5: Import

![Image: Import complete](/images/registry/import-success.png)

Click **"Import"** button:

**What happens**:
1. Server verifies path exists
2. Checks file permissions
3. Registers image in database
4. Success notification appears
5. Image available immediately

**Speed**: Nearly instant (no file copy)

---

### Path Import Tips

**Verify path first**:
- Double-check spelling
- Include full path
- Use forward slashes (/)

**Common path errors**:
```
❌ images/ubuntu.ext4  (missing /)
❌ C:\images\file.ext4  (Windows path)
❌ ~/images/file.ext4  (tilde not expanded)
✅ /srv/images/ubuntu.ext4  (correct)
```

**Work with administrator**:
- Ask them to copy file to standard location
- Get exact path from them
- Confirm permissions are correct

---

## Import from DockerHub

Pull container images from Docker Hub.

### When to Use

**Best for**:
- Official OS images
- Well-maintained containers
- Standard distributions
- Proven, tested images

**Available images**:
- Ubuntu (various versions)
- Alpine Linux
- Debian
- Fedora
- Many others

---

### Step 1: Open DockerHub Import

![Image: DockerHub button](/images/registry/dockerhub-button.png)

1. Click **"Import from DockerHub"** button
2. DockerHub import dialog opens

---

### Step 2: Enter Image Name

![Image: DockerHub image name](/images/registry/dockerhub-name.png)

Enter Docker Hub image identifier:

**Format**: `namespace/repository:tag`

**Examples**:
```
ubuntu:22.04
alpine:3.18
debian:12
fedora:39
```

**Common patterns**:
```
Official images: ubuntu:22.04
Versioned: alpine:3.18.4
Latest: ubuntu:latest (not recommended)
Specific: ubuntu:22.04-20240101
```

---

### Step 3: Enter Registry Name

Provide a name for your registry:

**Example**:
```
Docker image: ubuntu:22.04
Registry name: ubuntu-22.04-base
```

---

### Step 4: Import

![Image: DockerHub import progress](/images/registry/dockerhub-progress.png)

Click **"Import"** button:

**What happens**:
1. Server connects to Docker Hub
2. Downloads image layers
3. Converts to rootfs format
4. Registers in database
5. Image ready to use

**Import time**:
- Small (Alpine): 1-2 minutes
- Medium (Debian): 3-5 minutes
- Large (Ubuntu): 5-10 minutes

---

### DockerHub Tips

**Use specific tags**:
```
❌ ubuntu:latest  (changes over time)
✅ ubuntu:22.04  (stable version)
✅ alpine:3.18  (specific version)
```

**Popular official images**:
```
ubuntu:22.04 - Ubuntu LTS
alpine:3.18 - Lightweight Linux
debian:12 - Debian Bookworm
fedora:39 - Fedora Linux
```

**Check Docker Hub first**:
- Browse hub.docker.com
- Verify image exists
- Check available tags
- Read image documentation

---

## Import from URL

Download images from web URLs.

### When to Use

**Best for**:
- Official distribution downloads
- Public image repositories
- Direct download links
- One-time imports

**Requirements**:
- Valid HTTP/HTTPS URL
- Direct download link (not webpage)
- Publicly accessible
- Reasonable file size

---

### Step 1: Open URL Import

![Image: URL import button](/images/registry/url-import-button.png)

1. Click **"Import from URL"** button
2. URL import dialog opens

---

### Step 2: Enter URL

![Image: URL input](/images/registry/url-input.png)

Paste the download URL:

**Valid URLs**:
```
✅ https://example.com/images/ubuntu-22.04.ext4
✅ https://releases.ubuntu.com/22.04/ubuntu.img
✅ https://dl-cdn.alpinelinux.org/alpine/v3.18/releases/x86_64/alpine-minirootfs-3.18.0-x86_64.tar.gz
```

**Invalid**:
```
❌ https://ubuntu.com  (webpage, not file)
❌ ftp://example.com/file  (FTP not supported)
❌ /local/path/file  (local path, not URL)
```

---

### Step 3: Enter Image Name

Provide a name for the registry:

**Example**:
```
URL: https://example.com/ubuntu-22.04.ext4
Name: ubuntu-22.04-custom
```

---

### Step 4: Select Type

Choose Kernel or Rootfs.

---

### Step 5: Import

![Image: URL download progress](/images/registry/url-progress.png)

Click **"Import"** button:

**What happens**:
1. Server downloads from URL
2. Progress monitored
3. File validated
4. Registered in database
5. Image ready to use

**Download time**: Depends on file size and connection

---

### URL Import Tips

**Test URL first**:
- Open URL in browser
- Verify it downloads a file
- Check file isn't corrupted

**Direct downloads only**:
```
✅ Direct file link
❌ Webpage with download button
❌ JavaScript redirect
❌ Login required
```

**Use official sources**:
```
✅ releases.ubuntu.com
✅ dl-cdn.alpinelinux.org
✅ download.fedoraproject.org
❌ random-site.com
❌ untrusted-host.net
```

---

## After Import

### Verify Import Success

![Image: Import success notification](/images/registry/import-success-notification.png)

**Success indicators**:
- Green success notification
- Image appears in registry table
- No error messages
- Image details are correct

---

### Check Image Details

Verify imported image:

![Image: Verify details](/images/registry/verify-details.png)

**Check**:
- Name is correct
- Type is correct (Kernel/Rootfs)
- Size is reasonable
- Creation date is today

---

### Test the Image

Before production use:

1. Create test VM with this image
2. Verify VM boots correctly
3. Check OS functionality
4. Confirm applications work
5. Then use in production

---

## Common Issues

### Issue: Upload Fails

**Symptoms**:
- Upload stops at X%
- Error notification appears
- File not in registry

**Possible causes**:
1. Network interruption
2. File too large
3. Insufficient disk space
4. Browser timeout

**Solution**:
1. Check internet connection
2. Try smaller file
3. Use "Import from Path" for large files
4. Contact administrator about disk space

---

### Issue: Path Not Found

**Symptoms**:
- Error: "File not found"
- Import fails immediately

**Possible causes**:
1. Typo in path
2. File doesn't exist
3. Incorrect permissions
4. Wrong server

**Solution**:
1. Verify exact path with administrator
2. Check file exists on server
3. Confirm permissions
4. Use absolute path (starts with /)

---

### Issue: DockerHub Timeout

**Symptoms**:
- Import hangs
- Eventually fails
- Timeout error

**Possible causes**:
1. Network issues
2. Docker Hub unavailable
3. Image too large
4. Server proxy issues

**Solution**:
1. Try again later
2. Check smaller image
3. Verify internet connectivity
4. Contact administrator about proxy

---

### Issue: Invalid URL

**Symptoms**:
- Error: "Invalid URL"
- Download fails

**Possible causes**:
1. URL is a webpage, not file
2. Authentication required
3. SSL certificate issues
4. URL format incorrect

**Solution**:
1. Test URL in browser
2. Ensure direct file download
3. Use HTTPS, not HTTP
4. Check URL formatting

---

## Best Practices

### Choose Right Method

**Decision flowchart**:
```
File on your computer?
  → Small (< 2GB)? → Upload
  → Large (> 2GB)? → Ask admin to copy to server → Path

File on server already?
  → Path

Need official OS?
  → DockerHub

Have download URL?
  → URL
```

---

### Naming Conventions

Follow consistent naming:

**Pattern**: `os-version-purpose`

**Examples**:
```
ubuntu-22.04-base
alpine-3.18-container
debian-12-webserver
fedora-39-dev
vmlinux-6.1-lts
```

---

### Verify Before Using

Always test imported images:

1. Import to registry
2. Create test VM
3. Boot and verify
4. Run basic tests
5. Use in production only after verification

---

### Document Custom Images

Keep notes on custom images:

```
Image: ubuntu-22.04-webserver
Source: Imported from URL
Original: https://company.com/images/web-v1.ext4
Modifications: Nginx pre-installed
Purpose: Production web servers
Imported by: admin@company.com
Date: 2025-01-13
```

---

## Quick Reference

### Import Methods Comparison

| Method | Speed | Size Limit | Requires |
|--------|-------|------------|----------|
| Upload | Medium | ~2 GB | Local file |
| Path | Instant | Unlimited | File on server |
| DockerHub | Medium | Varies | Internet |
| URL | Slow | Varies | Valid URL |

### Supported Formats

| Image Type | Formats |
|------------|---------|
| Kernel | vmlinux, vmlinuz |
| Rootfs | ext4, qcow2, raw, img |

### Naming Best Practices

| Element | Example | Purpose |
|---------|---------|---------|
| OS name | ubuntu | Identify distribution |
| Version | 22.04 | Track versions |
| Purpose | base/webserver | Indicate use case |
| Separator | - | Readable naming |

---

## Next Steps

- **[Browse Images](browse-images/)** - Find and select imported images
- **[Manage Images](manage-images/)** - Organize and clean up images
- **[Registry Overview](../)** - Learn about image types
- **[Create VM](/docs/vm/create-vm/)** - Use imported images
