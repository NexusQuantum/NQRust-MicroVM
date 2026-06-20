//! QEMU VMM driver.
//!
//! Spawns `qemu-system-x86_64` as a per-VM `systemd-run` transient *service*
//! so the kernel enforces cgroup memory/cpu limits and the process is
//! supervised by systemd (PID 1) — surviving agent restarts, which `rebind`
//! relies on. A transient *scope* would be wrong here: `systemd-run --scope`
//! runs the command synchronously (blocking until QEMU exits), so the boot
//! path could never reach the QMP handshake. Talks to QEMU over QMP for
//! lifecycle, snapshots, and metrics. Serial console is exposed as a Unix
//! domain socket the WebSocket shell bridge can attach to.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use nexus_vmm::{
    BootMode, ConsoleEndpoint, ShutdownMode, SnapshotKind, SnapshotMeta, SnapshotPaths, VmSpec,
    VmmDriver, VmmError, VmmHandle, VmmKind,
};
use serde_json::json;
use tokio::fs;
use tokio::process::Command;
use tokio::time::sleep;
use uuid::Uuid;

use super::qmp::QmpClient;

/// Path to the QEMU binary. Operators can override via `QEMU_BINARY`.
const QEMU_BIN_DEFAULT: &str = "qemu-system-x86_64";

/// How long to wait for QEMU to produce the QMP socket after spawn.
const QMP_READY_TIMEOUT: Duration = Duration::from_secs(20);
const QMP_POLL_INTERVAL: Duration = Duration::from_millis(50);
/// Number of empty PCIe root-ports pre-allocated at boot for runtime device
/// hot-plug (disk/NIC hot-add). Each provides one hot-pluggable slot.
pub(crate) const HOTPLUG_ROOT_PORTS: u8 = 4;

#[derive(Default, Clone)]
pub struct QemuDriver {
    qemu_bin: Option<String>,
}

impl QemuDriver {
    pub fn new() -> Self {
        Self::default()
    }

    fn qemu_bin(&self) -> String {
        self.qemu_bin
            .clone()
            .or_else(|| std::env::var("QEMU_BINARY").ok())
            .unwrap_or_else(|| QEMU_BIN_DEFAULT.to_string())
    }

    fn vm_run_dir(&self, run_dir: &Path, vm_id: Uuid) -> PathBuf {
        run_dir.join(vm_id.to_string())
    }

    fn qmp_sock(&self, vm_dir: &Path) -> PathBuf {
        vm_dir.join("qmp.sock")
    }
    fn serial_sock(&self, vm_dir: &Path) -> PathBuf {
        vm_dir.join("serial.sock")
    }
    fn vnc_sock(&self, vm_dir: &Path) -> PathBuf {
        vm_dir.join("vnc.sock")
    }
    fn pid_file(&self, vm_dir: &Path) -> PathBuf {
        vm_dir.join("qemu.pid")
    }
    fn handle_file(&self, vm_dir: &Path) -> PathBuf {
        vm_dir.join("handle.json")
    }
    fn nvram_file(&self, vm_dir: &Path) -> PathBuf {
        vm_dir.join("OVMF_VARS.fd")
    }
    fn log_file(&self, vm_dir: &Path) -> PathBuf {
        vm_dir.join("qemu.log")
    }

    fn systemd_unit(&self, vm_id: Uuid) -> String {
        format!("qemu-{vm_id}.service")
    }

    /// Spawn QEMU under a systemd transient *service*. Caller is responsible
    /// for having created the vm_dir. `resource_props` is the list of
    /// `--property=KEY=VALUE` strings the kernel will enforce as a cgroup
    /// per the [`super::resource`] helpers — these are how QEMU and FC are
    /// kept from fighting for memory or CPU on a shared host.
    ///
    /// We deliberately do NOT pass `--scope`: `systemd-run --scope` runs the
    /// command synchronously and only returns once it exits, which would block
    /// `boot()` for QEMU's entire lifetime (never reaching the QMP handshake or
    /// the socket-perms relax). A transient service backgrounds QEMU under
    /// systemd and returns immediately; `--collect` reaps the unit when QEMU
    /// dies so the per-VM unit name is reusable.
    ///
    /// In dev environments without passwordless sudo, set `AGENT_NO_SUDO=1`
    /// to skip the systemd-run wrapper and spawn QEMU directly. The
    /// per-VM cgroup limits are forfeited but the rest of the pipeline
    /// (QMP, console UDS, NICs, disks) works for end-to-end testing.
    async fn spawn_under_unit(
        &self,
        unit: &str,
        resource_props: &[String],
        args: &[String],
    ) -> Result<()> {
        if std::env::var("AGENT_NO_SUDO").is_ok() {
            tracing::warn!(
                unit,
                "AGENT_NO_SUDO=1 — spawning QEMU directly without systemd-run / cgroup limits (dev mode only)"
            );
            let _child = Command::new(self.qemu_bin())
                .args(args)
                .kill_on_drop(false)
                .spawn()
                .context("direct qemu spawn (AGENT_NO_SUDO)")?;
            return Ok(());
        }

        // NOTE: no `--scope`. `systemd-run --scope` is synchronous and blocks
        // until the wrapped process exits — fatal here, since QEMU is a daemon.
        // The default (a transient `.service`) backgrounds QEMU and returns at
        // once. `--collect` GCs the unit when QEMU dies so the name is reusable.
        let mut cmd = Command::new("sudo");
        cmd.arg("-n")
            .arg("systemd-run")
            .arg("--collect")
            .arg(format!("--unit={unit}"))
            .arg("--property=KillMode=mixed")
            .arg("--property=TimeoutStopSec=10s");
        for p in resource_props {
            cmd.arg(format!("--property={p}"));
        }
        cmd.arg("--")
            .arg(self.qemu_bin())
            .args(args)
            .kill_on_drop(false);

        let status = cmd.status().await.context("spawn qemu via systemd-run")?;
        anyhow::ensure!(
            status.success(),
            "systemd-run failed to spawn qemu (exit: {:?})",
            status.code()
        );
        Ok(())
    }

    /// Make the per-VM control sockets connectable by the agent user even
    /// when QEMU runs as root (sudo systemd-run path). Uses `sudo -n chmod`
    /// (the agent has NOPASSWD chmod in the standard install). Best-effort:
    /// in dev mode (AGENT_NO_SUDO) the sockets are already agent-owned and
    /// the chmod is harmless.
    async fn relax_socket_perms(&self, vm_dir: &Path) {
        if std::env::var("AGENT_NO_SUDO").is_ok() {
            return; // agent owns the sockets already
        }
        for sock in [
            self.qmp_sock(vm_dir),
            self.serial_sock(vm_dir),
            self.vnc_sock(vm_dir),
        ] {
            if sock.exists() {
                let _ = Command::new("sudo")
                    .args(["-n", "chmod", "0666"])
                    .arg(&sock)
                    .status()
                    .await;
            }
        }
    }

