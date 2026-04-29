#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SPDK_DIR="${SPDK_DIR:-${ROOT}/.worktrees/spdk}"
SPDK_VENV="${SPDK_VENV:-${ROOT}/.worktrees/spdk-venv}"
SPDK_REPO="${SPDK_REPO:-https://github.com/spdk/spdk.git}"
SPDK_RUN_PKGDEP="${SPDK_RUN_PKGDEP:-auto}"
RPC_SOCKET="${AGENT_SPDK_IT_RPC_SOCKET:-${AGENT_SPDK_RPC_SOCKET:-/tmp/nqrust-spdk-rpc.sock}}"
LVS_NAME="${AGENT_SPDK_IT_LVS_NAME:-${AGENT_SPDK_LVS_NAME:-nexus}}"
MALLOC_BDEV="${SPDK_MALLOC_BDEV:-Malloc0}"
MALLOC_SIZE_MB="${SPDK_MALLOC_SIZE_MB:-32}"
MALLOC_BLOCK_SIZE="${SPDK_MALLOC_BLOCK_SIZE:-512}"
NBD_DEVICES="${AGENT_SPDK_IT_NBD_DEVICES:-${AGENT_SPDK_NBD_DEVICES:-/dev/nbd0,/dev/nbd1}}"
PID_FILE="${SPDK_PID_FILE:-/tmp/nqrust-spdk-tgt.pid}"
LOG_FILE="${SPDK_LOG_FILE:-/tmp/nqrust-spdk-tgt.log}"
CONFIG_FILE="${SPDK_CONFIG_FILE:-/tmp/nqrust-spdk-tgt.json}"
MEM_SIZE_MB="${SPDK_MEM_SIZE_MB:-64}"
IOBUF_SMALL_POOL_COUNT="${SPDK_IOBUF_SMALL_POOL_COUNT:-512}"
IOBUF_LARGE_POOL_COUNT="${SPDK_IOBUF_LARGE_POOL_COUNT:-64}"
IOBUF_SMALL_BUFSIZE="${SPDK_IOBUF_SMALL_BUFSIZE:-4096}"
IOBUF_LARGE_BUFSIZE="${SPDK_IOBUF_LARGE_BUFSIZE:-8192}"
BDEV_IO_POOL_SIZE="${SPDK_BDEV_IO_POOL_SIZE:-64}"
BDEV_IO_CACHE_SIZE="${SPDK_BDEV_IO_CACHE_SIZE:-1}"
BDEV_IOBUF_SMALL_CACHE_SIZE="${SPDK_BDEV_IOBUF_SMALL_CACHE_SIZE:-1}"
BDEV_IOBUF_LARGE_CACHE_SIZE="${SPDK_BDEV_IOBUF_LARGE_CACHE_SIZE:-1}"

need() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "Missing required command: $1" >&2
    exit 2
  }
}

need git
need sudo
need python3
need gcc
need make
need pkg-config

python_has_module() {
  python3 - "$1" <<'PY'
import importlib.util
import sys
sys.exit(0 if importlib.util.find_spec(sys.argv[1]) else 1)
PY
}

ensure_python_build_tools() {
  local need_venv=0
  command -v meson >/dev/null 2>&1 || need_venv=1
  command -v ninja >/dev/null 2>&1 || need_venv=1
  python_has_module jinja2 || need_venv=1
  python_has_module elftools || need_venv=1
  python_has_module tabulate || need_venv=1

  if [[ "${need_venv}" == 0 ]]; then
    return
  fi

  echo "Installing SPDK Python build helpers in ${SPDK_VENV}..."
  python3 -m venv "${SPDK_VENV}"
  "${SPDK_VENV}/bin/python" -m pip install --upgrade pip
  "${SPDK_VENV}/bin/python" -m pip install meson ninja jinja2 pyelftools tabulate
  export PATH="${SPDK_VENV}/bin:${PATH}"
}

run_pkgdep=false
case "${SPDK_RUN_PKGDEP}" in
  1|true|yes) run_pkgdep=true ;;
  0|false|no) run_pkgdep=false ;;
  auto)
    # On Arch, pkgdep.sh may trigger a partial system upgrade if package
    # databases are newer than installed base packages. Prefer building with
    # already-installed deps and let configure report anything missing.
    if [[ -r /etc/arch-release ]]; then
      run_pkgdep=false
    else
      run_pkgdep=true
    fi
    ;;
  *)
    echo "Invalid SPDK_RUN_PKGDEP=${SPDK_RUN_PKGDEP}; use auto, 1, or 0." >&2
    exit 2
    ;;
