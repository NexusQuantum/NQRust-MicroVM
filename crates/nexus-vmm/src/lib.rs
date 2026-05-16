//! Shared types and trait for pluggable VMM backends.
//!
//! Imported by both `apps/manager` (for feature gating and API request types)
//! and `apps/agent` (for the trait and the per-VMM impls).
//!
//! Adding a new backend means: implement [`VmmDriver`], add a variant to
//! [`VmmKind`], extend [`features`] with the (kind, guest_os) capability matrix,
//! and register an instance in the agent's `VmmRegistry`.

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Which VMM binary runs a particular VM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum VmmKind {
    /// Firecracker — minimal microVM, no UEFI, kernel-direct boot only.
    /// Used for serverless functions, container-per-VM, microVM workloads.
    Firecracker,
    /// QEMU — full-fat VMM, supports UEFI/BIOS, classic devices, Windows guests,
    /// VNC console. Used for "VM" tier workloads.
    Qemu,
}

impl VmmKind {
    pub fn as_str(self) -> &'static str {
        match self {
            VmmKind::Firecracker => "firecracker",
            VmmKind::Qemu => "qemu",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "firecracker" => Some(VmmKind::Firecracker),
            "qemu" => Some(VmmKind::Qemu),
            _ => None,
        }
    }
}

impl fmt::Display for VmmKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Type of guest OS in broad strokes — used to compute feature support and
/// to pick console transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum GuestOs {
    /// Linux booted via the 64-bit boot protocol (vmlinux + optional initrd).
    /// Firecracker's native mode; QEMU also supports it via `-kernel`.
    LinuxKernel,
    /// Linux booted from a full disk image (UEFI/OVMF or PVH).
    /// QEMU-only in this release.
    LinuxDisk,
    /// Windows. QEMU-only. Requires UEFI boot + virtio-win drivers in guest.
    Windows,
    /// Catch-all for BSD, Haiku, classic-mode niche guests.
    Other,
}

impl GuestOs {
    pub fn as_str(self) -> &'static str {
        match self {
            GuestOs::LinuxKernel => "linux_kernel",
            GuestOs::LinuxDisk => "linux_disk",
            GuestOs::Windows => "windows",
            GuestOs::Other => "other",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "linux_kernel" => Some(GuestOs::LinuxKernel),
            "linux_disk" => Some(GuestOs::LinuxDisk),
            "windows" => Some(GuestOs::Windows),
            "other" => Some(GuestOs::Other),
            _ => None,
        }
    }
}

impl fmt::Display for GuestOs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// How the VM boots. Carries the data needed to assemble the corresponding
/// VMM CLI/API call (kernel + cmdline, firmware path, etc.).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum BootMode {
    /// vmlinux/bzImage + optional initrd. Firecracker native; QEMU via `-kernel`.
    LinuxKernel {
        kernel: PathBuf,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        initrd: Option<PathBuf>,
        #[serde(default)]
        cmdline: String,
    },
    /// PVH ELF kernel. QEMU-only.
    Pvh {
        kernel: PathBuf,
        #[serde(default)]
        cmdline: String,
    },
    /// UEFI firmware boot (OVMF/EDK2). QEMU-only. Required for Windows
    /// and most modern distro cloud images.
    Uefi {
        /// Path to OVMF_CODE.fd (read-only firmware).
        firmware: PathBuf,
        /// Path to OVMF_VARS template; agent will copy this to a per-VM file.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        nvram_template: Option<PathBuf>,
    },
}

impl BootMode {
    pub fn mode_str(&self) -> &'static str {
        match self {
            BootMode::LinuxKernel { .. } => "linux_kernel",
            BootMode::Pvh { .. } => "pvh",
            BootMode::Uefi { .. } => "uefi",
        }
    }
}

