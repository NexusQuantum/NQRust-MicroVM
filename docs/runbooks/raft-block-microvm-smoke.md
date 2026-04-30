# Raft-Block Replicated Storage — microVM Smoke Test

This runbook walks through bringing up a real three-agent Raft-replicated
block group, attaching it to a Firecracker microVM as a `vhost-user-blk`
disk, and proving that a guest write survives a leader kill. It covers the
two B-II Exit Criteria items that require operator action:

- **Item 4 — Move committed block bytes from JSON to SPDK lvol/NBD-backed
  replicas.** The current raft-block storage adapter writes committed
  bytes to a JSON file per replica. Production replaces that with an
  SPDK lvol on each host, exposed via NBD for the populate path and via
  vhost-user for the guest data path. This step is documented here
  because building, running, and validating SPDK requires sudo and a
  particular host kernel/hugepage configuration.
- **Item 8 — Real microVM smoke.** Boot a Firecracker guest with a
  vhost-user-blk drive backed by `raftblk-vhost`, write a known pattern
  from inside the guest, kill the leader agent, observe failover, and
  verify the bytes still read correctly.

## What's already done (no operator action needed)

These have landed on `feature/raft-block-prototype` and are exercised by
unit tests:

- `nexus-raft-block`: pure replicated-block correctness model, Openraft
  storage harness, `Adaptor`-wrapped v1->v2 storage.
- `apps/agent/src/features/raft_block.rs`:
  - HTTP transport (`/v1/raft_block/openraft/{append_entries,vote,
    install_snapshot}`)
  - `RaftBlockNetworkFactory` + `RaftBlockNetworkConnection` Openraft
    network adapter (translates reqwest errors to `RPCError` taxonomy)
  - `RaftBlockRuntime` (per-group `openraft::Raft` instance, storage,
    network factory)
  - Per-group runtime registry on `RaftBlockState`
  - `runtime_start`, `runtime_initialize`, `runtime_write` routes
  - 24 unit tests including 3-node cluster integration (replicate,
    leader-kill failover, quorum-loss block) — all in-process.
- `apps/manager/src/features/storage/backends/raft_spdk.rs`:
  - `production_provisioning_enabled = true` provisions a real Raft group
    by calling `create` -> `runtime_start` (each replica) ->
    `runtime_initialize` (leader). Validates the locator carries
    `production_replica` instead of `prototype_replica`.
- `crates/raftblk-vhost`:
  - Virtio-blk request parsing (alignment, oversized-read caps,
    GET_ID serial format).
  - `BlockBackend` trait + `RaftBlockBackend` (HTTP -> agent ->
    `runtime_client_write` -> Raft commit) + `InMemoryBlockBackend` (test).
  - 12 unit tests.
- `apps/raftblk-vhost`: daemon binary that connects to the agent,
  smoke-tests with a GET_ID round-trip, and parks. The vhost-user
  protocol layer that turns this into a live device is the operator-only
  step (see "Wire the vhost-user-backend daemon" below).

## Topology

```text
                       ┌────────────────────┐
                       │ Manager (1 host)   │
                       │ raft_spdk backend  │
                       │ provision()        │
                       └──┬───────┬──────┬──┘
                          │       │      │
            ┌─────────────┘       │      └──────────────┐
            ▼                     ▼                     ▼
   ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐
   │ Agent host A     │  │ Agent host B     │  │ Agent host C     │
   │ NodeId 1 (leader)│  │ NodeId 2         │  │ NodeId 3         │
   │                  │  │                  │  │                  │
   │ /v1/raft_block   │  │ /v1/raft_block   │  │ /v1/raft_block   │
   │ openraft Raft    │◄─┤ openraft Raft    │◄─┤ openraft Raft    │
   │ SPDK lvol N1     │  │ SPDK lvol N2     │  │ SPDK lvol N3     │
   │                  │  │                  │  │                  │
   │ raftblk-vhost ── vhost-user-blk socket ──► Firecracker guest │
   └──────────────────┘  └──────────────────┘  └──────────────────┘
```

The leader's host runs `raftblk-vhost` and Firecracker. Followers replicate
through HTTP/JSON over the agents' bind addresses.

## Prerequisites per host

On all three hosts:

```bash
# Kernel modules + KVM
sudo modprobe kvm_intel              # or kvm_amd
sudo modprobe vhost_vsock            # for raft_block vsock control plane (optional)
sudo modprobe nbd nbds_max=16        # for SPDK NBD imports

# Hugepages for SPDK (1GB pages preferred; falls back to 2MB)
sudo sh -c "echo 1024 > /proc/sys/vm/nr_hugepages"
sudo mount -t hugetlbfs none /dev/hugepages

# Firecracker binary (B-I PR pinned a specific version)
firecracker --version  # must match
```

On the leader-eligible host (host A) additionally:

