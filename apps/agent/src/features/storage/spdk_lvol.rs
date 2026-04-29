//! SPDK lvol host backend.
//!
//! Attach creates an SPDK vhost-blk controller for the lvol and returns the
//! Unix socket Firecracker must use as a vhost-user block device.

use nexus_storage::{
    spdk_vhost_controller_name, AttachedPath, BackendKind, HostBackend, SpdkJsonRpcClient,
    SpdkLvolLocator, StorageError, VolumeHandle, VolumeSnapshotHandle,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::sync::{Mutex, OwnedSemaphorePermit, Semaphore};

#[derive(Clone)]
pub struct SpdkLvolHostBackend {
    pub client: SpdkJsonRpcClient,
    pub vhost_socket_dir: PathBuf,
    #[cfg_attr(not(test), allow(dead_code))]
    pub nbd_devices: Vec<PathBuf>,
    nbd_pool: Arc<NbdDevicePool>,
    attached_lvols: Arc<Mutex<HashMap<PathBuf, String>>>,
}

impl SpdkLvolHostBackend {
    #[cfg(test)]
    pub fn new(rpc_socket: impl Into<PathBuf>, vhost_socket_dir: impl Into<PathBuf>) -> Self {
        Self::with_import_nbd_device(rpc_socket, vhost_socket_dir, "/dev/nbd0")
    }

    #[cfg(test)]
    pub fn with_import_nbd_device(
        rpc_socket: impl Into<PathBuf>,
        vhost_socket_dir: impl Into<PathBuf>,
        import_nbd_device: impl Into<PathBuf>,
    ) -> Self {
        Self::with_nbd_devices(rpc_socket, vhost_socket_dir, vec![import_nbd_device.into()])
    }

    pub fn with_nbd_devices(
        rpc_socket: impl Into<PathBuf>,
        vhost_socket_dir: impl Into<PathBuf>,
        nbd_devices: Vec<PathBuf>,
    ) -> Self {
        let nbd_devices = if nbd_devices.is_empty() {
            vec![PathBuf::from("/dev/nbd0")]
        } else {
            nbd_devices
        };
        Self {
            client: SpdkJsonRpcClient::new(rpc_socket),
            vhost_socket_dir: vhost_socket_dir.into(),
            nbd_pool: Arc::new(NbdDevicePool::new(nbd_devices.clone())),
            nbd_devices,
            attached_lvols: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn socket_for_controller(&self, ctrlr: &str) -> PathBuf {
        self.vhost_socket_dir.join(ctrlr)
    }
}

#[async_trait::async_trait]
impl HostBackend for SpdkLvolHostBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::SpdkLvol
    }

    async fn attach(&self, volume: &VolumeHandle) -> Result<AttachedPath, StorageError> {
        let locator = SpdkLvolLocator::from_locator_str(&volume.locator)?;
        let ctrlr = spdk_vhost_controller_name(volume.volume_id);
        self.client
            .vhost_create_blk_controller(&ctrlr, &locator.lvol_uuid)
            .await?;
        let socket = self.socket_for_controller(&ctrlr);
        self.attached_lvols
            .lock()
            .await
            .insert(socket.clone(), locator.lvol_uuid);
        Ok(AttachedPath::VhostUserSock(socket))
    }

    async fn detach(
        &self,
        volume: &VolumeHandle,
        _attached: AttachedPath,
    ) -> Result<(), StorageError> {
        let ctrlr = spdk_vhost_controller_name(volume.volume_id);
        let result = self.client.vhost_delete_controller(&ctrlr).await;
        let socket = self.socket_for_controller(&ctrlr);
        self.attached_lvols.lock().await.remove(&socket);
        result
    }

    async fn populate_streaming(
        &self,
        attached: &AttachedPath,
        source: &Path,
        target_size_bytes: u64,
    ) -> Result<(), StorageError> {
        let lvol_uuid = self.lvol_uuid_for_attachment(attached).await?;
        let exported = self.export_lvol_to_nbd(&lvol_uuid).await?;
        let copy_result = copy_image_to_nbd(source, exported.device(), target_size_bytes).await;
        let stop_result = self.stop_nbd_export(&exported).await;
        copy_result?;
        stop_result
    }

    async fn resize2fs(&self, attached: &AttachedPath) -> Result<(), StorageError> {
        let lvol_uuid = self.lvol_uuid_for_attachment(attached).await?;
        let exported = self.export_lvol_to_nbd(&lvol_uuid).await?;
        let resize_result = run_resize2fs(exported.device()).await;
        let stop_result = self.stop_nbd_export(&exported).await;
        resize_result?;
        stop_result
    }

    async fn read_snapshot(
        &self,
        snap: &VolumeSnapshotHandle,
    ) -> Result<Box<dyn tokio::io::AsyncRead + Send + Unpin>, StorageError> {
        let locator = SpdkLvolLocator::from_locator_str(&snap.locator)?;
        let exported = self.export_lvol_to_nbd(&locator.lvol_uuid).await?;
        let file = match tokio::fs::File::open(exported.device()).await {
            Ok(file) => file,
            Err(err) => {
                let _ = self.stop_nbd_export(&exported).await;
                return Err(StorageError::Io(err));
            }
        };
        Ok(Box::new(SpdkNbdSnapshotReader {
            inner: file,
            client: self.client.clone(),
            export: Some(exported),
        }))
    }
}

struct SpdkNbdSnapshotReader {
    inner: tokio::fs::File,
    client: SpdkJsonRpcClient,
    export: Option<NbdExport>,
}

impl tokio::io::AsyncRead for SpdkNbdSnapshotReader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}

