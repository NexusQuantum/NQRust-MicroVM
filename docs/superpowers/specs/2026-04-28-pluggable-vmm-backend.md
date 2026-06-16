# NQRust-MicroVM: Pluggable VMM Backend Architecture

**Status:** Design
**Date:** 2026-04-28
**Owner:** kleopasevan
**Scope:** Foundation PR for pluggable VMM backends. Adds Cloud Hypervisor as a parallel backend to Firecracker. Unlocks but does not include Windows guest support, GPU/VFIO passthrough, live migration, host-side IMDS replacement, or QEMU-full backend.

## Context

Today every VM in the platform runs on Firecracker. The integration is hardcoded across both manager and agent:

- **Agent — `apps/agent/src/features/vm/spawn.rs`** spawns the literal `firecracker` binary inside a `systemd-run` scope wrapped in a `screen` session. The `--api-sock` argument points at a Unix domain socket the manager will subsequently drive.
- **Agent — `apps/agent/src/features/vm/proxy.rs`** exposes a generic HTTP-over-UDS proxy that allowlists the Firecracker REST API endpoints (`/machine-config`, `/boot-source`, `/drives/*`, `/network-interfaces/*`, `/snapshot/*`, `/vm`, `/cpu-config`, `/vsock`, `/mmds`, `/entropy`, `/serial`, `/actions`, `/logger`, `/metrics`).
- **Manager — `apps/manager/src/features/vms/service.rs`** builds Firecracker-shaped JSON payloads (`MachineConfig`, `BootSource`, `Drive`, `NetworkInterface`) and PUT/PATCHes them through the agent proxy. `spawn_firecracker` does the lifecycle dance (POST `/spawn`, then sequence of PUTs, then PUT `/actions {InstanceStart}`).
- **Manager — `apps/manager/src/features/snapshots/routes.rs`** assumes Firecracker's `/snapshot/create` and `/snapshot/load` shapes. Snapshot files are Firecracker-format `state` + `mem` (and a `diff_dir` for diff snapshots).
- **Agent — `apps/agent/src/features/vm/snapshot.rs`** allocates Firecracker snapshot directory layout (`snapshot.fc`, `mem.fc`, `diff/`).
- **Agent — `apps/agent/src/core/uds_proxy.rs`** is the WebSocket shell bridge to the Firecracker serial-on-screen session.
- **VM record — `apps/manager/migrations/...`** has no `vmm_kind` column. The schema implicitly assumes Firecracker.

This is a fine baseline but blocks every workload that Firecracker cannot host: UEFI Linux distros (most cloud images use UEFI now), Windows guests (no UEFI on FC x86_64, no virtio-pci ABI Windows knows), GPU/VFIO workloads, kernels that need ACPI, anything needing a real chipset.

## Intent

Replace the hardcoded Firecracker path with a **pluggable VMM backend abstraction**. After this work, "which VMM is launched for this VM" is a swappable backend chosen per-VM, configured per-host. The manager talks to a uniform agent API; the agent talks to a per-VMM trait; the trait owns all backend-specific JSON shapes, socket protocols, and process lifecycle.

This is the **foundation PR**. It unlocks — but does not include — Windows guest support, GPU/VFIO passthrough, memory hot-plug exposure, live migration, host-side IMDS replacement, and QEMU-full backend. Those are additive work on top of the abstraction this PR establishes.

## Why this matters

- **Today**: VM = Firecracker microVM = no UEFI on x86_64, no virtio-pci, no ACPI, no chipset, no Windows. This excludes a meaningful slice of customer workloads (legacy Linux distros that require UEFI, Windows Server, anything needing a real PCI bus).
- **After this PR**: VM = `vmm_kind` × `boot_mode`. `Firecracker + LinuxKernel` is the existing hot path (sub-125ms boot, snapshot-based serverless cold-start, container-per-VM). New: `CloudHypervisor + Uefi` runs full distro disk images and is the on-ramp to Windows guests in a future PR.
- **Strategic**: customers with mixed estates (legacy Linux UEFI, Windows Server, modern Linux microVMs) currently need separate platforms. We can offer a single platform that routes by workload.
- **Forward-compatible**: the trait shape established here lets us add QEMU-full (for VFIO/SR-IOV/heavy device emulation) and a kata-containers-style runtime later without disturbing existing backends.
- **Bounded blast radius**: Firecracker remains the default for everything that already works. CH is opt-in per VM. Existing VMs, existing snapshots, existing serverless functions, existing containers are not touched.

## Scope

### In this PR

1. **One agent-side trait** plus supporting types: `VmmDriver`, `VmSpec`, `VmmHandle`, `BootMode`, `VmmKind`, `GuestOs`, `FeatureSupport`, `ConsoleEndpoint`, `SnapshotKind`, `SnapshotPaths`, `SnapshotMeta`, `ShutdownMode`, `VmmError`. Lives in a new `crates/nexus-vmm` crate, depended on by both `apps/manager` (for types and feature-gating) and `apps/agent` (for the trait and impls).
2. **Two `VmmDriver` implementations**:
   - **`FirecrackerDriver`** — wraps the existing `apps/agent/src/features/vm/spawn.rs` logic. Existing behaviour bit-for-bit. This is a refactor that extracts the current code path into a trait impl. No new functionality.
   - **`CloudHypervisorDriver`** — new. Spawns `cloud-hypervisor` binary inside a `systemd-run` scope, talks to its REST API via `--api-socket`, supports PVH (Linux) and UEFI (any) boot modes.
3. **Schema migration** for multi-VMM VMs:
   - `vms.vmm_kind TEXT NOT NULL DEFAULT 'firecracker'` with check constraint on enum values.
   - `vms.boot_mode JSONB NOT NULL` (carries the variant). For existing rows, backfilled from current implicit Linux-kernel boot config; serialization shape defined below.
   - `vms.guest_os TEXT NOT NULL DEFAULT 'linux_kernel'` with check constraint.
   - Existing VMs become `vmm_kind = 'firecracker'`, `guest_os = 'linux_kernel'`. Zero data migration pain.
4. **Agent-side dispatch.** A `VmmRegistry` (constructed at agent startup) holds one `VmmDriver` per installed `VmmKind`. The agent's per-VM HTTP routes are restructured to dispatch by `vmm_kind` (passed in the request body) rather than calling `firecracker` directly.
5. **Manager-side feature gating.** `FeatureSupport` is a pure function `(VmmKind, GuestOs) -> FeatureSupport`, encoded in the `nexus-vmm` crate. The manager refuses requests that depend on a feature the chosen backend does not support (e.g., `mmds` config on `CloudHypervisor` returns 400). UI greys out unsupported controls.
6. **Manager-side selection logic.** New VM creation accepts `vmm_kind: Option<VmmKind>` and `boot_mode: BootModeRequest`. If unspecified, manager picks: `Firecracker` for `LinuxKernel` boot mode, `CloudHypervisor` for `Pvh` and `Uefi` modes. Failing this selection (e.g., user asks for FC + UEFI) returns a 400 with a clear error.
7. **Agent registration reports installed VMM kinds.** `apps/agent/src/features/inventory` is extended to report `vmm_kinds_installed: Vec<VmmKind>`. Manager refuses to schedule VMs onto agents lacking the requested driver.
8. **Console abstraction.** `VmmDriver::console_endpoint()` returns a `ConsoleEndpoint` enum. For FC and CH-Linux this is `UnixSerial(PathBuf)` (existing screen-based bridge keeps working). For future Windows guests this will become `Vnc` / `Rdp`; out of scope here, but the enum carries the variant.
9. **API + UI for backend selection.**
   - `POST /v1/vms` accepts `vmm_kind: Option<VmmKind>`, `boot_mode: BootModeRequest`. Default behaviour preserved when unspecified.
   - `GET /v1/hosts/:id` returns each host's `vmm_kinds_installed` so the UI can validate placement client-side.
   - New shared types in `crates/nexus-types`: `VmmKind`, `GuestOs`, `BootModeRequest`, `FeatureSupport` (all `ToSchema`-derived).
   - UI: VM-create form gains a backend selector that defaults to "Auto" (manager picks). Advanced mode lets the user override. UEFI image upload is a new path in the image registry (separate concern, also gated to this PR — see §Image registry below).