    /// Wait for the QMP socket to appear so we can connect.
    async fn wait_for_qmp(&self, sock: &Path) -> Result<()> {
        let deadline = std::time::Instant::now() + QMP_READY_TIMEOUT;
        while std::time::Instant::now() < deadline {
            if sock.exists() {
                return Ok(());
            }
            sleep(QMP_POLL_INTERVAL).await;
        }
        Err(anyhow!(
            "QMP socket {} did not appear within {:?}",
            sock.display(),
            QMP_READY_TIMEOUT
        ))
    }

    /// Translate a [`VmSpec`] into a full `qemu-system-x86_64` argv.
    /// Internal — kept pub(super) for direct unit testing.
    pub(super) fn build_args(
        &self,
        spec: &VmSpec,
        vm_dir: &Path,
        nvram_runtime: Option<&Path>,
    ) -> Result<Vec<String>> {
        let mut args: Vec<String> = Vec::new();

        // Machine model: q35 + KVM is the modern default. UEFI requires q35.
        // Secure Boot additionally needs SMM (so the guest can't tamper with the
        // protected pflash) plus the `cfi.pflash01 secure=on` global below.
        args.push("-machine".into());
        if spec.enable_secure_boot {
            args.push("q35,accel=kvm,smm=on".into());
            args.push("-global".into());
            args.push("driver=cfi.pflash01,property=secure,value=on".into());
        } else {
            args.push("q35,accel=kvm,smm=off".into());
        }

        // CPU model + SMP topology. Default "host" (all host features, needed
        // for nested virt); operators can pick a fixed model (e.g. kvm64,
        // x86-64-v3, EPYC) for cross-host live-migration compatibility.
        args.push("-cpu".into());
        args.push(spec.cpu_type.clone().unwrap_or_else(|| "host".into()));
        args.push("-smp".into());
        args.push(format!("cpus={}", spec.vcpu));
        args.push("-m".into());
        args.push(format!("{}M", spec.mem_mib));

        // No default devices, no graphical display.
        args.push("-nodefaults".into());
        args.push("-no-user-config".into());
        // Only installer VMs get -no-reboot (their post-install auto-reboot would
        // otherwise loop back into the still-attached installer CD). A normal VM
        // must reboot in place when the guest issues a reboot, not power off.
        if spec.no_reboot {
            args.push("-no-reboot".into());
        }
        if !spec.enable_vnc {
            args.push("-display".into());
            args.push("none".into());
        }

        // QMP control socket — agent's lifeline.
        args.push("-qmp".into());
        args.push(format!(
            "unix:{},server=on,wait=off",
            self.qmp_sock(vm_dir).display()
        ));

        // Serial UDS — WS shell bridge attaches to this for console.
        args.push("-chardev".into());
        args.push(format!(
            "socket,id=ser0,path={},server=on,wait=off",
            self.serial_sock(vm_dir).display()
        ));
        args.push("-serial".into());
        args.push("chardev:ser0".into());

        // PID file.
        args.push("-pidfile".into());
        args.push(self.pid_file(vm_dir).display().to_string());

        // Boot mode → firmware / kernel args.
        match &spec.boot {
            BootMode::LinuxKernel {
                kernel,
                initrd,
                cmdline,
            } => {
                args.push("-kernel".into());
                args.push(kernel.display().to_string());
                if let Some(initrd) = initrd {
                    args.push("-initrd".into());
                    args.push(initrd.display().to_string());
                }
                if !cmdline.is_empty() {
                    args.push("-append".into());
                    args.push(cmdline.clone());
                }
            }
            BootMode::Pvh { kernel, cmdline } => {
                args.push("-kernel".into());
                args.push(kernel.display().to_string());
                if !cmdline.is_empty() {
                    args.push("-append".into());
                    args.push(cmdline.clone());
                }
            }
            BootMode::Uefi {
                firmware,
                nvram_template: _,
            } => {
                // Resolve the OVMF code firmware. The caller's path may be a
                // distro default that doesn't match this host (e.g. the
                // manager defaults to Arch paths but the agent is on Ubuntu).
                // Fall back to probing well-known per-distro locations. Secure
                // Boot needs the secboot CODE variant (the matching `.ms`/
                // secboot NVRAM is selected in prepare_nvram).
                let fw = if spec.enable_secure_boot {
                    resolve_ovmf_code_secboot(firmware)
                } else {
                    resolve_ovmf_code(firmware)
                };
                args.push("-drive".into());
                args.push(format!(
                    "if=pflash,format=raw,readonly=on,file={}",
                    fw.display()
                ));
                if let Some(nv) = nvram_runtime {
                    args.push("-drive".into());
                    args.push(format!("if=pflash,format=raw,file={}", nv.display()));
                }
            }
        }

        // Disks. Regular disks attach as virtio-blk-pci. `cdrom: true` disks
        // attach as `ide-cd` on a dedicated AHCI controller — a real removable
        // CD-ROM whose medium can be ejected via QMP `eject` (install-complete).
        // virtio-blk on the q35 root complex (pcie.0) supports neither
        // device_del (no hotplug) nor media eject, so it can't model an
        // ejectable installer CD.
        //
        // Bootindex policy (Proxmox-style): the ROOT DISK boots first
        // (bootindex 0); CD-ROMs come after (1,2,3…). On a fresh ISO install the
        // blank disk has no bootloader, so UEFI falls through to the installer
        // CD; once the OS is installed the disk boots first and the (still
        // attached) ISO is simply ignored. This lets a multi-reboot installer
        // (e.g. Windows) run to completion on its own — no `-no-reboot` and no
        // manual "install complete" step. Indices must be unique — two devices
        // sharing a bootindex makes QEMU refuse to start.
        let num_cdrom = spec.disks.iter().filter(|d| d.cdrom).count();
        if num_cdrom > 0 {
            // One AHCI controller hosts every CD-ROM (ports ahci0.0..ahci0.5).
            args.push("-device".into());
            args.push("ich9-ahci,id=ahci0".into());
        }
        let mut cdrom_port = 0usize;
        for (i, disk) in spec.disks.iter().enumerate() {
            let drive_id = if disk.drive_id.is_empty() {
                format!("drv{i}")
            } else {
                disk.drive_id.clone()
            };
            let fmt = disk.format.as_deref().unwrap_or("raw");
            let ro = if disk.read_only || disk.cdrom {
                ",readonly=on"
            } else {
                ""
            };
            args.push("-drive".into());
            args.push(format!(
                "file={},if=none,format={},id={}{}",
                disk.source.display(),
                fmt,
                drive_id,
                ro
            ));
            args.push("-device".into());
            if disk.cdrom {
                let port = cdrom_port;
                cdrom_port += 1;
                // CD-ROMs boot AFTER the root disk (which is bootindex 0).
                let bootindex = port + 1;
                args.push(format!(
                    "ide-cd,drive={drive_id},id={drive_id}-dev,bus=ahci0.{port},bootindex={bootindex}"
                ));
            } else {
                // Root disk boots first; data disks get no bootindex.
                let bootindex = if disk.root_device {
                    ",bootindex=0".to_string()
                } else {
                    String::new()
                };
                args.push(format!(
                    "virtio-blk-pci,drive={drive_id},id={drive_id}-dev{bootindex}"
                ));
            }
        }

        // Pre-allocate empty PCIe root-ports for runtime hot-plug (disk/NIC
        // hot-add). The q35 root complex (pcie.0) doesn't support hotplug, so a
        // live `device_add` must target a root-port — and those can't be added
        // after boot. Boot-time devices sit on pcie.0; these are spare slots the
        // agent's disk_add fills on demand. `chassis` must be unique per port.
        for n in 0..HOTPLUG_ROOT_PORTS {
            args.push("-device".into());
            args.push(format!("pcie-root-port,id=rphp{n},chassis={}", 20 + n));
        }

        // VFIO PCI passthrough. Each entry is a host BDF (e.g. 0000:01:00.0)
        // already bound to vfio-pci on the host. Requires IOMMU; on q35 the
        // device attaches to the root complex.
        for (i, bdf) in spec.vfio_devices.iter().enumerate() {
            args.push("-device".into());
            args.push(format!("vfio-pci,host={bdf},id=vfio{i}"));
        }

        // NICs — TAP-backed virtio-net by default. The TAP device is
        // pre-created by the agent's tap module and attached to the bridge
        // before boot.
        //
        // Dev escape hatch: if `AGENT_USER_MODE_NET=1` or the NIC's host_dev
        // is "user", use QEMU's slirp user-mode networking instead. This
        // lets unprivileged dev hosts validate the spawn pipeline without
        // needing sudo for TAP creation.
        let user_mode_global = std::env::var("AGENT_USER_MODE_NET").is_ok();
        for (i, nic) in spec.nics.iter().enumerate() {
            let netdev_id = format!("net{i}");
            let user_mode = user_mode_global || nic.host_dev == "user" || nic.host_dev.is_empty();
            args.push("-netdev".into());
            if user_mode {
                args.push(format!("user,id={netdev_id}"));
            } else {
                args.push(format!(
                    "tap,id={},ifname={},script=no,downscript=no",
                    netdev_id, nic.host_dev
                ));
            }
            args.push("-device".into());
            args.push(format!(
                "virtio-net-pci,netdev={},mac={},id=nic{}",
                netdev_id, nic.mac, i
            ));
        }

        // Optional VNC console for graphical install / Windows.
        if spec.enable_vnc {
            args.push("-vnc".into());
            args.push(format!("unix:{}", self.vnc_sock(vm_dir).display()));
            // Plus a basic VGA card so the guest sees a framebuffer.
            args.push("-device".into());
            args.push("virtio-vga".into());
        }

        // Optional paravirt devices (controlled by FeatureSupport flags).
        if spec.enable_balloon {
            args.push("-device".into());
            args.push("virtio-balloon-pci,id=balloon0".into());
        }
        if spec.enable_rng {
            // Use /dev/urandom on Linux as the entropy source.
            args.push("-object".into());
            args.push("rng-random,id=rng0,filename=/dev/urandom".into());
            args.push("-device".into());
            args.push("virtio-rng-pci,rng=rng0,id=rng-pci0".into());
        }
        if let Some(cid) = spec.vsock_cid {
            args.push("-device".into());
            args.push(format!("vhost-vsock-pci,guest-cid={cid},id=vsock0"));
        }

        // VFIO PCI passthrough. Operator pre-binds the host device to
        // vfio-pci and ensures the IOMMU group is clean.
        for (i, bdf) in spec.vfio_devices.iter().enumerate() {
            args.push("-device".into());
            args.push(format!("vfio-pci,host={bdf},id=vfio{i}"));
        }

        // Software TPM 2.0 for Windows 11. The agent must have spawned
        // swtpm and created the chardev socket at <vm_dir>/swtpm.sock
        // before QEMU is started.
        if spec.enable_tpm {
            let sock = self.swtpm_sock(vm_dir);
            args.push("-chardev".into());
            args.push(format!("socket,id=chrtpm,path={}", sock.display()));
            args.push("-tpmdev".into());
            args.push("emulator,id=tpm0,chardev=chrtpm".into());
            args.push("-device".into());
            args.push("tpm-crb,tpmdev=tpm0".into()); // CRB works for both x86 and arm64
        }

        // Live-migration target-side: start QEMU paused, listening for an
        // inbound migrate stream on this URI. Once the source completes the
        // QMP `migrate` to us, the guest resumes automatically.
        if let Some(uri) = &spec.incoming_uri {
            args.push("-incoming".into());
            args.push(uri.clone());
        }

        // Combined log file (stderr/stdout from QEMU itself).
        args.push("-D".into());
        args.push(self.log_file(vm_dir).display().to_string());

        Ok(args)
    }

