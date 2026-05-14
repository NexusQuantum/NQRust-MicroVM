+++
title = "Add external iSCSI backend"
description = "Use a shared iSCSI target as external block storage with auto-provisioned per-VM LVs"
weight = 89
date = 2026-05-14
+++

The **external iSCSI** backend (formally `iscsi_lvm`) lets you put VM disks on any iSCSI target without committing to a specific vendor. Each VM gets its own **Logical Volume** carved out of the shared LUN, with `lvchange -aey` ensuring only one host activates a given LV at a time — safe for multi-host deployments.

This is the same model Proxmox VE uses for shared block storage (LVM-on-iSCSI).

---

## When to use it

- You already have an iSCSI target (FreeNAS / TrueNAS / Synology / Pure / etc.) and want to use it for VM disks.
- You need **block-level** shared storage so live VM migration between hosts is possible later.
- You want vendor-agnostic shared storage — no per-vendor REST APIs, just standard iSCSI + LVM.

If you need shared **file-level** storage instead (simpler ops, no per-VM LV management), use the [external NFS backend](../nfs-backend/) or [external SMB / CIFS backend](../smb-backend/).

---

## Prerequisites

On the **host running the agent**:

- `open-iscsi` and `lvm2` packages (installed by the NQRust-MicroVM installer).
- The `iscsid` daemon enabled and running.
- Network reachability to the iSCSI target's TCP port 3260.

On the **iSCSI target**:

- A LUN of the size you want for your VM pool — this LUN becomes the Volume Group, sliced into per-VM LVs.
- The agent's initiator IQN (configured at install time) allowed to log in.

{{% alert icon="⚠️" context="warning" %}}
**Initializing this backend wipes the LUN.** The agent runs `pvcreate` and `vgcreate` on the device, which destroys any pre-existing data. Only point a fresh LUN at this backend — never one with data you care about.
{{% /alert %}}

---

## Adding an external iSCSI backend

Go to **Storage** → **Add backend**. In the dialog, set **Kind** to **iSCSI + LVM (vendor-agnostic)**.

### Basic fields

| Field | Required | Notes |
|---|---|---|
| **Name** | Yes | Backend identifier, e.g. `iscsi-prod` |
| **Portal address** | Yes | iSCSI target endpoint, e.g. `10.0.0.50:3260` |
| **Target IQN** | Yes | The full IQN of the target, e.g. `iqn.2024-04.com.example:vms` |
| **LUN** | Yes | LUN number (typically `0`) |
| **Volume Group name** | Yes | Name to give the LVM Volume Group created on this LUN (e.g. `vg_nqrust`) |

### Advanced fields

| Field | Default | Notes |
|---|---|---|
| **CHAP username / password** | — | Optional CHAP authentication. Forwarded to the agent's credential store at `/etc/nqrust/storage-creds/<id>.cred`; never persisted in the manager DB |

Click **Add backend**. The manager creates the backend row but does **not** yet wipe the LUN.

---

## Initialize the backend

After create, the backend row appears in the Storage table with status **needs initialization**. Click **Initialize** in the Actions column.

A confirmation dialog appears asking you to type the backend name to confirm. This is intentional friction — Initialize is **destructive** and runs:

1. `iscsiadm` login to the portal + target.
2. `pvcreate` on the resulting block device.
3. `vgcreate <vg-name>` to create the LVM Volume Group.

After successful initialization, the backend status changes to **ok** and you can target it from VM-create.

---

## Creating a VM on the iSCSI backend

In the **Create VM** wizard, **Boot Source** step (step 4), set the **Storage backend** dropdown to your iSCSI backend.

When the VM is created:

1. The manager asks the backend to `lvcreate` a per-VM Logical Volume inside the Volume Group.
2. The agent runs `lvchange -aey` to exclusively activate the LV on this host.
3. Firecracker is spawned with the `/dev/<vg>/<lv>` block device as the rootfs.

On stop, `lvchange -aln` deactivates the LV so a different host could later activate it. On delete, the LV is `lvremove`'d.

---

## Removing an external iSCSI backend

Click **Remove** on the backend row.

The manager:

1. Refuses with HTTP 409 if there are live volumes (active LVs) on the backend.
2. Removes the backend row from the database.
3. Notifies the agent to clear the credential file (if CHAP).
4. Drops the entry from the live registry.

The Volume Group is **not** automatically destroyed — the underlying LUN is left intact in case you want to recover.

---

## Architecture

```
┌─────────────────┐ HTTP /v1/storage/iscsi_lvm/*  ┌─────────────────────────┐
│  Manager        │ ──────────────────────────▶  │  Agent (root)           │
│  (unprivileged) │                               │                         │
│                 │                               │  iscsiadm (login,       │
│  Backend config │                               │   discovery, sessions)  │
│  in Postgres    │                               │                         │
│                 │                               │  pvcreate / vgcreate    │
│                 │                               │   on initialize         │
└─────────────────┘                               │                         │
                                                  │  lvcreate / lvremove    │
                                                  │   per VM lifecycle      │
                                                  │                         │
                                                  │  lvchange -aey / -aln   │
                                                  │   per VM start / stop   │
                                                  └─────────────────────────┘
                                                              │
                                                              │ iSCSI over TCP/3260
                                                              ▼
                                                  ┌─────────────────────────┐
                                                  │  iSCSI Target           │
                                                  │  (any vendor)           │
                                                  └─────────────────────────┘
```

The agent owns all privileged operations: `iscsiadm`, `pvcreate`, `vgcreate`, `lvcreate`, `lvchange`. The manager never executes these directly.

The `lvchange -aey` mode (exclusive activate) is what makes this safe for multi-host: a given LV can only be active on one host at a time, so no two Firecracker processes can mount the same rootfs concurrently. This is also a prerequisite for live VM migration.

---

## Troubleshooting

See the [iSCSI-LVM troubleshooting runbook](https://github.com/NexusQuantum/NQRust-MicroVM/blob/main/docs/runbooks/iscsi-lvm-troubleshooting.md) for common failure modes:

- `Initialize fails with "device is already part of VG 'X'"` — the LUN already has LVM metadata; confirm it's yours to wipe, then `vgremove` first.
- `iscsiadm: cannot make connection to 10.0.0.50:3260` — firewall, wrong portal, or target not exporting to your initiator IQN.
- `lvchange -aey: Logical volume is already active exclusively on another host` — the LV is locked by another agent; verify only one host should be running the VM.

---

## See also

- [Manage storage backends](../manage-backends/)
- [Add external NFS backend](../nfs-backend/)
- [Add external SMB / CIFS backend](../smb-backend/)
