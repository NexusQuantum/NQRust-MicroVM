#!/usr/bin/env bash
# Real microVM smoke test for B-II — closes Exit Criteria item 7.
#
# Boots a Firecracker guest with a vhost-user-blk drive backed by the
# raftblk-vhost daemon. The daemon talks to a single-node in-process
# Raft group on the local agent. The guest writes a known pattern to
# /dev/vda, reads it back, and asserts cmp succeeds.
#
# Verifies, from inside a real Linux guest VM:
#   1. agent starts and serves /v1/raft_block routes
#   2. create_group + runtime_start + runtime_initialize succeed
#   3. raftblk-vhost daemon binds the vhost-user UDS
#   4. Firecracker accepts the vhost_user_blk drive config
#   5. vhost-user negotiation (incl. PROTOCOL_FEATURES bit 30) completes
#   6. Linux sees /dev/vda at the correct capacity
#   7. Guest writes 4KiB at sector 8 to /dev/vda
#   8. Guest reads it back, bytes match
#
# Step 7's write goes through:
#   guest virtio-blk -> virtio-mmio -> Firecracker -> vhost-user UDS ->
#   daemon::handle_event -> handle_chain -> RaftBlockBackend::dispatch ->
#   POST /runtime_write -> RaftBlockState::runtime_client_write ->
#   openraft::Raft::client_write -> InMemoryOpenraftBlockStore::apply
#
# Step 8 reads via /v1/raft_block/read, which sources from the local
# replica that Raft just applied to. Read-back matching is end-to-end
# proof of the full data plane.
#
# Usage
# -----
# Prereqs (operator / CI runner):
#   - Firecracker v1.13.1 binary (default: ~/.local/bin/firecracker)
#   - Linux kernel image (default: /tmp/raftblk-test/vmlinux)
#   - initramfs.cpio with /init from `raftblk-init-template.sh` (default:
#     /tmp/raftblk-test/initramfs-custom.cpio)
#   - /dev/kvm reachable as the running user
#
# Override defaults via env vars:
#   FC_BIN, KERNEL, INITRD, AGENT_BIN, DAEMON_BIN, WORKDIR
#
# Exits 0 when the guest prints `RAFTBLK-SMOKE-IO-VERIFIED`. Exits non-zero
# (with logs surfaced) on any failure.

set -u

WORKDIR="${WORKDIR:-/tmp/raftblk-smoke}"
FC_BIN="${FC_BIN:-$HOME/.local/bin/firecracker}"
KERNEL="${KERNEL:-/tmp/raftblk-test/vmlinux}"
INITRD="${INITRD:-/tmp/raftblk-test/initramfs-custom.cpio}"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
AGENT_BIN="${AGENT_BIN:-$REPO_ROOT/target/release/agent}"
DAEMON_BIN="${DAEMON_BIN:-$REPO_ROOT/target/release/raftblk-vhost}"

mkdir -p "$WORKDIR/run" "$WORKDIR/log"
LOG="$WORKDIR/log/run.log"
: > "$LOG"

echo "=== raftblk-vhost real microVM smoke ===" | tee -a "$LOG"
echo "WORKDIR=$WORKDIR  FC=$FC_BIN" >> "$LOG"
echo "AGENT=$AGENT_BIN  DAEMON=$DAEMON_BIN" >> "$LOG"
echo "KERNEL=$KERNEL  INITRD=$INITRD" >> "$LOG"

# Sanity-check inputs upfront so a missing artifact fails clearly rather
# than after a cascade of partial setup.
for f in "$FC_BIN" "$KERNEL" "$INITRD" "$AGENT_BIN" "$DAEMON_BIN"; do
    if [[ ! -e "$f" ]]; then
        echo "missing required artifact: $f" | tee -a "$LOG"
        exit 1
    fi
done

cleanup() {
    [[ -n "${FC_PID:-}" ]] && kill "$FC_PID" 2>/dev/null
    [[ -n "${DAEMON_PID:-}" ]] && kill "$DAEMON_PID" 2>/dev/null
    [[ -n "${AGENT_PID:-}" ]] && kill "$AGENT_PID" 2>/dev/null
    sleep 0.5
    [[ -n "${FC_PID:-}" ]] && kill -9 "$FC_PID" 2>/dev/null
    [[ -n "${DAEMON_PID:-}" ]] && kill -9 "$DAEMON_PID" 2>/dev/null
    [[ -n "${AGENT_PID:-}" ]] && kill -9 "$AGENT_PID" 2>/dev/null
}
trap cleanup EXIT

echo "[1] starting agent on 127.0.0.1:9090" | tee -a "$LOG"
AGENT_BIND=127.0.0.1:9090 \
    FC_RUN_DIR="$WORKDIR/run" \
    MANAGER_BASE=http://127.0.0.1:1 \
    "$AGENT_BIN" >> "$LOG" 2>&1 &
AGENT_PID=$!

for i in {1..50}; do
    if curl -s --max-time 1 http://127.0.0.1:9090/ > /dev/null 2>&1; then
        break
    fi
    sleep 0.2
