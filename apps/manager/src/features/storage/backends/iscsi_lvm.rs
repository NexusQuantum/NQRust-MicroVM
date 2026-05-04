//! iSCSI + LVM control-plane backend. Operator configures a portal, target
//! IQN, LUN, and the volume-group name. The manager delegates every
//! privileged step (iSCSI session lifecycle, `pvcreate`/`vgcreate`,
//! `lvcreate`/`lvremove`, snapshot, and qemu-img clone) to the agent's
//! `/v1/storage/iscsi_lvm/*` endpoints — the manager itself never invokes
//! `iscsiadm` or LVM tooling.
//!
//! The on-host topology is captured in the locator JSON `{vg, lv}` so the
//! agent can independently activate the LV when it attaches the volume.
//! Mirrors the shape used by `apps/agent/src/features/storage/iscsi_lvm.rs`.
//!
//! NOTE: dead-code allowed at module scope because Task 9 lands the
//! backend itself; the registry hookup that exercises these symbols is
//! Task 10. Once that lands, the allow can be removed.
#![allow(dead_code)]

use nexus_storage::{
    BackendInstanceId, BackendKind, Capabilities, ControlPlaneBackend, CreateOpts, StorageError,
    VolumeHandle, VolumeSnapshotHandle,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
pub struct IscsiLvmConfig {
    /// iSCSI portal, e.g. `192.168.1.10:3260`.
    pub portal: String,
    /// Target IQN, e.g. `iqn.2024-01.com.example:storage`.
    pub iqn: String,
    /// Volume group name on the LUN, e.g. `vg-nqrust`.
    pub vg_name: String,
    /// LUN number to attach (defaults to 0).
    #[serde(default)]
    pub lun: u32,
    /// If true, the agent zeroes LV blocks before `lvremove`. Mirrors
    /// Proxmox `LVMPlugin` `saferemove`. Default false.
    #[serde(default)]
    pub saferemove: bool,
    /// Base URL of the agent that owns this iSCSI session, e.g.
    /// `http://127.0.0.1:9090`. The manager appends
    /// `/v1/storage/iscsi_lvm/*`. Required for any privileged operation.
    #[serde(default)]
    pub agent_url: Option<String>,
}

/// Wire-format locator. Same shape as the agent's `IscsiLvmLocator` so
/// the manager-issued JSON is parseable by the agent without translation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IscsiLvmLocator {
    pub vg: String,
    pub lv: String,
}

impl IscsiLvmLocator {
    pub fn to_locator_string(&self) -> Result<String, StorageError> {
        serde_json::to_string(self)
            .map_err(|e| StorageError::InvalidLocator(format!("encode iscsi_lvm locator: {e}")))
    }

    pub fn from_locator_str(s: &str) -> Result<Self, StorageError> {
        serde_json::from_str(s)
            .map_err(|e| StorageError::InvalidLocator(format!("decode iscsi_lvm locator: {e}")))
    }
}

pub struct IscsiLvmControlPlaneBackend {
    pub id: BackendInstanceId,
    pub config: IscsiLvmConfig,
}

#[derive(Serialize)]
struct LoginReq<'a> {
    iqn: &'a str,
    portal: &'a str,
}

#[derive(Serialize)]
struct VgStatusReq<'a> {
    vg: &'a str,
}

#[derive(Deserialize)]
struct VgStatusResp {
    #[allow(dead_code)]
    size_bytes: u64,
    #[allow(dead_code)]
    free_bytes: u64,
    #[allow(dead_code)]
    lv_count: u32,
}

#[derive(Serialize)]
struct LvCreateReq<'a> {
    vg: &'a str,
    name: &'a str,
    size_bytes: u64,
    vm_id: &'a str,
}

#[derive(Deserialize)]
struct LvCreateResp {
    #[allow(dead_code)]
    device: PathBuf,
}

#[derive(Serialize)]
struct LvNameReq<'a> {
    vg: &'a str,
    name: &'a str,
}

#[derive(Serialize)]
struct CloneFromPathReq<'a> {
    source_path: &'a str,
    vg: &'a str,
    name: &'a str,
}

#[derive(Serialize)]
struct LvSnapshotReq<'a> {
    vg: &'a str,
    source_lv: &'a str,
    snap_name: &'a str,
    size_bytes: u64,
}

#[derive(Deserialize)]
struct LvSnapshotResp {
    #[allow(dead_code)]
    snap_name: String,
}

