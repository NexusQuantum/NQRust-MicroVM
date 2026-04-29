use crate::error::StorageError;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpdkLvolLocator {
    pub lvs_name: String,
    pub lvol_name: String,
    pub lvol_uuid: String,
    pub size_bytes: u64,
}

impl SpdkLvolLocator {
    pub fn to_locator_string(&self) -> Result<String, StorageError> {
        serde_json::to_string(self).map_err(StorageError::backend)
    }

    pub fn from_locator_str(s: &str) -> Result<Self, StorageError> {
        serde_json::from_str(s).map_err(|e| StorageError::InvalidLocator(e.to_string()))
    }
}

#[derive(Debug, Clone)]
pub struct SpdkJsonRpcClient {
    socket: PathBuf,
}

impl SpdkJsonRpcClient {
    pub fn new(socket: impl Into<PathBuf>) -> Self {
        Self {
            socket: socket.into(),
        }
    }

    pub fn socket(&self) -> &Path {
        &self.socket
    }

    pub async fn bdev_lvol_create(
        &self,
        lvs_name: &str,
        lvol_name: &str,
        size_bytes: u64,
    ) -> Result<String, StorageError> {
        let size_in_mib = size_bytes.div_ceil(1024 * 1024).max(1);
        self.call(
            "bdev_lvol_create",
            json!({
                "lvs_name": lvs_name,
                "lvol_name": lvol_name,
                "size_in_mib": size_in_mib,
                "thin_provision": true,
                "clear_method": "unmap"
            }),
        )
        .await
    }

    pub async fn bdev_lvol_delete(&self, lvol_name: &str) -> Result<(), StorageError> {
        let _: Value = self
            .call("bdev_lvol_delete", json!({ "name": lvol_name }))
            .await?;
        Ok(())
    }

    pub async fn bdev_lvol_snapshot(
        &self,
        lvol_name: &str,
        snapshot_name: &str,
    ) -> Result<String, StorageError> {
        self.call(
            "bdev_lvol_snapshot",
            json!({
                "lvol_name": lvol_name,
                "snapshot_name": snapshot_name
            }),
        )
        .await
    }

    pub async fn bdev_lvol_clone(
        &self,
        snapshot_name: &str,
        clone_name: &str,
    ) -> Result<String, StorageError> {
        self.call(
            "bdev_lvol_clone",
            json!({
                "snapshot_name": snapshot_name,
                "clone_name": clone_name
            }),
        )
        .await
    }

    pub async fn vhost_create_blk_controller(
        &self,
        ctrlr: &str,
        dev_name: &str,
    ) -> Result<(), StorageError> {
        let _: Value = self
            .call(
                "vhost_create_blk_controller",
                json!({
                    "ctrlr": ctrlr,
                    "dev_name": dev_name
                }),
            )
            .await?;
        Ok(())
    }

    pub async fn vhost_delete_controller(&self, ctrlr: &str) -> Result<(), StorageError> {
        let _: Value = self
            .call("vhost_delete_controller", json!({ "ctrlr": ctrlr }))
            .await?;
        Ok(())
    }

    pub async fn nbd_start_disk(
        &self,
        bdev_name: &str,
        nbd_device: &Path,
    ) -> Result<PathBuf, StorageError> {
        let exported: String = self
            .call(
                "nbd_start_disk",
                json!({
                    "bdev_name": bdev_name,
                    "nbd_device": nbd_device
                }),
            )
            .await?;
        Ok(PathBuf::from(exported))
    }

    pub async fn nbd_stop_disk(&self, nbd_device: &Path) -> Result<(), StorageError> {
        let _: Value = self
            .call("nbd_stop_disk", json!({ "nbd_device": nbd_device }))
            .await?;
        Ok(())
    }

    async fn call<T: DeserializeOwned>(
        &self,
        method: &str,
        params: Value,
    ) -> Result<T, StorageError> {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let req = json!({
            "jsonrpc": "2.0",
            "method": method,
            "id": id,
            "params": params
        });

        let stream = UnixStream::connect(&self.socket).await?;
        let mut stream = BufReader::new(stream);
        let mut line = serde_json::to_vec(&req).map_err(StorageError::backend)?;
        line.push(b'\n');
        stream.get_mut().write_all(&line).await?;
        stream.get_mut().flush().await?;

        let mut response = String::new();
        stream.read_line(&mut response).await?;
        if response.trim().is_empty() {
            return Err(StorageError::InvalidLocator(format!(
                "empty SPDK JSON-RPC response for {method}"
            )));
        }

        let response: SpdkRpcResponse<T> =
            serde_json::from_str(&response).map_err(StorageError::backend)?;
        if let Some(error) = response.error {
            return Err(StorageError::InvalidLocator(format!(
                "SPDK {method} failed: code={} message={}",
                error.code, error.message
            )));
        }
        response.result.ok_or_else(|| {
            StorageError::InvalidLocator(format!("SPDK {method} response missing result"))
        })
    }
}

#[derive(Debug, Deserialize)]
struct SpdkRpcResponse<T> {
    result: Option<T>,
    error: Option<SpdkRpcError>,
}

#[derive(Debug, Deserialize)]
struct SpdkRpcError {
    code: i64,
    message: String,
}

pub fn spdk_vhost_controller_name(volume_id: uuid::Uuid) -> String {
    format!("nq.{}", volume_id.simple())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locator_round_trips_json() {
        let locator = SpdkLvolLocator {
            lvs_name: "nexus".into(),
            lvol_name: "vol-a".into(),
            lvol_uuid: "4f60".into(),
            size_bytes: 4096,
        };
        let encoded = locator.to_locator_string().unwrap();
        assert_eq!(
            SpdkLvolLocator::from_locator_str(&encoded).unwrap(),
            locator
        );
    }

    #[test]
    fn controller_name_is_stable_and_spdk_safe() {
        let id = uuid::Uuid::parse_str("018f64ba-97aa-70d9-a7d2-6459256fd111").unwrap();
        assert_eq!(
            spdk_vhost_controller_name(id),
            "nq.018f64ba97aa70d9a7d26459256fd111"
        );
    }
}
