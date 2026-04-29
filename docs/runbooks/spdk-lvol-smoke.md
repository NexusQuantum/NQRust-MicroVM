# SPDK Lvol Smoke Test

This runbook exercises the B-I SPDK lvol backend against a real local SPDK process.

## Prerequisites

- SPDK is running and exposes a JSON-RPC Unix socket, default `/run/spdk/rpc.sock`.
- An SPDK lvol store exists, default name `nexus`.
- Linux NBD devices exist and the module is loaded:

```bash
sudo modprobe nbd nbds_max=8
```

For a local development-only SPDK target backed by memory, run:

```bash
./scripts/spdk-dev-bootstrap.sh
```

This builds SPDK under `.worktrees/spdk` if needed, starts `spdk_tgt` with a malloc bdev, creates an lvol store, and prints the matching smoke-test command.

The bootstrap is a developer convenience, not production SPDK lifecycle management. It intentionally:

- creates a Python virtualenv for SPDK build helpers (`meson`, `ninja`, `jinja2`, `pyelftools`, `tabulate`);
- passes DPDK `max_numa_nodes=1` for machines without `libnuma`;
- prunes optional SPDK build targets that require extra host packages or large memory pools (`bdev_aio`, iSCSI target pieces, and libaio-backed utility apps);
- starts `spdk_tgt` with a small memory/iobuf/bdev profile suitable for the smoke test;
- chmods the configured NBD devices so the Rust test can access them as the current user.

On Arch, the bootstrap skips SPDK's `pkgdep.sh` by default to avoid accidental partial upgrades. If configure reports missing dependencies, either install them manually after a full system upgrade:

```bash
sudo pacman -Syu
```

or explicitly allow SPDK's dependency script:

```bash
SPDK_RUN_PKGDEP=1 ./scripts/spdk-dev-bootstrap.sh
```

## Run

```bash
AGENT_SPDK_IT_RPC_SOCKET=/run/spdk/rpc.sock \
AGENT_SPDK_IT_LVS_NAME=nexus \
AGENT_SPDK_IT_NBD_DEVICES=/dev/nbd0,/dev/nbd1 \
./scripts/spdk-lvol-smoke.sh
```

The test creates a temporary lvol, attaches it through the agent backend, imports deterministic bytes through NBD, snapshots the lvol, reads the snapshot through NBD, verifies the bytes, detaches, and deletes the lvol.

## Notes

- The test is ignored by default and is not part of normal CI.
- NBD devices in `AGENT_SPDK_IT_NBD_DEVICES` must be unused.
- `scripts/spdk-lvol-smoke.sh` does not start or configure SPDK; it assumes the operator or dev bootstrap has already created the lvstore.
- Stop the dev target with `sudo kill $(cat /tmp/nqrust-spdk-tgt.pid)` when finished.