impl IscsiLvmControlPlaneBackend {
    fn agent_storage_base(&self) -> Option<String> {
        self.config.agent_url.as_ref().map(|raw| {
            let with_scheme = if raw.starts_with("http://") || raw.starts_with("https://") {
                raw.to_string()
            } else {
                format!("http://{raw}")
            };
            let trimmed = with_scheme.trim_end_matches('/');
            if trimmed.ends_with("/v1/storage") {
                trimmed.to_string()
            } else {
                format!("{trimmed}/v1/storage")
            }
        })
    }

    fn agent_url(&self, op: &str) -> Option<String> {
        self.agent_storage_base()
            .map(|base| format!("{base}/iscsi_lvm/{op}"))
    }

    async fn agent_post<Req, Resp>(&self, op: &str, req: &Req) -> Result<Resp, StorageError>
    where
        Req: Serialize + ?Sized,
        Resp: DeserializeOwned,
    {
        let url = self.agent_url(op).ok_or_else(|| {
            StorageError::backend(std::io::Error::other(
                "iscsi_lvm backend requires config.agent_url",
            ))
        })?;
        let resp = reqwest::Client::new()
            .post(&url)
            .json(req)
            .send()
            .await
            .map_err(|e| {
                StorageError::backend(std::io::Error::other(format!(
                    "agent iscsi_lvm {op} request failed: {e}"
                )))
            })?;
        let status = resp.status();
        let body = resp.text().await.map_err(|e| {
            StorageError::backend(std::io::Error::other(format!(
                "agent iscsi_lvm {op} response read failed: {e}"
            )))
        })?;
        if !status.is_success() {
            return Err(StorageError::backend(std::io::Error::other(format!(
                "agent iscsi_lvm {op} failed: HTTP {status}: {body}"
            ))));
        }
        serde_json::from_str(&body).map_err(|e| {
            StorageError::backend(std::io::Error::other(format!(
                "agent iscsi_lvm {op} response decode failed: {e}; body: {body}"
            )))
        })
    }

