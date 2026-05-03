//! TrueNAS iSCSI control-plane backend. Provisions one **target per
//! volume** (Harvester-CSI style) so the operator never has to pre-create
//! anything on TrueNAS — they only configure the endpoint, API key, and
//! pool. Per-volume targets give clean isolation, simple per-target ACLs,
//! straightforward cleanup, and TrueNAS-UI clarity (one row per volume).
//!
//! Flow per provision (4 REST calls):
//! 1. `POST /api/v2.0/pool/dataset`      — create the zvol
//! 2. `POST /api/v2.0/iscsi/extent`      — create the extent backed by the zvol
//! 3. `POST /api/v2.0/iscsi/target`      — create the per-volume target
//! 4. `POST /api/v2.0/iscsi/targetextent` — associate target + extent at LUN 0
//!
//! Destroy reverses the order. The locator stores all four ids so destroy
//! can clean up cleanly.

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
    /// Optional iSCSI portal `<ip>:<port>` recorded in the locator so the
    /// agent can pass it to `iscsiadm -p`. If unset, the agent uses the
    /// portal it discovers via `iscsiadm -m discovery`.
    #[serde(default)]
    pub portal: Option<String>,
    /// Optional prefix for per-volume target names. Final TrueNAS target
    /// name is `<target_name_prefix><uuid>`. Default is `nqrust-v-`. The
    /// full IQN is computed by TrueNAS as `<global.basename>:<name>` so
    /// operators see e.g. `iqn.2005-10.org.freenas.ctl:nqrust-v-<uuid>`.
    #[serde(default)]
    pub target_name_prefix: Option<String>,
    /// Optional iSCSI portal-group id to associate the target with. If
    /// unset, the per-volume target is created with an empty `groups`
    /// array — the operator must associate a portal group via the
    /// TrueNAS UI before clients can connect. Set this to the id of an
    /// existing portal group to make provisioning fully hands-off.
    #[serde(default)]
    pub portal_group_id: Option<u32>,
    /// Optional iSCSI initiator-group id to associate the target with.
    /// Same semantics as `portal_group_id` — set it to make
    /// provisioning fully hands-off; leave unset and the operator must
    /// add an initiator group via the UI.
    #[serde(default)]
    pub initiator_group_id: Option<u32>,
}

impl TrueNasConfig {
    fn target_name_prefix(&self) -> &str {
        self.target_name_prefix.as_deref().unwrap_or("nqrust-v-")
    }
}

pub struct TrueNasIscsiControlPlaneBackend {
    pub id: BackendInstanceId,
    pub config: TrueNasConfig,
    pub api_key: String,
    pub http: Client,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct LocatorJson {
    pub iqn: String,
    pub lun: u32,
    pub dataset: String,
    /// TrueNAS extent id, recorded so destroy can delete cleanly.
    pub extent_id: u32,
    /// TrueNAS target id, recorded so destroy can delete cleanly.
    pub target_id: u32,
    /// TrueNAS targetextent (association) id.
    pub targetextent_id: u32,
    #[serde(default)]
    pub portal: Option<String>,
}

impl TrueNasIscsiControlPlaneBackend {
    fn auth_header(&self) -> String {
        format!("Bearer {}", self.api_key)
    }

