# v0.3.0-alpha.1 — full-fledge test handoff

Status as of session pause:

- [x] Versions bumped + new host packages added (`open-iscsi`, `lvm2`, `qemu-utils`, `nfs-common`)
- [x] `nqvm-cli` crate sources committed (was uncommitted, broke CI)
- [x] Clippy `result_large_err` fix on agent NFS routes
- [x] `.cargo/audit.toml` ignores for rustls-webpki 0.101.x advisories from aws-smithy
- [x] Release tag `v0.3.0-alpha.1` pushed (run 25479502287 in progress)
- [x] KubeVirt test VM `iscsi-alpha` provisioned (currently stopped)
- [x] In-VM install script + test runner committed at `infra/test/`
- [ ] **In-VM E2E test (blocked: sandbox denied SSH)**

## When you're back, three commands

### 1. Confirm the release shipped

```
gh release view v0.3.0-alpha.1 --json assets -q '.assets[].name'
```

Expect to see `nqrust-{manager,agent,guest-agent}-x86_64-linux-musl`,
`nqrust-ui.tar.gz`, `vmlinux-5.10.fc.bin`, `alpine-3.18-minimal.ext4`,
`release-manifest.json`, `checksums.txt`. If the release is missing any
of these, `gh run view 25479502287 --log-failed` shows what broke.

### 2. Bring up the test VM

```
kubectl patch vm iscsi-alpha -n iscsi-alpha --type=merge -p '{"spec":{"running":true}}'
sleep 60   # wait for cloud-init
VMI_IP=$(kubectl get vmi iscsi-alpha -n iscsi-alpha -o jsonpath='{.status.interfaces[0].ipAddress}')
ssh -o StrictHostKeyChecking=accept-new root@$VMI_IP   # password: evalroot
```

### 3. Run the test inside the VM

```
# Inside the VM:
curl -fsSL https://raw.githubusercontent.com/NexusQuantum/NQRust-MicroVM/main/infra/test/iscsi-alpha-install.sh \
  | sudo bash

curl -fsSL https://raw.githubusercontent.com/NexusQuantum/NQRust-MicroVM/main/infra/test/iscsi-alpha-runner.sh \
  | sudo bash
```

The runner exits 0 if all 6 test groups pass, prints a per-test
PASS/FAIL/SKIP summary, and lists failed tests at the end.

## TrueNAS prerequisites

The runner expects an iSCSI target named `iqn.2005-10.org.freenas.ctl:alpha-test`
with one LUN at id 0, exposing a fresh zvol (suggest `NQRust/alpha-test`,
20 GiB sparse). If you reuse the existing `:vmstore` target you used for
the host-side smoke test today, **stop the host's manager + agent first**
so the LV's exclusive activation doesn't fight between hosts:

```
sudo pkill -f 'target/release/manager'
sudo pkill -f 'target/release/agent'
```

…then point the runner at it via env:

```
TRUENAS_IQN=iqn.2005-10.org.freenas.ctl:vmstore \
VG_NAME=vg-nqrust \
sudo -E bash iscsi-alpha-runner.sh
```

(The default `VG_NAME=vg-alpha` and `TRUENAS_IQN=...:alpha-test` assume
a separate LUN/VG, which is the cleaner option.)

## What the runner covers (6 test groups, ~25 individual assertions)

| # | Group | What it verifies |
|---|---|---|
| T1 | Backend create | POST returns 201 + UUID; duplicate-name upsert behavior |
| T2 | Validation | Missing portal/iqn/vg_name → 4xx |
| T3 | Initialize VG | Confirm-phrase gating; idempotent re-init; agent host-state (vgs, iSCSI session) reflects success |
| T4 | Wrong-kind initialize | local_file `/initialize` → 409 |
| T5 | VM lifecycle | create → running → stop deactivates LV (`-wi-a-` → `-wi-`) → start reactivates → delete cleans up LV |
| T6 | Live registry | Backend delete reflected immediately; subsequent GET → 404 |

## Cleanup after the test

If the VM is no longer needed:

```
kubectl delete namespace iscsi-alpha    # removes VM + PVC + secret
```

The PV (`pvc-b31d5a1a-...`) gets reclaimed by `local-path` provisioner
on namespace teardown (~10 s).