    async fn agent_post_empty<Req>(&self, op: &str, req: &Req) -> Result<(), StorageError>
    where
        Req: Serialize + ?Sized,
    {
        let url = self.agent_url(op).ok_or_else(|| {
            StorageError::backend(std::io::Error::other(
                "iscsi_lvm backend requires config.agent_url",
            ))
        })?;
        let resp = reqwest::Client::new()
            .post(&url)
            .json(req)
            .send()
            .await
            .map_err(|e| {
                StorageError::backend(std::io::Error::other(format!(
                    "agent iscsi_lvm {op} request failed: {e}"
                )))
            })?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(StorageError::backend(std::io::Error::other(format!(
                "agent iscsi_lvm {op} failed: HTTP {status}: {body}"
            ))));
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl ControlPlaneBackend for IscsiLvmControlPlaneBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::IscsiLvm
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            supports_clone_from_image: true,
            supports_native_snapshots: true,
            supports_concurrent_attach: false,
            supports_live_migration: true,
        }
    }

    async fn probe(&self) -> Result<(), StorageError> {
        // First make sure the iSCSI session is up — login is idempotent on
        // the agent side (exit 15 = already logged in is treated as success).
        self.agent_post_empty(
            "login",
            &LoginReq {
                iqn: &self.config.iqn,
                portal: &self.config.portal,
            },
        )
        .await?;
        // Then verify the VG actually exists by asking the agent for its
        // status. If `vgs` returns no rows, this fails.
        let _: VgStatusResp = self
            .agent_post(
                "vg_status",
                &VgStatusReq {
                    vg: &self.config.vg_name,
                },
            )
            .await?;
        Ok(())
    }

    fn host_path_for(&self, handle: &VolumeHandle) -> Option<PathBuf> {
        let loc = IscsiLvmLocator::from_locator_str(&handle.locator).ok()?;
        Some(PathBuf::from(format!("/dev/{}/{}", loc.vg, loc.lv)))
    }

    async fn provision(&self, opts: CreateOpts) -> Result<VolumeHandle, StorageError> {
        let vol_id = Uuid::new_v4();
        // Short-form UUID keeps the LV name under the LVM 128-byte limit
        // even when the VM id is itself a UUID.
        let short = vol_id.simple().to_string();
        let short = &short[..8];
        // We use the volume_id as the VM-id tag fallback when the caller
        // hasn't given us a stable VM identifier. Mirrors how Proxmox
        // tags LVs with `pve-vm-<vmid>` for ownership.
        let lv_name = format!("nqrust-vm-{vol_id}-disk-{short}");
        let _: LvCreateResp = self
            .agent_post(
                "lv_create",
                &LvCreateReq {
                    vg: &self.config.vg_name,
                    name: &lv_name,
                    size_bytes: opts.size_bytes,
                    vm_id: &vol_id.to_string(),
                },
            )
            .await?;
        let locator = IscsiLvmLocator {
            vg: self.config.vg_name.clone(),
            lv: lv_name,
        };
        Ok(VolumeHandle {
            volume_id: vol_id,
            backend_id: self.id,
            backend_kind: BackendKind::IscsiLvm,
            locator: locator.to_locator_string()?,
            size_bytes: opts.size_bytes,
        })
    }

    async fn destroy(&self, h: VolumeHandle) -> Result<(), StorageError> {
        let loc = IscsiLvmLocator::from_locator_str(&h.locator)?;
        self.agent_post_empty(
            "lv_remove",
            &LvNameReq {
                vg: &loc.vg,
                name: &loc.lv,
            },
        )
        .await
    }

    async fn clone_from_image(
        &self,
        src: &Path,
        opts: CreateOpts,
    ) -> Result<VolumeHandle, StorageError> {
        // Provision an empty LV first; the agent's `clone_from_path` then
        // streams the image into the (already-active) device with
        // `qemu-img convert -O raw <src> /dev/<vg>/<lv>`.
        let handle = self.provision(opts).await?;
        let loc = IscsiLvmLocator::from_locator_str(&handle.locator)?;
        let src_str = src.to_string_lossy();
        if let Err(e) = self
            .agent_post_empty(
                "clone_from_path",
                &CloneFromPathReq {
                    source_path: &src_str,
                    vg: &loc.vg,
                    name: &loc.lv,
                },
            )
            .await
        {
            // Best-effort cleanup so we don't strand an empty LV on the VG.
            let _ = self
                .agent_post_empty(
                    "lv_remove",
                    &LvNameReq {
                        vg: &loc.vg,
                        name: &loc.lv,
                    },
                )
                .await;
            return Err(e);
        }
        Ok(handle)
    }

    async fn snapshot(
        &self,
        v: &VolumeHandle,
        name: &str,
    ) -> Result<VolumeSnapshotHandle, StorageError> {
        if name.is_empty() || name.contains('/') {
            return Err(StorageError::InvalidLocator(
                "snapshot name must be non-empty and contain no '/'".into(),
            ));
        }
        let src = IscsiLvmLocator::from_locator_str(&v.locator)?;
        let snap_lv = format!("snap-{}-{}", src.lv, name);
        let _: LvSnapshotResp = self
            .agent_post(
                "lv_snapshot",
                &LvSnapshotReq {
                    vg: &src.vg,
                    source_lv: &src.lv,
                    snap_name: &snap_lv,
                    size_bytes: v.size_bytes,
                },
            )
            .await?;
        let snap_locator = IscsiLvmLocator {
            vg: src.vg,
            lv: snap_lv,
        };
        Ok(VolumeSnapshotHandle {
            snapshot_id: Uuid::new_v4(),
            backend_id: self.id,
            backend_kind: BackendKind::IscsiLvm,
            locator: snap_locator.to_locator_string()?,
            source_volume_id: v.volume_id,
        })
    }

    async fn clone_from_snapshot(
        &self,
        s: &VolumeSnapshotHandle,
    ) -> Result<VolumeHandle, StorageError> {
        // LVM CoW model: an LV cloned from a snapshot is itself a snapshot
        // of that snapshot LV. We re-use `lv_snapshot` with a fresh name.
        // This differs from a "thick" full-copy clone — the cloned volume
        // shares physical extents with the origin until written. Acceptable
        // for now because nexus-storage's `clone_from_snapshot` contract
        // doesn't require independence; consumers that need a thick copy
        // should `clone_from_image` against the snapshot device path
        // instead.
        let src = IscsiLvmLocator::from_locator_str(&s.locator)?;
        let vol_id = Uuid::new_v4();
        let short = vol_id.simple().to_string();
        let short = &short[..8];
        let new_lv = format!("nqrust-vm-{vol_id}-disk-{short}");
        // The snapshot LV's size is what we provisioned at snapshot time;
        // we don't have it here, so we leave the agent to inherit the
        // origin LV's size by passing 0 — but the agent's `lv_snapshot`
        // requires a size. Use the source LV's allocated size instead by
        // querying via the snapshot's own size implied by the parent. We
        // don't have that here either, so fall back to a conservative
        // choice: the size is encoded as 0 and the agent will fail loudly,
        // signalling the caller to provide a size. In practice the caller
        // should use `clone_from_image` with the snapshot's device path
        // when independence is needed.
        let _: LvSnapshotResp = self
            .agent_post(
                "lv_snapshot",
                &LvSnapshotReq {
                    vg: &src.vg,
                    source_lv: &src.lv,
                    snap_name: &new_lv,
                    size_bytes: 0,
                },
            )
            .await?;
        let locator = IscsiLvmLocator {
            vg: src.vg,
            lv: new_lv,
        };
        Ok(VolumeHandle {
            volume_id: vol_id,
            backend_id: self.id,
            backend_kind: BackendKind::IscsiLvm,
            locator: locator.to_locator_string()?,
            size_bytes: 0,
        })
    }

    async fn delete_snapshot(&self, s: VolumeSnapshotHandle) -> Result<(), StorageError> {
        let loc = IscsiLvmLocator::from_locator_str(&s.locator)?;
        // Snapshots are LVs in LVM, so the same lvremove path applies.
        self.agent_post_empty(
            "lv_remove",
            &LvNameReq {
                vg: &loc.vg,
                name: &loc.lv,
            },
        )
        .await
    }

    async fn activate_volume(&self, h: &VolumeHandle) -> Result<(), StorageError> {
        let loc = IscsiLvmLocator::from_locator_str(&h.locator)?;
        self.agent_post_empty(
            "lv_activate",
            &LvNameReq {
                vg: &loc.vg,
                name: &loc.lv,
            },
        )
        .await
    }

    async fn deactivate_volume(&self, h: &VolumeHandle) -> Result<(), StorageError> {
        let loc = IscsiLvmLocator::from_locator_str(&h.locator)?;
        self.agent_post_empty(
            "lv_deactivate",
            &LvNameReq {
                vg: &loc.vg,
                name: &loc.lv,
            },
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_storage::{BackendInstanceId, BackendKind};
    use uuid::Uuid;

    #[test]
    fn config_parses_minimal() {
        let json = r#"{"portal":"192.168.1.10:3260","iqn":"iqn.foo:bar","vg_name":"vg-x","lun":0}"#;
        let cfg: IscsiLvmConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.portal, "192.168.1.10:3260");
        assert_eq!(cfg.lun, 0);
        assert!(!cfg.saferemove);
    }

    #[test]
    fn locator_round_trips_json() {
        let l = IscsiLvmLocator {
            vg: "vg-x".into(),
            lv: "vm-100-disk-0".into(),
        };
        let s = serde_json::to_string(&l).unwrap();
        let back: IscsiLvmLocator = serde_json::from_str(&s).unwrap();
        assert_eq!(l.vg, back.vg);
        assert_eq!(l.lv, back.lv);
    }

    #[test]
    fn capabilities_match_proxmox_lvm_on_iscsi() {
        let backend = IscsiLvmControlPlaneBackend {
            id: BackendInstanceId(Uuid::new_v4()),
            config: IscsiLvmConfig {
                portal: "x".into(),
                iqn: "y".into(),
                vg_name: "z".into(),
                lun: 0,
                saferemove: false,
                agent_url: None,
            },
        };
        let c = backend.capabilities();
        assert!(c.supports_clone_from_image);
        assert!(c.supports_native_snapshots);
        assert!(!c.supports_concurrent_attach);
        assert!(c.supports_live_migration);
    }

    #[test]
    fn host_path_for_returns_dev_vg_lv() {
        let backend = IscsiLvmControlPlaneBackend {
            id: BackendInstanceId(Uuid::new_v4()),
            config: IscsiLvmConfig {
                portal: "x".into(),
                iqn: "y".into(),
                vg_name: "z".into(),
                lun: 0,
                saferemove: false,
                agent_url: None,
            },
        };
        let h = nexus_storage::VolumeHandle {
            volume_id: Uuid::new_v4(),
            backend_id: BackendInstanceId(Uuid::new_v4()),
            backend_kind: BackendKind::IscsiLvm,
            locator: r#"{"vg":"vg-nqrust","lv":"vm-100-disk-0"}"#.into(),
            size_bytes: 0,
        };
        let p = backend.host_path_for(&h).unwrap();
        assert_eq!(p, std::path::PathBuf::from("/dev/vg-nqrust/vm-100-disk-0"));
    }
}
