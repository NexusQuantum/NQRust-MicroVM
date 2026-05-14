+++
title = "Storage Backends"
description = "Where VM disks live — local files plus external SMB/CIFS, NFS, and iSCSI."
weight = 85
date = 2026-05-14
+++

A **storage backend** is a place where VM rootfs and data disks live. NQRust-MicroVM ships with **local file** storage out of the box, plus three **external storage** integrations so you can put VM disks on your existing NAS or SAN.

---

## Local vs external storage

- **Local file (default)** — VM disks live as `.raw` files on the manager host's filesystem. Zero setup, ideal for dev and single-host deployments.
- **External storage** — VM disks live on a remote server you already operate. Three external kinds are supported:

  | External backend | What you need | What lives where |
  |---|---|---|
  | [**External SMB / CIFS**](smb-backend/) (new in v0.4.0) | Samba server, Windows share, or NAS appliance | `.raw` files on an SMB share. Agent runs `mount.cifs` |
  | [**External NFS**](nfs-backend/) | NFS server with an export | `.raw` files on the NFS export. Agent runs `mount.nfs` |
  | [**External iSCSI**](iscsi-backend/) | iSCSI target (any vendor) | Per-VM Logical Volume on a shared iSCSI LUN. Agent runs `iscsiadm` + `lvchange` |

Pick the external backend that matches your existing infrastructure — there's no vendor lock-in.

---

## Where to next

- [**Manage storage backends**](manage-backends/) — Add, edit, set as default, and remove backends through the UI
- [**Add an external SMB / CIFS backend**](smb-backend/) — Samba, Windows shares, NAS appliances
- [**Add an external NFS backend**](nfs-backend/) — Linux/Unix NFS servers
- [**Add an external iSCSI backend**](iscsi-backend/) — Vendor-agnostic block storage with auto-provisioned per-VM LVs
- [**Volumes**](../volumes/) — Persistent data disks attached to VMs

---

## Architecture (short version)

- **Manager** is unprivileged. It tracks each backend's config in Postgres and orchestrates VM lifecycle.
- **Agent** runs as root on the KVM host. It owns the privileged operations: `mount.nfs`, `mount.cifs`, `iscsiadm`, `lvchange`. For each backend kind the manager calls the agent over HTTP at `/v1/storage/<kind>/*`.
- **Credentials never live in the manager DB.** For SMB and other authenticated backends, the manager forwards credentials to the agent's on-disk credential store at `/etc/nqrust/storage-creds/<backend-id>.cred` (`mode 0600`, root:root) — the Proxmox pattern.

---

## Adding a backend live (no restart)

Backends added through the UI take effect immediately. The manager's in-memory backend registry is updated atomically when you click **Add backend**, so VM-create against the new backend works on the next API call. The same applies to deletes — once you confirm removal in the UI the backend is gone from the registry. This means you can iterate on backend config (e.g. trying different SMB versions) without restarting the manager.
