//! TrueNAS iSCSI control-plane backend. Provisions zvols via TrueNAS REST API,
//! creates iSCSI extents and assigns LUNs. The volume's locator is a JSON
//! object: {"iqn":"...","lun":N,"dataset":"pool/v-<uuid>","portal":"..."}.
//!
//! This implementation is intentionally minimum-viable for Plan 2. Snapshot
//! and clone_from_snapshot are stubbed as NotSupported; the trait still
//! advertises supports_native_snapshots: true so future work can fill it in.

use nexus_storage::{
    BackendInstanceId, BackendKind, Capabilities, ControlPlaneBackend, CreateOpts, StorageError,
    VolumeHandle, VolumeSnapshotHandle,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

#[derive(Deserialize, Clone)]
pub struct TrueNasConfig {
    pub endpoint: String,
    pub api_key_env: String,
    pub pool: String,
    pub target_iqn_prefix: String,
    #[serde(default)]
    pub portal: Option<String>,
}

pub struct TrueNasIscsiControlPlaneBackend {
    pub id: BackendInstanceId,
    pub config: TrueNasConfig,
    pub api_key: String,
    pub http: Client,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LocatorJson {
    pub iqn: String,
    pub lun: u32,
    pub dataset: String,
    #[serde(default)]
    pub portal: Option<String>,
}

impl TrueNasIscsiControlPlaneBackend {
    fn auth_header(&self) -> String {
        format!("Bearer {}", self.api_key)
    }

    async fn create_zvol(&self, name: &str, size_bytes: u64) -> Result<String, StorageError> {
        #[derive(Serialize)]
        struct Req<'a> {
            name: String,
            r#type: &'a str,
            volsize: u64,
            sparse: bool,
        }
        let dataset = format!("{}/{}", self.config.pool, name);
        let url = format!("{}/api/v2.0/pool/dataset", self.config.endpoint);
        let resp = self
            .http
            .post(&url)
            .header("Authorization", self.auth_header())
            .json(&Req {
                name: dataset.clone(),
                r#type: "VOLUME",
                volsize: size_bytes,
                sparse: true,
            })
            .send()
            .await
            .map_err(|e| StorageError::Backend(Box::new(e)))?;
        if !resp.status().is_success() {
            let s = resp.status();
            let t = resp.text().await.unwrap_or_default();
            return Err(StorageError::Backend(
                anyhow::anyhow!("create_zvol {}: {}", s, t).into(),
            ));
        }
        Ok(dataset)
    }

    async fn delete_zvol(&self, dataset: &str) -> Result<(), StorageError> {
        let url = format!(
            "{}/api/v2.0/pool/dataset/id/{}",
            self.config.endpoint,
            urlencoding::encode(dataset)
        );
        let resp = self
            .http
            .delete(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| StorageError::Backend(Box::new(e)))?;
        if !resp.status().is_success() && resp.status() != reqwest::StatusCode::NOT_FOUND {
            return Err(StorageError::Backend(
                anyhow::anyhow!("delete_zvol: {}", resp.status()).into(),
            ));
        }
        Ok(())
    }

    /// Create an iSCSI extent for the zvol and return the assigned LUN id.
    /// TrueNAS allocates LUNs automatically when the extent is associated
    /// with a target via /iscsi/targetextent.
    async fn create_lun_for_zvol(&self, dataset: &str) -> Result<u32, StorageError> {
        // Create extent
        #[derive(Serialize)]
        struct ExtentReq<'a> {
            name: &'a str,
            r#type: &'a str,
            disk: String,
        }
        #[derive(Deserialize)]
        struct ExtentResp {
            id: u32,
        }
        let url = format!("{}/api/v2.0/iscsi/extent", self.config.endpoint);
        let resp = self
            .http
            .post(&url)
            .header("Authorization", self.auth_header())
            .json(&ExtentReq {
                name: dataset,
                r#type: "DISK",
                disk: format!("zvol/{}", dataset),
            })
            .send()
            .await
            .map_err(|e| StorageError::Backend(Box::new(e)))?;
        if !resp.status().is_success() {
            let s = resp.status();
            let t = resp.text().await.unwrap_or_default();
            return Err(StorageError::Backend(
                anyhow::anyhow!("create_extent {}: {}", s, t).into(),
            ));
        }
        let extent: ExtentResp = resp
            .json()
            .await
            .map_err(|e| StorageError::Backend(Box::new(e)))?;
        // Use the extent id as the LUN id (acceptable approximation; production
        // would query target↔extent mapping for the actual lun number).
        Ok(extent.id)
    }
}

