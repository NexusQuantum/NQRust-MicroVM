# B-III live smoke runbook

The B-III code-side is complete (commits `689a418`..`6832b6f` on
`feature/raft-block-prototype`). What's left is the live KubeVirt
validation. This runbook covers the prerequisites and the smoke steps.

## Why this isn't already validated

The previous KubeVirt smoke VM (`raftblk-smoke` in namespace
`raftblk-smoke`) uses `masquerade` networking, which NATs the launcher
pod's port 22 to a different IP on the VM. Direct `ssh
root@10.42.0.169` from the host returns `no route to host` because
nothing on the host's routing table reaches the VM's masquerade-side
IP, and `virtctl ssh` returns the same error because the launcher's
SSH proxy depends on the VM having `accessCredentials` wired into its
spec — the smoke VM's cloud-init only baked the key in on first boot.

Earlier sessions worked because the smoke VM at the time had either
`bridge` networking or an explicit Service exposing port 22. Whatever
that plumbing was, it didn't survive the cluster's lifecycle.

## Prerequisites before running the smoke

Pick one of these three:

### Option A — recreate the VM with `bridge` networking

```yaml
spec:
  template:
    spec:
      domain:
        devices:
          interfaces:
          - bridge: {}     # was: masquerade: {}
            name: default
      networks:
      - name: default
        pod: {}
```

`kubectl apply -f manifests.yaml`, wait for VMI Ready, then SSH directly
on the new pod IP from the host's routing table.

### Option B — NodePort Service to expose VM port 22

```yaml
apiVersion: v1
kind: Service
metadata:
  name: raftblk-smoke-ssh
  namespace: raftblk-smoke
spec:
  type: NodePort
  selector:
    kubevirt.io/domain: raftblk-smoke
  ports:
  - port: 22
    targetPort: 22
    nodePort: 32222
```

Then `ssh -p 32222 root@<node-ip>`.

### Option C — wire `accessCredentials` for virtctl

```yaml
spec:
  template:
    spec:
      accessCredentials:
      - sshPublicKey:
          source:
            secret:
              secretName: raftblk-smoke-ssh-keys
          propagationMethod:
            qemuGuestAgent:
              users: ["root"]
```

Create the secret with the public key, restart the VMI. After that,
`virtctl ssh -n raftblk-smoke vmi/raftblk-smoke --username root
--identity-file /tmp/raftblk-kubevirt/raftblk-key` works.

## The smoke itself

Once SSH access is restored, follow the prior runbook
`docs/runbooks/raft-block-microvm-smoke.md` for the basic 1-node and
3-node setup, then run the B-III live tests below.

### Test L1 — repair a lagging follower (Task 2)

1. Bring up 3-node cluster, create a VM with `backend_id=raft-three`,
   confirm md5 matches across all 3 stub files.
2. `pkill -9 -f /root/bundle/agent` for agent-3.
3. Write through openraft on the surviving leader (any
   `runtime_write` POST against agent-1's address).
4. Restart agent-3.
5. `nqvm storage repair --backend $BID --group $GID --node 3` and
   poll `/repair_status` until `last_applied_index` matches the
   leader's commit.

Expect: agent-3's last_applied_index converges within ~10 s of the
repair call.

### Test L2 — replica add (Task 3)

1. Bring up 3-node cluster, create a VM. Cluster has nodes 1/2/3.
2. Bring up agent-4 on a 4th port (or 4th host). Set its
   `spdk_backend_id` via `nqvm hosts spdk-backend-id --host $H4
   --id $LVOL`.
3. `nqvm storage add-replica --backend $BID --group $GID --node 4
   --agent-base-url http://127.0.0.1:9093/v1/raft_block
   --spdk-backend-id $LVOL`.
4. After commit: `dd if=/var/lib/spdk-stub/node-4.dev | md5sum`
   matches the source rootfs ext4.

Expect: 4th replica reaches the same applied index as the leader,
md5 of capacity region matches.

### Test L3 — replica remove (Task 4) + leader transfer (Task 4a)

1. From the 4-replica cluster from L2, transfer leadership off node 1
   (`nqvm storage replicas` lists current leader; use the leader-transfer
   endpoint).
2. `nqvm storage remove-replica --backend $BID --group $GID --node 1`.
3. Confirm DB row is removed (`removed_at` set), agent-1's spdk stub
   file is unlinked.

Expect: cluster continues to commit writes through node 2/3/4, no data
loss.

### Test L4 — host decommission auto-drain (Task 6)

1. Bring up 4-node cluster (3 voters + 1 hot-spare): set
   `nqvm hosts hot-spare --host $H4 --on`.
2. Place all groups on hosts 1/2/3.
3. `nqvm hosts decommission --host $H1`.
4. Within `SCAN_INTERVAL` (60 s) the auto-reconciler should run
   `plan_decommission` for host 1, drive add/remove pairs onto host 4,
   and transition host 1 to `decommissioned`.

Expect: every group's md5 matches across hosts 2/3/4 after drain.
Host 1's lifecycle column reads `decommissioned`.

### Test L5 — hot-spare promotion (Task 7)

1. Bring up 4-node cluster as in L4. Confirm host 4 is hot-spare.
2. `pkill -9 -f /root/bundle/agent` on agent-1 host (or `kubectl
   delete pod` if running in-cluster) to simulate failure. Do NOT
   restart it.
3. Wait `PROMOTION_THRESHOLD` (10 min by default).
4. The auto-reconciler runs `plan_hot_spare_promotion`, adds host 4
   as a 4th replica to every group host 1 was hosting.

Expect: all groups have 4 replicas (1, 2, 3, 4) and md5 matches across
hosts 2/3/4. Host 1 is still listed as a member but unreachable; the
operator runs `nqvm storage remove-replica --node 1` to clean up.

### Test L6 — UI panel acceptance

1. Visit `/storage` in the UI.
2. Verify Groups tab shows the cluster from L1's setup with correct
   `quorum_state: leader_steady`, all 3 replicas reachable, applied
   indexes match.
3. Toggle hot-spare on a host via the Hosts tab.
4. Trigger a Repair on a lagging follower (after L1's stop/start of
   agent-3) and confirm the spinner clears + applied_index updates.
5. Click Execute on the Rebalance tab when no moves are needed; confirm
   `Rebalance no-op` note shows and no Execute is allowed.

Expect: UI reflects backend state within the configured refetch
intervals (10–30 s) without a manual refresh.

## Cleanup

```bash
nqvm storage groups --backend $BID            # list everything
# for each group:
nqvm storage remove-replica --backend $BID --group $GID --node $N  # one at a time

# delete VMs and volumes through the normal API
# verify /var/lib/spdk-stub/node-*.dev are unlinked
```

## What "done done" means for B-III

Tests L1–L6 pass on the live env. At that point the checklist in
`docs/superpowers/plans/2026-05-02-raft-block-reconfiguration.md` is
fully ticked. Until then, every code path in this doc is exercised by
the unit tests in `cargo test --workspace` (261 tests passing); the
live smoke is the operator-environment confirmation that the unit
tests' assumptions about agent + Openraft + KVM behavior hold under
real wire conditions.
