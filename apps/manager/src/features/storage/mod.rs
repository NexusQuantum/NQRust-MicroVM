use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tokio::fs;
use uuid::Uuid;

#[derive(Clone)]
pub struct LocalStorage {
    base: PathBuf,
}

impl LocalStorage {
    pub fn new() -> Self {
        let base = std::env::var("MANAGER_STORAGE_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/srv/fc/vms"));
        Self { base }
    }

    pub async fn init(&self) -> Result<()> {
        fs::create_dir_all(&self.base).await?;
        Ok(())
    }

    pub fn vm_dir(&self, vm_id: Uuid) -> PathBuf {
        self.base.join(vm_id.to_string())
    }

    pub async fn ensure_vm_dirs(&self, vm_id: Uuid) -> Result<()> {
        let dir = self.vm_dir(vm_id);
        fs::create_dir_all(dir.join("logs")).await?;
        fs::create_dir_all(dir.join("storage")).await?;
        fs::create_dir_all(dir.join("snapshots")).await?;
        fs::create_dir_all(dir.join("sock")).await?;
        Ok(())
    }

    pub async fn alloc_rootfs(&self, vm_id: Uuid, src: &Path) -> Result<String> {
        let target_dir = self.vm_dir(vm_id).join("storage");
        fs::create_dir_all(&target_dir).await?;
        let ext = src
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| format!(".{s}"))
            .unwrap_or_else(|| "".to_string());
        let target = target_dir.join(format!("rootfs-{uid}{ext}", uid = Uuid::new_v4()));
        fs::copy(src, &target)
            .await
            .with_context(|| format!("failed to copy rootfs {:?} -> {:?}", src, target))?;
        Ok(target.display().to_string())
    }

    pub async fn alloc_data_disk(&self, vm_id: Uuid, size_bytes: u64) -> Result<String> {
        let target_dir = self.vm_dir(vm_id).join("storage");
        fs::create_dir_all(&target_dir).await?;
        let target = target_dir.join(format!("disk-{uid}.img", uid = Uuid::new_v4()));
        let file = tokio::fs::File::create(&target)
            .await
            .with_context(|| format!("failed to create disk file {:?}", target))?;
        file.set_len(size_bytes)
            .await
            .with_context(|| format!("failed to size disk {:?}", target))?;
        Ok(target.display().to_string())
    }

    pub fn sock_path(&self, vm_id: Uuid) -> String {
        self.vm_dir(vm_id)
            .join("sock/fc.sock")
            .display()
            .to_string()
    }

    pub fn log_path(&self, vm_id: Uuid) -> String {
        self.vm_dir(vm_id)
            .join("logs/firecracker.log")
            .display()
            .to_string()
    }

    pub fn metrics_path(&self, vm_id: Uuid) -> String {
        self.vm_dir(vm_id)
            .join("logs/metrics.json")
            .display()
            .to_string()
    }

    pub fn snapshot_dir(&self, vm_id: Uuid, snapshot_id: Uuid) -> PathBuf {
        self.vm_dir(vm_id)
            .join("snapshots")
            .join(snapshot_id.to_string())
    }

    pub async fn cleanup_vm(&self, vm_id: Uuid) -> Result<()> {
        let dir = self.vm_dir(vm_id);
        if dir.exists() {
            fs::remove_dir_all(&dir)
                .await
                .with_context(|| format!("failed to cleanup storage for vm {vm_id}"))?;
        }
        Ok(())
    }
}