/// Image kind — discriminates how the image registry treats an asset.
/// Drives validation: a `LinuxKernel` image cannot pair with `BootMode::Uefi`,
/// a `UefiDisk` image requires an `nvram_template_path`, etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ImageKind {
    /// Kernel + rootfs pair (existing FC use case).
    LinuxKernel,
    /// Bootable Linux disk image (PVH or UEFI-capable).
    LinuxDisk,
    /// Bootable disk image that requires UEFI firmware (Windows, modern Linux).
    UefiDisk,
    /// Installer ISO (Windows Setup, Linux netinst, etc.).
    InstallerIso,
}

impl ImageKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ImageKind::LinuxKernel => "linux_kernel",
            ImageKind::LinuxDisk => "linux_disk",
            ImageKind::UefiDisk => "uefi_disk",
            ImageKind::InstallerIso => "installer_iso",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "linux_kernel" => Some(ImageKind::LinuxKernel),
            "linux_disk" => Some(ImageKind::LinuxDisk),
            "uefi_disk" => Some(ImageKind::UefiDisk),
            "installer_iso" => Some(ImageKind::InstallerIso),
            _ => None,
        }
    }
}

/// Console transport for the WebSocket shell bridge.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ConsoleEndpoint {
    /// Unix domain socket carrying serial bytes. Used by FC and by QEMU
    /// when started with `-serial unix:<path>,server,nowait`.
    UnixSerial { path: PathBuf },
    /// PTY-backed console. QEMU's `-serial pty` output.
    Pty { path: PathBuf },
    /// VNC server. QEMU's `-vnc unix:<sock>` or `-vnc :N`.
    Vnc {
        host: String,
        port: u16,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        password: Option<String>,
    },
}

/// Per-VM disk specification for the VMM to attach.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DiskSpec {
    pub drive_id: String,
    /// Resolved filesystem path (file-backed) or block device path.
    pub source: PathBuf,
    /// If true, attach as virtio-blk read-only (covers ISO attachment too).
    pub read_only: bool,
    /// Whether the guest should treat this as the boot device.
    pub root_device: bool,
    /// On-disk image format hint for QEMU (`raw`, `qcow2`, `iso`).
    /// Firecracker ignores this (raw only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// If true, attach as a CD-ROM device (QEMU virtio-blk readonly with iso).
    #[serde(default)]
    pub cdrom: bool,
}

/// Per-VM NIC specification.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NicSpec {
    pub iface_id: String,
    /// Pre-created TAP device name (e.g., `tapXXXX`).
    pub host_dev: String,
    pub mac: String,
}

/// Full per-VM spec passed to `VmmDriver::boot`. Backend-agnostic.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VmSpec {
    pub id: Uuid,
    pub vcpu: u32,
    pub mem_mib: u32,
    pub boot: BootMode,
    pub disks: Vec<DiskSpec>,
    pub nics: Vec<NicSpec>,
    /// If true, the VMM exposes a VNC console in addition to (or instead of)
    /// the serial UDS. Only honored by backends with `features.vnc_console`.
    #[serde(default)]
    pub enable_vnc: bool,
    /// If true, attach a software TPM 2.0 (swtpm sidecar) to the guest.
    /// Required for Windows 11. Other Windows versions + Linux ignore it.
    #[serde(default)]
    pub enable_tpm: bool,
    /// virtio-balloon device (memory pressure cooperation). QEMU only.
    #[serde(default)]
    pub enable_balloon: bool,
    /// virtio-rng device (entropy source). QEMU only.
    #[serde(default)]
    pub enable_rng: bool,
    /// virtio-vsock device with the given context id. QEMU only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vsock_cid: Option<u32>,
    /// VFIO PCI devices to pass through. Each entry is a host PCI BDF like
    /// "0000:01:00.0". Operator is responsible for unbinding from host drivers
    /// and IOMMU group isolation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub vfio_devices: Vec<String>,
    /// When set, this VM boots in "incoming migration" mode: QEMU starts
    /// paused, listening on the given URI (e.g. `tcp:0.0.0.0:54321`) for an
    /// inbound `migrate` stream. Once the source completes, the guest
    /// resumes automatically. Used for live migration target-side.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub incoming_uri: Option<String>,
    /// Log file path (one combined stderr/stdout log for the VMM process).
    pub log_path: PathBuf,
    /// Run directory — agent-owned per-VM directory for sockets, NVRAM, etc.
    pub run_dir: PathBuf,
}

