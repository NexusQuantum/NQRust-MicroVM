# Storage UX Proxmox-parity smoke

Validates the discovery + status + edit improvements end-to-end via the
running UI. Run once after merging plan
`2026-05-03-proxmox-parity-storage-ux.md`.

## Prerequisites

- Manager + UI + Postgres running (see CLAUDE.md dev setup).
- `nfs-common` installed on the manager host (`showmount`, `mount.nfs`).
- `open-iscsi` installed on the manager host (`iscsiadm`).
- A reachable NFS server with at least one export (the project's TrueNAS
  works — `NQRust/harvester-nfs` is exported by default).
- A reachable iSCSI portal (the TrueNAS at `<ip>:3260`).

## Test 1 — NFS export discovery

1. Open `http://localhost:3000/storage`.
2. Click *Add backend*.
3. Pick kind = NFS.
4. Type the NFS server IP into *NFS server*. Tab out.
5. Within ~5 seconds the *Export path* field should turn into a
   dropdown listing exports (e.g. `/mnt/NQRust/harvester-nfs`,
   `/mnt/NQRust/iso`).
6. Pick one, click *Add backend*. Confirm a 201 + new row in the
   list.

If the field stays as a text input with an "Could not list exports"
warning, `showmount` failed — check the manager log for the exact
error (server not allowing showmount, firewall, etc.) and confirm
the text input still accepts a hand-typed path.

## Test 2 — iSCSI target discovery

1. *Add backend → iSCSI (generic)* (or TrueNAS).
2. Type the portal `<ip>:3260` into *Discovery portal*.
3. Below the field, click *Show reachable targets*.
4. Confirm the popover lists at least one IQN (the existing
   `csi-pvc-*-harvester` rows on the project TrueNAS, plus any
   `nqrust-v-*` rows once the iSCSI runbook has been run).

## Test 3 — Status indicator

1. With at least one NFS or TrueNAS backend in the list, observe the
   leftmost dot:
   - Green: backend reachable.
   - Red: unreachable. Hover for the exact error.
2. Stop the NFS server (or block port 2049 with `iptables`) and wait
   15s. The dot turns red.
3. Restart the server. Dot returns to green within 15s.

## Test 4 — Capacity display

1. For a TrueNAS backend with `TRUENAS_API_KEY` env var set on the
   manager process, the Capacity column shows
   `<used> / <total> (<pct>%)`.
2. For NFS / local file / iSCSI backends, the column shows `—`
   (those probes don't return capacity).

## Test 5 — Edit in place

1. Click the pencil icon next to a non-default backend.
2. Wait briefly for the dialog to load the backend's existing config
   (the Save button is disabled until the config arrives).
3. Toggle "Set as default" on. Click Save.
4. Confirm the table refreshes with the *default* badge moved to
   that row.
5. Toggle off again to revert.

Edit only changes `is_default` in v1. Editing kind-specific config
(URLs, keys, mount paths, group ids) is not yet exposed as form
fields — operators should Remove + re-Add the backend to change
those.

## Cleanup

Nothing to undo — all operations are read-only or in-place.

## What "done done" means

Tests 1–5 pass, status dot updates within 15s, capacity numbers
match TrueNAS's own *Datasets* page (modulo rounding), edit save
returns 200.