    /// Read TrueNAS's global iSCSI `basename`. The full IQN of any target
    /// is `<basename>:<target.name>`.
    async fn iscsi_basename(&self) -> Result<String, StorageError> {
        #[derive(Deserialize)]
        struct GlobalResp {
            basename: String,
        }
        let url = format!("{}/api/v2.0/iscsi/global", self.config.endpoint);
        let resp = self
            .http
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(StorageError::backend)?;
        if !resp.status().is_success() {
            let s = resp.status();
            let t = resp.text().await.unwrap_or_default();
            return Err(StorageError::backend(std::io::Error::other(format!(
                "iscsi/global GET {s}: {t}"
            ))));
        }
        let g: GlobalResp = resp.json().await.map_err(StorageError::backend)?;
        Ok(g.basename)
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
            .map_err(StorageError::backend)?;
        if !resp.status().is_success() {
            let s = resp.status();
            let t = resp.text().await.unwrap_or_default();
            return Err(StorageError::backend(std::io::Error::other(format!(
                "create_zvol {s}: {t}"
            ))));
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
            .map_err(StorageError::backend)?;
        if !resp.status().is_success() && resp.status() != reqwest::StatusCode::NOT_FOUND {
            return Err(StorageError::backend(std::io::Error::other(format!(
                "delete_zvol: {}",
                resp.status()
            ))));
        }
        Ok(())
    }

    async fn create_extent(&self, dataset: &str) -> Result<u32, StorageError> {
        #[derive(Serialize)]
        struct Req<'a> {
            name: &'a str,
            r#type: &'a str,
            disk: String,
        }
        #[derive(Deserialize)]
        struct Resp {
            id: u32,
        }
        let url = format!("{}/api/v2.0/iscsi/extent", self.config.endpoint);
        let resp = self
            .http
            .post(&url)
            .header("Authorization", self.auth_header())
            .json(&Req {
                name: dataset,
                r#type: "DISK",
                disk: format!("zvol/{dataset}"),
            })
            .send()
            .await
            .map_err(StorageError::backend)?;
        if !resp.status().is_success() {
            let s = resp.status();
            let t = resp.text().await.unwrap_or_default();
            return Err(StorageError::backend(std::io::Error::other(format!(
                "create_extent {s}: {t}"
            ))));
        }
        let r: Resp = resp.json().await.map_err(StorageError::backend)?;
        Ok(r.id)
    }

    async fn delete_extent(&self, extent_id: u32) -> Result<(), StorageError> {
        let url = format!(
            "{}/api/v2.0/iscsi/extent/id/{extent_id}",
            self.config.endpoint
        );
        let resp = self
            .http
            .delete(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(StorageError::backend)?;
        if !resp.status().is_success() && resp.status() != reqwest::StatusCode::NOT_FOUND {
            return Err(StorageError::backend(std::io::Error::other(format!(
                "delete_extent: {}",
                resp.status()
            ))));
        }
        Ok(())
    }

    async fn create_target(&self, target_name: &str) -> Result<u32, StorageError> {
        #[derive(Serialize)]
        struct GroupRef {
            portal: Option<u32>,
            initiator: Option<u32>,
            authmethod: &'static str,
            auth: Option<u32>,
        }
        #[derive(Serialize)]
        struct Req<'a> {
            name: &'a str,
            mode: &'a str,
            groups: Vec<GroupRef>,
        }
        #[derive(Deserialize)]
        struct Resp {
            id: u32,
        }
        // If the operator configured a portal group, build a single
        // groups entry for it. Without one, TrueNAS accepts an empty
        // `groups` array but the target is unreachable until the
        // operator adds one via the UI.
        let groups: Vec<GroupRef> =
            if self.config.portal_group_id.is_some() || self.config.initiator_group_id.is_some() {
                vec![GroupRef {
                    portal: self.config.portal_group_id,
                    initiator: self.config.initiator_group_id,
                    authmethod: "NONE",
                    auth: None,
                }]
            } else {
                Vec::new()
            };
        let url = format!("{}/api/v2.0/iscsi/target", self.config.endpoint);
        let resp = self
            .http
            .post(&url)
            .header("Authorization", self.auth_header())
            .json(&Req {
                name: target_name,
                mode: "ISCSI",
                groups,
            })
            .send()
            .await
            .map_err(StorageError::backend)?;
        if !resp.status().is_success() {
            let s = resp.status();
            let t = resp.text().await.unwrap_or_default();
            return Err(StorageError::backend(std::io::Error::other(format!(
                "create_target {s}: {t}"
            ))));
        }
        let r: Resp = resp.json().await.map_err(StorageError::backend)?;
        Ok(r.id)
    }

