#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
export DATABASE_URL="${DATABASE_URL:-postgres://nexus:nexus@localhost:5435/nexus}"
export MANAGER_BIND="${MANAGER_BIND:-127.0.0.1:18080}"
export MANAGER_IMAGE_ROOT="${MANAGER_IMAGE_ROOT:-/srv/images}"
export MANAGER_STORAGE_ROOT="${MANAGER_STORAGE_ROOT:-/srv/fc/vms}"
export LICENSE_DEV_MODE="${LICENSE_DEV_MODE:-1}"
export MANAGER_ALLOW_IMAGE_PATHS="${MANAGER_ALLOW_IMAGE_PATHS:-true}"
export MANAGER_RECONCILER_DISABLED="${MANAGER_RECONCILER_DISABLED:-true}"
exec ./target/release/manager
