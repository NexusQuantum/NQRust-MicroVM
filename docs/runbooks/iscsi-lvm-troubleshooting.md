# iSCSI-LVM Backend Troubleshooting

This runbook covers the iscsi_lvm storage backend (added 2026-05-04). See
`docs/superpowers/plans/2026-05-04-iscsi-lvm-backend.md` for the design and
`apps/manager/src/features/storage/backends/iscsi_lvm.rs` plus
`apps/agent/src/features/storage/iscsi_lvm.rs` for implementation.

## Architecture recap

Manager talks to agent over HTTP at `/v1/storage/iscsi_lvm/*`. Agent runs as
root, owns the iSCSI session lifecycle (`iscsiadm` with `node.startup=automatic`
so sessions persist across reboots) and all LVM operations on the host.
A single backend = one iSCSI target/LUN with one Volume Group; per-VM disks
are Logical Volumes inside that VG. `lvchange -aey` ensures only one host
holds a given LV active at a time.

## Common failure modes

### Initialize fails with "device is already part of VG 'X'"

The agent's idempotency check refused to overwrite an existing PV/VG signature.
This is by design — `pvcreate` is destructive and we never run it on a device
that already has a recognised volume group on it.

Recovery: confirm with the storage owner that the LUN is yours to wipe, then
on the agent host:

```
wipefs -a /dev/disk/by-path/ip-<portal>:3260-iscsi-<iqn>-lun-<n>
```

Re-issue Initialize. If `wipefs` itself complains the device is in use, the
old VG may still be active — run `vgchange -an <old-vg>` first.

### Initialize fails with "pvcreate failed: exit ..."

Usually leftover signatures or a partition table that `pvcreate` won't step on.
Note that LVMPlugin.pm:96-103 in Proxmox zero-outs only the first sector; that
isn't enough when there's a GPT secondary header at the end of the device.

Recovery, in order of preference:

```
wipefs -a /dev/disk/by-path/...
# if that doesn't clear it:
dd if=/dev/zero of=/dev/disk/by-path/... bs=1M count=10
# and clear the trailing GPT if present:
sgdisk --zap-all /dev/disk/by-path/...
```

### Session keeps disconnecting / agent log shows "iscsiadm session timeout"

First, confirm `node.startup=automatic` was actually written for the node:

```
iscsiadm -m node -T <iqn> -p <portal> -o show | grep node.startup
```

If it says `manual`, the persistence step on the agent never ran — re-run
Initialize, or set it by hand:

```
iscsiadm -m node -T <iqn> -p <portal> -o update -n node.startup -v automatic
```

If startup is automatic and sessions still drop, look outside the agent:
switch port flapping, MTU mismatch on jumbo-frame storage networks, or a
firewall reset on idle TCP. Multipath is out of scope for this backend; if
you need it for production, that's the next milestone.

### VG free space wrong (showing 0 free or "VG not found" in health)

The agent's iSCSI session likely died and the block device disappeared
underneath LVM. Check on the agent host:

```
iscsiadm -m session
ls -l /dev/disk/by-path/ | grep iscsi
vgs <vg-name>
```

If the session is gone, `iscsiadm -m node -T <iqn> -p <portal> --login`. If
the device is back but `vgs` still doesn't see it, try `vgscan --mknodes`. As
a last resort, log out of the target and log back in:

```
iscsiadm -m node -T <iqn> -p <portal> --logout
iscsiadm -m node -T <iqn> -p <portal> --login
```

### lvremove says "Logical volume <name> in use"

Something has the device open — usually a stuck Firecracker process that
didn't release the LV, or a previous deactivate that didn't fire.

Recovery:

```
lvchange -aln /dev/<vg>/<lv>
lvremove /dev/<vg>/<lv>
```

If the deactivate itself hangs, find the holder:

```
lsof /dev/<vg>/<lv>
fuser -mv /dev/<vg>/<lv>
```

Kill the holder before retrying. Do not `--force` an active LV remove; that's
how you corrupt other VMs that happen to share the VG.

### Two managers fighting (HA scenario, not currently supported but documented)

This codebase doesn't yet ship HA-manager. If two manager processes are
pointed at the same database (intentionally or by accident), concurrent
`lvcreate` / `lvremove` against the agent could create duplicate names or
destroy a volume that another manager is mid-attach on.

Recovery: stop the second manager. There is no automatic reconciliation today.
Mitigation tracked in roadmap is Postgres advisory locks around mutating
storage ops; until then, treat manager as a singleton.

### Manager startup says "storage_backend X probe failed"

WARN-level message from `registry.rs` during boot probe. The backend is still
loaded into the registry — the warning just means `health()` failed at
startup. The probe doesn't gate VM-create flow directly, but it's a strong
signal something is wrong: iSCSI session not yet established, agent
unreachable, or VG missing.

Investigate by hitting the backend's health endpoint manually
(`GET /v1/storage_backends/:id/health`) and checking the agent log for the
underlying error.

### VM-create fails with "no backend with id ..."

Either the backend was soft-deleted between the UI add and the VM-create
call, or the registry never loaded it. Look for these log lines on manager
startup:

```
storage_backend loaded into registry id=<uuid>
storage_backend skipped id=<uuid> reason=...
```

If you see `skipped`, the reason field tells you why (config parse error,
unknown kind, etc.). If you see neither, the row isn't in the DB — check
`storage_backends` table directly.

### Agent crashes mid-`lvcreate` -> orphan LV that never got tracked in DB

If the agent crashes after `lvcreate` succeeded but before the manager's RPC
response made it back, the LV exists on the array while the volume row was
never inserted. The volume is invisible to the manager but is consuming VG
space.

Manual recovery: list LVs on the agent (`lvs <vg-name>`), cross-reference
against the manager's `storage_volumes` table, and after confirming nothing
is using them:

```
lvremove <vg-name>/<orphan-lv>
```

A reconciler that prunes orphans is on the roadmap; until it lands, this is
a manual step.