done

GROUP_ID=$(uuidgen)
CAPACITY=$((100 * 1024 * 1024))
BLOCK_SIZE=4096

echo "[2] creating raft group $GROUP_ID ($CAPACITY bytes, block_size=$BLOCK_SIZE)" | tee -a "$LOG"
curl -s -X POST http://127.0.0.1:9090/v1/raft_block/create \
    -H 'content-type: application/json' \
    -d "{\"group_id\":\"$GROUP_ID\",\"node_id\":1,\"capacity_bytes\":$CAPACITY,\"block_size\":$BLOCK_SIZE}" >> "$LOG"
echo "" >> "$LOG"

echo "[3] starting Raft runtime + initializing membership" | tee -a "$LOG"
curl -s -X POST http://127.0.0.1:9090/v1/raft_block/runtime_start \
    -H 'content-type: application/json' \
    -d "{\"group_id\":\"$GROUP_ID\",\"peers\":{\"1\":\"http://127.0.0.1:9090\"}}" >> "$LOG"
echo "" >> "$LOG"
curl -s -X POST http://127.0.0.1:9090/v1/raft_block/runtime_initialize \
    -H 'content-type: application/json' \
    -d "{\"group_id\":\"$GROUP_ID\",\"members\":[1]}" >> "$LOG"
echo "" >> "$LOG"
sleep 1

SOCKET="$WORKDIR/run/vhost.sock"
rm -f "$SOCKET"
echo "[4] starting raftblk-vhost daemon on $SOCKET" | tee -a "$LOG"
RUST_LOG=info "$DAEMON_BIN" \
    --socket "$SOCKET" \
    --agent-base-url "http://127.0.0.1:9090/v1/raft_block" \
    --group-id "$GROUP_ID" \
    --block-size $BLOCK_SIZE \
    --capacity-bytes $CAPACITY \
    >> "$LOG" 2>&1 &
DAEMON_PID=$!

for i in {1..50}; do
    [[ -S "$SOCKET" ]] && break
    sleep 0.2
done
[[ -S "$SOCKET" ]] || { echo "FAIL: daemon socket never bound" | tee -a "$LOG"; exit 1; }

cat > "$WORKDIR/run/vm-config.json" <<EOF
{
  "boot-source": {
    "kernel_image_path": "$KERNEL",
    "boot_args": "console=ttyS0 reboot=k panic=1 pci=off rdinit=/init",
    "initrd_path": "$INITRD"
  },
  "drives": [
    {
      "drive_id": "raftblk0",
      "is_root_device": false,
      "socket": "$SOCKET"
    }
  ],
  "machine-config": {
    "vcpu_count": 1,
    "mem_size_mib": 256
  },
  "logger": {
    "log_path": "$WORKDIR/log/fc.log",
    "level": "Info"
  }
}
EOF
: > "$WORKDIR/log/fc.log"

echo "[5] launching Firecracker" | tee -a "$LOG"
"$FC_BIN" --no-api --config-file "$WORKDIR/run/vm-config.json" \
    > "$WORKDIR/log/fc-stdout.log" 2>&1 &
FC_PID=$!

# Wait for guest to print the verification marker. Filter out the kernel
# cmdline echo (lines starting with "[ <timestamp>]") so we only match
# the actual init script's stdout.
echo "[6] waiting up to 60s for guest to write+read+verify" | tee -a "$LOG"
RESULT=fail
for i in {1..300}; do
    if grep -E '^[^[]' "$WORKDIR/log/fc-stdout.log" 2>/dev/null | grep -q "RAFTBLK-SMOKE-IO-VERIFIED"; then
        RESULT=pass
        sleep 1
        kill "$FC_PID" 2>/dev/null
        break
    fi
    if grep -E '^[^[]' "$WORKDIR/log/fc-stdout.log" 2>/dev/null | grep -q "RAFTBLK-SMOKE-IO-MISMATCH"; then
        RESULT=mismatch
        kill "$FC_PID" 2>/dev/null
        break
    fi
    if ! kill -0 "$FC_PID" 2>/dev/null; then
        break
    fi
    sleep 0.2
done

echo "" | tee -a "$LOG"
echo "=== guest stdout (RAFTBLK lines + virtio_blk dmesg) ===" | tee -a "$LOG"
grep -E '^=====|^\[smoke\]|virtio_blk virtio0|vda:' "$WORKDIR/log/fc-stdout.log" | tee -a "$LOG"
echo "" | tee -a "$LOG"

case "$RESULT" in
    pass)
        echo "PASS: real microVM wrote+read 4096 bytes through vhost-user-blk -> Raft" | tee -a "$LOG"
        exit 0
        ;;
    mismatch)
        echo "FAIL: read bytes did not match written bytes" | tee -a "$LOG"
        exit 2
        ;;
    *)
        echo "FAIL: guest never reached IO-VERIFIED marker; see $WORKDIR/log/fc-stdout.log" | tee -a "$LOG"
        exit 3
        ;;
esac
