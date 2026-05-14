+++
title = "Storage Backends"
description = "Where VM disks live — local files, NFS, SMB / CIFS, iSCSI, LVM-on-iSCSI, and SPDK lvol."
weight = 85
date = 2026-05-14
+++

A **storage backend** is a place where VM rootfs and data disks live. NQRust-MicroVM ships with several backend kinds so you can pick whichever matches your existing infrastructure — from a single laptop directory to enterprise SAN.

---

## Backend kinds at a glance

| Kind | Best for | What lives where | Setup |
|---|---|---|---|
| **Local file** | Dev / single-host | `.raw` files under `MANAGER_STORAGE_ROOT` (default `/srv/fc/vms`) | Zero — works out of the box |
| **NFS** | Shared file storage, simple ops | `.raw` files on a remote NFS export, agent runs `mount.nfs` | Provide server + export path |
| **SMB / CIFS** | Windows / Samba / NAS appliances | `.raw` files on an SMB share, agent runs `mount.cifs` | Provide server + share, optional credentials |
| **iSCSI + LVM** | Shared block storage, vendor-agnostic | Per-VM `LV` on a shared iSCSI target, agent runs `iscsiadm` + `lvchange` | One-time `Initialize` button wipes the LUN |
| **iSCSI (generic)** | Pre-provisioned LUNs (advanced) | One LUN per VM, no LVM | Provide IQN + LUN map |
| **TrueNAS iSCSI** | TrueNAS / Core / Scale | iSCSI extents managed via TrueNAS REST API | API key + dataset |
| **SPDK lvol** | High-performance NVMe over fabrics | Logical volumes inside an SPDK pool | Provide RPC socket + pool name |

Three kinds (`local_file`, `nfs`, `smb`, `iscsi_lvm`) are recommended for most users and show up as the default options in the **Add backend** dialog. The remaining kinds are visible under **Show advanced kinds**.

---

## Where to next

- [**Manage storage backends**](manage-backends/) — Add, edit, set default, and remove backends through the UI
- [**SMB / CIFS backend**](smb-backend/) — Vendor-agnostic SMB share support (added in v0.4.0)
- [**Volumes**](../volumes/) — Persistent data disks attached to VMs

---

## Architecture (short version)

- **Manager** is unprivileged. It tracks each backend's config in Postgres and orchestrates VM lifecycle.
- **Agent** runs as root on the KVM host. It owns the privileged operations: `mount.nfs`, `mount.cifs`, `iscsiadm`, `lvchange`. For each backend kind the manager calls the agent over HTTP at `/v1/storage/<kind>/*`.
- **Credentials never live in the manager DB.** For SMB and other authenticated backends, the manager forwards credentials to the agent's on-disk credential store at `/etc/nqrust/storage-creds/<backend-id>.cred` (`mode 0600`, root:root) — the Proxmox pattern.

---

## Adding a backend live (no restart)

Backends added through the UI take effect immediately. The manager's in-memory backend registry is updated atomically when you click **Add backend**, so VM-create against the new backend works on the next API call. The same applies to deletes — once you confirm removal in the UI the backend is gone from the registry. This means you can iterate on backend config (e.g. trying different SMB versions) without restarting the manager.