    async fn delete_target(&self, target_id: u32) -> Result<(), StorageError> {
        let url = format!(
            "{}/api/v2.0/iscsi/target/id/{target_id}",
            self.config.endpoint
        );
        let resp = self
            .http
            .delete(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(StorageError::backend)?;
        if !resp.status().is_success() && resp.status() != reqwest::StatusCode::NOT_FOUND {
            return Err(StorageError::backend(std::io::Error::other(format!(
                "delete_target: {}",
                resp.status()
            ))));
        }
        Ok(())
    }

    async fn associate_target_extent(
        &self,
        target_id: u32,
        extent_id: u32,
    ) -> Result<u32, StorageError> {
        #[derive(Serialize)]
        struct Req {
            target: u32,
            extent: u32,
            lunid: u32,
        }
        #[derive(Deserialize)]
        struct Resp {
            id: u32,
        }
        let url = format!("{}/api/v2.0/iscsi/targetextent", self.config.endpoint);
        let resp = self
            .http
            .post(&url)
            .header("Authorization", self.auth_header())
            .json(&Req {
                target: target_id,
                extent: extent_id,
                lunid: 0,
            })
            .send()
            .await
            .map_err(StorageError::backend)?;
        if !resp.status().is_success() {
            let s = resp.status();
            let t = resp.text().await.unwrap_or_default();
            return Err(StorageError::backend(std::io::Error::other(format!(
                "create_targetextent {s}: {t}"
            ))));
        }
        let r: Resp = resp.json().await.map_err(StorageError::backend)?;
        Ok(r.id)
    }

    async fn delete_target_extent(&self, te_id: u32) -> Result<(), StorageError> {
        let url = format!(
            "{}/api/v2.0/iscsi/targetextent/id/{te_id}",
            self.config.endpoint
        );
        let resp = self
            .http
            .delete(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(StorageError::backend)?;
        if !resp.status().is_success() && resp.status() != reqwest::StatusCode::NOT_FOUND {
            return Err(StorageError::backend(std::io::Error::other(format!(
                "delete_targetextent: {}",
                resp.status()
            ))));
        }
        Ok(())
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
        let target_name = format!("{}{vol_id}", self.config.target_name_prefix());

        // Read TrueNAS basename so we can store the full IQN in the
        // locator. One extra REST call per provision; acceptable. A
        // future optimization can cache it on the backend struct.
        let basename = self.iscsi_basename().await?;
        let iqn = format!("{basename}:{target_name}");

        // 1. zvol
        let dataset = self.create_zvol(&zvol_name, opts.size_bytes).await?;

        // 2. extent — rollback zvol on failure.
        let extent_id = match self.create_extent(&dataset).await {
            Ok(id) => id,
            Err(e) => {
                let _ = self.delete_zvol(&dataset).await;
                return Err(e);
            }
        };

        // 3. target — rollback extent + zvol on failure.
        let target_id = match self.create_target(&target_name).await {
            Ok(id) => id,
            Err(e) => {
                let _ = self.delete_extent(extent_id).await;
                let _ = self.delete_zvol(&dataset).await;
                return Err(e);
            }
        };

        // 4. associate — rollback target + extent + zvol on failure.
        let targetextent_id = match self.associate_target_extent(target_id, extent_id).await {
            Ok(id) => id,
            Err(e) => {
                let _ = self.delete_target(target_id).await;
                let _ = self.delete_extent(extent_id).await;
                let _ = self.delete_zvol(&dataset).await;
                return Err(e);
            }
        };

        let locator = LocatorJson {
            iqn,
            lun: 0,
            dataset,
            extent_id,
            target_id,
            targetextent_id,
            portal: self.config.portal.clone(),
        };
        let locator_str = serde_json::to_string(&locator).map_err(StorageError::backend)?;
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
        // Reverse order. Each step is best-effort: if a downstream object
        // is already gone (404) we treat that as success and continue.
        let _ = self.delete_target_extent(loc.targetextent_id).await;
        let _ = self.delete_target(loc.target_id).await;
        let _ = self.delete_extent(loc.extent_id).await;
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

    fn backend_with(server_url: &str) -> TrueNasIscsiControlPlaneBackend {
        TrueNasIscsiControlPlaneBackend {
            id: BackendInstanceId(uuid::Uuid::new_v4()),
            config: TrueNasConfig {
                endpoint: server_url.to_string(),
                api_key_env: "_unused_".into(),
                pool: "tank".into(),
                portal: None,
                target_name_prefix: None,
                portal_group_id: Some(1),
                initiator_group_id: Some(1),
            },
            api_key: "test".into(),
            http: reqwest::Client::new(),
        }
    }

    #[tokio::test]
    async fn provision_creates_zvol_extent_target_and_association() {
        let mut server = mockito::Server::new_async().await;

        let global_mock = server
            .mock("GET", "/api/v2.0/iscsi/global")
            .with_status(200)
            .with_body(r#"{"basename":"iqn.2005-10.org.freenas.ctl"}"#)
            .create_async()
            .await;

        let zvol_mock = server
            .mock("POST", "/api/v2.0/pool/dataset")
            .with_status(200)
            .with_body(r#"{"id":"tank/v-x","volsize":1048576}"#)
            .create_async()
            .await;

        let extent_mock = server
            .mock("POST", "/api/v2.0/iscsi/extent")
            .with_status(200)
            .with_body(r#"{"id":42}"#)
            .create_async()
            .await;

        let target_mock = server
            .mock("POST", "/api/v2.0/iscsi/target")
            .with_status(200)
            .with_body(r#"{"id":7}"#)
            .create_async()
            .await;

        let assoc_mock = server
            .mock("POST", "/api/v2.0/iscsi/targetextent")
            .with_status(200)
            .with_body(r#"{"id":99}"#)
            .create_async()
            .await;

        let backend = backend_with(&server.url());
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

        let loc: LocatorJson = serde_json::from_str(&h.locator).unwrap();
        assert!(
            loc.iqn.starts_with("iqn.2005-10.org.freenas.ctl:nqrust-v-"),
            "iqn: {}",
            loc.iqn
        );
        assert_eq!(loc.lun, 0);
        assert!(
            loc.dataset.starts_with("tank/v-"),
            "dataset: {}",
            loc.dataset
        );
        assert_eq!(loc.extent_id, 42);
        assert_eq!(loc.target_id, 7);
        assert_eq!(loc.targetextent_id, 99);

        global_mock.assert_async().await;
        zvol_mock.assert_async().await;
        extent_mock.assert_async().await;
        target_mock.assert_async().await;
        assoc_mock.assert_async().await;
    }

    #[tokio::test]
    async fn provision_rolls_back_zvol_when_extent_fails() {
        let mut server = mockito::Server::new_async().await;

        server
            .mock("GET", "/api/v2.0/iscsi/global")
            .with_status(200)
            .with_body(r#"{"basename":"iqn.x"}"#)
            .create_async()
            .await;

        server
            .mock("POST", "/api/v2.0/pool/dataset")
            .with_status(200)
            .with_body(r#"{"id":"tank/v-x"}"#)
            .create_async()
            .await;

        server
            .mock("POST", "/api/v2.0/iscsi/extent")
            .with_status(500)
            .with_body("boom")
            .create_async()
            .await;

        // The rollback delete on the zvol must hit this endpoint.
        let cleanup_mock = server
            .mock(
                "DELETE",
                mockito::Matcher::Regex(r"/api/v2.0/pool/dataset/id/.*".into()),
            )
            .with_status(200)
            .expect_at_least(1)
            .create_async()
            .await;

        let backend = backend_with(&server.url());
        let err = backend
            .provision(CreateOpts {
                name: "x".into(),
                size_bytes: 1024,
                description: None,
            })
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("create_extent"),
            "expected create_extent error, got: {err}"
        );

        cleanup_mock.assert_async().await;
    }

    #[tokio::test]
    async fn destroy_walks_back_targetextent_target_extent_zvol() {
        let mut server = mockito::Server::new_async().await;

        let te_mock = server
            .mock("DELETE", "/api/v2.0/iscsi/targetextent/id/99")
            .with_status(200)
            .create_async()
            .await;
        let target_mock = server
            .mock("DELETE", "/api/v2.0/iscsi/target/id/7")
            .with_status(200)
            .create_async()
            .await;
        let extent_mock = server
            .mock("DELETE", "/api/v2.0/iscsi/extent/id/42")
            .with_status(200)
            .create_async()
            .await;
        let zvol_mock = server
            .mock(
                "DELETE",
                mockito::Matcher::Regex(r"/api/v2.0/pool/dataset/id/.*".into()),
            )
            .with_status(200)
            .create_async()
            .await;

        let backend = backend_with(&server.url());
        let locator = LocatorJson {
            iqn: "iqn.x:nqrust-v-zzz".into(),
            lun: 0,
            dataset: "tank/v-zzz".into(),
            extent_id: 42,
            target_id: 7,
            targetextent_id: 99,
            portal: None,
        };
        let h = VolumeHandle {
            volume_id: uuid::Uuid::new_v4(),
            backend_id: backend.id,
            backend_kind: BackendKind::TrueNasIscsi,
            locator: serde_json::to_string(&locator).unwrap(),
            size_bytes: 1024,
        };
        backend.destroy(h).await.expect("destroy");

        te_mock.assert_async().await;
        target_mock.assert_async().await;
        extent_mock.assert_async().await;
        zvol_mock.assert_async().await;
    }
}
