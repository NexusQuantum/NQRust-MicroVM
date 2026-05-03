#!/usr/bin/env bash
# Run raftblk vhost-user-blk smoke inside a KubeVirt VM. Verified
# end-to-end in this same shape; see commit message for marker output.
#
# Single-VM by design — see commit message for the rationale (manager
# is single-node, 3-node Raft semantics covered by in-process tests).
#
# Prereqs: kubeconfig with KubeVirt + CDI, host nested-virt enabled.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
NS="${NS:-raftblk-smoke}"
VM="${VM:-raftblk-smoke}"
KEY="${KEY:-/tmp/raftblk-kubevirt/raftblk-key}"
KEY_DIR="$(dirname "$KEY")"
KNOWN_HOSTS="$KEY_DIR/known_hosts"

FC_BIN="${FC_BIN:-$HOME/.local/bin/firecracker}"
KERNEL="${KERNEL:-/tmp/raftblk-test/vmlinux}"
INITRD="${INITRD:-/tmp/raftblk-test/initramfs-custom.cpio}"
AGENT_BIN="${AGENT_BIN:-$REPO_ROOT/target/release/agent}"
DAEMON_BIN="${DAEMON_BIN:-$REPO_ROOT/target/release/raftblk-vhost}"

for f in "$FC_BIN" "$KERNEL" "$INITRD" "$AGENT_BIN" "$DAEMON_BIN"; do
    [[ -e "$f" ]] || { echo "missing: $f"; exit 1; }
done
mkdir -p "$KEY_DIR"
[[ -f "$KEY" ]] || ssh-keygen -t ed25519 -N '' -f "$KEY" -C "raftblk-smoke-bot" -q
PUBKEY="$(cat "$KEY.pub")"

cleanup() {
    kubectl delete ns "$NS" --wait=false --ignore-not-found 2>&1 | head -1 || true
}
trap cleanup EXIT

echo "[1/5] applying namespace + DataVolume + cloud-init + VM"
cat <<EOF | kubectl apply -f -
---
apiVersion: v1
kind: Namespace
metadata: { name: $NS }
---
apiVersion: cdi.kubevirt.io/v1beta1
kind: DataVolume
metadata: { name: $VM-disk, namespace: $NS }
spec:
  source:
    http:
      url: https://cloud-images.ubuntu.com/noble/current/noble-server-cloudimg-amd64.img
  pvc:
    accessModes: [ReadWriteOnce]
    resources: { requests: { storage: 25Gi } }
    storageClassName: local-path
---
apiVersion: v1
kind: Secret
metadata: { name: $VM-ud, namespace: $NS }
type: Opaque
stringData:
  userdata: |
    #cloud-config
    hostname: $VM
    ssh_pwauth: false
    disable_root: false
    users:
      - name: root
        lock_passwd: true
        ssh_authorized_keys: ['$PUBKEY']
    package_update: true
    packages: [openssh-server, curl]
---
apiVersion: kubevirt.io/v1
kind: VirtualMachine
metadata: { name: $VM, namespace: $NS }
spec:
  runStrategy: Always
  template:
    spec:
      domain:
        cpu: { model: host-passthrough, cores: 4 }
        memory: { guest: 6Gi }
        devices:
          disks:
            - { disk: { bus: virtio }, name: rootdisk }
            - { disk: { bus: virtio }, name: cidisk }
          interfaces:
            - { masquerade: {}, model: virtio, name: default }
        machine: { type: q35 }
      networks:
        - { name: default, pod: {} }
      volumes:
        - { dataVolume: { name: $VM-disk }, name: rootdisk }
        - { cloudInitNoCloud: { secretRef: { name: $VM-ud } }, name: cidisk }
EOF

echo "[2/5] waiting for VM Ready"
kubectl -n "$NS" wait --for=jsonpath='{.status.ready}'=true vm/"$VM" --timeout=10m
IP="$(kubectl -n "$NS" get vmi "$VM" -o jsonpath='{.status.interfaces[0].ipAddress}')"
echo "    VM IP: $IP"

echo "[3/5] waiting for SSH"
for _ in {1..120}; do
    if ssh -i "$KEY" -o ConnectTimeout=2 -o StrictHostKeyChecking=accept-new \
        -o UserKnownHostsFile="$KNOWN_HOSTS" root@"$IP" 'true' 2>/dev/null; then
        break
    fi
    sleep 5
done

echo "[4/5] uploading bundle"
BUNDLE="$KEY_DIR/bundle"
mkdir -p "$BUNDLE"
cp "$AGENT_BIN" "$BUNDLE/agent"
cp "$DAEMON_BIN" "$BUNDLE/raftblk-vhost"
cp "$FC_BIN" "$BUNDLE/firecracker"
cp "$KERNEL" "$BUNDLE/vmlinux"
cp "$INITRD" "$BUNDLE/initramfs-custom.cpio"
cp "$REPO_ROOT/scripts/raftblk-microvm-smoke.sh" "$BUNDLE/"
cp "$REPO_ROOT/scripts/raftblk-init-template.sh" "$BUNDLE/"
scp -i "$KEY" -o UserKnownHostsFile="$KNOWN_HOSTS" -o StrictHostKeyChecking=no \
    -r "$BUNDLE" root@"$IP":/root/

echo "[5/5] running smoke inside VM"
ssh -i "$KEY" -o UserKnownHostsFile="$KNOWN_HOSTS" -o StrictHostKeyChecking=no root@"$IP" '
    set -euo pipefail
    cp /root/bundle/firecracker /usr/local/bin/firecracker
    chmod +x /usr/local/bin/firecracker
    mkdir -p /tmp/raftblk-test
    cp /root/bundle/vmlinux /root/bundle/initramfs-custom.cpio /tmp/raftblk-test/
    FC_BIN=/usr/local/bin/firecracker \
    AGENT_BIN=/root/bundle/agent \
    DAEMON_BIN=/root/bundle/raftblk-vhost \
    KERNEL=/tmp/raftblk-test/vmlinux \
    INITRD=/tmp/raftblk-test/initramfs-custom.cpio \
        bash /root/bundle/raftblk-microvm-smoke.sh
'

echo "PASS: KubeVirt-hosted smoke completed"