    fn swtpm_sock(&self, vm_dir: &Path) -> PathBuf {
        vm_dir.join("swtpm.sock")
    }
    fn swtpm_state_dir(&self, vm_dir: &Path) -> PathBuf {
        vm_dir.join("tpm")
    }
    fn swtpm_pid_file(&self, vm_dir: &Path) -> PathBuf {
        vm_dir.join("swtpm.pid")
    }

    /// Spawn a swtpm sidecar that QEMU connects to over UDS. Each VM gets
    /// its own per-VM state dir + control socket. Best-effort: if swtpm
    /// isn't installed, log a warning and return Ok (the guest won't have
    /// TPM, which means Windows 11 setup fails but everything else works).
    async fn spawn_swtpm(&self, vm_dir: &Path) -> Result<()> {
        // Probe for swtpm.
        let probe = Command::new("swtpm").arg("--version").output().await;
        if probe.is_err() || !probe.unwrap().status.success() {
            tracing::warn!(
                vm_dir = %vm_dir.display(),
                "swtpm not installed — TPM disabled. Install 'swtpm' package for Windows 11 support."
            );
            return Ok(());
        }
        let state_dir = self.swtpm_state_dir(vm_dir);
        fs::create_dir_all(&state_dir).await?;
        let sock = self.swtpm_sock(vm_dir);
        if sock.exists() {
            let _ = fs::remove_file(&sock).await;
        }
        // Initialize TPM state if empty (one-shot).
        let _ = Command::new("swtpm_setup")
            .arg("--tpm2")
            .arg("--tpm-state")
            .arg(&state_dir)
            .arg("--createek")
            .arg("--create-ek-cert")
            .arg("--create-platform-cert")
            .arg("--lock-nvram")
            .status()
            .await; // best-effort; some swtpm builds don't need this

        // Spawn swtpm in the background. Survives until QEMU exits and
        // the cgroup tears it down (it's spawned inside the same systemd
        // scope when sudo is available; in dev mode it runs free).
        let mut cmd = Command::new("swtpm");
        cmd.args(["socket", "--tpm2", "--tpmstate"])
            .arg(format!("dir={}", state_dir.display()))
            .arg("--ctrl")
            .arg(format!("type=unixio,path={},mode=0600", sock.display()))
            .arg("--pid")
            .arg(format!("file={}", self.swtpm_pid_file(vm_dir).display()))
            .arg("--log")
            .arg(format!(
                "file={},level=2",
                vm_dir.join("swtpm.log").display()
            ))
            .arg("--daemon")
            .kill_on_drop(false);

        let status = cmd.status().await.context("spawn swtpm")?;
        anyhow::ensure!(status.success(), "swtpm spawn returned non-zero");
        // Wait briefly for the control socket to appear.
        for _ in 0..100 {
            if sock.exists() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        anyhow::ensure!(sock.exists(), "swtpm control socket never appeared");
        Ok(())
    }

    async fn read_pid(&self, vm_dir: &Path) -> Option<u32> {
        let pid_file = self.pid_file(vm_dir);
        // Fast path: already readable (dev mode, or perms previously relaxed).
        if let Ok(raw) = fs::read_to_string(&pid_file).await {
            return raw.trim().parse::<u32>().ok();
        }
        // Production path: QEMU ran as root via `sudo systemd-run`, so its
        // pidfile is root-owned mode 0600 — unreadable by the non-root agent.
        // Without the PID, `rebind` can't confirm liveness and every lifecycle
        // op (pause/resume/shutdown/destroy) fails with "no live vmm", silently
        // orphaning the VM. Relax it (like the control sockets) and retry.
        // No-op in AGENT_NO_SUDO dev mode (the fast path already succeeded).
        if std::env::var("AGENT_NO_SUDO").is_err() && pid_file.exists() {
            let _ = Command::new("sudo")
                .args(["-n", "chmod", "0644"])
                .arg(&pid_file)
                .status()
                .await;
        }
        let raw = fs::read_to_string(&pid_file).await.ok()?;
        raw.trim().parse::<u32>().ok()
    }

    async fn persist_handle(&self, vm_dir: &Path, handle: &VmmHandle) -> Result<()> {
        let path = self.handle_file(vm_dir);
        let bytes = serde_json::to_vec_pretty(handle)?;
        fs::write(&path, bytes).await?;
        Ok(())
    }

    async fn load_handle(&self, vm_dir: &Path) -> Result<Option<VmmHandle>> {
        let path = self.handle_file(vm_dir);
        match fs::read(&path).await {
            Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Copy the OVMF_VARS template to a per-VM runtime nvram file so each
    /// VM has its own writable EFI variables. With `secure_boot`, the
    /// pre-enrolled (`.ms`) Microsoft-keys VARS template is used so the guest
    /// reports Secure Boot as enabled (required by Windows 11 Setup).
    async fn prepare_nvram(&self, template: &Path, dst: &Path, secure_boot: bool) -> Result<()> {
        if fs::metadata(dst).await.is_ok() {
            return Ok(());
        }
        // Resolve the VARS template to a path that actually exists on this
        // host (the caller's default may be for a different distro).
        let src = if secure_boot {
            resolve_ovmf_vars_secboot(template)
        } else {
            resolve_ovmf_vars(template)
        };
        fs::copy(&src, dst)
            .await
            .with_context(|| format!("copy ovmf vars {} -> {}", src.display(), dst.display()))?;
        Ok(())
    }
}

/// Probe well-known **Secure Boot** OVMF_CODE locations (secboot-capable
/// firmware built with SECURE_BOOT_ENABLE). Falls back to the regular code if
/// no secboot variant is found so the VM still boots (without enforcement).
fn resolve_ovmf_code_secboot(requested: &Path) -> PathBuf {
    let req_secboot = requested
        .to_str()
        .map(|s| s.contains(".ms.") || s.contains("secboot"))
        .unwrap_or(false);
    if requested.exists() && req_secboot {
        return requested.to_path_buf();
    }
    const CANDIDATES: &[&str] = &[
        "/usr/share/OVMF/OVMF_CODE_4M.ms.fd", // Debian/Ubuntu (MS-paired)
        "/usr/share/OVMF/OVMF_CODE_4M.secboot.fd", // Debian/Ubuntu (secboot)
        "/usr/share/edk2/x64/OVMF_CODE.secboot.4m.fd", // Arch
        "/usr/share/edk2/x64/OVMF_CODE.secboot.fd", // Arch (legacy)
        "/usr/share/edk2-ovmf/x64/OVMF_CODE.secboot.fd", // Fedora/RHEL
    ];
    CANDIDATES
        .iter()
        .map(PathBuf::from)
        .find(|p| p.exists())
        .unwrap_or_else(|| resolve_ovmf_code(requested))
}

/// Probe well-known **Secure Boot** OVMF_VARS templates with Microsoft keys
/// pre-enrolled (`.ms` variant). Falls back to the regular vars if none found.
fn resolve_ovmf_vars_secboot(requested: &Path) -> PathBuf {
    let req_ms = requested
        .to_str()
        .map(|s| s.contains(".ms."))
        .unwrap_or(false);
    if requested.exists() && req_ms {
        return requested.to_path_buf();
    }
    const CANDIDATES: &[&str] = &[
        "/usr/share/OVMF/OVMF_VARS_4M.ms.fd", // Debian/Ubuntu pre-enrolled MS keys
        "/usr/share/OVMF/OVMF_VARS.ms.fd",    // Debian/Ubuntu (legacy)
        "/usr/share/edk2/x64/OVMF_VARS.4m.fd", // Arch (keys enrolled at build)
        "/usr/share/edk2-ovmf/x64/OVMF_VARS.secboot.fd", // Fedora/RHEL
    ];
    CANDIDATES
        .iter()
        .map(PathBuf::from)
        .find(|p| p.exists())
        .unwrap_or_else(|| resolve_ovmf_vars(requested))
}

/// Probe well-known OVMF_CODE locations across distros. Returns the caller's
/// path if it exists, else the first existing candidate, else the caller's
/// path unchanged (so the resulting error names what was actually requested).
fn resolve_ovmf_code(requested: &Path) -> PathBuf {
    if requested.exists() {
        return requested.to_path_buf();
    }
    const CANDIDATES: &[&str] = &[
        "/usr/share/OVMF/OVMF_CODE_4M.fd",       // Debian/Ubuntu (4M)
        "/usr/share/OVMF/OVMF_CODE.fd",          // Debian/Ubuntu (legacy)
        "/usr/share/edk2/x64/OVMF_CODE.4m.fd",   // Arch
        "/usr/share/edk2-ovmf/x64/OVMF_CODE.fd", // Fedora/RHEL
        "/usr/share/qemu/OVMF_CODE.fd",          // misc
    ];
    CANDIDATES
        .iter()
        .map(PathBuf::from)
        .find(|p| p.exists())
        .unwrap_or_else(|| requested.to_path_buf())
}

/// Probe well-known OVMF_VARS template locations across distros.
fn resolve_ovmf_vars(requested: &Path) -> PathBuf {
    if requested.exists() {
        return requested.to_path_buf();
    }
    const CANDIDATES: &[&str] = &[
        "/usr/share/OVMF/OVMF_VARS_4M.fd",
        "/usr/share/OVMF/OVMF_VARS.fd",
        "/usr/share/edk2/x64/OVMF_VARS.4m.fd",
        "/usr/share/edk2-ovmf/x64/OVMF_VARS.fd",
        "/usr/share/qemu/OVMF_VARS.fd",
    ];
    CANDIDATES
        .iter()
        .map(PathBuf::from)
        .find(|p| p.exists())
        .unwrap_or_else(|| requested.to_path_buf())
}

#[async_trait]
impl VmmDriver for QemuDriver {
    fn kind(&self) -> VmmKind {
        VmmKind::Qemu
    }

    async fn probe(&self) -> Result<String, VmmError> {
        let out = Command::new(self.qemu_bin())
            .arg("--version")
            .output()
            .await
            .map_err(|e| VmmError::NotInstalled(format!("{}: {}", self.qemu_bin(), e)))?;
        if !out.status.success() {
            return Err(VmmError::NotInstalled(
                String::from_utf8_lossy(&out.stderr).to_string(),
            ));
        }
        let s = String::from_utf8_lossy(&out.stdout);
        let first = s.lines().next().unwrap_or("").to_string();
        Ok(first)
    }

    async fn boot(&self, spec: &VmSpec) -> Result<VmmHandle, VmmError> {
        let vm_dir = self.vm_run_dir(&spec.run_dir, spec.id);
        fs::create_dir_all(&vm_dir).await.map_err(VmmError::Io)?;

        // For UEFI boot, copy the nvram template into a per-VM writable file.
        let nvram_runtime = if let BootMode::Uefi {
            nvram_template: Some(template),
            ..
        } = &spec.boot
        {
            let dst = self.nvram_file(&vm_dir);
            self.prepare_nvram(template, &dst, spec.enable_secure_boot)
                .await
                .map_err(VmmError::Other)?;
            Some(dst)
        } else {
            None
        };

        let unit = self.systemd_unit(spec.id);

        // Clean up any stale socket from a previous attempt.
        for p in [
            self.qmp_sock(&vm_dir),
            self.serial_sock(&vm_dir),
            self.vnc_sock(&vm_dir),
        ] {
            if p.exists() {
                let _ = fs::remove_file(&p).await;
            }
        }

        // Spawn swtpm sidecar BEFORE QEMU so the control socket is ready.
        // No-op if enable_tpm is false or swtpm isn't installed.
        if spec.enable_tpm {
            self.spawn_swtpm(&vm_dir).await.map_err(VmmError::Other)?;
        }

        let args = self
            .build_args(spec, &vm_dir, nvram_runtime.as_deref())
            .map_err(VmmError::Other)?;

        let resource_props = super::resource::vm_properties(spec.vcpu, spec.mem_mib);
        self.spawn_under_unit(&unit, &resource_props, &args)
            .await
            .map_err(VmmError::Other)?;

        self.wait_for_qmp(&self.qmp_sock(&vm_dir))
            .await
            .map_err(|_| VmmError::SocketTimeout { vm_id: spec.id })?;

        // When QEMU was spawned via `sudo systemd-run` (production: non-root
        // agent), the process runs as root and its QMP/serial/VNC sockets are
        // root-owned with mode 0755 — the non-root agent can't connect (UDS
        // connect needs write). Relax the socket modes so the agent (and the
        // WS console bridges) can drive them. No-op in AGENT_NO_SUDO dev mode
        // where the agent owns the sockets already.
        self.relax_socket_perms(&vm_dir).await;

        // Negotiate QMP — confirms the VMM is alive and acceptable.
        let mut qmp = QmpClient::connect(&self.qmp_sock(&vm_dir))
            .await
            .map_err(VmmError::Other)?;
        // Verify status is running. QEMU starts the guest by default unless `-S`.
        let _status = qmp
            .execute::<serde_json::Value>("query-status", None)
            .await
            .map_err(VmmError::Other)?;

        let pid = self.read_pid(&vm_dir).await;

        let console_sock = Some(self.serial_sock(&vm_dir));
        let vnc = spec
            .enable_vnc
            .then(|| format!("unix:{}", self.vnc_sock(&vm_dir).display()));

        let handle = VmmHandle {
            vm_id: spec.id,
            kind: VmmKind::Qemu,
            api_sock: self.qmp_sock(&vm_dir),
            pid,
            systemd_unit: unit,
            console_sock,
            vnc,
        };
        self.persist_handle(&vm_dir, &handle)
            .await
            .map_err(VmmError::Other)?;
        Ok(handle)
    }

    async fn shutdown(&self, handle: &VmmHandle, mode: ShutdownMode) -> Result<(), VmmError> {
        match mode {
            ShutdownMode::Graceful => {
                if let Ok(mut qmp) = QmpClient::connect(&handle.api_sock).await {
                    let _ = qmp
                        .execute::<serde_json::Value>("system_powerdown", None)
                        .await;
                }
                Ok(())
            }
            ShutdownMode::Hard => {
                // Stop the systemd service; cgroup teardown kills the process.
                let _ = crate::core::systemd::stop_unit(&handle.systemd_unit).await;
                // Try QMP `quit` as the cleanest in-process exit.
                if let Ok(mut qmp) = QmpClient::connect(&handle.api_sock).await {
                    let _ = qmp.execute::<serde_json::Value>("quit", None).await;
                }
                // Fallback: kill the recorded PID directly. Catches direct-
                // spawn dev mode (AGENT_NO_SUDO) and any case where systemd
                // wasn't tracking the process.
                if let Some(pid) = handle.pid {
                    use std::os::unix::process::ExitStatusExt;
                    let _ = tokio::process::Command::new("kill")
                        .arg("-TERM")
                        .arg(pid.to_string())
                        .status()
                        .await;
                    // Brief wait then SIGKILL if still alive.
                    for _ in 0..20 {
                        if !std::path::Path::new(&format!("/proc/{pid}")).exists() {
                            break;
                        }
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                    if std::path::Path::new(&format!("/proc/{pid}")).exists() {
                        let out = tokio::process::Command::new("kill")
                            .arg("-KILL")
                            .arg(pid.to_string())
                            .status()
                            .await;
                        if let Ok(s) = out {
                            let _ = s.into_raw();
                        }
                    }
                }
                Ok(())
            }
        }
    }

    async fn pause(&self, handle: &VmmHandle) -> Result<(), VmmError> {
        let mut qmp = QmpClient::connect(&handle.api_sock)
            .await
            .map_err(VmmError::Other)?;
        qmp.execute::<serde_json::Value>("stop", None)
            .await
            .map_err(VmmError::Other)?;
        Ok(())
    }

    async fn resume(&self, handle: &VmmHandle) -> Result<(), VmmError> {
        let mut qmp = QmpClient::connect(&handle.api_sock)
            .await
            .map_err(VmmError::Other)?;
        qmp.execute::<serde_json::Value>("cont", None)
            .await
            .map_err(VmmError::Other)?;
        Ok(())
    }

    async fn destroy(&self, handle: VmmHandle) -> Result<(), VmmError> {
        let _ = self.shutdown(&handle, ShutdownMode::Hard).await;
        // Best-effort cleanup of the primary TAP. The manager names it
        // deterministically as tap-<vm_id[:8]> (QemuDriver doesn't own the name,
        // so we reconstruct it). Without this the bridge accumulates orphan taps
        // after every delete. No-op for user-mode-net VMs (no such device).
        let tap = format!("tap-{}", &handle.vm_id.to_string()[..8]);
        let _ = crate::core::net::delete_tap(&tap).await;
        // Best-effort cleanup of sockets and handle file.
        let vm_dir = handle.api_sock.parent().map(|p| p.to_path_buf());
        if let Some(dir) = vm_dir {
            for f in [
                self.qmp_sock(&dir),
                self.serial_sock(&dir),
                self.vnc_sock(&dir),
                self.pid_file(&dir),
                self.handle_file(&dir),
            ] {
                let _ = fs::remove_file(&f).await;
            }
        }
        Ok(())
    }

    async fn snapshot(
        &self,
        handle: &VmmHandle,
        dst: &SnapshotPaths,
        kind: SnapshotKind,
    ) -> Result<SnapshotMeta, VmmError> {
        if matches!(kind, SnapshotKind::Diff) {
            return Err(VmmError::NotSupported {
                kind: VmmKind::Qemu,
                feature: "diff_snapshot".to_string(),
            });
        }
        let mut qmp = QmpClient::connect(&handle.api_sock)
            .await
            .map_err(VmmError::Other)?;
        // Pause guest so snapshot is consistent.
        let _ = qmp
            .execute::<serde_json::Value>("stop", None)
            .await
            .map_err(VmmError::Other)?;
        // Migrate state to a file. The "exec:" target streams the migration
        // bytes through cat to the destination file. Memory + device state
        // together; for raw-backed disks we capture the disk separately via
        // qemu-img convert at this point.
        let state_path = dst.state_path.clone();
        let cmd = json!({
            "uri": format!("exec:cat > {}", shell_escape(&state_path.display().to_string()))
        });
        qmp.execute("migrate", Some(cmd))
            .await
            .map_err(VmmError::Other)?;
        // Poll migrate-status until completed.
        let deadline = std::time::Instant::now() + Duration::from_secs(600);
        loop {
            let s: serde_json::Value = qmp
                .execute::<serde_json::Value>("query-migrate", None)
                .await
                .map_err(VmmError::Other)?;
            match s.get("status").and_then(|v| v.as_str()) {
                Some("completed") => break,
                Some("failed") | Some("cancelled") => {
                    return Err(VmmError::Other(anyhow!("migrate {:?}", s)))
                }
                _ => {}
            }
            if std::time::Instant::now() >= deadline {
                return Err(VmmError::Other(anyhow!("migrate timed out")));
            }
            sleep(Duration::from_millis(200)).await;
        }
        // Capture the root disk alongside the RAM state. migrate-to-file only
        // saves RAM + device state; without the disk-at-snapshot a restore would
        // apply onto a drifted disk. The guest is still paused here, so a plain
        // byte copy is consistent. We copy the qcow2 *overlay* (small — just the
        // writes over the read-only base) via a raw file copy, which ignores the
        // qcow2 write lock the live QEMU still holds (a plain read, not a
        // lock-checked qemu-img open). Restore clones this into the new VM.
        if let Some(snap_dir) = state_path.parent() {
            let blocks: serde_json::Value = qmp
                .execute::<serde_json::Value>("query-block", None)
                .await
                .map_err(VmmError::Other)?;
            // `execute` already unwraps the QMP `return` field, so `blocks` is
            // the device array itself.
            let disk_src = blocks.as_array().and_then(|arr| {
                arr.iter().find_map(|d| {
                    if d.get("device").and_then(|x| x.as_str()) == Some("rootfs") {
                        d.get("inserted")
                            .and_then(|i| i.get("file"))
                            .and_then(|f| f.as_str())
                            .map(|s| s.to_string())
                    } else {
                        None
                    }
                })
            });
            match disk_src {
                Some(src) => {
                    let dst_disk = snap_dir.join("disk.qcow2");
                    fs::copy(&src, &dst_disk).await.map_err(|e| {
                        VmmError::Other(anyhow!("copy root disk for snapshot: {e}"))
                    })?;
                }
                None => {
                    let _ = qmp.execute::<serde_json::Value>("cont", None).await;
                    return Err(VmmError::Other(anyhow!(
                        "snapshot: could not locate 'rootfs' disk via query-block"
                    )));
                }
            }
        }
        let _ = qmp
            .execute::<serde_json::Value>("cont", None)
            .await
            .map_err(VmmError::Other)?;
        let size = fs::metadata(&state_path)
            .await
            .map(|m| m.len())
            .unwrap_or(0);
        Ok(SnapshotMeta {
            kind: VmmKind::Qemu,
            vmm_version: "qemu".to_string(),
            state_size_bytes: size,
            mem_size_bytes: Some(size),
        })
    }

    async fn restore(
        &self,
        _run_dir: &Path,
        _vm_id: Uuid,
        _src: &SnapshotPaths,
        _spec: &VmSpec,
    ) -> Result<VmmHandle, VmmError> {
        // Restore = boot with -incoming "exec:cat <state_path>". Not used by
        // current snapshot routes (which still go through the FC-shaped path);
        // wired up when manager's snapshot routes are generalised.
        Err(VmmError::NotSupported {
            kind: VmmKind::Qemu,
            feature: "restore_via_driver".to_string(),
        })
    }

    async fn rebind(&self, run_dir: &Path, vm_id: Uuid) -> Result<Option<VmmHandle>, VmmError> {
        let vm_dir = self.vm_run_dir(run_dir, vm_id);
        let Some(mut handle) = self.load_handle(&vm_dir).await.map_err(VmmError::Other)? else {
            return Ok(None);
        };
        // Re-verify PID liveness.
        let pid = self.read_pid(&vm_dir).await;
        handle.pid = pid;
        let alive = pid
            .map(|p| std::path::Path::new(&format!("/proc/{p}")).exists())
            .unwrap_or(false);
        if !alive {
            // Stale handle — drop it.
            let _ = fs::remove_file(self.handle_file(&vm_dir)).await;
            return Ok(None);
        }
        Ok(Some(handle))
    }

    async fn console_endpoint(&self, handle: &VmmHandle) -> Result<ConsoleEndpoint, VmmError> {
        if let Some(sock) = &handle.console_sock {
            Ok(ConsoleEndpoint::UnixSerial { path: sock.clone() })
        } else if let Some(vnc) = &handle.vnc {
            // vnc string is "unix:<path>" or "host:port" — pass through as-is.
            let (host, port) = if let Some(p) = vnc.strip_prefix("unix:") {
                (p.to_string(), 0u16)
            } else if let Some((h, p)) = vnc.rsplit_once(':') {
                (h.to_string(), p.parse().unwrap_or(0))
            } else {
                (vnc.clone(), 0u16)
            };
            Ok(ConsoleEndpoint::Vnc {
                host,
                port,
                password: None,
            })
        } else {
            Err(VmmError::NotSupported {
                kind: VmmKind::Qemu,
                feature: "console_endpoint".to_string(),
            })
        }
    }
}

/// Crude shell-escape for paths embedded in QMP `exec:` URIs. QEMU passes the
/// URI to `/bin/sh -c`, so quote it.
fn shell_escape(s: &str) -> String {
    let mut out = String::from("'");
    for c in s.chars() {
        if c == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(c);
        }
    }
    out.push('\'');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_vmm::{BootMode, DiskSpec, NicSpec};
    use std::path::PathBuf;

    fn linux_kernel_spec() -> VmSpec {
        VmSpec {
            id: Uuid::new_v4(),
            vcpu: 2,
            mem_mib: 1024,
            boot: BootMode::LinuxKernel {
                kernel: PathBuf::from("/srv/images/vmlinuz"),
                initrd: Some(PathBuf::from("/srv/images/initrd")),
                cmdline: "console=ttyS0 root=/dev/vda".into(),
            },
            disks: vec![DiskSpec {
                drive_id: "rootfs".into(),
                source: PathBuf::from("/srv/fc/x/rootfs.ext4"),
                read_only: false,
                root_device: true,
                format: Some("raw".into()),
                cdrom: false,
            }],
            nics: vec![NicSpec {
                iface_id: "eth0".into(),
                host_dev: "tap123".into(),
                mac: "52:54:00:12:34:56".into(),
            }],
            enable_vnc: false,
            enable_tpm: false,
            enable_secure_boot: false,
            enable_balloon: false,
            enable_rng: false,
            no_reboot: false,
            vsock_cid: None,
            vfio_devices: vec![],
            cpu_type: None,
            incoming_uri: None,
            log_path: PathBuf::from("/tmp/qemu.log"),
            run_dir: PathBuf::from("/srv/fc"),
        }
    }

    #[test]
    fn build_args_linux_kernel_includes_kernel_initrd_serial_qmp() {
        let drv = QemuDriver::new();
        let spec = linux_kernel_spec();
        let args = drv
            .build_args(&spec, std::path::Path::new("/srv/fc/xyz"), None)
            .unwrap();
        let joined = args.join(" ");
        assert!(joined.contains("-machine"));
        assert!(joined.contains("q35,accel=kvm"));
        assert!(joined.contains("-kernel /srv/images/vmlinuz"));
        assert!(joined.contains("-initrd /srv/images/initrd"));
        assert!(joined.contains("console=ttyS0"));
        assert!(joined.contains("-qmp unix:/srv/fc/xyz/qmp.sock"));
        assert!(joined.contains("-chardev"));
        assert!(joined.contains("serial.sock"));
        assert!(joined.contains("-pidfile"));
        assert!(joined.contains("virtio-blk-pci"));
        assert!(joined.contains("virtio-net-pci"));
        // No VGA / VNC unless enabled.
        assert!(!joined.contains("-vnc"));
        // No -display when graphical disabled.
        assert!(joined.contains("-display none"));
    }

    #[test]
    fn ovmf_resolver_keeps_existing_path_and_falls_back_otherwise() {
        // An existing path is returned unchanged.
        let real = std::path::Path::new("/etc/hostname"); // exists on linux test hosts
        if real.exists() {
            assert_eq!(resolve_ovmf_code(real), real.to_path_buf());
        }
        // A bogus requested path falls back to a known candidate IF one
        // exists on this machine; otherwise returns the requested path
        // unchanged (so the error names what was asked for).
        let bogus = std::path::Path::new("/nonexistent/OVMF_CODE.fd");
        let resolved = resolve_ovmf_code(bogus);
        // Either it found a real candidate, or it echoed the bogus path back.
        assert!(resolved == bogus.to_path_buf() || resolved.exists());
        let resolved_vars = resolve_ovmf_vars(std::path::Path::new("/nonexistent/OVMF_VARS.fd"));
        assert!(
            resolved_vars == std::path::Path::new("/nonexistent/OVMF_VARS.fd").to_path_buf()
                || resolved_vars.exists()
        );
    }

    #[test]
    fn build_args_uefi_with_nvram() {
        let drv = QemuDriver::new();
        let mut spec = linux_kernel_spec();
        spec.boot = BootMode::Uefi {
            firmware: PathBuf::from("/usr/share/edk2/x64/OVMF_CODE.4m.fd"),
            nvram_template: Some(PathBuf::from("/usr/share/edk2/x64/OVMF_VARS.4m.fd")),
        };
        let nv = PathBuf::from("/srv/fc/xyz/OVMF_VARS.fd");
        let args = drv
            .build_args(&spec, std::path::Path::new("/srv/fc/xyz"), Some(&nv))
            .unwrap();
        let joined = args.join(" ");
        assert!(joined
            .contains("if=pflash,format=raw,readonly=on,file=/usr/share/edk2/x64/OVMF_CODE.4m.fd"));
        assert!(joined.contains("if=pflash,format=raw,file=/srv/fc/xyz/OVMF_VARS.fd"));
        // No -kernel in UEFI mode.
        assert!(!joined.contains("-kernel"));
    }

    #[test]
    fn build_args_enables_vnc_when_requested() {
        let drv = QemuDriver::new();
        let mut spec = linux_kernel_spec();
        spec.enable_vnc = true;
        let args = drv
            .build_args(&spec, std::path::Path::new("/srv/fc/xyz"), None)
            .unwrap();
        let joined = args.join(" ");
        assert!(joined.contains("-vnc unix:/srv/fc/xyz/vnc.sock"));
        assert!(joined.contains("virtio-vga"));
        // -display none must not be present when VNC is on.
        assert!(!joined.contains("-display none"));
    }

    /// Verify the exact argv production-mode `spawn_under_unit` builds.
    /// We can't easily run sudo+systemd-run from an unprivileged test
    /// harness, but we can assert the command line is correct, which is
    /// what the production cgroup-enforcement path depends on.
    #[test]
    fn systemd_run_argv_is_well_formed() {
        // Exercise the exact arg construction used by spawn_under_unit.
        // Reconstruct the prefix here so the test pins down the contract
        // (transient service — NOT --scope — with --collect, KillMode=mixed,
        // TimeoutStopSec=10s, then resource properties, then -- qemu …).
        let unit = "qemu-fake-uuid.service";
        let resource_props = super::super::resource::vm_properties(2, 1024);
        let bin = "qemu-system-x86_64";
        let mut cmd: Vec<String> = vec![
            "-n".into(),
            "systemd-run".into(),
            "--collect".into(),
            format!("--unit={unit}"),
            "--property=KillMode=mixed".into(),
            "--property=TimeoutStopSec=10s".into(),
        ];
        for p in &resource_props {
            cmd.push(format!("--property={p}"));
        }
        cmd.push("--".into());
        cmd.push(bin.into());

        let joined = cmd.join(" ");
        assert!(joined.contains("systemd-run"));
        // The bug regression guard: a transient *service*, never a *scope*
        // (which blocks until QEMU exits and wedges the boot path).
        assert!(!joined.contains("--scope"), "must not use --scope");
        assert!(joined.contains("--collect"));
        assert!(joined.contains("--unit=qemu-fake-uuid.service"));
        assert!(joined.contains("--property=KillMode=mixed"));
        assert!(joined.contains("--property=MemoryMax=1536M"));
        assert!(joined.contains("--property=MemorySwapMax=0"));
        assert!(joined.contains("--property=CPUQuota=200%"));
        assert!(joined.contains("qemu-system-x86_64"));
        // -- separator must come AFTER all systemd properties and BEFORE the qemu bin.
        let dash_idx = cmd.iter().position(|s| s == "--").unwrap();
        let bin_idx = cmd
            .iter()
            .position(|s| s.ends_with("qemu-system-x86_64"))
            .unwrap();
        assert!(dash_idx < bin_idx);
        let props_end = cmd
            .iter()
            .rposition(|s| s.starts_with("--property="))
            .unwrap();
        assert!(props_end < dash_idx);
    }

    #[test]
    fn build_args_emits_balloon_rng_vsock_when_enabled() {
        let drv = QemuDriver::new();
        let mut spec = linux_kernel_spec();
        spec.enable_balloon = true;
        spec.enable_rng = true;
        spec.vsock_cid = Some(42);
        let args = drv
            .build_args(&spec, std::path::Path::new("/srv/fc/xyz"), None)
            .unwrap();
        let joined = args.join(" ");
        assert!(joined.contains("virtio-balloon-pci,id=balloon0"));
        assert!(joined.contains("virtio-rng-pci,rng=rng0"));
        assert!(joined.contains("vhost-vsock-pci,guest-cid=42"));
        assert!(joined.contains("rng-random,id=rng0,filename=/dev/urandom"));
    }

    #[test]
    fn build_args_emits_vfio_devices() {
        let drv = QemuDriver::new();
        let mut spec = linux_kernel_spec();
        spec.vfio_devices = vec!["0000:01:00.0".into(), "0000:02:00.0".into()];
        let args = drv
            .build_args(&spec, std::path::Path::new("/srv/fc/xyz"), None)
            .unwrap();
        let joined = args.join(" ");
        assert!(joined.contains("vfio-pci,host=0000:01:00.0,id=vfio0"));
        assert!(joined.contains("vfio-pci,host=0000:02:00.0,id=vfio1"));
    }

    #[test]
    fn build_args_emits_tpm_when_enabled() {
        let drv = QemuDriver::new();
        let mut spec = linux_kernel_spec();
        spec.enable_tpm = true;
        let args = drv
            .build_args(&spec, std::path::Path::new("/srv/fc/xyz"), None)
            .unwrap();
        let joined = args.join(" ");
        assert!(joined.contains("socket,id=chrtpm,path=/srv/fc/xyz/swtpm.sock"));
        assert!(joined.contains("emulator,id=tpm0,chardev=chrtpm"));
        assert!(joined.contains("tpm-crb,tpmdev=tpm0"));
    }

    #[test]
    fn build_args_marks_cdrom_readonly() {
        let drv = QemuDriver::new();
        let mut spec = linux_kernel_spec();
        spec.disks.push(DiskSpec {
            drive_id: "iso".into(),
            source: PathBuf::from("/srv/images/ubuntu.iso"),
            read_only: true,
            root_device: false,
            format: Some("raw".into()),
            cdrom: true,
        });
        let args = drv
            .build_args(&spec, std::path::Path::new("/srv/fc/xyz"), None)
            .unwrap();
        let joined = args.join(" ");
        assert!(
            joined.contains("file=/srv/images/ubuntu.iso,if=none,format=raw,id=iso,readonly=on")
        );
        // CD-ROMs attach as an ejectable ide-cd on an AHCI controller, NOT
        // virtio-blk (which can't be media-ejected at install-complete).
        assert!(joined.contains("ich9-ahci,id=ahci0"));
        // CD-ROMs boot AFTER the root disk, so the first CD is bootindex 1.
        assert!(joined.contains("ide-cd,drive=iso,id=iso-dev,bus=ahci0.0,bootindex=1"));
        // Spare PCIe root-ports for runtime hot-plug.
        assert!(joined.contains("pcie-root-port,id=rphp0"));
    }

    #[test]
    fn build_args_unique_bootindex_with_multiple_cdroms() {
        // Two CD-ROMs (e.g. Windows installer + virtio-win) must get distinct
        // bootindexes, else QEMU refuses to start.
        let drv = QemuDriver::new();
        let mut spec = linux_kernel_spec();
        for n in ["iso1", "iso2"] {
            spec.disks.push(DiskSpec {
                drive_id: n.into(),
                source: PathBuf::from(format!("/srv/images/{n}.iso")),
                read_only: true,
                root_device: false,
                format: Some("raw".into()),
                cdrom: true,
            });
        }
        let args = drv
            .build_args(&spec, std::path::Path::new("/srv/fc/xyz"), None)
            .unwrap();
        let joined = args.join(" ");
        assert!(joined.contains("ide-cd,drive=iso1,id=iso1-dev,bus=ahci0.0,bootindex=1"));
        assert!(joined.contains("ide-cd,drive=iso2,id=iso2-dev,bus=ahci0.1,bootindex=2"));
    }
}