/// Handle to a running VMM, persisted to disk so the agent can rebind after
/// restart. Always serializable.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VmmHandle {
    pub vm_id: Uuid,
    pub kind: VmmKind,
    /// Path to the VMM's control socket (FC api-sock or QEMU QMP socket).
    pub api_sock: PathBuf,
    /// Best-effort PID. May be stale across agent restarts; use `rebind` to
    /// re-verify liveness.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    /// systemd unit name for the per-VM cgroup scope.
    pub systemd_unit: String,
    /// Serial UDS path if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub console_sock: Option<PathBuf>,
    /// VNC listener if any (host:port).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vnc: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ShutdownMode {
    /// Send an ACPI shutdown / i8042 reset bit. Cooperates with the guest OS.
    Graceful,
    /// Kill the VMM process directly. Equivalent to pulling the power cord.
    Hard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotKind {
    /// Full state + memory dump.
    Full,
    /// Diff snapshot (delta vs a prior full). FC-only in this release.
    Diff,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SnapshotPaths {
    pub state_path: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mem_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SnapshotMeta {
    pub kind: VmmKind,
    pub vmm_version: String,
    pub state_size_bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mem_size_bytes: Option<u64>,
}

/// Capability matrix per (VmmKind, GuestOs). Pure function, computed in code.
/// Manager refuses requests that depend on a feature this backend does not
/// support before contacting any agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct FeatureSupport {
    pub fast_snapshot: bool,
    pub diff_snapshot: bool,
    pub mmds: bool,
    pub uefi_boot: bool,
    pub bios_boot: bool,
    pub virtio_pci: bool,
    pub virtio_console: bool,
    pub vnc_console: bool,
    pub cdrom: bool,
    pub balloon: bool,
    pub entropy: bool,
    pub vsock: bool,
    pub memory_hotplug: bool,
    pub gpu_passthrough: bool,
    pub vfio_passthrough: bool,
    pub live_migration: bool,
    pub windows_guest: bool,
}

impl FeatureSupport {
    /// All-false capability profile. Used as the result for invalid
    /// (kind, guest_os) combinations so the manager's gating check rejects them.
    pub const NONE: FeatureSupport = FeatureSupport {
        fast_snapshot: false,
        diff_snapshot: false,
        mmds: false,
        uefi_boot: false,
        bios_boot: false,
        virtio_pci: false,
        virtio_console: false,
        vnc_console: false,
        cdrom: false,
        balloon: false,
        entropy: false,
        vsock: false,
        memory_hotplug: false,
        gpu_passthrough: false,
        vfio_passthrough: false,
        live_migration: false,
        windows_guest: false,
    };
}

/// Compute the capability matrix for a backend × guest_os combo.
/// Single source of truth; manager and UI consult this.
pub fn features(kind: VmmKind, guest: GuestOs) -> FeatureSupport {
    match (kind, guest) {
        (VmmKind::Firecracker, GuestOs::LinuxKernel) => FeatureSupport {
            fast_snapshot: true,
            diff_snapshot: true,
            mmds: true,
            uefi_boot: false,
            bios_boot: false,
            virtio_pci: false,
            virtio_console: false,
            vnc_console: false,
            cdrom: false,
            balloon: true,
            entropy: true,
            vsock: true,
            memory_hotplug: false,
            gpu_passthrough: false,
            vfio_passthrough: false,
            live_migration: false,
            windows_guest: false,
        },
        (VmmKind::Firecracker, _) => FeatureSupport::NONE,
        (VmmKind::Qemu, GuestOs::LinuxKernel | GuestOs::LinuxDisk) => FeatureSupport {
            fast_snapshot: true,
            diff_snapshot: false,
            mmds: false,
            uefi_boot: true,
            bios_boot: true,
            virtio_pci: true,
            virtio_console: true,
            vnc_console: true,
            cdrom: true,
            balloon: true,
            entropy: true,
            vsock: true,
            memory_hotplug: false,
            gpu_passthrough: false,
            vfio_passthrough: true,
            live_migration: false,
            windows_guest: false,
        },
        (VmmKind::Qemu, GuestOs::Windows) => FeatureSupport {
            fast_snapshot: true,
            diff_snapshot: false,
            mmds: false,
            uefi_boot: true,
            bios_boot: true,
            virtio_pci: true,
            virtio_console: true,
            vnc_console: true,
            cdrom: true,
            balloon: false,
            entropy: true,
            vsock: false,
            memory_hotplug: false,
            gpu_passthrough: false,
            vfio_passthrough: true,
            live_migration: false,
            windows_guest: true,
        },
        (VmmKind::Qemu, GuestOs::Other) => FeatureSupport {
            fast_snapshot: false,
            diff_snapshot: false,
            mmds: false,
            uefi_boot: true,
            bios_boot: true,
            virtio_pci: true,
            virtio_console: true,
            vnc_console: true,
            cdrom: true,
            balloon: false,
            entropy: true,
            vsock: false,
            memory_hotplug: false,
            gpu_passthrough: false,
            vfio_passthrough: true,
            live_migration: false,
            windows_guest: false,
        },
    }
}

/// Auto-select a backend given only the boot mode. Used when the caller did
/// not specify `vmm_kind` explicitly. Manager rejects explicit
/// `(vmm_kind, boot_mode)` combinations that conflict with this routing.
pub fn auto_select(boot: &BootMode) -> VmmKind {
    match boot {
        BootMode::LinuxKernel { .. } => VmmKind::Firecracker,
        BootMode::Pvh { .. } | BootMode::Uefi { .. } => VmmKind::Qemu,
    }
}

#[derive(Debug, thiserror::Error)]
pub enum VmmError {
    #[error("VMM not installed on host: {0}")]
    NotInstalled(String),
    #[error("feature {feature} not supported by {kind}")]
    NotSupported { kind: VmmKind, feature: String },
    #[error("API socket timeout for vm {vm_id}")]
    SocketTimeout { vm_id: Uuid },
    #[error("VMM process exited unexpectedly: {0}")]
    ProcessGone(String),
    #[error("snapshot kind mismatch: expected {expected}, found {found}")]
    SnapshotKindMismatch { expected: VmmKind, found: VmmKind },
    #[error("invalid spec: {0}")]
    InvalidSpec(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// The pluggable VMM backend trait. Implementations live in `apps/agent/src/vmm/`.
///
/// Impls are stateless — per-VM state lives in [`VmmHandle`] (which is
/// serializable to disk for crash recovery) and in the VMM process itself.
/// The agent's `VmmRegistry` holds one `Arc<dyn VmmDriver>` per kind.
#[async_trait]
pub trait VmmDriver: Send + Sync {
    fn kind(&self) -> VmmKind;

    /// Pure capability lookup. Default impl delegates to [`features`].
    fn features(&self, guest: GuestOs) -> FeatureSupport {
        features(self.kind(), guest)
    }

    /// Probe whether this backend is actually installed on the host
    /// (binary present, version >= minimum). Called at agent startup
    /// and on each heartbeat.
    async fn probe(&self) -> Result<String, VmmError>;

    /// Combined launch + configure + start. Backends with multi-stage
    /// state machines (FC) implement this internally. Returns a handle
    /// the agent persists per-VM.
    async fn boot(&self, spec: &VmSpec) -> Result<VmmHandle, VmmError>;

    /// Graceful or hard shutdown.
    async fn shutdown(&self, handle: &VmmHandle, mode: ShutdownMode) -> Result<(), VmmError>;

    async fn pause(&self, handle: &VmmHandle) -> Result<(), VmmError>;
    async fn resume(&self, handle: &VmmHandle) -> Result<(), VmmError>;

    /// Release all resources (process, sockets, scope). After this returns Ok,
    /// the handle is invalid.
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
        spec: &VmSpec,
    ) -> Result<VmmHandle, VmmError>;

    /// Re-attach to a still-running VMM after agent restart.
    /// Returns Ok(Some) when a live process is found,
    /// Ok(None) when no process is running for this id,
    /// Err on filesystem/permission failures.
    async fn rebind(&self, run_dir: &Path, vm_id: Uuid) -> Result<Option<VmmHandle>, VmmError>;

    /// Console transport for the WebSocket shell bridge.
    async fn console_endpoint(&self, handle: &VmmHandle) -> Result<ConsoleEndpoint, VmmError>;

    /// Optional best-effort metrics (cpu time, memory, exit code). May return
    /// an empty map if the backend has no introspection.
    async fn metrics_snapshot(
        &self,
        _handle: &VmmHandle,
    ) -> Result<HashMap<String, serde_json::Value>, VmmError> {
        Ok(HashMap::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vmm_kind_round_trips() {
        for s in ["firecracker", "qemu"] {
            let k = VmmKind::parse(s).unwrap();
            assert_eq!(k.as_str(), s);
        }
        assert!(VmmKind::parse("nope").is_none());
    }

    #[test]
    fn guest_os_round_trips() {
        for s in ["linux_kernel", "linux_disk", "windows", "other"] {
            let g = GuestOs::parse(s).unwrap();
            assert_eq!(g.as_str(), s);
        }
    }

    #[test]
    fn fc_supports_only_linux_kernel() {
        assert!(features(VmmKind::Firecracker, GuestOs::LinuxKernel).mmds);
        for g in [GuestOs::LinuxDisk, GuestOs::Windows, GuestOs::Other] {
            assert_eq!(features(VmmKind::Firecracker, g), FeatureSupport::NONE);
        }
    }

    #[test]
    fn qemu_supports_uefi_and_windows() {
        let f = features(VmmKind::Qemu, GuestOs::LinuxDisk);
        assert!(f.uefi_boot);
        assert!(f.vnc_console);
        assert!(f.cdrom);

        let w = features(VmmKind::Qemu, GuestOs::Windows);
        assert!(w.windows_guest, "windows_guest should be enabled in 0.5.0");
        assert!(w.uefi_boot);
        assert!(!w.balloon, "qemu+windows excludes virtio-balloon by policy");
    }

    #[test]
    fn auto_select_linux_kernel_to_fc() {
        let bm = BootMode::LinuxKernel {
            kernel: PathBuf::from("/vmlinux"),
            initrd: None,
            cmdline: String::new(),
        };
        assert_eq!(auto_select(&bm), VmmKind::Firecracker);
    }

    #[test]
    fn auto_select_uefi_to_qemu() {
        let bm = BootMode::Uefi {
            firmware: PathBuf::from("/OVMF.fd"),
            nvram_template: None,
        };
        assert_eq!(auto_select(&bm), VmmKind::Qemu);
    }

    #[test]
    fn boot_mode_round_trips_json() {
        let bm = BootMode::Uefi {
            firmware: PathBuf::from("/usr/share/edk2/x64/OVMF_CODE.4m.fd"),
            nvram_template: Some(PathBuf::from("/usr/share/edk2/x64/OVMF_VARS.4m.fd")),
        };
        let j = serde_json::to_value(&bm).unwrap();
        assert_eq!(j["mode"], "uefi");
        let back: BootMode = serde_json::from_value(j).unwrap();
        matches!(back, BootMode::Uefi { .. });
    }

    #[test]
    fn image_kind_round_trips() {
        for s in ["linux_kernel", "linux_disk", "uefi_disk", "installer_iso"] {
            let k = ImageKind::parse(s).unwrap();
            assert_eq!(k.as_str(), s);
        }
    }
}
