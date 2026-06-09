# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- **QEMU production boot path was completely broken (critical).** The agent
  spawned QEMU via `systemd-run --scope`, which runs *synchronously* and only
  returns once the wrapped process exits — so `boot()` blocked for the VM's
  entire lifetime and never reached the QMP handshake, the socket-perms relax,
  or handle persistence. Every production-mode (non-`AGENT_NO_SUDO`) QEMU boot
  hung until the manager's timeout and was then torn down. Fixed by spawning
  QEMU as a transient **service** (`systemd-run --collect`, no `--scope`),
  which backgrounds it under systemd (PID 1) — also letting VMs survive agent
  restarts, which `rebind` depends on. (`qemu-<id>.scope` → `.service`.)
- **QEMU lifecycle ops orphaned VMs (critical).** QEMU's root-written pidfile
  is mode 0600, unreadable by the non-root agent, so `read_pid` returned
  `None`, `rebind` couldn't confirm liveness, and pause / resume / shutdown /
  destroy all failed with "no live vmm" — `destroy` silently no-opped and left
  the VM and its cgroup running. Fixed by self-healing `read_pid`: relax the
  pidfile via `sudo -n chmod` and retry (mirrors the socket-perms relax).
- **Reconciler marked every running QEMU VM `stopped` (critical, manager-side).**
  `diff_host` decides liveness from the agent inventory, which is
  Firecracker-specific (`fc-*.scope` units + `vms/<id>/sock` paths). A QEMU VM
  runs as `qemu-<id>.service` with its QMP socket elsewhere, so it always looked
  "absent" → the reconciler flagged it for the FC-only in-place `restart_vm`,
  which fails on the empty `kernel_path` and flips the VM to `stopped` (while
  QEMU keeps running) ~seconds after boot. Fixed by excluding QEMU VMs from the
  FC restart path in `diff_host` (QEMU recovery goes through `qemu_service`
  reschedule, per the existing design).
- **QEMU VMs never got network (manager-side).** The cloud-init network-config
  hardcoded `eth0`, but modern cloud images use predictable names (`enp0s3`,
  `ens3`, …) so the stanza never matched — the NIC stayed down and the guest
  never DHCP'd. Fixed by matching `name: "e*"` (covers `en*` + legacy `eth*`).
- All four found and fixed via real end-to-end testing on a **stock Ubuntu
  24.04** host with the agent running **non-root** (the true production
  posture). Agent-path: QEMU UEFI boot of an Ubuntu cloud image → cloud-init
  NoCloud seed → DHCP + serial login → pause/resume/destroy with no orphan,
  cgroup limits enforced. Full-stack: manager + Postgres + registered agent →
  `POST /v1/vms` → qcow2 thin overlay over the base image + cloud-init ISO +
  **real TAP on `fcbr0`** + agent boot → VM stays `running` → guest DHCPs to
  `10.0.0.x` and pings the gateway. This exercises the "production
  sudo/systemd-run/cgroup path" and "TAP bridging for QEMU" that the alpha.1
  caveats flagged as not-yet-validated.