10. **Image registry handles disk images.** Today the registry stores kernel + rootfs pairs. CH UEFI boot needs a single bootable disk image with an EFI System Partition. The registry gains an `image_kind: ImageKind = LinuxKernel | LinuxDisk | UefiDisk` discriminator. `UefiDisk` images record an optional `nvram_template` path. Existing image rows backfill to `LinuxKernel`.
11. **Snapshots are per-backend.** `snapshot.vmm_kind` column added; restore refuses to load a snapshot whose `vmm_kind` does not match the target VM's `vmm_kind`. No cross-backend snapshot translation in this PR.
12. **CH config validation.** Each `BootMode` variant has required-field rules validated at the manager *before* the agent is contacted. Malformed requests return 400 with the failing field name.
13. **Tests** per backend impl, integration tests covering FC-default and CH-UEFI-Linux, migration tests, feature-gating tests.

### Explicitly out of scope

Design the trait so these fit cleanly later, but do **not** implement:

- **Windows guest support.** Booting Windows in CH requires: image registry support for Windows ISO ingestion + virtio-win driver injection, a sysprep/cloudbase-init equivalent flow, a Windows port of `apps/guest-agent` (currently `x86_64-unknown-linux-musl`-only), a VNC or RDP-bridge console replacement (the existing serial-on-screen bridge does not work for Windows), and per-feature reductions across `containers` / `functions` (which are Linux-rootfs-only). These are independent concerns and ship in a follow-up PR. The trait already supports `BootMode::Uefi` and `GuestOs::Windows` so the follow-up does not need to revisit the abstraction.
- **GPU passthrough / VFIO.** CH supports VFIO; FC does not. The `FeatureSupport` matrix records this. Wiring VFIO devices through the agent and exposing them in the API is a separate PR.
- **Memory hot-plug exposure.** CH supports `virtio-mem` since v37+; FC supports it as of v1.14. The trait permits `mem_hotplug_add` as a future method, but the operations and DB shape needed (mem_slot table, hotplug events) are out of scope here. Both backends boot at a fixed `mem_mib` only.
- **Live migration.** Both VMMs have partial live-migration support that is fragile. Out of scope; the `FeatureSupport.live_migration` flag stays `false` for both backends shipped in this PR.
- **Host-side IMDS service.** Firecracker has built-in MMDS (`169.254.169.254`); CH does not. This PR's gating returns 400 when MMDS config is requested on CH. A host-side IMDS service replicating the FC contract is a separate future PR; the trait has no method for it because the replacement is not VMM-bound.
- **QEMU-full backend.** Trait supports it (just add a `Qemu` variant), but no impl in this PR.
- **Removing the FC HTTP-over-UDS proxy.** `apps/agent/src/features/vm/proxy.rs` keeps working for FC-specific endpoints (most notably for runtime drive PATCH, NIC PATCH, MMDS data writes). The new `VmmDriver` trait covers VMM-agnostic lifecycle; the FC-only proxy is the FC-only escape hatch. The proxy is not extended to CH — the CH driver covers everything CH-related.
- **Cross-backend snapshot translation.** A FC snapshot cannot be restored on CH and vice versa. This PR enforces same-backend restore at the manager. No translation tooling.
- **Agent inventory of CH binary version compatibility checks.** The agent reports `vmm_kinds_installed`; it does not pin the manager to a specific CH version. Operator is responsible for keeping the CH binary version stable across agents.
- **Routing functions and containers through the new trait.** The serverless `functions` feature and `containers` feature both depend on Firecracker-specific snapshot semantics (sub-125ms restore is the value prop). They keep using the FC code path directly. A `// TODO(vmm-backends): consider CH path for non-Linux runtimes` comment is added at `apps/manager/src/features/functions/vm.rs` and `apps/manager/src/features/containers/vm.rs` call sites. Routing them later is a separate PR concern.
- **Per-host VMM auto-install.** This PR assumes the operator installs the `cloud-hypervisor` binary on each agent host. The installer (`apps/installer/src/installer/deps.rs`) gains an opt-in step for CH installation, but auto-detection and forced installation are out of scope.

## Architectural intent (constraints, not implementation)

### The trait abstracts four things that vary between VMMs

1. **Process lifecycle** — how the VMM is launched (binary path, sandboxing, API socket creation, readiness signal). FC needs `--api-sock`, expects systemd-run + screen wrapping, has a 20s socket-readiness window. CH needs `--api-socket`, uses a slightly different ABI for the readiness signal, supports being launched with a config TOML or driven entirely over the API socket. Both are wrapped in `systemd-run` scopes for cgroup isolation.
2. **Configuration ABI** — the JSON/REST shape used to configure the VM before boot. FC's `/machine-config + /boot-source + /drives + /network-interfaces` differs from CH's `vm.create` (single combined payload) and incremental `vm.add-disk` / `vm.add-net`. The trait abstracts this away: callers pass a `VmSpec`, the impl translates.
3. **Snapshot file layout and ABI** — incompatible across backends. The trait owns the on-disk layout for its kind; the manager records `snapshot.vmm_kind` so restore is gated.
4. **Console transport** — FC and CH both expose serial over a Unix domain socket but with different protocol framing. The future Windows path replaces serial with VNC/RDP. The trait returns a discriminated `ConsoleEndpoint` and the WebSocket bridge handles each variant.

Things that **do not** vary and stay outside the trait: TAP/bridge networking (host concern, VMM-agnostic), volume bytes (storage backend concern, separate trait), guest agent (in-VM concern, VMM-agnostic for Linux guests).

### One trait, agent-side

Unlike storage (which split into `ControlPlaneBackend` + `HostBackend` because storage operations physically span manager and agent), VMM operations are agent-local: the VMM process runs on the agent, the API socket is on the agent's filesystem, snapshot files live on the agent's disk. The manager's role is "tell agent X to boot VM Y with spec Z," which is one HTTP call. So:

- **Agent-side `VmmDriver` trait** owns all per-VMM logic.
- **Manager-side** is plain orchestration: build `VmSpec` from DB rows, choose `VmmKind`, POST to agent.
- **Shared `nexus-vmm` crate** holds the trait definition, the spec/handle/feature types, and the `VmmKind`/`GuestOs`/`BootMode` enums (so manager can do feature gating and the request types are typed end-to-end).

```rust
// crates/nexus-vmm/src/lib.rs (new crate)

pub trait VmmDriver: Send + Sync {
    fn kind(&self) -> VmmKind;

    /// Pure function. Computed by code, not runtime; safe to call without an instance.
    fn features(&self, guest: GuestOs) -> FeatureSupport;

    /// Combined launch + configure + boot. Backends with multi-stage state machines
    /// (FC) implement this internally. Returns a handle the agent stores per-VM.
    async fn boot(
        &self,
        run_dir: &Path,
        spec: &VmSpec,
    ) -> Result<VmmHandle, VmmError>;

    async fn shutdown(
        &self,
        handle: &VmmHandle,
        mode: ShutdownMode,
    ) -> Result<(), VmmError>;

    async fn pause(&self, handle: &VmmHandle) -> Result<(), VmmError>;
    async fn resume(&self, handle: &VmmHandle) -> Result<(), VmmError>;
    async fn destroy(&self, handle: VmmHandle) -> Result<(), VmmError>;

    async fn snapshot(
        &self,
        handle: &VmmHandle,
        dst: &SnapshotPaths,
        kind: SnapshotKind,
    ) -> Result<SnapshotMeta, VmmError>;

    async fn restore(
        &self,
        run_dir: &Path,
        vm_id: Uuid,
        src: &SnapshotPaths,
        spec_overrides: &SpecOverrides,
    ) -> Result<VmmHandle, VmmError>;

    /// Hot-add/-update/-remove. Implementations may return
    /// VmmError::NotSupported when the running guest cannot accept the change
    /// (e.g., FC without virtio-pci-hotplug for nic_remove).
    async fn drive_add(&self, handle: &VmmHandle, disk: &DiskSpec) -> Result<(), VmmError>;
    async fn drive_update(&self, handle: &VmmHandle, drive_id: &str, new: &DiskSpec) -> Result<(), VmmError>;
    async fn drive_remove(&self, handle: &VmmHandle, drive_id: &str) -> Result<(), VmmError>;
    async fn nic_add(&self, handle: &VmmHandle, nic: &NicSpec) -> Result<(), VmmError>;
    async fn nic_update(&self, handle: &VmmHandle, iface_id: &str, new: &NicSpec) -> Result<(), VmmError>;
    async fn nic_remove(&self, handle: &VmmHandle, iface_id: &str) -> Result<(), VmmError>;

    async fn console_endpoint(&self, handle: &VmmHandle) -> Result<ConsoleEndpoint, VmmError>;
    async fn metrics_snapshot(&self, handle: &VmmHandle) -> Result<HashMap<String, serde_json::Value>, VmmError>;

    /// Re-attach to a still-running VMM after agent restart. Returns Ok(Some)
    /// when a live process is found, Ok(None) when there is no running VMM
    /// for this id, Err on filesystem/permission failures.
    async fn rebind(&self, run_dir: &Path, vm_id: Uuid) -> Result<Option<VmmHandle>, VmmError>;
}
```