```bash
# vhost-user-master test driver — needed once we plug raftblk-vhost into
# vhost-user-backend. Until then, raftblk-vhost smoke-tests the agent
# without opening a vhost-user socket.
sudo modprobe vhost
sudo modprobe vhost_iotlb
```

## Step 1 — Bring up SPDK on each host

Use the existing dev bootstrap from B-I:

```bash
./scripts/spdk-dev-bootstrap.sh
# prints the smoke command and the lvstore name (default: nexus)
```

In production, replace this with managed SPDK lifecycle (systemd unit,
hugepage allocation, persistent lvstore on real NVMe). The dev bootstrap
is for the smoke run only.

Validate the agent can talk to SPDK on each host:

```bash
AGENT_SPDK_IT_RPC_SOCKET=/run/spdk/rpc.sock \
AGENT_SPDK_IT_LVS_NAME=nexus \
AGENT_SPDK_IT_NBD_DEVICES=/dev/nbd0,/dev/nbd1 \
./scripts/spdk-lvol-smoke.sh
```

This is the B-I smoke. It must pass on all three hosts before continuing.

## Step 2 — Configure manager `nqrust.toml`

```toml
# Manager-side raft_spdk backend definition.
[[storage_backend]]
name = "raft-three"
kind = "raft_spdk"
is_default = false

[storage_backend.config]
block_size = 4096
production_provisioning_enabled = true

# Each entry references the SPDK backend on its host plus the agent base URL.
# node_id values must be nonzero and unique across all three.
[[storage_backend.config.replicas]]
node_id = 1
agent_base_url = "http://10.0.0.1:9090"
spdk_backend_id = "11111111-1111-1111-1111-111111111111"  # the SPDK backend uuid on host A

[[storage_backend.config.replicas]]
node_id = 2
agent_base_url = "http://10.0.0.2:9090"
spdk_backend_id = "22222222-2222-2222-2222-222222222222"

[[storage_backend.config.replicas]]
node_id = 3
agent_base_url = "http://10.0.0.3:9090"
spdk_backend_id = "33333333-3333-3333-3333-333333333333"
```

Restart the manager. Validate the backend with:

```bash
curl -s http://localhost:18080/v1/storage_backends | jq '.[] | select(.kind=="raft_spdk")'
```

It should appear with `capabilities.supports_native_snapshots = true` and
the three configured replicas.

## Step 3 — Provision a Raft-replicated volume

```bash
curl -s -X POST http://localhost:18080/v1/volumes \
  -H 'content-type: application/json' \
  -d '{
    "name": "guest-rootfs",
    "size_bytes": 1073741824,
    "backend_id": "<id from Step 2>"
  }' | jq .
```

Manager's `RaftSpdkControlPlaneBackend.provision` will:
1. POST `/v1/raft_block/create` to all three agents.
2. POST `/v1/raft_block/runtime_start` to all three with the peer URL map.
3. POST `/v1/raft_block/runtime_initialize` to host A (the leader).
4. Return a `VolumeHandle` whose locator records `production_replica:
   true` per replica.

Verify a leader was elected:

```bash
curl -s http://10.0.0.1:9090/v1/raft_block/<group_id>/status | jq .
# state: "started", node_id: 1, last_applied_index: 1 (the bootstrap entry)
```

## Step 4 — Wire the vhost-user-backend daemon (operator-only)

This is the bounded remaining work. The data-plane translation layer is
fully implemented and tested in `crates/raftblk-vhost`; the daemon binary
in `apps/raftblk-vhost` parks after the agent smoke test. Replace the park
with a `vhost-user-backend` integration:

```rust
// apps/raftblk-vhost/src/main.rs — Stage 2 sketch
use vhost_user_backend::{VhostUserBackendMut, VhostUserDaemon};
use vhost::vhost_user::message::*;

struct RaftBlkVhostBackend<B: BlockBackend> {
    backend: B,
    // ... vrings, mem table, event_idx ...
}

impl<B: BlockBackend> VhostUserBackendMut for RaftBlkVhostBackend<B> {
    type Bitmap = ...;
    type Vring = ...;

    fn num_queues(&self) -> usize { 1 }
    fn max_queue_size(&self) -> usize { 256 }
    fn features(&self) -> u64 {
        (1 << VIRTIO_F_VERSION_1) | (1 << VIRTIO_BLK_F_SEG_MAX) | ...
    }
    fn handle_event(&mut self, ...) -> io::Result<()> {
        // 1. Pull descriptor chains off the vring
        // 2. Parse outhdr -> request::parse_request(...)
        // 3. block_backend.dispatch(request).await
        // 4. Fill data buffer + inhdr.status
        // 5. Push to used ring + notify guest
    }
}
```

Once that compiles, run:

```bash
sudo /usr/local/bin/raftblk-vhost \
  --socket /var/run/raftblk-<group_id>.sock \
  --agent-base-url http://127.0.0.1:9090/v1/raft_block \
  --group-id <group_id> \
  --block-size 4096 \
  --capacity-bytes 1073741824
```