impl Drop for SpdkNbdSnapshotReader {
    fn drop(&mut self) {
        let Some(export) = self.export.take() else {
            return;
        };
        let client = self.client.clone();
        let nbd_device = export.device().to_path_buf();
        tokio::spawn(async move {
            let _ = client.nbd_stop_disk(&nbd_device).await;
            let _ = wait_for_nbd_device_released(&nbd_device).await;
            drop(export);
        });
    }
}

impl SpdkLvolHostBackend {
    async fn lvol_uuid_for_attachment(
        &self,
        attached: &AttachedPath,
    ) -> Result<String, StorageError> {
        let AttachedPath::VhostUserSock(socket) = attached else {
            return Err(StorageError::InvalidLocator(
                "spdk_lvol requires a VhostUserSock attachment".into(),
            ));
        };
        self.attached_lvols
            .lock()
            .await
            .get(socket)
            .cloned()
            .ok_or_else(|| {
                StorageError::InvalidLocator(format!(
                    "no SPDK lvol is registered for vhost socket {}",
                    socket.display()
                ))
            })
    }

    async fn export_lvol_to_nbd(&self, lvol_uuid: &str) -> Result<NbdExport, StorageError> {
        let lease = self.nbd_pool.acquire().await?;
        let exported = self.client.nbd_start_disk(lvol_uuid, lease.path()).await?;
        if let Err(err) = wait_for_nbd_device_size(&exported).await {
            let _ = self.client.nbd_stop_disk(&exported).await;
            return Err(err);
        }
        Ok(NbdExport {
            device: exported,
            _lease: lease,
        })
    }

    async fn stop_nbd_export(&self, exported: &NbdExport) -> Result<(), StorageError> {
        self.client.nbd_stop_disk(exported.device()).await?;
        wait_for_nbd_device_released(exported.device()).await
    }
}

struct NbdExport {
    device: PathBuf,
    _lease: NbdLease,
}

impl NbdExport {
    fn device(&self) -> &Path {
        &self.device
    }
}

struct NbdDevicePool {
    available: std::sync::Mutex<Vec<PathBuf>>,
    semaphore: Arc<Semaphore>,
}

impl NbdDevicePool {
    fn new(devices: Vec<PathBuf>) -> Self {
        let capacity = devices.len();
        Self {
            available: std::sync::Mutex::new(devices),
            semaphore: Arc::new(Semaphore::new(capacity)),
        }
    }

    async fn acquire(self: &Arc<Self>) -> Result<NbdLease, StorageError> {
        let permit = self.semaphore.clone().acquire_owned().await.map_err(|e| {
            StorageError::InvalidLocator(format!("SPDK NBD device pool closed: {e}"))
        })?;
        let path = self
            .available
            .lock()
            .expect("NBD device pool mutex poisoned")
            .pop()
            .ok_or_else(|| StorageError::InvalidLocator("SPDK NBD pool exhausted".into()))?;
        Ok(NbdLease {
            path: Some(path),
            pool: self.clone(),
            _permit: Some(permit),
        })
    }
}