`VmmDriver` impls are stateless — all per-VM state lives in `VmmHandle` (which is serializable to disk for crash recovery) and in the VMM process itself. Agent's `VmmRegistry` holds one `Arc<dyn VmmDriver>` per kind, all constructed at startup.

### Core types

```rust
#[derive(Clone, Copy, Serialize, Deserialize, ToSchema, sqlx::Type)]
#[sqlx(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum VmmKind {
    Firecracker,
    CloudHypervisor,
    // future: Qemu
}

#[derive(Clone, Copy, Serialize, Deserialize, ToSchema, sqlx::Type)]
#[sqlx(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum GuestOs {
    /// Linux booted via Linux 64-bit boot protocol with vmlinux + optional initrd.
    /// FC-only on x86_64; CH supports it too.
    LinuxKernel,
    /// Linux booted from a full disk image (PVH or UEFI).
    LinuxDisk,
    /// Windows. CH-only. Reserved; not supported in this PR.
    Windows,
    /// Catch-all for BSD, Haiku, etc. Reserved.
    Other,
}

#[derive(Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum BootMode {
    /// vmlinux/bzImage + optional initrd. FC native; CH supports too.
    LinuxKernel {
        kernel: PathBuf,
        initrd: Option<PathBuf>,
        cmdline: String,
    },
    /// PVH ELF kernel. CH-only path; faster than UEFI for Linux.
    Pvh {
        kernel: PathBuf,
        cmdline: String,
    },
    /// UEFI firmware boot (OVMF/EDK2). CH-only.
    /// Required for Windows; required for many modern distro disk images.
    Uefi {
        firmware: PathBuf,
        nvram_template: Option<PathBuf>,
    },
}

#[derive(Clone, Serialize, Deserialize)]
pub struct VmSpec {
    pub id: Uuid,
    pub vcpu: u32,
    pub mem_mib: u32,
    pub boot: BootMode,
    pub disks: Vec<DiskSpec>,
    pub nics: Vec<NicSpec>,
    pub vsock: Option<VsockSpec>,
    pub mmds: Option<MmdsSpec>,         // FC-only feature; gated
    pub balloon: Option<BalloonSpec>,
    pub entropy: bool,
    pub console: ConsoleSpec,
}

pub struct DiskSpec {
    pub drive_id: String,
    pub source: AttachedPath,            // from storage backend (AttachedPath::File / BlockDevice / VhostUserSock)
    pub read_only: bool,
    pub root_device: bool,
    pub rate_limiter: Option<RateLimiter>,
}

pub struct NicSpec {
    pub iface_id: String,
    pub host_dev: String,                // tap name
    pub mac: String,
    pub rate_limiter: Option<RateLimiter>,
}

pub struct VmmHandle {
    pub vm_id: Uuid,
    pub kind: VmmKind,
    pub api_sock: PathBuf,
    pub pid: u32,
    pub systemd_unit: String,
    pub console_sock: Option<PathBuf>,   // serial UDS; None for non-serial backends
}

pub enum ConsoleEndpoint {
    /// Unix domain socket carrying serial bytes (FC + CH-Linux).
    UnixSerial(PathBuf),
    /// PTY exposed by CH. Different framing from UDS serial.
    Pty(PathBuf),
    /// VNC. Reserved for Windows guests; out of scope this PR.
    Vnc { host: String, port: u16, password: Option<String> },
    /// RDP-bridge process (e.g., FreeRDP relay). Reserved for Windows.
    Rdp { host: String, port: u16 },
}

pub enum ShutdownMode {
    /// FC: i8042 reset bit (only graceful path FC has).
    /// CH: vm.shutdown (ACPI shutdown signal).
    Graceful,
    /// SIGKILL the VMM process.
    Hard,
}

pub enum SnapshotKind { Full, Diff }

pub struct SnapshotPaths {
    pub state_path: PathBuf,
    pub mem_path: Option<PathBuf>,       // None for Diff
    pub diff_dir: Option<PathBuf>,       // Some only for Diff
}

pub struct SnapshotMeta {
    pub kind: VmmKind,                   // restore is gated on this matching the target
    pub vmm_version: String,             // for diagnostics; not gated (operator promises stability)
    pub state_size_bytes: u64,
    pub mem_size_bytes: Option<u64>,
}

pub struct FeatureSupport {
    pub fast_snapshot: bool,
    pub diff_snapshot: bool,
    pub mmds: bool,
    pub uefi_boot: bool,
    pub virtio_pci: bool,
    pub virtio_console: bool,
    pub balloon: bool,
    pub entropy: bool,
    pub vsock: bool,
    pub memory_hotplug: bool,            // future-PR; stays false for both backends in this PR
    pub gpu_passthrough: bool,           // future-PR; stays false
    pub vfio_passthrough: bool,          // future-PR; stays false
    pub live_migration: bool,            // false for both backends
    pub windows_guest: bool,             // false for both in this PR (CH true in follow-up)
}

#[derive(Debug, thiserror::Error)]
pub enum VmmError {
    #[error("VMM not installed: {0}")]
    NotInstalled(String),
    #[error("not supported by {kind:?}: {feature}")]
    NotSupported { kind: VmmKind, feature: String },
    #[error("API socket timeout for vm {vm_id}")]
    SocketTimeout { vm_id: Uuid },
    #[error("VMM process exited unexpectedly: {0}")]
    ProcessGone(String),
    #[error("snapshot kind mismatch: expected {expected:?}, found {found:?}")]
    SnapshotKindMismatch { expected: VmmKind, found: VmmKind },
    #[error("invalid spec: {0}")]
    InvalidSpec(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
```

### `FeatureSupport` is a pure function of (VmmKind, GuestOs)

```rust
impl VmmKind {
    pub fn features(&self, guest: GuestOs) -> FeatureSupport {
        match (self, guest) {
            (VmmKind::Firecracker, GuestOs::LinuxKernel) => FeatureSupport {
                fast_snapshot: true,
                diff_snapshot: true,
                mmds: true,
                uefi_boot: false,
                virtio_pci: false,
                virtio_console: false,
                balloon: true,
                entropy: true,
                vsock: true,
                memory_hotplug: false,    // FC v1.14 added it; this PR pinned to v1.13.1 — see §upgrade
                gpu_passthrough: false,
                vfio_passthrough: false,
                live_migration: false,
                windows_guest: false,
            },
            (VmmKind::Firecracker, _) => FeatureSupport::NONE,
            (VmmKind::CloudHypervisor, GuestOs::LinuxKernel | GuestOs::LinuxDisk) => FeatureSupport {
                fast_snapshot: true,            // slower than FC, but supported
                diff_snapshot: false,           // CH does not have diff snapshots
                mmds: false,                    // CH has no MMDS
                uefi_boot: true,                // x86_64 + aarch64
                virtio_pci: true,
                virtio_console: true,
                balloon: true,
                entropy: true,
                vsock: true,
                memory_hotplug: false,          // future-PR
                gpu_passthrough: false,         // future-PR
                vfio_passthrough: false,        // future-PR
                live_migration: false,
                windows_guest: false,           // follow-up PR; flips to true when image registry, console bridge, guest agent ship
            },
            (VmmKind::CloudHypervisor, GuestOs::Windows) => FeatureSupport::NONE, // disabled until follow-up
            (VmmKind::CloudHypervisor, GuestOs::Other) => FeatureSupport::NONE,
        }
    }
}
```

`FeatureSupport::NONE` is a const with every flag false; manager rejects creation with a 400 on any feature check.

### Manager-side gating policy

The manager validates incoming VM-create requests **before** contacting any agent:

1. Compute `features = vmm_kind.features(guest_os)`.
2. For every requested feature, check the corresponding bit. Reject with 400 + the failing feature name.
3. For every backend-specific config (e.g., `boot_mode = Uefi` requires `features.uefi_boot`), check accordingly.
4. Lookup the target host. If `host.vmm_kinds_installed` does not include `vmm_kind`, reject with 400 referencing the missing driver.