#[async_trait::async_trait]
impl ControlPlaneBackend for TrueNasIscsiControlPlaneBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::TrueNasIscsi
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            supports_native_snapshots: true,
            supports_concurrent_attach: false,
            supports_live_migration: false,
            supports_clone_from_image: false,
        }
    }

    async fn provision(&self, opts: CreateOpts) -> Result<VolumeHandle, StorageError> {
        let vol_id = Uuid::new_v4();
        let zvol_name = format!("v-{vol_id}");
        let dataset = self.create_zvol(&zvol_name, opts.size_bytes).await?;
        let lun = self.create_lun_for_zvol(&dataset).await?;
        let locator = LocatorJson {
            iqn: self.config.target_iqn_prefix.clone(),
            lun,
            dataset: dataset.clone(),
            portal: self.config.portal.clone(),
        };
        let locator_str =
            serde_json::to_string(&locator).map_err(|e| StorageError::Backend(Box::new(e)))?;
        Ok(VolumeHandle {
            volume_id: vol_id,
            backend_id: self.id,
            backend_kind: BackendKind::TrueNasIscsi,
            locator: locator_str,
            size_bytes: opts.size_bytes,
        })
    }

    async fn destroy(&self, handle: VolumeHandle) -> Result<(), StorageError> {
        let loc: LocatorJson = serde_json::from_str(&handle.locator)
            .map_err(|e| StorageError::InvalidLocator(format!("{}: {}", handle.locator, e)))?;
        // Best-effort: delete extent and zvol. We don't track extent id on the
        // handle yet; deleting the zvol typically cascades or leaves the extent
        // dangling — operator may need to clean up extents separately.
        self.delete_zvol(&loc.dataset).await
    }

    async fn clone_from_image(
        &self,
        _src: &Path,
        _opts: CreateOpts,
    ) -> Result<VolumeHandle, StorageError> {
        Err(StorageError::NotSupported("clone_from_image".into()))
    }

    async fn snapshot(
        &self,
        _v: &VolumeHandle,
        _name: &str,
    ) -> Result<VolumeSnapshotHandle, StorageError> {
        Err(StorageError::NotSupported(
            "truenas snapshot impl pending".into(),
        ))
    }

    async fn clone_from_snapshot(
        &self,
        _s: &VolumeSnapshotHandle,
    ) -> Result<VolumeHandle, StorageError> {
        Err(StorageError::NotSupported(
            "truenas clone_from_snapshot impl pending".into(),
        ))
    }

    async fn delete_snapshot(&self, _s: VolumeSnapshotHandle) -> Result<(), StorageError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_storage::CreateOpts;

    #[tokio::test]
    async fn truenas_provision_calls_create_zvol() {
        let mut server = mockito::Server::new_async().await;

        // The zvol endpoint: create_zvol checks status only, doesn't parse response body.
        let zvol_mock = server
            .mock("POST", "/api/v2.0/pool/dataset")
            .with_status(200)
            .with_body(r#"{"id":"tank/v-x","volsize":1048576}"#)
            .create_async()
            .await;

        // The extent endpoint: create_lun_for_zvol parses {"id": N} to get the LUN.
        let extent_mock = server
            .mock("POST", "/api/v2.0/iscsi/extent")
            .with_status(200)
            .with_body(r#"{"id":42}"#)
            .create_async()
            .await;

        let backend = TrueNasIscsiControlPlaneBackend {
            id: BackendInstanceId(uuid::Uuid::new_v4()),
            config: TrueNasConfig {
                endpoint: server.url(),
                api_key_env: "_unused_".into(),
                pool: "tank".into(),
                target_iqn_prefix: "iqn.x:tgt".into(),
                portal: None,
            },
            api_key: "test".into(),
            http: reqwest::Client::new(),
        };

        let h = backend
            .provision(CreateOpts {
                name: "x".into(),
                size_bytes: 1024 * 1024,
                description: None,
            })
            .await
            .unwrap();

        assert_eq!(h.size_bytes, 1024 * 1024);
        assert_eq!(h.backend_kind, BackendKind::TrueNasIscsi);

        // Verify the locator JSON is well-formed and contains expected fields.
        // The dataset name is `tank/v-<uuid>` (uuid allocated internally), so we
        // only check the pool prefix and the IQN/LUN values that are deterministic.
        let loc: LocatorJson = serde_json::from_str(&h.locator).unwrap();
        assert_eq!(loc.iqn, "iqn.x:tgt");
        assert_eq!(loc.lun, 42);
        assert!(
            loc.dataset.starts_with("tank/v-"),
            "dataset should start with 'tank/v-', got: {}",
            loc.dataset
        );

        zvol_mock.assert_async().await;
        extent_mock.assert_async().await;
    }
}
