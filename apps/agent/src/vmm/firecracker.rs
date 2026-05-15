//! Firecracker driver — thin adapter around the existing FC spawn/proxy code.
//!
//! The manager still drives FC configuration via the legacy
//! `apps/agent/src/features/vm/{spawn,proxy}.rs` HTTP path because FC's
//! REST API is its native ABI. This driver covers the parts the trait owns:
//! probing for installation, spawning the FC binary inside a per-VM
//! systemd scope, persisting a `VmmHandle`, and providing the console
//! endpoint for the WebSocket shell bridge.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::anyhow;
use async_trait::async_trait;
use nexus_vmm::{
    ConsoleEndpoint, ShutdownMode, SnapshotKind, SnapshotMeta, SnapshotPaths, VmSpec, VmmDriver,
    VmmError, VmmHandle, VmmKind,
};
use tokio::fs;
use tokio::process::Command;
use tokio::time::sleep;
use uuid::Uuid;

const FC_BIN_DEFAULT: &str = "firecracker";

#[derive(Default, Clone)]
pub struct FirecrackerDriver;

impl FirecrackerDriver {
    pub fn new() -> Self {
        Self
    }

    fn vm_run_dir(&self, run_dir: &Path, vm_id: Uuid) -> PathBuf {
        run_dir.join(vm_id.to_string())
    }

    fn api_sock(&self, vm_dir: &Path) -> PathBuf {
        vm_dir.join("firecracker.sock")
    }

    fn handle_file(&self, vm_dir: &Path) -> PathBuf {
        vm_dir.join("handle.json")
    }

    fn systemd_unit(&self, vm_id: Uuid) -> String {
        // Match the legacy unit format so existing systemd scopes
        // and screen sessions keep working.
        format!("fc-{vm_id}.scope")
    }

    async fn persist_handle(&self, vm_dir: &Path, handle: &VmmHandle) -> anyhow::Result<()> {
        let path = self.handle_file(vm_dir);
        let bytes = serde_json::to_vec_pretty(handle)?;
        fs::write(&path, bytes).await?;
        Ok(())
    }