This is computed in `apps/manager/src/features/vms/service.rs::create_vm` before any DB row is written.

### Manager-side selection logic

When `vmm_kind` is unspecified in `POST /v1/vms`:

```
if boot_mode is LinuxKernel  -> Firecracker
if boot_mode is Pvh or Uefi  -> CloudHypervisor
```

When the user explicitly specifies `vmm_kind` and it conflicts with `boot_mode` (e.g., `Firecracker + Uefi`), reject with 400 and a message naming both the kind and the boot mode. Auto-selection is documented as the default; explicit specification is the override.

Image registry implicitly constrains this: a `LinuxKernel` image cannot be used with `BootMode::Uefi`, etc. The image's `image_kind` field is the source of truth and the request validator cross-checks.

### Agent registration reports installed VMM kinds

`apps/agent/src/features/inventory` is extended:

```rust
pub struct AgentRegistration {
    // ...existing fields...
    pub vmm_kinds_installed: Vec<VmmKind>,
}
```

Determined at agent startup by probing for binaries (`firecracker --version`, `cloud-hypervisor --version`) and checking whether the corresponding driver was compiled in (feature flag — see Cargo features below).

Manager records this in the `hosts` table:

```sql
ALTER TABLE hosts ADD COLUMN vmm_kinds_installed TEXT[] NOT NULL DEFAULT '{firecracker}';
```

`vmm_kinds_installed` updates on every heartbeat — the agent re-probes on each, so an operator-installed CH binary becomes available without a manager restart.

### Console abstraction and the WebSocket shell bridge

The existing `apps/agent/src/core/uds_proxy.rs` knows only Firecracker's serial-on-screen layout. Refactor:

- `VmmDriver::console_endpoint(handle)` returns a `ConsoleEndpoint` enum.
- `apps/agent/src/features/vm/shell.rs` (the WS handler) dispatches:
   - `UnixSerial(path)` → existing screen-based bridge (works for FC and CH-Linux).
   - `Pty(path)` → tty-pass-through bridge (CH-PTY mode; new but trivial).
   - `Vnc { ... }`, `Rdp { ... }` → returns `501 Not Implemented` in this PR. Wired up when the Windows PR lands.

The DB column `vms.api_sock` is preserved. A new `vms.console_kind TEXT NOT NULL DEFAULT 'unix_serial'` column tags which variant the row uses; the WS handler reads it to choose the bridge.

### Snapshot lifecycle

- `snapshots.vmm_kind TEXT NOT NULL` column added (backfilled to `firecracker` for existing rows).
- `apps/agent/src/features/vm/snapshot.rs::prepare` becomes generic: directory layout is determined by `vmm_kind` from the request. FC layout (`snapshot.fc`, `mem.fc`, `diff/`) preserved; CH gets its own (`vmstate.json`, `memory-ranges/<index>`, etc.).
- Manager-side `apps/manager/src/features/snapshots/routes.rs::create` calls `VmmDriver::snapshot()` via the agent. The agent picks the driver from the VM's `vmm_kind`.
- Restore: manager looks up `snapshots.vmm_kind`, target VM's `vmm_kind`, refuses if they differ. No translation.
- Snapshot-from-template flows (`apps/manager/src/features/templates`) carry `vmm_kind` through; instantiating a template targets the same kind.

### Image registry — `image_kind` discriminator

```sql
ALTER TABLE images ADD COLUMN image_kind TEXT NOT NULL DEFAULT 'linux_kernel';
ALTER TABLE images ADD COLUMN nvram_template_path TEXT;
ALTER TABLE images
  ADD CONSTRAINT image_kind_chk
  CHECK (image_kind IN ('linux_kernel', 'linux_disk', 'uefi_disk'));
```

- `linux_kernel`: kernel + rootfs pair. Existing rows. Compatible with `BootMode::LinuxKernel`.
- `linux_disk`: bootable Linux disk image, no separate kernel. Compatible with `BootMode::Pvh` or `BootMode::Uefi` (latter requires `nvram_template_path`).
- `uefi_disk`: bootable disk image that requires UEFI firmware. `nvram_template_path` is mandatory.

Image upload UI gains a kind selector. Image scanning (`apps/manager/src/features/images/scan.rs`) gains a heuristic-based detector for backwards compat, but the kind is the source of truth.

### Backwards compatibility is non-negotiable

Every existing VM keeps booting on Firecracker. Every existing snapshot keeps restoring. Every existing template keeps instantiating. Every existing image keeps working. Achieved by:

- Default values in the migration (`vmm_kind = 'firecracker'`, `guest_os = 'linux_kernel'`, `image_kind = 'linux_kernel'`, etc.).
- `FirecrackerDriver` is a refactor of existing code, not a rewrite. Its outputs match current behaviour byte-for-byte.
- The agent's existing `apps/agent/src/features/vm/proxy.rs` route stays operational. FC-specific PATCHes (drive rate-limiter updates, MMDS data writes) keep going through it. The new `VmmDriver` covers VMM-agnostic lifecycle; the proxy is the FC-only escape hatch.
- A reconciler-driven dry-run on agent startup verifies all known FC VMs can be `rebind`-ed.

### Cargo feature flags

Each backend driver lives behind a Cargo feature so operators can build a slimmer agent if they only need one VMM:

```toml
# apps/agent/Cargo.toml
[features]
default = ["vmm-firecracker", "vmm-cloud-hypervisor"]
vmm-firecracker = []
vmm-cloud-hypervisor = []
```

The `VmmRegistry` only registers drivers whose feature is enabled. The agent registration reports only enabled kinds.

### Firecracker version pin

This PR does not bump Firecracker. Current pin (v1.13.1) is preserved to keep snapshot compatibility for existing VMs. A separate Firecracker upgrade PR (v1.13.1 → v1.15.x) is tracked independently.

### Cloud Hypervisor version pin

CH `v50.0` (or latest stable at PR time). Pin in `install-cloud-hypervisor.sh` (new) and document in `SETUP.md`. Snapshot compatibility within a minor version is the operator's responsibility — same posture as FC.

## DB migration

```sql
-- apps/manager/migrations/0034_vmm_backends.sql

-- 1. VM-level VMM kind, guest OS, boot mode.
ALTER TABLE vms ADD COLUMN vmm_kind TEXT NOT NULL DEFAULT 'firecracker';
ALTER TABLE vms ADD COLUMN guest_os TEXT NOT NULL DEFAULT 'linux_kernel';
ALTER TABLE vms ADD COLUMN boot_mode JSONB;             -- nullable initially
ALTER TABLE vms ADD COLUMN console_kind TEXT NOT NULL DEFAULT 'unix_serial';

ALTER TABLE vms
  ADD CONSTRAINT vms_vmm_kind_chk
  CHECK (vmm_kind IN ('firecracker', 'cloud_hypervisor'));
ALTER TABLE vms
  ADD CONSTRAINT vms_guest_os_chk
  CHECK (guest_os IN ('linux_kernel', 'linux_disk', 'windows', 'other'));
ALTER TABLE vms
  ADD CONSTRAINT vms_console_kind_chk
  CHECK (console_kind IN ('unix_serial', 'pty', 'vnc', 'rdp'));

-- 2. Backfill boot_mode for existing rows from kernel/rootfs columns
UPDATE vms SET boot_mode = jsonb_build_object(
  'mode', 'linux_kernel',
  'kernel', kernel_path,            -- existing column
  'initrd', initrd_path,            -- existing column, may be NULL
  'cmdline', kernel_cmdline         -- existing column
)
WHERE boot_mode IS NULL;
ALTER TABLE vms ALTER COLUMN boot_mode SET NOT NULL;

-- 3. Snapshot kind tag
ALTER TABLE snapshots ADD COLUMN vmm_kind TEXT NOT NULL DEFAULT 'firecracker';
ALTER TABLE snapshots
  ADD CONSTRAINT snapshots_vmm_kind_chk
  CHECK (vmm_kind IN ('firecracker', 'cloud_hypervisor'));

-- 4. Image kind discriminator
ALTER TABLE images ADD COLUMN image_kind TEXT NOT NULL DEFAULT 'linux_kernel';
ALTER TABLE images ADD COLUMN nvram_template_path TEXT;
ALTER TABLE images
  ADD CONSTRAINT images_image_kind_chk
  CHECK (image_kind IN ('linux_kernel', 'linux_disk', 'uefi_disk'));
ALTER TABLE images
  ADD CONSTRAINT images_uefi_nvram_chk
  CHECK (image_kind <> 'uefi_disk' OR nvram_template_path IS NOT NULL);

-- 5. Host inventory
ALTER TABLE hosts ADD COLUMN vmm_kinds_installed TEXT[] NOT NULL DEFAULT ARRAY['firecracker'];

-- 6. Templates carry vmm_kind so instantiations stay homogeneous.
ALTER TABLE templates ADD COLUMN vmm_kind TEXT NOT NULL DEFAULT 'firecracker';
ALTER TABLE templates
  ADD CONSTRAINT templates_vmm_kind_chk
  CHECK (vmm_kind IN ('firecracker', 'cloud_hypervisor'));
```

