# NFS backend live smoke

Validates the NFS backend end-to-end against a Docker-hosted NFS server.

## Prerequisites

- Manager and agent built from this branch.
- Docker installed on the test host.
- `nfs-common` package installed (provides `mount.nfs`, `findmnt`).

## Setup

```bash
docker run -d --name nfs-smoke \
  --privileged \
  -p 2049:2049 \
  -e SHARED_DIRECTORY=/data \
  -v /tmp/nfs-smoke-data:/data \
  itsthenetwork/nfs-server-alpine:latest

# On the manager host, mount the export so the manager can write to it
sudo mkdir -p /mnt/nfs-mgr
sudo mount -t nfs 127.0.0.1:/ /mnt/nfs-mgr
ls /mnt/nfs-mgr   # should be empty
```

Add to the manager's storage TOML:

```toml
[[storage_backend]]
name = "nfs-smoke"
kind = "nfs"
is_default = false

[storage_backend.config]
server = "127.0.0.1"
export = "/"
manager_mount_path = "/mnt/nfs-mgr"
```

Start the agent with:

```bash
AGENT_NFS_MOUNT_BASE=/var/lib/nqrust/nfs ./target/release/agent
```

## Test L1 — provision + attach + populate + boot

1. Create a VM via the manager API with `backend_id` pointing to the `nfs-smoke` backend.
2. Confirm a sparse `nfs-<uuid>.raw` appears under `/tmp/nfs-smoke-data/`.
3. Confirm `findmnt --target /var/lib/nqrust/nfs/127.0.0.1:` shows the share mounted on the agent host.
4. Boot the VM; verify `cat /etc/os-release` over the shell endpoint.
5. Delete the VM; confirm the file is unlinked.

Expect: VM boots, file is unlinked on delete, no orphan mounts.

## Test L2 — snapshot + clone

1. Create a VM as in L1, write a marker file inside.
2. Snapshot the volume. Confirm `nfs-<uuid>.raw.snap-<name>` appears alongside.
3. Create a new VM with `clone_from_snapshot` against that snapshot.
4. Boot the new VM; confirm the marker file is present.

## Cleanup

```bash
sudo umount /mnt/nfs-mgr
docker rm -f nfs-smoke
sudo rm -rf /tmp/nfs-smoke-data
```