    async fn load_handle(&self, vm_dir: &Path) -> anyhow::Result<Option<VmmHandle>> {
        let path = self.handle_file(vm_dir);
        match fs::read(&path).await {
            Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn fc_bin(&self) -> String {
        std::env::var("FC_BINARY").unwrap_or_else(|_| FC_BIN_DEFAULT.to_string())
    }
}

#[async_trait]
impl VmmDriver for FirecrackerDriver {
    fn kind(&self) -> VmmKind {
        VmmKind::Firecracker
    }

    async fn probe(&self) -> Result<String, VmmError> {
        let out = Command::new(self.fc_bin())
            .arg("--version")
            .output()
            .await
            .map_err(|e| VmmError::NotInstalled(format!("{}: {}", self.fc_bin(), e)))?;
        if !out.status.success() {
            return Err(VmmError::NotInstalled(
                String::from_utf8_lossy(&out.stderr).to_string(),
            ));
        }
        // Firecracker prints "Firecracker v1.x.y\n..." — capture first line.
        let s = String::from_utf8_lossy(&out.stdout);
        Ok(s.lines().next().unwrap_or("").to_string())
    }

    async fn boot(&self, spec: &VmSpec) -> Result<VmmHandle, VmmError> {
        // The manager configures FC over the proxy after spawn. Here we just
        // start the FC process and return a handle pointing at the api-sock.
        let vm_dir = self.vm_run_dir(&spec.run_dir, spec.id);
        fs::create_dir_all(&vm_dir).await.map_err(VmmError::Io)?;
        let api_sock = self.api_sock(&vm_dir);
        let unit = self.systemd_unit(spec.id);

        if api_sock.exists() {
            let _ = fs::remove_file(&api_sock).await;
        }

        crate::core::systemd::spawn_fc_scope(
            &unit,
            api_sock
                .to_str()
                .ok_or_else(|| VmmError::Other(anyhow!("api-sock path is not valid UTF-8")))?,
        )
        .await
        .map_err(VmmError::Other)?;

        // Wait for the api-sock to appear (FC's startup time).
        let deadline = std::time::Instant::now() + Duration::from_secs(20);
        while !api_sock.exists() && std::time::Instant::now() < deadline {
            sleep(Duration::from_millis(50)).await;
        }
        if !api_sock.exists() {
            return Err(VmmError::SocketTimeout { vm_id: spec.id });
        }

        let handle = VmmHandle {
            vm_id: spec.id,
            kind: VmmKind::Firecracker,
            api_sock,
            pid: None,
            systemd_unit: unit,
            // FC's serial is exposed through screen by the legacy spawn; the
            // existing WS shell handler attaches via screen, not via this
            // path. Leave None to signal "use the legacy bridge".
            console_sock: None,
            vnc: None,
        };
        self.persist_handle(&vm_dir, &handle)
            .await
            .map_err(VmmError::Other)?;
        Ok(handle)
    }

    async fn shutdown(&self, handle: &VmmHandle, _mode: ShutdownMode) -> Result<(), VmmError> {
        // Stopping the systemd scope kills FC. The manager's existing
        // /v1/vms/:id/actions InstanceShutdown call covers the graceful path.
        let _ = crate::core::systemd::stop_unit(&handle.systemd_unit).await;
        Ok(())
    }

    async fn pause(&self, _handle: &VmmHandle) -> Result<(), VmmError> {
        // The manager drives pause/resume via the FC REST proxy.
        // Trait method is a no-op for FC in this release.
        Ok(())
    }

    async fn resume(&self, _handle: &VmmHandle) -> Result<(), VmmError> {
        Ok(())
    }

    async fn destroy(&self, handle: VmmHandle) -> Result<(), VmmError> {
        let _ = crate::core::systemd::stop_unit(&handle.systemd_unit).await;
        if let Some(parent) = handle.api_sock.parent() {
            let _ = fs::remove_file(self.handle_file(parent)).await;
            let _ = fs::remove_file(&handle.api_sock).await;
        }
        Ok(())
    }

    async fn snapshot(
        &self,
        _handle: &VmmHandle,
        _dst: &SnapshotPaths,
        _kind: SnapshotKind,
    ) -> Result<SnapshotMeta, VmmError> {
        // The manager still drives FC snapshots via the proxy in
        // apps/manager/src/features/snapshots — same call path as before.
        Err(VmmError::NotSupported {
            kind: VmmKind::Firecracker,
            feature: "snapshot_via_driver".to_string(),
        })
    }

    async fn restore(
        &self,
        _run_dir: &Path,
        _vm_id: Uuid,
        _src: &SnapshotPaths,
        _spec: &VmSpec,
    ) -> Result<VmmHandle, VmmError> {
        Err(VmmError::NotSupported {
            kind: VmmKind::Firecracker,
            feature: "restore_via_driver".to_string(),
        })
    }

    async fn rebind(&self, run_dir: &Path, vm_id: Uuid) -> Result<Option<VmmHandle>, VmmError> {
        let vm_dir = self.vm_run_dir(run_dir, vm_id);
        let Some(handle) = self.load_handle(&vm_dir).await.map_err(VmmError::Other)? else {
            return Ok(None);
        };
        // FC has no PID file by convention; we rely on the api-sock + systemd
        // unit state. If the sock exists, treat as live.
        if !handle.api_sock.exists() {
            return Ok(None);
        }
        Ok(Some(handle))
    }

    async fn console_endpoint(&self, _handle: &VmmHandle) -> Result<ConsoleEndpoint, VmmError> {
        // FC's serial console is exposed via a screen session keyed by the
        // systemd unit name. The WS shell handler in
        // apps/agent/src/features/vm/shell.rs handles this case directly;
        // the trait method is not used for FC in this release.
        Err(VmmError::NotSupported {
            kind: VmmKind::Firecracker,
            feature: "console_endpoint_via_driver".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_name_matches_legacy_convention() {
        let d = FirecrackerDriver::new();
        let id = Uuid::parse_str("0e6e9bcc-1111-2222-3333-444444444444").unwrap();
        assert_eq!(
            d.systemd_unit(id),
            "fc-0e6e9bcc-1111-2222-3333-444444444444.scope"
        );
    }

    #[test]
    fn api_sock_path_under_vm_dir() {
        let d = FirecrackerDriver::new();
        let p = d.api_sock(std::path::Path::new("/srv/fc/abc"));
        assert_eq!(p, PathBuf::from("/srv/fc/abc/firecracker.sock"));
    }
}