Expected: a vhost-user socket appears at `/var/run/raftblk-<group_id>.sock`.

## Step 5 — Boot a Firecracker guest with the vhost-user disk

```bash
# Create the FC config
cat > /tmp/vm.json <<EOF
{
  "boot-source": {
    "kernel_image_path": "/srv/images/vmlinux-5.10.fc.bin",
    "boot_args": "console=ttyS0 reboot=k panic=1 pci=off"
  },
  "drives": [
    {
      "drive_id": "rootfs",
      "is_root_device": true,
      "is_read_only": false,
      "vhost_user_socket": "/var/run/raftblk-<group_id>.sock"
    }
  ],
  "machine-config": {
    "vcpu_count": 1,
    "mem_size_mib": 256
  }
}
EOF

# Boot
firecracker --api-sock /tmp/fc.sock --config-file /tmp/vm.json
```

Inside the guest:

```bash
# Pattern write
echo 'raftblk-test-pattern' | dd of=/dev/vda bs=4096 count=1 seek=10 oflag=direct
sync

# Confirm
dd if=/dev/vda bs=4096 count=1 skip=10 iflag=direct | head -c 32
# expect: raftblk-test-pattern
```

## Step 6 — Leader-kill failover

From the manager host, kill the leader's agent process:

```bash
ssh root@10.0.0.1 systemctl stop nqrust-agent
```

Within ~1s the surviving agents elect a new leader. Verify:

```bash
curl -s http://10.0.0.2:9090/v1/raft_block/<group_id>/status | jq .
# Should show this node as the new leader, last_applied_index unchanged.
```

The guest's I/O may briefly stall (election timeout window, ~500-1000ms)
then resume against the new leader. From inside the guest:

```bash
dd if=/dev/vda bs=4096 count=1 skip=10 iflag=direct | head -c 32
# Still: raftblk-test-pattern -- pre-failure committed bytes survived.
```

Write a new pattern post-failover:

```bash
echo 'after-failover' | dd of=/dev/vda bs=4096 count=1 seek=20 oflag=direct
sync

dd if=/dev/vda bs=4096 count=1 skip=20 iflag=direct | head -c 32
# expect: after-failover
```

## Troubleshooting

| Symptom | Likely cause | Fix |
|---|---|---|
| `provision` returns 502 with "raft_spdk runtime_start on node N failed" | Agent on node N can't bind, or the storage group wasn't created on N. | `curl http://<agent>/v1/raft_block/<group>/status` — should show "started". If not, restart the agent. |
| `runtime_initialize` succeeds but `status.state` stays "started" with no leader | Election timeout fires but no quorum (peer agents unreachable). | Check `curl http://<peer-N>/v1/raft_block/<group>/status` is reachable from the leader host. Inspect agent logs for `RaftBlockNetworkFactory` errors. |
| Guest sees I/O hang after leader kill but never recovers | The new leader was elected but the daemon (`raftblk-vhost`) is pointed at the dead agent. | The daemon connects to a fixed local agent. After failover, the agent the daemon talks to is now a follower, which forwards writes via `Raft::client_write` -> `ForwardToLeader`. The current implementation does not auto-redirect; restart `raftblk-vhost` after failover, or run one daemon per agent (only the leader's daemon services I/O). |
| `vhost_user_socket` rejected by Firecracker as unknown field | The Firecracker version pinned in this repo (v1.13.1) accepts vhost-user-blk drives via the `vhost_user_socket` field. If the FC runtime is older, the operator must upgrade. | `firecracker --version`; bump per `install-firecracker.sh`. |

## What's still pending (not in this PR)

- **Stage 2 of `raftblk-vhost`** (vhost-user-backend daemon) — the data
  plane is tested; the protocol glue is mechanical and gated on an
  operator host with hugepages + `vhost` modules + a guest VM to verify
  against.
- **SPDK-lvol-backed bytes** — the agent's storage adapter still writes
  committed bytes to a JSON file (`PersistentReplicaState` ->
  `FileReplicaStore`). Replacing this with an `SpdkLvolReplicaStore` that
  writes through the SPDK NBD path requires:
  - A `ReplicaStore` trait in `nexus-raft-block` so the storage backend
    is pluggable. (Today `FileReplicaStore` is the only impl.)
  - An `SpdkLvolReplicaStore` impl on the agent side that performs
    writes through the NBD device pool already used by the B-I import
    path.
  - A migration step: existing JSON-backed groups would need to be
    re-bootstrapped onto SPDK (operator-driven; no in-place migration in
    this PR).
- **Snapshot streaming through Raft** — `read_snapshot` on the host
  backend reads through the local Raft snapshot, but the manager-side
  backup pipeline doesn't yet drive it. Tracked under B-II item 5
  follow-on.
- **Cluster reconfiguration (B-III)** — not started; this runbook is
  static-three-node only.

When all of the above lands, this runbook becomes the canonical end-to-end
validation for the B-II story and the gating step for declaring B-II done.