esac

mkdir -p "$(dirname "${SPDK_DIR}")"
if [[ ! -d "${SPDK_DIR}/.git" ]]; then
  git clone --recursive "${SPDK_REPO}" "${SPDK_DIR}"
else
  git -C "${SPDK_DIR}" submodule update --init --recursive
fi

patch_spdk_dev_build() {
  local marker="${SPDK_DIR}/.nqrust-spdk-dev-patch-v3"
  (
    cd "${SPDK_DIR}"
    # The local smoke target only needs malloc/lvol/nbd/vhost-blk. Prune
    # optional modules/apps that pull in extra host deps or large mempools.
    sed -i \
      -e '/^BLOCKDEV_MODULES_LIST += bdev_aio$/d' \
      -e '/^BLOCKDEV_MODULES_PRIVATE_LIBS += -laio$/d' \
      -e '/^INTR_BLOCKDEV_MODULES_LIST += bdev_aio$/d' \
      mk/spdk.modules.mk
    sed -i \
      -e 's/^DIRS-y += aio ftl$/DIRS-y += ftl/' \
      -e '/^DIRS-y += aio$/d' \
      module/bdev/Makefile
    sed -i \
      -e '/^DIRS-y += spdk_nvme_perf$/d' \
      -e '/^DIRS-y += spdk_dd$/d' \
      -e '/^DIRS-y += iscsi_tgt$/d' \
      app/Makefile
    sed -i \
      -e 's/^DIRS-y += bdev accel scheduler iscsi nvmf scsi vmd sock iobuf keyring$/DIRS-y += bdev accel scheduler nvmf scsi vmd sock iobuf keyring/' \
      -e 's/^DIRS-$(CONFIG_VHOST) += vhost_blk vhost_scsi$/DIRS-$(CONFIG_VHOST) += vhost_blk/' \
      -e '/^DEPDIRS-iscsi := scsi$/d' \
      -e '/^DEPDIRS-vhost_scsi := scsi$/d' \
      module/event/subsystems/Makefile
    sed -i \
      -e 's/ iscsi notify init/ notify init/' \
      lib/Makefile
    sed -i \
      -e 's/^SPDK_LIB_LIST += event event_iscsi event_nvmf$/SPDK_LIB_LIST += event event_nvmf/' \
      -e 's/^SPDK_LIB_LIST += event_vhost_blk event_vhost_scsi$/SPDK_LIB_LIST += event_vhost_blk/' \
      app/spdk_tgt/Makefile
  )
  if [[ ! -f "${marker}" ]]; then
    rm -f "${SPDK_DIR}/build/bin/spdk_tgt"
    : >"${marker}"
  fi
}

patch_spdk_dev_build

if [[ ! -x "${SPDK_DIR}/build/bin/spdk_tgt" ]]; then
  ensure_python_build_tools
  if [[ "${run_pkgdep}" == true ]]; then
    echo "Installing SPDK build dependencies through SPDK pkgdep script..."
    if ! sudo "${SPDK_DIR}/scripts/pkgdep.sh"; then
      echo
      echo "SPDK pkgdep failed." >&2
      if [[ -r /etc/arch-release ]]; then
        echo "On Arch this is often a partial-upgrade guard. Run:" >&2
        echo "  sudo pacman -Syu" >&2
        echo "Then retry, or skip pkgdep with:" >&2
        echo "  SPDK_RUN_PKGDEP=0 ./scripts/spdk-dev-bootstrap.sh" >&2
      fi
      exit 1
    fi
  else
    echo "Skipping SPDK pkgdep script (SPDK_RUN_PKGDEP=${SPDK_RUN_PKGDEP})."
    echo "If configure reports missing dependencies, either install them or run:"
    echo "  SPDK_RUN_PKGDEP=1 ./scripts/spdk-dev-bootstrap.sh"
  fi
  (
    cd "${SPDK_DIR}"
    rm -rf dpdk/build dpdk/build-tmp
    rm -f include/spdk_internal/rpc_autogen.h
    ./configure \
      --disable-tests \
      --disable-unit-tests \
      --disable-examples \
      --max-numa-nodes=1 \
      --without-rdma \
      --without-rbd \
      --without-crypto \
      --without-fio \
      --without-idxd \
      --without-vfio-user \
      --without-fc \
      --without-daos \
      --without-aio-fsdev \
      --without-uring \
      --without-xnvme \
      --without-ublk \
      --without-usdt
    patch_spdk_dev_build
    make DPDKBUILD_FLAGS="-Dmax_numa_nodes=1" -j"$(nproc)"
  )