### Known issues (QEMU, found in the same testing)
- Deleting a QEMU VM stops the process but leaves its `tap-<id>` on the bridge
  (orphan tap; the reconciler's orphan-cleanup eventually reaps it).
- `guest_ip` stays null for bring-your-own images: IP reporting relies on the
  in-guest agent, which isn't present in stock cloud images.

## [0.5.0-alpha.1] - 2026-06-08

**Alpha** — first pluggable-VMM release. Adds **QEMU** as a second VMM
backend alongside Firecracker, turning NQRust-MicroVM into a platform
that runs both fast microVMs (Firecracker) and full classic VMs (QEMU:
UEFI Linux, Windows, bring-your-own-ISO). Download and test; **not yet
production-ready** — see caveats below.

### Added
- **Pluggable VMM backend abstraction** (`crates/nexus-vmm`): a
  `VmmDriver` trait + `VmmKind` / `GuestOs` / `BootMode` / `ImageKind`
  enums + a per-backend `FeatureSupport` matrix. Firecracker is a
  thin trait-adapter over the existing path (unchanged behaviour);
  QEMU is a new driver.
- **QEMU q35 + KVM driver**: UEFI/OVMF boot, virtio-blk / virtio-net,
  serial UDS console, optional VNC, QMP lifecycle, snapshot via QMP
  migrate, swtpm (TPM 2.0 for Windows 11), virtio-balloon / -rng /
  -vsock, VFIO PCI passthrough.
- **VM-mode in the UI**: create wizard now offers microVM (Firecracker)
  vs VM (QEMU) with disk-image / installer-ISO / VNC / SSH-key inputs.
- **Image registry**: `image_kind` discriminator (linux_kernel /
  linux_disk / uefi_disk / installer_iso), UEFI NVRAM template path,
  dedicated upload buttons, and **virt-v2v VMware VMDK import**.
- **ISO install lifecycle**: `installing` VM state + browser noVNC
  console + one-click "Install Complete" (QMP CD-ROM eject).
- **Day-2 ops for QEMU VMs** (UI + API): live migration (full
  target-side orchestration), HA reschedule, VM backup (chunked
  nexus-backup for volume-backed VMs, qemu-img for overlays),
  cloud-init / cloudbase-init credential + SSH-key seeding.
- **Resource orchestration**: per-VM `systemd-run` cgroup limits
  (MemoryMax / CPUQuota / MemorySwapMax=0) + per-host atomic vcpu/mem
  reservation so Firecracker and QEMU don't over-commit a shared host.
- **Storage backend pairing**: QEMU disks route through the 0.4.x
  storage registry (local_file / iSCSI / NFS / SPDK / TrueNAS).
- DB migration `0040_vmm_backends.sql` — additive / forward-only.

### Alpha caveats (read before deploying)
- **Validated end-to-end on this build:** QEMU UEFI Linux cloud-image
  VM create → pause → resume → delete (dev mode). Everything else is
  code-complete + unit-tested but **not yet exercised against real
  infrastructure**: Windows install, real iSCSI/NFS/SPDK, the
  production sudo/systemd-run/cgroup path, TAP bridging for QEMU,
  cross-host live migration, auto-HA, and virt-v2v.
- **Dev escape hatches must stay OFF in any real deployment:**
  `AGENT_NO_SUDO`, `AGENT_USER_MODE_NET`, `MANAGER_TEST_MODE`,
  `LICENSE_DEV_MODE`. They bypass cgroup isolation, real networking,
  and licensing respectively.
- **Optional host packages** for full functionality: `swtpm` (Windows
  11 TPM), `virtio-win` ISO (Windows drivers), `genisoimage`/`xorriso`
  (cloud-init seed), `libguestfs-tools` (virt-v2v). The platform
  degrades with warnings when absent.
- No security review of the new routes yet; treat as untrusted-input
  surface until reviewed.

## [0.4.1] - 2026-05-14

Patch release rolling up post-v0.4.0 fixes surfaced while shipping
the release: a real UI bug in the storage backend dialog, the
installer's non-interactive entry point that was a stub since the
file was created, and three release-pipeline workflow fixes that
unblocked the air-gapped bundle build for the first time since
v0.3.0.

### Fixed
- **UI: storage backend dialog grew off-screen with advanced options**.
  Expanding SMB or iSCSI + LVM advanced fields pushed the dialog to
  ~1263px tall — Cancel and Add buttons clipped below the viewport.
  Cap `DialogContent` at `max-h-[90vh]` with flex-column layout; the
  form body scrolls internally with the header and footer pinned
  (commit `837b9b6`).
- **Installer: `--non-interactive` was a stub**. `run_non_interactive`
  printed "Non-interactive installation not yet implemented" and
  exited 0, leaving the Test Installer CI red since November 2025
  and any scripted production install dead in the water. Implemented
  by reusing `executor::run_installation` (the same orchestrator the
  TUI uses) and streaming `InstallMessage` events to stdout, with a
  clear root-required error when run as non-root (commit `b7aed60`).
- **Installer config.sh: glob matching used `[ ]` instead of `[[ ]]`**.
  Two `[ "$INSTALL_COMPONENTS" == *"manager"* ]` checks silently
  evaluated to false (POSIX `[` treats `*` literally), so the
  generated `config.yaml` always reported `manager: false` and
  `agent: false` regardless of `--mode`. Caught by shellcheck SC2081
  in the new Test Installer workflow (commit `ae8d1c6`).

### CI
- **Air-gapped bundle: dropped `sudo apt-get`** on the self-hosted
  Arch (omarchy) runner; replaced with a tool-presence check that
  fails fast with a pacman hint. Bundle build had been red since
  v0.3.0 (commit `09cdcb4`).
- **Air-gapped bundle: pnpm now ships as `.tar.gz`** (since v11),
  not a bare binary. Update `scripts/airgap/bundle-node.sh` to
  download and extract the tarball + `dist/` runtime tree (commit
  `66e8dd8`).
- **Test Installer rewritten** to build the installer from source,
  verify the CLI surface (`--help` for install + uninstall both
  list `--non-interactive`), assert the non-interactive path is no
  longer a stub, and shellcheck every `lib/*.sh` at severity=error.
  Replaces the old workflow that tried full systemd / postgres /
  KVM installs in GHA containers — paths now exercised in
  `infra/test/*-runner.sh` instead.

## [0.4.0] - 2026-05-14

Adds SMB / CIFS as a first-class external storage backend, mirroring the
NFS integration delivered in v0.3.0. The agent owns the privileged
`mount.cifs` call and a per-backend 0600 credential file (Proxmox-style,
outside the DB); the manager stays unprivileged and configures the
backend through the existing storage_backends API + UI.

Verified end-to-end inside a fresh Ubuntu 24.04 KubeVirt VM (Firecracker
nested-KVM), against a real Samba 4.19.5 server with `mount.cifs` from
`cifs-utils 7.5`: 19/19 assertions across backend CRUD, validation,
health probe, anonymous (guest) backend, Firecracker VM lifecycle with
rootfs on the SMB share, edit-in-place password rotation, and protected
backend delete. See `infra/test/smb-runner.sh` for the runner, and
`infra/test/smb-docker-runner.sh` for the lower-level privileged-Docker
variant that exercises the agent's `/v1/storage/smb/*` routes directly
(27/27).

### Added
- **`smb` storage backend (CIFS)** — Vendor-agnostic SMB share support, parallel to `nfs`. Agent runs `mount.cifs` with per-backend credential files (`/etc/nqrust/storage-creds/<id>.cred`, mode 0600). Manager talks to agent over `/v1/storage/smb/*` for set/clear credentials, mount/umount, file lifecycle (create_file, delete_file, snapshot, clone_from_path, clone_from_snapshot).
- **UI form for SMB** with authenticated and anonymous (`-o guest`) modes, password rotation in the Edit dialog, and a typed SMB-version select (`default` / 2.0 / 2.1 / 3 / 3.0 / 3.11). Domain, subdir, and freeform mount options exposed under "Show advanced".
- **Host package**: `cifs-utils` added to apt + dnf installer flows and to the air-gapped Debian bundle.
- **Migration `0039_smb_backend_kind.sql`** — allows `kind = 'smb'` in the `storage_backend` table CHECK constraint (forward-only, per release migration policy).
- `docs/runbooks/smb-troubleshooting.md` — eight failure modes (exit 13/32, version mismatch, anonymous, password rotation, probe timeout, missing cred file, etc.) with reference commands.
- `infra/test/smb-docker-runner.sh` — privileged-container E2E test that brings up Samba and exercises every agent SMB route (set/clear credentials, mount/umount, idempotency, create/delete files, snapshot, clone-from-snapshot, clone-from-path, anonymous mount, wrong-password rejection).

### Changed
- `CreateStorageBackendReq` accepts an optional top-level `password` field for SMB; it is never persisted to the DB but is forwarded to the agent on create/update.
- UI `Field` type extended with `"password"` and `"select"` (with `options`) so the storage form schema can describe each backend kind declaratively.
- `BackendKind` enum + wire string `"smb"` added; `Smb` variant registered in the registry, config validator, and health probe (`df -B1 --output=used,size` for capacity).

## [0.3.0] - 2026-05-09

Stable release. Same code as `0.3.0-alpha.2` — re-tagged after the
in-VM integration suite (23/23) ran clean against the released musl
artifact, confirming the alpha label no longer reflected reality.
Aggregated content of `0.3.0-alpha.1` + `0.3.0-alpha.2` shown below.

## [0.3.0-alpha.2] - 2026-05-09

Bug-fix alpha brings the in-VM E2E integration test suite to 23/23
passing. Two iscsi_lvm bugs found and fixed during the test run:

### Fixed
- **`ensure_volume_registered` failed for block-device rootfs paths**
  (iscsi_lvm, generic iscsi). `fs::metadata().len()` returns 0 for
  block devices, which violated the `positive_size` CHECK constraint
  on the `volume` table — and as a side effect no `volume_attachment`
  row got written, so `lookup_rootfs_volume_handle` returned None and
  the `deactivate_volume` hook never fired on stop. Replaced with a
  direct attachment INSERT when `provision_rootfs` already provided
  the volume handle (commit `98c99a6`).
- **`restart_vm` rejected `/dev/<vg>/<lv>` paths**. `ensure_allowed_path`
  only permits `MANAGER_IMAGE_ROOT` and `MANAGER_STORAGE_ROOT` subtrees;
  backend-resolved block-device paths from `host_path_for()` are
  trusted (they came from a backend we control, not user input) and
  now bypass the check (commit `49eb4a7`). VM start used to return 500
  on iscsi_lvm-backed VMs after a stop.

### Added
- `infra/test/iscsi-alpha-vm.yaml` — KubeVirt VM spec (Ubuntu 24.04 +
  bridge networking + nested-KVM-friendly) for running the integration
  suite without TrueNAS.
- `infra/test/iscsi-alpha-install.sh` — installs the alpha into the
  test VM, falling back to the prior stable for kernel/rootfs.
- `infra/test/iscsi-alpha-runner.sh` — comprehensive runner: 23
  assertions across backend CRUD, validation, initialize lifecycle,
  VM lifecycle on iscsi_lvm, and live registry behaviour.
- `infra/test/HANDOFF.md` — operator runbook for replaying the test
  inside a fresh KubeVirt VM.

## [0.3.0-alpha.1] - 2026-05-05

Alpha release introducing the `iscsi_lvm` storage backend (vendor-agnostic
shared block storage), live registry updates without manager restart, and
the platform auto-update mechanism.

### Added
- Platform auto-update: Settings → Updates page lets admins apply new releases either by uploading a `.nqupdate` bundle (airgap) or by enabling internet checks against a configured manifest URL. Apply order is manager → agents (rolling) → UI; running VMs are not disturbed by agent restart.
- **`iscsi_lvm` storage backend** — vendor-agnostic auto-provisioning of per-VM block devices on top of any iSCSI target. Mirrors Proxmox VE's LVM-on-iSCSI mode. Adds `BackendKind::IscsiLvm`, `activate_volume`/`deactivate_volume` trait hooks, agent routes under `/v1/storage/iscsi_lvm/*`, and a manager `POST /v1/storage_backends/:id/initialize` endpoint with destructive-confirmation UI flow. See `docs/runbooks/iscsi-lvm-troubleshooting.md`.
- **NFS auto-mount via the agent** — manager runs unprivileged and delegates `mount.nfs` to the agent over `/v1/storage/nfs/*` so operators don't need to SSH to mount NFS exports manually.
- **Storage backend live registry** — adding or deleting a backend through the UI is reflected in the manager's in-memory registry immediately; no restart needed before VM-create can pick up the new backend.
- **UI-sourced backends survive manager restart** — `storage_backend.source` column tracks `'toml'` vs `'ui'`; the startup TOML reconciler only soft-deletes rows it owns.
- **Tiered backend kind dropdown** — Add Backend wizard shows three recommended kinds by default (`local_file`, `nfs`, `iscsi_lvm`) with "Show advanced kinds" disclosure for `iscsi`, `truenas_iscsi`, `spdk_lvol`. Default selection is `local_file` (zero-deps).
- **Host package dependencies expanded** — installer + air-gapped bundle now ship `open-iscsi`, `lvm2`, `qemu-utils`, `nfs-common`. `iscsid` is enabled automatically post-install.

### Changed
- Manager and agent binaries are now installed under `/opt/nqrust/bin/<name>.<version>` with a `<name>` symlink, to support atomic self-update.
- systemd units now set `RestartForceExitStatus=42` so a clean self-update exit triggers a restart on the new binary.
- `ControlPlaneBackend` trait gains `probe()`, `host_path_for()`, `activate_volume()`, and `deactivate_volume()` methods (default no-ops). `iscsi_lvm` overrides them; other backends inherit the defaults.
- VM lifecycle calls `activate_volume` before Firecracker spawn and `deactivate_volume` on stop. No-op for stateless backends; iscsi_lvm uses these for `lvchange -aey` exclusive activation.

## [0.1.0] - 2024-XX-XX

### Added

#### Core Platform
- **Manager Service**: Central orchestration service for VM lifecycle management
  - REST API (Axum-based) for VM operations
  - PostgreSQL database with SQLx ORM
  - Automatic database migrations on startup
  - OpenAPI/Swagger documentation
  - Health check endpoint

- **Agent Service**: Host-level VM execution via Firecracker
  - KVM-based microVM isolation
  - Firecracker VMM integration
  - Unix domain socket communication
  - Multi-host support with auto-registration
  - Heartbeat monitoring

- **Guest Agent**: In-VM metrics collection
  - CPU, memory, disk, network metrics
  - Automatic IP reporting to manager
  - Static musl binary for portability
  - Auto-start on VM boot (systemd/OpenRC/sysvinit)

#### Virtual Machine Management
- Complete VM lifecycle (create, start, stop, pause, resume, delete)
- 5-step VM creation wizard
- Multiple boot source support (kernel + rootfs)
- Configurable vCPU and memory allocation
- Multiple network interfaces (NICs)
- Multiple block devices (drives)
- Browser-based terminal access via WebSocket
- Real-time metrics streaming via WebSocket
- VM snapshots (full and differential)
- Snapshot restore functionality
- VM templates for reusable configurations

#### Linux Distribution Support
- **Alpine Linux** (minimal, musl-based)
- **BusyBox** (ultra-minimal)
- **Ubuntu 24.04 LTS** (systemd, cloud-init)
- **Debian 12 Bookworm** (systemd, cloud-init)
- Distribution-aware credential injection
- Automatic init system detection
- Cloud-init support for Ubuntu/Debian
- Build scripts for custom rootfs images

#### Container Management
- Container-per-VM architecture for strong isolation
- Docker Remote API compatibility
- Alpine Linux + Docker daemon in microVM
- Bridge networking for external access
- Container lifecycle management
- Image pull from registries

#### Serverless Functions
- Function execution in isolated microVMs
- Runtime support: Node.js, Python, Ruby
- HTTP invocation API
- Function logs and stdout/stderr capture
- Browser-based code editor (Monaco)
- Automatic function packaging

#### Infrastructure Management
- **Networks Registry**
  - Bridge network management
  - VLAN support (802.1Q tagging)
  - Auto-registration on VM creation
  - NAT and bridged modes

- **Volumes Registry**
  - Central volume tracking
  - Support for ext4, qcow2, raw formats
  - Volume attachment/detachment
  - Auto-registration for rootfs

- **Hosts Management**
  - Agent registration and discovery
  - Real-time host metrics
  - Heartbeat monitoring
  - Multi-datacenter support

- **Image Registry**
  - Kernel and rootfs image management
  - DockerHub browser integration
  - Local file upload support
  - Image metadata tracking

#### Web UI (Next.js 15)
- Modern React 19 with TypeScript
- shadcn/ui component library
- Tailwind CSS 4 styling
- TanStack Query for state management
- Dashboard with resource overview
- VM management pages
- Container management UI
- Function editor with syntax highlighting
- Real-time terminal (xterm.js)
- Live metrics charts
- Responsive design

#### Installation & Deployment
- Automated installer script
- Multiple installation modes (production, dev, manager, agent)
- systemd service integration
- Network bridge auto-setup
- Firecracker binary management
- Database setup automation
- Uninstaller with cleanup options

#### CI/CD Pipeline
- GitHub Actions workflows
- Lint checks (rustfmt, clippy)
- Unit and integration tests
- Multi-profile builds (debug, release)
- UI build verification
- Security audit (cargo-audit)
- Shell script validation (shellcheck)
- Self-hosted runner support

### Security
- Firecracker microVM isolation (kernel-level)
- Per-VM network namespacing
- No shared kernel between VMs
- Secure credential injection
- Input validation on all API endpoints

### Performance
- sccache for faster Rust compilation
- Cargo workspace caching
- Optimized LLD linker
- Stripped release binaries
- Static musl builds for guest-agent

### Documentation
- Comprehensive README
- Feature documentation (FEATURES.md)
- Container guide (CONTAINER.md)
- Network management guide (NETWORKING.md)
- Volume management guide (VOLUMES.md)
- Template documentation (TEMPLATES.md)
- Quick start guide (RUN.md)
- Development instructions (CLAUDE.md)

### Known Limitations
- x86_64 architecture only (no ARM support yet)
- Single region deployment
- No built-in authentication/authorization (planned for v0.2.0)
- Container UI needs implementation
- Template UI partially implemented
- No live migration support

### Requirements
- Linux host with KVM support
- Ubuntu 22.04+ / Debian 11+ / RHEL 8+
- 2GB+ RAM minimum
- 20GB+ disk space
- PostgreSQL 14+
- Node.js 20+ (for UI)

---

## Future Plans

### v0.2.0 (Planned)
- User authentication and RBAC
- Multi-tenant support
- Resource quotas
- API rate limiting
- Audit logging

### v0.3.0 (Planned)
- ARM64 support
- GPU passthrough
- Custom kernel support
- Performance monitoring dashboard
- Alerting system

[Unreleased]: https://github.com/user/nqrust-microvm/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/user/nqrust-microvm/releases/tag/v0.1.0
