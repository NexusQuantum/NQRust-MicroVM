#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

RPC_SOCKET="${AGENT_SPDK_IT_RPC_SOCKET:-${AGENT_SPDK_RPC_SOCKET:-/run/spdk/rpc.sock}}"
LVS_NAME="${AGENT_SPDK_IT_LVS_NAME:-${AGENT_SPDK_LVS_NAME:-nexus}}"
NBD_DEVICES="${AGENT_SPDK_IT_NBD_DEVICES:-${AGENT_SPDK_NBD_DEVICES:-/dev/nbd0,/dev/nbd1}}"
VHOST_SOCKET_DIR="${AGENT_SPDK_IT_VHOST_SOCKET_DIR:-${AGENT_SPDK_VHOST_SOCKET_DIR:-/var/tmp}}"

if [[ ! -S "${RPC_SOCKET}" ]]; then
  echo "SPDK JSON-RPC socket not found: ${RPC_SOCKET}" >&2
  echo "Set AGENT_SPDK_IT_RPC_SOCKET or AGENT_SPDK_RPC_SOCKET." >&2
  exit 2
fi

IFS=',' read -r -a NBD_ARRAY <<< "${NBD_DEVICES}"
for dev in "${NBD_ARRAY[@]}"; do
  dev="$(echo "${dev}" | xargs)"
  [[ -z "${dev}" ]] && continue
  if [[ ! -b "${dev}" ]]; then
    echo "NBD device not found: ${dev}" >&2
    echo "Load the module first, for example: sudo modprobe nbd nbds_max=8" >&2
    exit 2
  fi
done

cd "${ROOT}"
AGENT_SPDK_IT_RPC_SOCKET="${RPC_SOCKET}" \
AGENT_SPDK_IT_LVS_NAME="${LVS_NAME}" \
AGENT_SPDK_IT_NBD_DEVICES="${NBD_DEVICES}" \
AGENT_SPDK_IT_VHOST_SOCKET_DIR="${VHOST_SOCKET_DIR}" \
cargo test -p agent spdk_lvol_real_smoke -- --ignored --nocapture