fi

sudo modprobe nbd nbds_max=8
IFS=',' read -r -a NBD_ARRAY <<< "${NBD_DEVICES}"
for dev in "${NBD_ARRAY[@]}"; do
  dev="$(echo "${dev}" | xargs)"
  [[ -z "${dev}" ]] && continue
  if [[ -b "${dev}" ]]; then
    sudo chmod 666 "${dev}" || true
  fi
done
sudo sysctl -w vm.nr_hugepages="${SPDK_HUGEPAGES:-512}" >/dev/null

cat >"${CONFIG_FILE}" <<JSON
{
  "subsystems": [
    {
      "subsystem": "iobuf",
      "config": [
        {
          "method": "iobuf_set_options",
          "params": {
            "small_pool_count": ${IOBUF_SMALL_POOL_COUNT},
            "large_pool_count": ${IOBUF_LARGE_POOL_COUNT},
            "small_bufsize": ${IOBUF_SMALL_BUFSIZE},
            "large_bufsize": ${IOBUF_LARGE_BUFSIZE},
            "enable_numa": false
          }
        }
      ]
    },
    {
      "subsystem": "bdev",
      "config": [
        {
          "method": "bdev_set_options",
          "params": {
            "bdev_io_pool_size": ${BDEV_IO_POOL_SIZE},
            "bdev_io_cache_size": ${BDEV_IO_CACHE_SIZE},
            "iobuf_small_cache_size": ${BDEV_IOBUF_SMALL_CACHE_SIZE},
            "iobuf_large_cache_size": ${BDEV_IOBUF_LARGE_CACHE_SIZE}
          }
        }
      ]
    }
  ]
}
JSON

if [[ -f "${PID_FILE}" ]] && kill -0 "$(cat "${PID_FILE}")" 2>/dev/null; then
  echo "SPDK already running with pid $(cat "${PID_FILE}")"
else
  sudo rm -f "${RPC_SOCKET}" "${PID_FILE}" "${LOG_FILE}"
  sudo mkdir -p "$(dirname "${RPC_SOCKET}")"
  echo "Starting spdk_tgt..."
  sudo "${SPDK_DIR}/build/bin/spdk_tgt" \
    -r "${RPC_SOCKET}" \
    -c "${CONFIG_FILE}" \
    -m 0x1 \
    -s "${MEM_SIZE_MB}" \
    --num-trace-entries 0 \
    >"${LOG_FILE}" 2>&1 &
  echo "$!" | sudo tee "${PID_FILE}" >/dev/null
fi

for _ in $(seq 1 100); do
  [[ -S "${RPC_SOCKET}" ]] && break
  sleep 0.1
done
if [[ ! -S "${RPC_SOCKET}" ]]; then
  echo "SPDK RPC socket did not appear: ${RPC_SOCKET}" >&2
  echo "Log: ${LOG_FILE}" >&2
  exit 1
fi
sudo chmod 666 "${RPC_SOCKET}" || true

RPC="${SPDK_DIR}/scripts/rpc.py -s ${RPC_SOCKET}"

if ! ${RPC} bdev_get_bdevs -b "${MALLOC_BDEV}" >/dev/null 2>&1; then
  ${RPC} bdev_malloc_create "${MALLOC_SIZE_MB}" "${MALLOC_BLOCK_SIZE}" -b "${MALLOC_BDEV}" >/dev/null
fi

if ! ${RPC} bdev_lvol_get_lvstores -l "${LVS_NAME}" >/dev/null 2>&1; then
  ${RPC} bdev_lvol_create_lvstore "${MALLOC_BDEV}" "${LVS_NAME}" >/dev/null
fi

echo
echo "SPDK dev target is ready."
echo "RPC socket: ${RPC_SOCKET}"
echo "Lvol store: ${LVS_NAME}"
echo
echo "Run:"
echo "  AGENT_SPDK_IT_RPC_SOCKET=${RPC_SOCKET} \\"
echo "  AGENT_SPDK_IT_LVS_NAME=${LVS_NAME} \\"
echo "  AGENT_SPDK_IT_NBD_DEVICES=${NBD_DEVICES} \\"
echo "  ./scripts/spdk-lvol-smoke.sh"
echo
echo "Stop later with:"
echo "  sudo kill \$(cat ${PID_FILE})"