The legacy `kernel_path`, `initrd_path`, `kernel_cmdline` columns on `vms` are kept (read-only) for one release cycle to allow rollback. A follow-up migration drops them once all flows source from `boot_mode`.

## Success criteria

- **No regression**: existing VMs continue to function with no operator action. `cargo test -p manager` and `cargo test -p agent` pass without modification. Existing snapshots restore. Existing templates instantiate. Existing serverless functions cold-start in ≤125ms (benchmark unchanged).
- **CH+UEFI Linux boots end-to-end**: an Ubuntu 24.04 cloud image (qcow2 → raw conversion) can be uploaded, registered as `image_kind = uefi_disk` with an OVMF firmware reference, and a VM created from it boots and is reachable via the WebSocket shell.
- **Feature gating works**: requesting `mmds` config on a CH VM returns 400 with `MMDS not supported by cloud_hypervisor`. Requesting `vmm_kind = firecracker` with `boot_mode = uefi` returns 400.
- **Selection works**: a VM created with `boot_mode = uefi` and no `vmm_kind` lands on CH. A VM created with `boot_mode = linux_kernel` and no `vmm_kind` lands on FC. A VM created with explicit `vmm_kind = cloud_hypervisor` and `boot_mode = linux_kernel` lands on CH (PVH path).
- **Agent inventory works**: an agent host without `cloud-hypervisor` installed reports `vmm_kinds_installed = ['firecracker']`; the manager refuses to schedule a CH VM onto it, returning a 400 referencing the missing driver.
- **Snapshot gating works**: attempting to restore a Firecracker snapshot onto a CH VM returns 400 with `snapshot kind mismatch`. Same-kind restore works for both FC and CH.
- **Trait separation is honest**: adding a hypothetical third backend (mock VMM in tests) requires implementing `VmmDriver`, registering in `VmmRegistry`, and adding the kind to enums. Zero changes to manager orchestration code, snapshot routes, or other backends.
- **Backwards compatibility holds**: a fixture loading the pre-migration `vms` and `images` tables (snapshot of an existing production-like deployment) runs `0034_vmm_backends.sql` cleanly, and every fixture VM remains bootable post-migration.
- **`cargo fmt --check` and `cargo clippy --all-targets --all-features -- -D warnings` pass.**
- **Lint check that no new code path bypasses gating**: a linter test asserts that all routes that mutate VM state go through `validate_against_features` (regex over manager source).

## Non-goals to be explicit about

- Not adding Windows guest support in this PR. The trait + `BootMode::Uefi` + `GuestOs::Windows` reservation lay the foundation; the actual Windows enablement (image flow, console bridge, guest agent port, container/function gating) is a separate follow-up PR.
- Not adding GPU/VFIO passthrough. CH supports it; the trait would gain device methods; out of scope.
- Not adding memory hot-plug, even though both backends now support it. New API surface, new DB shape, new UI controls — separate PR.
- Not implementing host-side IMDS replacement for CH. MMDS-on-CH returns 400 in this PR. The replacement is a separate Axum service that listens on a host network namespace and serves `169.254.169.254`; that work has its own PR.
- Not routing functions or containers through the trait. Both keep using FC directly. Routing them is a perf-sensitive design question (Linux containers don't benefit from CH; they want FC's snapshot-based cold-start) and deserves its own PR.
- Not removing the FC HTTP-over-UDS proxy. It is the FC-only escape hatch for endpoint-level operations the trait does not abstract (drive PATCH rate-limiter updates, MMDS data writes, etc.). The proxy keeps working for FC; CH operations route exclusively through the trait.
- Not bumping Firecracker version. v1.13.1 stays. Firecracker upgrade is a separate PR.
- Not auto-installing Cloud Hypervisor. The installer adds an opt-in step; auto-install is out of scope.
- Not building a "VMM benchmark" or capability discovery beyond `vmm_kinds_installed`. Operators are responsible for using the same CH version across agents.

## Open questions to resolve during implementation

These are tactical decisions the implementer makes; flag them in the PR description. None require revisiting the trait shape.

