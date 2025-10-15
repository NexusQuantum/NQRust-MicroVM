# UX Improvements Summary

We invested in smoothing the Firecracker management workflow so users no longer deal with low-level host plumbing. Below is a complete list of noteworthy changes.

## Storage & Snapshot Automation
- Added a `LocalStorage` helper that auto-creates `/srv/fc/vms/<vm>` directories for logs, sockets, snapshots, and data disks.
- Root file systems sourced from images are now copied into the managed storage tree. Restarts no longer rely on global host paths.
- `CreateDriveReq` can omit `path_on_host`; the manager provisions a sparse disk (optional `size_bytes` hint) and wires it to Firecracker.
- Cleanup is automatic: manager passes each VM’s storage path to the agent, and the agent deletes the directory during `POST /stop`.

## Networking Convenience
- Manager no longer hard-codes the bridge name. It learns the preferred bridge from host capabilities (populated by the agent) and falls back to `MANAGER_BRIDGE`/`fcbr0` only if needed.
- Agent exposes `GET /agent/v1/system/bridge`, returning the configured bridge (`AGENT_BRIDGE` env, default `fcbr0`).
- TAP creation now uses the discovered bridge everywhere (create, restart, snapshot restore).

## Host Capability Tracking
- On startup, the manager can register/update its host row with capability metadata (currently the active bridge name).
- Host capability JSON is now the single source for network selection when spinning up VMs.

## Drive & NIC API Polish
- Drive creation/updating still accepts explicit host paths when allowed, but defaults are path-free and safer.
- Data disks are stored alongside VM state, simplifying backup/cleanup.
- NIC workflow is unchanged functionally, but network selection provides the bridge context automatically for future pooling.

## Agent Enhancements
- `POST /agent/v1/vms/{id}/stop` accepts an optional `storage_path` and removes it after shutting down systemd scope/TAP.
- Added a lightweight system module to publish bridge metadata.

## Build & Code Hygiene
- Various warnings remain (e.g., reconciler temporary variables, unused imports in agent hotplug modules). They are tracked separately for cleanup but do not affect runtime.

These updates drastically reduce the amount of infrastructure detail end users need to manage: no more manual disk paths, the bridge is auto-selected, and VM artifacts are reclaimed automatically.