struct NbdLease {
    path: Option<PathBuf>,
    pool: Arc<NbdDevicePool>,
    _permit: Option<OwnedSemaphorePermit>,
}

impl NbdLease {
    fn path(&self) -> &Path {
        self.path
            .as_deref()
            .expect("NBD lease path already released")
    }
}

impl Drop for NbdLease {
    fn drop(&mut self) {
        if let Some(path) = self.path.take() {
            self.pool
                .available
                .lock()
                .expect("NBD device pool mutex poisoned")
                .push(path);
        }
    }
}

async fn copy_image_to_nbd(
    source: &Path,
    nbd_device: &Path,
    target_size_bytes: u64,
) -> Result<(), StorageError> {
    let source_len = tokio::fs::metadata(source).await?.len();
    if source_len > target_size_bytes {
        return Err(StorageError::InvalidLocator(format!(
            "source image {} is larger than target SPDK lvol: {} > {} bytes",
            source.display(),
            source_len,
            target_size_bytes
        )));
    }

    let mut src = tokio::fs::File::open(source).await?;
    let mut dst = tokio::fs::OpenOptions::new()
        .write(true)
        .open(nbd_device)
        .await?;
    tokio::io::copy(&mut src, &mut dst).await?;
    dst.flush().await?;
    dst.sync_all().await?;
    Ok(())
}

