+++
title = "Import Images"
description = "Add new kernel and rootfs images to the registry from multiple sources"
weight = 52
date = 2025-01-13
+++

Add images to the registry from four sources: local file upload, server path, DockerHub, or a direct URL.

---

## Import Methods Overview

| Method | Best for |
|---|---|
| **Upload File** | Images on your local machine |
| **Import from Path** | Images already on the server |
| **DockerHub** | Official OS rootfs images |
| **URL** | Images hosted on the web |

---

## Upload File

Upload an image directly from your browser.

### Steps

1. Go to **Image Registry** and click **Upload**
2. Select the image **Type**: Kernel or Rootfs
3. Click **Choose File** and select the file from your machine
4. Enter a **Name** for the image (e.g. `ubuntu-22.04`, `vmlinux-6.1`)
5. Click **Upload**

A progress bar shows the upload status. Large files may take a few minutes depending on your connection speed.

### Supported formats

- Kernel: uncompressed binary (`vmlinux`)
- Rootfs: `.ext4` filesystem image

### Naming tips

```
Good names:
  ubuntu-22.04
  alpine-3.18-minimal
  vmlinux-6.1-lts

Bad names:
  image1
  test
  final_v2
```

---

## Import from Path

Import an image that already exists on the server filesystem. Requires `MANAGER_ALLOW_IMAGE_PATHS=true` to be set.

### Steps

1. Click **Import from Path**
2. Select the image **Type**
3. Enter the **absolute path** to the file on the server (e.g. `/srv/images/custom/my-kernel`)
4. Enter a **Name**
5. Click **Import**

### Notes

- The path must be readable by the manager process
- No data is copied — the manager references the file at its existing location
- Useful for large images that are already on the server (avoids re-uploading)

---

## Import from DockerHub

Pull a rootfs image from DockerHub. The manager extracts the filesystem from the Docker image layers.

### Steps

1. Click **DockerHub**
2. Enter the image reference in Docker format:
   ```
   library/ubuntu:22.04
   library/alpine:3.18
   library/debian:12
   ```
3. Enter a **Name**
4. Click **Import**

The download runs in the background. Progress is shown on the page. Large images (e.g. Ubuntu) can take several minutes.

### Popular images

```
library/ubuntu:22.04
library/ubuntu:20.04
library/alpine:3.18
library/debian:12
library/fedora:39
```

---

## Import from URL

Download an image directly from a URL.

### Steps

1. Click **Import from URL**
2. Paste the direct download URL (must be a direct link to the file, not a web page)
3. Select the **Type**
4. Enter a **Name**
5. Click **Import**

Download progress is shown. The image is saved to the registry once the download completes.

### Valid URL examples

```
https://example.com/images/ubuntu-22.04.ext4
https://releases.example.org/kernels/vmlinux-6.1
```

---

## After Importing

Once an image appears in the registry list, it is ready to use in VM creation. Go to **Virtual Machines** → **Create VM** and select the image from the Kernel or Rootfs dropdown.

---

## Troubleshooting

### Upload fails or times out

- Check file format — kernels must be uncompressed binaries, rootfs must be `.ext4`
- For very large files (>1 GB), use Import from Path if the file is on the server, or URL import if hosted remotely

### Import from Path: "file not found"

- Verify the path is absolute (starts with `/`)
- Check the file exists and the manager process has read permission
- Confirm `MANAGER_ALLOW_IMAGE_PATHS=true` is set

### DockerHub import times out

- Large images (Ubuntu, Debian) can take 5–15 minutes
- Refresh the registry page — the import may have completed in the background
- Check server internet connectivity

### URL import fails

- Confirm the URL is a direct file link (not a redirect or HTML page)
- Test the URL in a browser — it should trigger a file download

---

## Next Steps

- **[Browse Images](browse-images/)** — Find and filter your imported images
- **[Manage Images](manage-images/)** — Rename or delete images
