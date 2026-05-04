#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
exec sudo -E env \
  AGENT_BIND="${AGENT_BIND:-127.0.0.1:9090}" \
  MANAGER_BASE="${MANAGER_BASE:-http://127.0.0.1:18080}" \
  FC_RUN_DIR="${FC_RUN_DIR:-/srv/fc}" \
  FC_BRIDGE="${FC_BRIDGE:-fcbr0}" \
  AGENT_NFS_MOUNT_BASE="${AGENT_NFS_MOUNT_BASE:-/var/lib/nqrust/nfs}" \
  ./target/release/agent