async fn wait_for_nbd_device_size(nbd_device: &Path) -> Result<(), StorageError> {
    for _ in 0..50 {
        let output = tokio::process::Command::new("blockdev")
            .arg("--getsize64")
            .arg(nbd_device)
            .output()
            .await?;
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.trim().parse::<u64>().unwrap_or(0) > 0 {
                return Ok(());
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    Err(StorageError::backend(std::io::Error::other(format!(
        "SPDK NBD device {} did not report a nonzero size",
        nbd_device.display()
    ))))
}

async fn wait_for_nbd_device_released(nbd_device: &Path) -> Result<(), StorageError> {
    for _ in 0..50 {
        let output = tokio::process::Command::new("blockdev")
            .arg("--getsize64")
            .arg(nbd_device)
            .output()
            .await?;
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.trim().parse::<u64>().unwrap_or(u64::MAX) == 0 {
                return Ok(());
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    Err(StorageError::backend(std::io::Error::other(format!(
        "SPDK NBD device {} did not release",
        nbd_device.display()
    ))))
}

async fn run_resize2fs(path: &Path) -> Result<(), StorageError> {
    let _ = tokio::process::Command::new("e2fsck")
        .args(["-f", "-y"])
        .arg(path)
        .output()
        .await?;
    let out = tokio::process::Command::new("resize2fs")
        .arg(path)
        .output()
        .await?;
    if out.status.success() {
        Ok(())
    } else {
        Err(StorageError::InvalidLocator(format!(
            "resize2fs {} failed: {}",
            path.display(),
            String::from_utf8_lossy(&out.stderr)
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_storage::{BackendInstanceId, SpdkLvolLocator, VolumeHandle, VolumeSnapshotHandle};
    use tokio::io::AsyncReadExt;
    use uuid::Uuid;

    #[test]
    fn socket_path_uses_controller_name_under_configured_dir() {
        let backend = SpdkLvolHostBackend::new("/run/spdk/rpc.sock", "/run/spdk/vhost");
        let p = backend.socket_for_controller("nq.abc");
        assert_eq!(p, PathBuf::from("/run/spdk/vhost/nq.abc"));
    }

    #[test]
    fn import_nbd_device_is_configurable() {
        let backend = SpdkLvolHostBackend::with_import_nbd_device(
            "/run/spdk/rpc.sock",
            "/run/spdk/vhost",
            "/dev/nbd7",
        );
        assert_eq!(backend.nbd_devices, vec![PathBuf::from("/dev/nbd7")]);
    }

    #[tokio::test]
    async fn nbd_pool_hands_out_unique_devices_then_reuses() {
        let pool = Arc::new(NbdDevicePool::new(vec![
            PathBuf::from("/dev/nbd0"),
            PathBuf::from("/dev/nbd1"),
        ]));

        let first = pool.acquire().await.unwrap();
        let second = pool.acquire().await.unwrap();
        assert_ne!(first.path(), second.path());
        let first_path = first.path().to_path_buf();
        drop(first);

        let third = pool.acquire().await.unwrap();
        assert_eq!(third.path(), first_path.as_path());
    }

    #[tokio::test]
    #[ignore = "requires a running SPDK process, pre-created lvol store, and loaded nbd module"]
    async fn spdk_lvol_real_smoke_create_import_snapshot_read_destroy() {
        let rpc_socket = std::env::var("AGENT_SPDK_IT_RPC_SOCKET")
            .or_else(|_| std::env::var("AGENT_SPDK_RPC_SOCKET"))
            .expect("set AGENT_SPDK_IT_RPC_SOCKET or AGENT_SPDK_RPC_SOCKET");
        let lvs_name = std::env::var("AGENT_SPDK_IT_LVS_NAME").expect("set AGENT_SPDK_IT_LVS_NAME");
        let vhost_socket_dir =
            std::env::var("AGENT_SPDK_IT_VHOST_SOCKET_DIR").unwrap_or_else(|_| "/var/tmp".into());
        let nbd_devices = std::env::var("AGENT_SPDK_IT_NBD_DEVICES")
            .unwrap_or_else(|_| "/dev/nbd0,/dev/nbd1".into())
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .collect::<Vec<_>>();

        let client = SpdkJsonRpcClient::new(&rpc_socket);
        let backend =
            SpdkLvolHostBackend::with_nbd_devices(&rpc_socket, vhost_socket_dir, nbd_devices);

        let volume_id = Uuid::new_v4();
        let volume_name = format!("nq-it-{}", volume_id.simple());
        let snapshot_id = Uuid::new_v4();
        let snapshot_name = format!("nq-it-snap-{}", snapshot_id.simple());
        let size_bytes = 16 * 1024 * 1024;
        let lvol_uuid = client
            .bdev_lvol_create(&lvs_name, &volume_name, size_bytes)
            .await
            .expect("create lvol");

        let volume_locator = SpdkLvolLocator {
            lvs_name: lvs_name.clone(),
            lvol_name: volume_name.clone(),
            lvol_uuid: lvol_uuid.clone(),
            size_bytes,
        };
        let volume = VolumeHandle {
            volume_id,
            backend_id: BackendInstanceId(Uuid::new_v4()),
            backend_kind: BackendKind::SpdkLvol,
            locator: volume_locator.to_locator_string().unwrap(),
            size_bytes,
        };

        let result = async {
            let dir = tempfile::tempdir().unwrap();
            let source = dir.path().join("source.raw");
            let payload = deterministic_payload(4 * 1024 * 1024);
            tokio::fs::write(&source, &payload).await.unwrap();

            let attached = backend.attach(&volume).await.expect("attach");
            backend
                .populate_streaming(&attached, &source, size_bytes)
                .await
                .expect("populate via NBD");

            let snap_uuid = client
                .bdev_lvol_snapshot(&lvol_uuid, &snapshot_name)
                .await
                .expect("snapshot");
            let snap_locator = SpdkLvolLocator {
                lvs_name: lvs_name.clone(),
                lvol_name: snapshot_name.clone(),
                lvol_uuid: snap_uuid.clone(),
                size_bytes,
            };
            let snap = VolumeSnapshotHandle {
                snapshot_id,
                source_volume_id: volume_id,
                backend_id: volume.backend_id,
                backend_kind: BackendKind::SpdkLvol,
                locator: snap_locator.to_locator_string().unwrap(),
            };

            let mut reader = backend.read_snapshot(&snap).await.expect("read snapshot");
            let mut got = vec![0u8; payload.len()];
            reader.read_exact(&mut got).await.expect("read payload");
            assert_eq!(got, payload);
            drop(reader);

            client
                .bdev_lvol_delete(&snap_uuid)
                .await
                .expect("delete snapshot");
            backend.detach(&volume, attached).await.expect("detach");
        }
        .await;

        let _ = client.bdev_lvol_delete(&lvol_uuid).await;
        result
    }

    fn deterministic_payload(len: usize) -> Vec<u8> {
        (0..len)
            .map(|i| ((i as u64 * 1103515245 + 12345) >> 16) as u8)
            .collect()
    }
}