- **CH binary acquisition.** (Recommendation: a parallel `install-cloud-hypervisor.sh` script that pulls a pinned release from `cloud-hypervisor/cloud-hypervisor` GitHub releases. Mirror the existing FC install pattern.)
- **CH config flow.** CH supports both single-shot `vm.create` with full config, and incremental device adds. (Recommendation: use single-shot for boot, incremental for hot-add. Single-shot is simpler and the spec already builds the full config.)
- **CH `seccomp` profile.** CH ships several. (Recommendation: use the strictest production profile — `--seccomp true`, the default. Document deviation in the PR if the CH error logs require relaxing.)
- **CH jailer equivalent.** CH does not have FC's jailer. (Recommendation: rely on `systemd-run` cgroup + namespaces; CH binary already does seccomp filtering internally. Operators wanting stricter sandboxing can wrap with `bubblewrap` later.)
- **Where OVMF firmware lives.** (Recommendation: shipped under `/srv/firmware/OVMF.fd` on each agent, installed by the installer. The image record points to this path. NVRAM templates live next to firmware.)
- **NVRAM template handling for UEFI.** Each UEFI VM needs its own NVRAM file (boot variables persist there). (Recommendation: the agent copies `nvram_template_path` to `<run_dir>/vms/<id>/nvram.fd` on first boot. Persisted across restarts but reset on `vm delete`. NVRAM bytes are NOT included in snapshots in this PR — same posture as FC's lack of UEFI altogether means there is no regression.)
- **CH PVH vs UEFI for Linux disk images.** PVH is faster but only works with Linux kernels that ship a PVH note. Most modern distro disk images do. (Recommendation: when `image_kind = linux_disk` and the image registers a PVH-capable kernel, use PVH; otherwise fall back to UEFI. The image registry detects this at upload time via `readelf -n`.)
- **CH snapshot file layout.** Pinned to CH's documented format; manager treats it as opaque. Diff snapshots are not implemented in CH; the trait's `SnapshotKind::Diff` returns `VmmError::NotSupported` from `CloudHypervisorDriver`.
- **Where `VmmHandle` lives across agent restarts.** (Recommendation: serialize to `<run_dir>/vms/<id>/handle.json` on every transition. Agent's startup `rebind` loop reads each file, calls `VmmDriver::rebind`, drops handles whose VMM process is gone.)
- **`metrics_snapshot` shape.** FC and CH expose different metrics shapes. (Recommendation: trait returns `HashMap<String, serde_json::Value>` and the manager UI tolerates missing keys per-backend. A future PR can introduce a normalized metric set if the divergence becomes noisy.)
- **Logging unification.** FC writes structured JSON to a FIFO; CH writes line-oriented logs to stderr. (Recommendation: the agent tags logs with `vmm_kind` and forwards to the existing log pipeline. No structural change in this PR.)
- **CH API socket retry semantics.** FC takes ~50ms to ready; CH varies. (Recommendation: same 20s timeout window for both. Calibrate after first integration test run.)
- **vsock CID assignment.** Both VMMs accept any CID. The existing CID allocator in `apps/manager/src/features/vms/service.rs` is VMM-agnostic and reused.
- **TAP setup divergence.** Identical for both — TAP is a kernel concept, not a VMM concept. The existing `apps/agent/src/features/networks` code is reused unchanged.

Resolve these as you go; document the choices in the PR description.

## File-level outline of the change

### New crate

- **`crates/nexus-vmm/`** — new workspace member.
  - `src/lib.rs` — trait definitions and shared types listed in §Core types.
  - `src/types/{spec,handle,boot,console,error}.rs` — split files for clarity.
  - `src/features.rs` — `FeatureSupport` and `(VmmKind, GuestOs) -> FeatureSupport` table.
  - `Cargo.toml` — depends on `serde`, `thiserror`, `uuid`, `utoipa`, `sqlx` (for `sqlx::Type` derives).
  - Add to `[workspace.members]` in root `Cargo.toml`.

### Agent

- `apps/agent/src/features/vmm/mod.rs` (new) — `VmmRegistry`, `Arc<dyn VmmDriver>` map by kind.
- `apps/agent/src/features/vmm/firecracker/mod.rs` (new) — `FirecrackerDriver` impl. Body extracted from `apps/agent/src/features/vm/spawn.rs` and `apps/agent/src/features/vm/snapshot.rs`. Existing files become thin shims that look up the driver and delegate.
- `apps/agent/src/features/vmm/cloud_hypervisor/mod.rs` (new) — `CloudHypervisorDriver` impl. Spawns `cloud-hypervisor` binary, talks to its REST API.
- `apps/agent/src/features/vmm/cloud_hypervisor/api.rs` (new) — typed CH REST client (HTTP-over-UDS). Mirrors existing FC client patterns.
- `apps/agent/src/features/vm/spawn.rs` — refactored to look up `vmm_kind` from request body, dispatch via `VmmRegistry`.
- `apps/agent/src/features/vm/snapshot.rs` — refactored similarly. The `prepare` endpoint takes `vmm_kind` and returns the per-kind directory layout.
- `apps/agent/src/features/vm/shell.rs` — refactored to call `VmmDriver::console_endpoint`, dispatch by `ConsoleEndpoint` variant.
- `apps/agent/src/features/inventory/mod.rs` — extended to probe and report `vmm_kinds_installed`.
- `apps/agent/src/main.rs` — wires `VmmRegistry` into `AppState` at startup.
- `apps/agent/src/features/vm/proxy.rs` — unchanged (FC-only escape hatch stays).
- `apps/agent/Cargo.toml` — adds `vmm-firecracker`, `vmm-cloud-hypervisor` features. Default = both. Depends on `nexus-vmm`.
- `apps/agent/src/core/uds_proxy.rs` — extended to handle `Pty` console endpoint variant (existing UDS flow stays for `UnixSerial`).

### Manager

- `apps/manager/src/features/vms/service.rs` — `create_vm`:
   - reads `vmm_kind` and `boot_mode` from request (or auto-selects).
   - validates against `VmmKind::features(GuestOs)`.
   - validates against `host.vmm_kinds_installed`.
   - sends `VmSpec` (constructed from row) to agent's `spawn` endpoint with `vmm_kind` discriminator.
- `apps/manager/src/features/vms/routes.rs` — `POST /v1/vms` accepts `vmm_kind`, `boot_mode`, validates and maps.
- `apps/manager/src/features/vms/repo.rs` — extended row model with new columns; `sqlx::FromRow` derives unchanged.
- `apps/manager/src/features/snapshots/routes.rs` — `create` and `instantiate` carry `vmm_kind`; `instantiate` rejects mismatch.
- `apps/manager/src/features/snapshots/repo.rs` — extended row.
- `apps/manager/src/features/templates/{routes,repo,service}.rs` — `vmm_kind` carried through template lifecycle.
- `apps/manager/src/features/images/{routes,repo,service,scan}.rs` — `image_kind`, `nvram_template_path` added; scan heuristics for kind detection.
- `apps/manager/src/features/hosts/{routes,repo}.rs` — `vmm_kinds_installed` exposed via `GET /v1/hosts/:id`; updated on heartbeat.
- `apps/manager/src/features/vms/agent.rs` (or the equivalent agent-call helper) — `spawn` payload extended with `vmm_kind` and `boot_mode`.
- `apps/manager/migrations/0034_vmm_backends.sql` (new) — see §DB migration.
- `apps/manager/Cargo.toml` — depends on `nexus-vmm`.

### Shared

- `crates/nexus-types/src/lib.rs` — re-exports `VmmKind`, `GuestOs`, `BootMode`, `FeatureSupport` from `nexus-vmm` (so generated OpenAPI uses one source of truth) and adds the new `vmm_kind`, `boot_mode`, `image_kind`, `nvram_template_path` fields on the relevant request/response types (`CreateVmReq`, `Vm`, `Image`, `CreateImageReq`, `Snapshot`, `Template`).

### UI

- `apps/ui/lib/types/index.ts` — `VmmKind`, `GuestOs`, `BootMode`, `ImageKind`, `FeatureSupport`.
- `apps/ui/lib/queries.ts` — extended hooks return new fields; new `useHostCapabilities()` hook reads `vmm_kinds_installed`.
- `apps/ui/components/vm/vm-create-form.tsx` — backend selector ("Auto" default, advanced override). Boot-mode selector cross-validates against image-kind. Greys out unsupported features per the resolved (kind, guest-os) tuple.
- `apps/ui/components/image/image-upload-form.tsx` — `image_kind` selector; UEFI requires NVRAM template upload.
- `apps/ui/components/host/host-detail.tsx` — shows installed VMM kinds.

### Configuration / installer

- `install-cloud-hypervisor.sh` (new) — pinned-version installer, mirrors `install-firecracker.sh`.
- `apps/installer/src/installer/deps.rs` — opt-in step for CH installation; `install_cloud_hypervisor(version: &str)` mirrors existing `install_firecracker`.
- `SETUP.md` — document the new opt-in step.
- `nqrust.toml` (operator config) — gains optional `[vmm]` section with default-kind override:
  ```toml
  [vmm]
  default_kind = "firecracker"   # or "cloud_hypervisor"
  ```
  Manager reads at startup; affects only the auto-selection fallback when the user does not specify `vmm_kind`.

## Testing strategy

The strategy is organized by layer (crate → driver → manager → integration → build matrix → manual smoke). Every change-surface item in §File-level outline must be covered by at least one bucket below; the reverse mapping is given at the end.

### `nexus-vmm` crate unit tests (`crates/nexus-vmm/src/...#[cfg(test)]`)

- **`FeatureSupport` purity:** `VmmKind::features(GuestOs)` is total over the cartesian product. Table-driven assertions for every `(VmmKind, GuestOs)` pair, including the `Other` and `Windows` rows that must return `FeatureSupport::NONE`.
- **`BootMode` serde round-trip:** every variant round-trips through JSON. Field-level coverage: `Uefi { firmware, nvram_template = None }` deserializes; `Uefi` with absent `firmware` returns a deserialization error.
- **`VmSpec` validator:** invalid combinations are rejected with `VmmError::InvalidSpec` referencing the failing field. Cases: `Uefi` boot mode with no UEFI image, `LinuxKernel` boot with empty cmdline, `mmds` set on a spec where `vmm_kind != Firecracker` (validator-level, before agent dispatch), more than one `root_device = true` disk, `vcpu == 0`, `mem_mib == 0`.
- **`VmmError` display:** every variant prints a non-empty, non-debug string. Trivial but catches accidental stripping of `#[error]` attrs.
- **`SnapshotMeta` ABI tag:** restore code can determine compatibility purely from `meta.kind` without reading file bytes.

### Per-driver unit tests

#### `FirecrackerDriver` (`apps/agent/src/features/vmm/firecracker/...#[cfg(test)]`)

Uses a FC-API mock server (axum on a tmpdir UDS) that records every request.

- **Lifecycle round-trip:** `boot → pause → resume → shutdown(Graceful) → destroy`. Asserts the recorded sequence of FC API calls matches the historical sequence (PUT `/machine-config`, PUT `/boot-source`, PUT `/drives/*`, PUT `/network-interfaces/*`, PUT `/actions {InstanceStart}`, PATCH `/vm Pause`, PATCH `/vm Resume`, PUT `/actions SendCtrlAltDel`).
- **Hot-add/-update/-remove for drives and NICs:** each method produces the correct PATCH/PUT and round-trips correctly through the mock.
- **`shutdown(Hard)`:** sends SIGKILL to the wrapped scope, returns once `/proc/<pid>` is gone.
- **Snapshot full + diff:** writes the expected files (`snapshot.fc`, `mem.fc`, or `diff.fc` + `diff/`); `SnapshotMeta.kind == Firecracker`.
- **Restore into compatible spec:** preserves CID, drive paths, NIC TAPs.
- **`rebind` after agent kill:** with the API socket still alive, `rebind` returns `Some(handle)` whose `pid` matches the live process; with the socket gone, returns `Ok(None)`.
- **Refactor invariance:** **byte-for-byte FC API-call sequence equality** test against a fixture captured from the pre-refactor code path. Failing this test means the FC behaviour has drifted.

#### `CloudHypervisorDriver` (`apps/agent/src/features/vmm/cloud_hypervisor/...#[cfg(test)]`)

Uses a CH-API mock server analogous to FC's.

- **PVH boot path:** `BootMode::Pvh` round-trips through `vm.create` with `payload.kernel` and no UEFI fields.
- **UEFI boot path:** `BootMode::Uefi` round-trips through `vm.create` with `payload.firmware`, NVRAM template copied to `<run_dir>/vms/<id>/nvram.fd` *before* the API call, and `vm.create` references the copied path (not the template path).
- **NVRAM lifecycle:** on `boot`, NVRAM template is copied to the VM run dir if absent; on subsequent boots (e.g., after stop/start), the existing per-VM NVRAM is reused (variables persist); on `destroy`, the per-VM NVRAM file is removed.
- **Hot-add/-update/-remove for drives and NICs:** each method produces the correct CH API call.
- **`shutdown(Graceful)`:** issues `vm.shutdown` (ACPI shutdown), waits for VMM exit.
- **`shutdown(Hard)`:** SIGKILL the scope.
- **Snapshot:** full snapshot writes CH-format state and memory range files; `SnapshotMeta.kind == CloudHypervisor`. **`SnapshotKind::Diff` returns `VmmError::NotSupported`** with a clear message naming `cloud_hypervisor`.
- **Restore into compatible spec:** preserves CID, drive paths, NIC TAPs, NVRAM file path.
- **`rebind` after agent kill:** mirrors FC's test.
- **API-socket readiness window:** mock delays socket creation by 0/100/500/2000ms; driver reports success at ≤2s and `VmmError::SocketTimeout` at >20s.

### Trait-purity / registry tests (`apps/agent/src/features/vmm/mod.rs#[cfg(test)]`)

- **Dispatch correctness:** `VmmRegistry.driver(VmmKind::Firecracker)` returns the FC impl; `VmmRegistry.driver(VmmKind::CloudHypervisor)` returns the CH impl. Calls through the registry produce identical effects to direct calls — verified by replaying the same operations against both paths and asserting recorded API-call equality.
- **Driver-not-installed:** asking the registry for a kind not registered (e.g., compiled with `vmm-firecracker` only, queried for `CloudHypervisor`) returns `VmmError::NotInstalled` rather than panicking.
- **Binary-missing-from-PATH:** `vmm-cloud-hypervisor` feature compiled in, but `which cloud-hypervisor` fails. Driver registers but reports `NotInstalled` from the inventory probe; manager-side gating treats the host as if the kind were absent.

### Manager-side gating tests (`apps/manager/tests/vmm_*.rs`)

- **Feature-gating** (`vmm_feature_gating.rs`): table-driven over `(VmmKind, GuestOs, requested_feature)`. Every flag in `FeatureSupport` has at least one `false` cell exercised. Asserts 400 with the failing feature name in the response body, no DB row written.
- **Selection** (`vmm_selection.rs`): `boot_mode = linux_kernel` + unspecified kind → FC; `boot_mode = pvh` + unspecified → CH; `boot_mode = uefi` + unspecified → CH; explicit `firecracker + uefi` → 400 referencing both fields. Default-kind TOML override flips the unspecified-kind fallback (when `[vmm] default_kind = "cloud_hypervisor"`, an unspecified kind with `boot_mode = linux_kernel` lands on CH, which is valid since CH supports that boot mode).
- **Inventory gating** (`vmm_inventory_gating.rs`): host with `vmm_kinds_installed = ['firecracker']` rejects CH VMs at the manager (400) before any agent call; host with both kinds accepts both. Heartbeat that drops a kind from inventory invalidates new requests but does not affect already-running VMs.
- **Snapshot kind gating** (`vmm_snapshot_gating.rs`): same-kind restore succeeds; cross-kind restore returns 400 with `snapshot kind mismatch`.
- **Image-kind cross-validation** (`vmm_image_kind.rs`): `boot_mode = uefi` requires `image_kind = uefi_disk`; `boot_mode = linux_kernel` requires `image_kind = linux_kernel`; mismatches return 400 referencing both. UEFI image without `nvram_template_path` is rejected at upload (CHECK constraint + service-layer test).
- **Template vmm_kind round-trip** (`vmm_template_roundtrip.rs`): create template from FC VM → template's `vmm_kind = firecracker`; instantiate template → child VM has `vmm_kind = firecracker`; instantiating an FC template into a request that overrides to CH returns 400. Same-kind override succeeds.
- **Container/function FC pinning** (`vmm_runtime_pinning.rs`): with `[vmm] default_kind = "cloud_hypervisor"` in TOML, creating a container or invoking a function still routes through the FC path (regression protection for the explicit non-goal). Asserts the agent receives a `vmm_kind = firecracker` payload regardless of cluster default.
- **Reconciler vmm_kind handling** (`vmm_reconciler.rs`): the reconciler reads `vms.vmm_kind` and dispatches the correct driver call. Mixed-kind cluster: 1 FC VM + 1 CH VM, both reconciled in the same loop iteration without crosstalk.
- **`metrics_snapshot` tolerance** (`vmm_metrics_shape.rs`): the manager's metrics ingestion accepts the FC shape (rich JSON) and the CH shape (sparser); UI-bound serialization tolerates missing keys per backend; no panic on unknown fields.
- **`SpecOverrides` on restore** (`vmm_restore_overrides.rs`): override allowed fields (e.g., new TAP name post-restore) are applied; override forbidden fields (e.g., changing `vcpu` count) are rejected per backend.

### Migration tests (`apps/manager/tests/vmm_migration.rs`)

Fixture-based:

- **Pre-migration fixture:** `apps/manager/tests/fixtures/pre_vmm_migration.sql` — loads a representative pre-migration snapshot of `vms`, `snapshots`, `images`, `templates`, `hosts`.
- **Migration applies cleanly:** running `0034_vmm_backends.sql` against the fixture produces no errors.
- **Backfill correctness:** every VM has `vmm_kind = 'firecracker'`, `guest_os = 'linux_kernel'`, `boot_mode` JSONB derived from legacy columns. Every snapshot has `vmm_kind = 'firecracker'`. Every image has `image_kind = 'linux_kernel'`. Every host has `vmm_kinds_installed = '{firecracker}'`. Every template has `vmm_kind = 'firecracker'`.
- **CHECK constraints active:** inserting an `images` row with `image_kind = 'uefi_disk'` and NULL `nvram_template_path` fails with the documented constraint name.
- **Legacy column readability:** `vms.kernel_path`, `vms.initrd_path`, `vms.kernel_cmdline` remain readable; selecting a fixture row returns the legacy values unchanged.
- **Boot from migrated row:** create a `VmSpec` from a fixture-migrated VM row, hand it to `FirecrackerDriver` (against the API mock); the resulting FC API calls are byte-identical to the pre-migration fixture's expected sequence.

### Image scanner tests (`apps/manager/src/features/images/scan.rs#[cfg(test)]`)

- **PVH note detection:** ELF kernel with a PVH note → detected as `Pvh`-capable; ELF without it → detected as not `Pvh`-capable. Uses fixtures in `apps/manager/tests/fixtures/kernels/`.
- **`image_kind` upload selector:** uploading without specifying `image_kind` defaults to `linux_kernel`; uploading with `uefi_disk` and missing `nvram_template_path` returns 400 at the route layer (before the DB constraint fires).

### Console / shell bridge tests (`apps/agent/src/features/vm/shell.rs#[cfg(test)]`)

- **`UnixSerial` dispatch (FC):** WS connection to an FC VM bridges through the existing screen-on-serial path. Refactor invariance: a fixture echo-test (write bytes upstream, read bytes downstream) yields the same byte sequence pre- and post-refactor.
- **`UnixSerial` dispatch (CH-Linux):** WS connection to a CH VM in serial mode bridges through the same path. Different socket layout, same outcome.
- **`Pty` dispatch (CH-PTY):** WS connection to a CH VM in PTY mode bridges through a tty pass-through. Echo round-trip works.
- **`Vnc` / `Rdp` dispatch:** unimplemented variants return HTTP 501 with a body referencing the Windows-PR follow-up.

### Cargo feature build matrix (CI gate)

CI must build and test under each combination:

- `--no-default-features --features vmm-firecracker` (slim agent: FC only).
- `--no-default-features --features vmm-cloud-hypervisor` (slim agent: CH only).
- `--features vmm-firecracker,vmm-cloud-hypervisor` (default).
- `--no-default-features` — should fail to compile with a clear `compile_error!` ("at least one VMM driver feature must be enabled"). Asserted by a `compiletest` or `trybuild` test in `crates/nexus-vmm/tests/`.

For each enabled combination, the agent's inventory probe must report exactly the matching `vmm_kinds_installed`.

### End-to-end tests

Marked `#[ignore]` in `cargo test`; gated to a CI job with the binaries and fixtures available.

- **FC end-to-end** (`apps/manager/tests/e2e_firecracker.rs`): existing suite, unchanged. Verifies no behavioural drift post-refactor. Run on every PR.
- **CH+PVH end-to-end** (`apps/manager/tests/e2e_ch_pvh.rs`): upload PVH-capable kernel, create VM, boot, exec via guest-agent, snapshot, restore, exec again, destroy. Wall-clock boot < 500ms asserted as a soft target (logged, not failed).
- **CH+UEFI end-to-end** (`apps/manager/tests/e2e_ch_uefi.rs`): upload Ubuntu 24.04 cloud image as `uefi_disk` with OVMF reference, create VM, boot, SSH (via TAP + DHCP), snapshot, restore, SSH again, destroy. Wall-clock boot < 2s asserted as a soft target.
- **Mixed-kind cluster** (`apps/manager/tests/e2e_mixed.rs`): one host with FC only, one host with both. Schedule mixes correctly per inventory.

### Agent-restart resilience

- **Rebind FC** (`apps/agent/tests/rebind_fc.rs`): boot FC VM, kill agent SIGKILL, restart agent, assert VM is rebound (handle reconstructed, API socket reachable, console bridge reconnects). Existing FC behaviour preserved.
- **Rebind CH** (`apps/agent/tests/rebind_ch.rs`): same flow for CH.
- **Rebind miss:** kill the VMM process while the agent is dead; on restart, agent's rebind sweep marks the VM as crashed and emits an inventory event. No phantom handles.

### No-regression gates

- `cargo test -p manager` (existing) passes without modification.
- `cargo test -p agent` (existing) passes without modification.
- `cargo test -p nexus-types` passes (existing tests + new fields covered by serde round-trip).
- `cargo test -p nexus-vmm` passes (new crate's tests).
- `cargo fmt --check` passes.
- `cargo clippy --all-targets --all-features -- -D warnings` passes.
- `cargo clippy --all-targets --no-default-features --features vmm-firecracker -- -D warnings` passes (slim build clippy-clean).
- `cargo clippy --all-targets --no-default-features --features vmm-cloud-hypervisor -- -D warnings` passes (slim build clippy-clean).
- `(cd apps/ui && pnpm lint)` passes.
- Existing serverless cold-start benchmark (sub-125ms target on FC) is unchanged. Re-run as part of the PR description's evidence.

### Manual smoke test (documented in PR description)

Reproducible on a single dev host with both VMM binaries installed:

1. Existing FC VM still boots and is reachable. (Auto-verified by integration test; smoked manually as belt-and-braces.)
2. New CH+PVH Linux VM (Ubuntu cloud image with PVH note) boots in <500ms wall-clock.
3. New CH+UEFI Linux VM (Ubuntu cloud image without PVH) boots in <2s wall-clock.
4. WS shell connects to both backends; keystroke echo works; resize signal honoured.
5. Snapshot+restore on CH within a single VMM lifecycle (snapshot, kill, restore, observe correct in-VM state).
6. UI: backend selector defaults to "Auto"; switching to "Cloud Hypervisor" greys out MMDS controls; image-upload form requires NVRAM template when `uefi_disk` is selected; host detail page shows the correct `vmm_kinds_installed` set.
7. Operator removes `cloud-hypervisor` binary from a host and restarts the agent; manager refuses to schedule new CH VMs onto that host (400) but existing CH VMs on that host keep running.

### Coverage map (change-surface ↔ test bucket)

| Change-surface item (§File-level outline) | Covered by |
|---|---|
| `crates/nexus-vmm` trait + types | `nexus-vmm` unit tests |
| `FeatureSupport` table | `nexus-vmm` purity test, manager feature-gating, slim-build matrix |
| `FirecrackerDriver` | per-driver FC tests, refactor-invariance fixture, e2e FC, rebind FC |
| `CloudHypervisorDriver` | per-driver CH tests, e2e CH+PVH, e2e CH+UEFI, rebind CH |
| `VmmRegistry` + dispatch | trait-purity / registry tests |
| Agent `inventory` extension | inventory gating, slim-build matrix, manual smoke step 7 |
| Manager `vms/service.rs` selection | feature-gating, selection, image-kind, default-kind override |
| Manager `snapshots/routes.rs` | snapshot kind gating, e2e snapshot+restore |
| Manager `templates/*` | template vmm_kind round-trip |
| Manager `images/{routes,scan}` | image-kind cross-validation, image scanner PVH detection |
| Manager `hosts/*` | inventory gating, manual smoke step 7 |
| Migration `0034_vmm_backends.sql` | migration tests (fixture, backfill, constraints, legacy readability, boot-from-migrated) |
| Console refactor (`shell.rs`) | console / shell bridge tests, FC e2e (regression), CH e2e |
| Cargo features (`vmm-*`) | build matrix, registry tests for `NotInstalled` |
| Installer (`install-cloud-hypervisor.sh`, `deps.rs`) | manual smoke (operator install/uninstall flow) |
| `nqrust.toml [vmm] default_kind` | selection tests, container/function FC pinning |
| NVRAM lifecycle | per-driver CH NVRAM lifecycle test |
| `SpecOverrides` on restore | manager restore-overrides test |
| `metrics_snapshot` divergence | metrics-shape tolerance test |
| Reconciler | reconciler vmm_kind handling test |
| `VmmError::NotInstalled` (binary-missing) | registry binary-missing test, slim-build matrix |
| FC HTTP-over-UDS proxy (unchanged) | existing `proxy.rs` tests, FC e2e |

If a future change-surface item appears that is not in the left column, the implementer must extend this map and add the corresponding test bucket. The map is the contract.

## Glossary

- **VMM (Virtual Machine Monitor)** — the user-space process that drives `/dev/kvm`, exposes virtual devices to the guest, and handles guest exits. Firecracker and Cloud Hypervisor are both VMMs.
- **`VmmKind`** — discriminator on which VMM is in use for a given VM. Persisted on the VM row.
- **`VmmDriver`** — agent-side trait abstracting VMM operations. One impl per `VmmKind`. Held in `VmmRegistry`.
- **`VmmHandle`** — per-VM runtime state owned by the agent: API socket path, PID, systemd unit name, console socket path. Serialized to disk for crash recovery.
- **`VmSpec`** — VMM-agnostic description of a VM that the agent translates into per-backend API calls. Built on the manager from DB rows.
- **`BootMode`** — discriminated union: `LinuxKernel { kernel, initrd, cmdline }`, `Pvh { kernel, cmdline }`, `Uefi { firmware, nvram_template }`. Persisted as JSONB on the VM row.
- **`GuestOs`** — coarse-grained guest classification: `LinuxKernel`, `LinuxDisk`, `Windows`, `Other`. Used (with `VmmKind`) to compute `FeatureSupport`.
- **`FeatureSupport`** — boolean matrix of capabilities for a `(VmmKind, GuestOs)` tuple. Pure function. Used for manager-side request gating and UI control disable.
- **`ConsoleEndpoint`** — discriminated console transport: `UnixSerial`, `Pty`, `Vnc`, `Rdp`. Returned by `VmmDriver::console_endpoint`.
- **`SnapshotPaths` / `SnapshotMeta`** — opaque-to-manager VMM snapshot artifacts. Manager owns the directory; the driver owns the layout within it. Cross-`VmmKind` restore is forbidden.
- **PVH** — ELF-based Linux boot path used by Cloud Hypervisor. Faster than UEFI for Linux. Requires the kernel to ship a PVH note.
- **OVMF / EDK2** — UEFI firmware implementations. CH's UEFI path loads OVMF.fd at boot.
- **NVRAM template** — UEFI variable store seed file. Each UEFI VM gets a private copy that persists boot order, secure-boot keys, etc.
- **Rebind** — agent operation that reattaches a `VmmHandle` to a still-running VMM process after agent restart. Implemented by each driver per its API socket layout.
- **Cargo feature flag** — compile-time toggle (`vmm-firecracker`, `vmm-cloud-hypervisor`) that controls which drivers are linked into the agent binary. Inventory reports only enabled drivers.
