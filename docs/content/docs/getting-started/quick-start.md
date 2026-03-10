+++
title = "Quick Start"
description = "Log in and create your first microVM"
weight = 3
date = 2025-12-01

[extra]
toc = true
+++

# Quick Start

This guide assumes you have completed the [Installation](../installation/). All services are already running — there is nothing to start manually.

---

## Open the Web UI

Navigate to your host in a browser. The URL is shown at the end of the installer output, typically:

```
http://<your-host-ip>
```

Log in with the default credentials:

- **Username:** `root`
- **Password:** `root`

> Change your password immediately via **Settings → Account**.

---

## Upload VM Images

Before creating a VM you need a Linux kernel and a root filesystem image.

1. Go to **Image Registry** in the sidebar
2. Click **Import Image**
3. Upload a kernel (`.bin`) and a rootfs (`.ext4`)

See [Image Registry](../../registry/) for detailed upload instructions and compatible image sources.

---

## Create Your First VM

1. Go to **Virtual Machines** in the sidebar
2. Click **Create VM**
3. Fill in the wizard steps:

| Step | What to set |
|---|---|
| **Basic** | Name (e.g. `my-first-vm`), optional description |
| **Credentials** | Root password for the VM |
| **Machine** | vCPUs: `1`, Memory: `512` MB |
| **Boot** | Select your uploaded kernel and rootfs |
| **Network** | Leave defaults — bridge `fcbr0`, Allow MMDS enabled |
| **Review** | Confirm settings and click **Create** |

4. On the VM detail page, click **Start**
5. Wait for the status badge to turn **Running**

---

## Access the VM

Click the **Terminal** tab on the VM detail page. A browser-based console will open — log in with `root` and the password you set.

For SSH access, check the **Overview** tab for the VM's IP address, then:
```bash
ssh root@<vm-ip>
```

---

## Next Steps

- **[Manage VMs](../../vm/manage-vm/)** — Start, stop, pause, resume, delete
- **[Snapshots](../../vm/backup-snapshot/)** — Save and restore VM state
- **[Networks](../../networks/)** — Create isolated virtual networks
- **[Image Registry](../../registry/)** — Manage kernels and root filesystems
- **[Users](../../users/)** — Add team members and assign roles
