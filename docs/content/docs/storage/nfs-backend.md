+++
title = "Add external NFS backend"
description = "Use a remote NFS export as external VM disk storage"
weight = 88
date = 2026-05-14
+++

The **external NFS** backend lets you put VM rootfs files on any NFS export — a Linux NFS server, a NAS appliance, anything that speaks NFSv3 or NFSv4. Each VM gets a `.raw` file on the export; the agent mounts the export once and reuses the mount across VMs on the same backend.

---

## When to use it

- You already have an NFS server and want VM disks to live there.
- You need shared file-level storage that any host can re-attach to.
- You want simple ops — adding a backend is just "server + export path".

If you need **authenticated** shares or are on a Windows-heavy environment, use the [external SMB / CIFS backend](../smb-backend/) instead. If you need **block-level** shared storage, use [external iSCSI](../iscsi-backend/).

---

## Prerequisites

On the **host running the agent**:

- `nfs-common` package (installed by the NQRust-MicroVM installer).
- Network reachability to the NFS server's TCP port 2049.

On the **NFS server**:

- An export that allows write access from the agent's IP / network.
- NFSv3 or NFSv4 (most current servers support both; the kernel client picks the highest version).

---

## Adding an external NFS backend

Go to **Storage** → **Add backend**. In the dialog, set **Kind** to **NFS**.

### Basic fields

| Field | Required | Notes |
|---|---|---|
| **Name** | Yes | Backend identifier, e.g. `nfs-prod` |
| **Server** | Yes | NFS server hostname or IP (e.g. `nas.local`, `10.0.0.50`) |
| **Export path** | Yes | Server-side export path, e.g. `/mnt/tank/vms` |

### Advanced fields (click **Show advanced options**)

| Field | Default | Notes |
|---|---|---|
| **NFS version** | `default` | Pin the protocol with `-o vers=`. Options: `default` (kernel negotiates), `3`, `4`, `4.1`, `4.2` |
| **Extra mount options** | — | Raw `-o` options appended to the mount command (advanced) |
| **Mount base directory** | `/var/lib/nqrust/nfs` | Where on the host the agent mounts exports |

Click **Add backend**. The manager validates the form, asks the agent to run `mount.nfs`, and probes the export with `df` to confirm reachability. On success the new backend is inserted into the live registry — VM-create can target it on the next API call.

---

## Creating a VM on the NFS backend

In the **Create VM** wizard, **Boot Source** step (step 4), set the **Storage backend** dropdown to your NFS backend. The manager asks the backend to provision a rootfs file on the export, records a `volume` row pointing at that file, and spawns Firecracker against the NFS-mounted path.

Stop, start, and delete all work as expected. Restart re-attaches to the same file on the export.

---

## Removing an external NFS backend

Click **Remove** on the backend row.

The manager:

1. Refuses with HTTP 409 if there are live volumes on the backend.
2. Removes the backend row from the database.
3. Unmounts the export.
4. Drops the entry from the live registry.

Rootfs files left on the export are not automatically deleted.

---

## Architecture

The agent owns `mount.nfs`. The manager talks to the agent over `/v1/storage/nfs/*` and never executes mount commands itself. NFS does not need credentials so there's nothing in the agent's credential store.

---

## See also

- [Manage storage backends](../manage-backends/)
- [Add external SMB / CIFS backend](../smb-backend/)
- [Add external iSCSI backend](../iscsi-backend/)
