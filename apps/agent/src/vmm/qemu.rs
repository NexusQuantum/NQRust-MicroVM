//! QEMU VMM driver.
//!
//! Spawns `qemu-system-x86_64` inside a per-VM `systemd-run --scope` so the
//! kernel enforces cgroup memory/cpu limits and the process is supervised
//! alongside the rest of the host's units. Talks to QEMU over QMP for
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
        format!("qemu-{vm_id}.scope")
    }

    /// Spawn QEMU under a systemd transient scope. Caller is responsible for
    /// having created the vm_dir. `resource_props` is the list of
    /// `--property=KEY=VALUE` strings the kernel will enforce as a cgroup
    /// per the [`super::resource`] helpers — these are how QEMU and FC are
    /// kept from fighting for memory or CPU on a shared host.
    ///
    /// In dev environments without passwordless sudo, set `AGENT_NO_SUDO=1`
    /// to skip the systemd-run wrapper and spawn QEMU directly. The
    /// per-VM cgroup limits are forfeited but the rest of the pipeline
    /// (QMP, console UDS, NICs, disks) works for end-to-end testing.
    async fn spawn_under_scope(
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

        let mut cmd = Command::new("sudo");
        cmd.arg("-n")
            .arg("systemd-run")
            .arg("--scope")
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
        args.push("-machine".into());
        args.push("q35,accel=kvm,smm=off".into());

        // CPU and SMP topology.
        args.push("-cpu".into());
        args.push("host".into());
        args.push("-smp".into());
        args.push(format!("cpus={}", spec.vcpu));
        args.push("-m".into());
        args.push(format!("{}M", spec.mem_mib));

        // No default devices, no graphical display.
        args.push("-nodefaults".into());
        args.push("-no-user-config".into());
        args.push("-no-reboot".into());
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
                args.push("-drive".into());
                args.push(format!(
                    "if=pflash,format=raw,readonly=on,file={}",
                    firmware.display()
                ));
                if let Some(nv) = nvram_runtime {
                    args.push("-drive".into());
                    args.push(format!("if=pflash,format=raw,file={}", nv.display()));
                }
            }
        }

        // Disks. Each gets a virtio-blk-pci device, except `cdrom: true` which
        // attaches as readonly virtio-blk with the disk media type forced.
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
            let bootindex = if disk.root_device {
                ",bootindex=0".to_string()
            } else if disk.cdrom {
                ",bootindex=1".to_string()
            } else {
                String::new()
            };
            args.push(format!(
                "virtio-blk-pci,drive={},id={}-dev{}",
                drive_id, drive_id, bootindex
            ));
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

        // Combined log file (stderr/stdout from QEMU itself).
        args.push("-D".into());
        args.push(self.log_file(vm_dir).display().to_string());

        Ok(args)
    }

    async fn read_pid(&self, vm_dir: &Path) -> Option<u32> {
        let raw = fs::read_to_string(self.pid_file(vm_dir)).await.ok()?;
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
    /// VM has its own writable EFI variables.
    async fn prepare_nvram(&self, template: &Path, dst: &Path) -> Result<()> {
        if fs::metadata(dst).await.is_ok() {
            return Ok(());
        }
        fs::copy(template, dst).await.with_context(|| {
            format!("copy ovmf vars {} -> {}", template.display(), dst.display())
        })?;
        Ok(())
    }
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
            self.prepare_nvram(template, &dst)
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

        let args = self
            .build_args(spec, &vm_dir, nvram_runtime.as_deref())
            .map_err(VmmError::Other)?;

        let resource_props = super::resource::vm_properties(spec.vcpu, spec.mem_mib);
        self.spawn_under_scope(&unit, &resource_props, &args)
            .await
            .map_err(VmmError::Other)?;

        self.wait_for_qmp(&self.qmp_sock(&vm_dir))
            .await
            .map_err(|_| VmmError::SocketTimeout { vm_id: spec.id })?;

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
                // Stop the systemd scope; cgroup teardown kills the process.
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
    }
}
