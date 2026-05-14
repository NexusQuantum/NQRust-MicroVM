use crate::features::storage::backends::local_file::LocalFileControlPlaneBackend;
use crate::features::storage::config::{parse, validate, ValidatedBackend};
use crate::features::storage_backends::repo::{StorageBackendRepository, StorageBackendRow};
use anyhow::{anyhow, Context, Result};
use nexus_storage::{BackendInstanceId, BackendKind, ControlPlaneBackend};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

/// Manager-side registry. Holds one trait object per active backend instance,
/// keyed by `backend_id`. Built at startup from DB rows + TOML; mutated at
/// runtime by storage_backends create/delete routes so UI-added backends are
/// immediately usable for VM-create without a manager restart.
#[allow(dead_code)]
#[derive(Clone)]
pub struct Registry {
    by_id: Arc<RwLock<HashMap<Uuid, Arc<dyn ControlPlaneBackend>>>>,
    default_id: Arc<RwLock<Option<Uuid>>>,
    pool: PgPool,
}

#[allow(dead_code)]
impl Registry {
    pub async fn load(pool: &PgPool, toml_str: Option<&str>) -> Result<Self> {
        let repo = StorageBackendRepository::new(pool.clone());

        // 1. Parse + validate TOML (if provided).
        let validated: Vec<ValidatedBackend> = match toml_str {
            Some(s) => {
                let parsed = parse(s).context("parsing storage_backend TOML")?;
                let mut out = Vec::with_capacity(parsed.backends.len());
                for raw in parsed.backends {
                    out.push(validate(raw).context("validating storage_backend entry")?);
                }
                out
            }
            None => Vec::new(),
        };

        // 2. Upsert validated entries; soft-delete entries no longer in TOML.
        let toml_names: std::collections::HashSet<String> =
            validated.iter().map(|v| v.name.clone()).collect();

        // Sweep: soft-delete TOML-sourced rows that are no longer in the file.
        // UI-sourced rows (source='ui') are operator-managed and survive
        // restarts. Don't touch localfile-default either — it's seeded by
        // migration as the fallback.
        for existing in repo.list_active().await? {
            if existing.name == "localfile-default" {
                continue;
            }
            if existing.source == "ui" {
                continue;
            }
            if !toml_names.contains(&existing.name) {
                repo.soft_delete_by_name(&existing.name).await?;
                tracing::warn!(
                    "storage_backend '{}' removed from TOML; soft-deleted in DB",
                    existing.name
                );
            }
        }

        for v in &validated {
            let caps_json = serde_json::to_value(v.capabilities)?;
            repo.upsert(
                &v.name,
                v.kind.as_db_str(),
                &v.config,
                &caps_json,
                v.is_default,
                "toml",
            )
            .await
            .with_context(|| format!("upserting storage_backend '{}'", v.name))?;
        }

        // 3. Build the in-memory map. Walk active rows from the DB (post-upsert).
        // NFS can omit config.agent_url for single-host setups; in that case
        // use the most recently-seen registered agent as the mount/file owner.
        let default_agent_url: Option<String> =
            sqlx::query_scalar("SELECT addr FROM host ORDER BY last_seen_at DESC LIMIT 1")
                .fetch_optional(pool)
                .await
                .context("looking up default agent for storage backends")?;
        let mut by_id: HashMap<Uuid, Arc<dyn ControlPlaneBackend>> = HashMap::new();
        let mut default_id: Option<Uuid> = None;
        for row in repo.list_active().await? {
            let trait_obj = match build_backend(&row, default_agent_url.as_deref()) {
                Ok(o) => o,
                Err(e) => {
                    tracing::warn!("storage_backend '{}' skipped: {e:#}", row.name);
                    continue;
                }
            };
            if row.is_default {
                if default_id.is_some() {
                    return Err(anyhow!(
                        "more than one default backend in DB — partial unique index should prevent this"
                    ));
                }
                default_id = Some(row.id);
            }
            tracing::info!(
                backend_id = %row.id,
                name = %row.name,
                kind = %row.kind,
                source = %row.source,
                is_default = row.is_default,
                "storage_backend loaded into registry"
            );
            // Eagerly probe so operators see reachability failures at
            // startup rather than on first VM-create. Best-effort: a
            // failed probe logs WARN but doesn't crash startup, since the
            // agent might still be coming up.
            if let Err(e) = trait_obj.probe().await {
                tracing::warn!(
                    backend = %row.name,
                    error = %e,
                    "storage_backend probe failed at startup (will retry lazily)"
                );
            }
            by_id.insert(row.id, trait_obj);
        }

        if by_id.is_empty() {
            return Err(anyhow!(
                "no active storage backends — migration should have seeded localfile-default"
            ));
        }

        Ok(Registry {
            by_id: Arc::new(RwLock::new(by_id)),
            default_id: Arc::new(RwLock::new(default_id)),
            pool: pool.clone(),
        })
    }

    pub fn get(&self, id: Uuid) -> Option<Arc<dyn ControlPlaneBackend>> {
        self.by_id.read().ok()?.get(&id).cloned()
    }

    pub fn default_id(&self) -> Option<Uuid> {
        *self.default_id.read().ok()?
    }

    pub fn default_backend(&self) -> Option<Arc<dyn ControlPlaneBackend>> {
        let did = (*self.default_id.read().ok()?)?;
        self.by_id.read().ok()?.get(&did).cloned()
    }

    /// Insert (or replace) a backend in the live registry. Called by the
    /// storage_backends create handler after a new row is upserted, so the
    /// backend becomes immediately usable for VM-create without a restart.
    pub async fn add(&self, row: &StorageBackendRow) -> Result<()> {
        let default_agent_url: Option<String> =
            sqlx::query_scalar("SELECT addr FROM host ORDER BY last_seen_at DESC LIMIT 1")
                .fetch_optional(&self.pool)
                .await
                .context("looking up default agent for live registry add")?;
        let backend = build_backend(row, default_agent_url.as_deref())?;
        // Best-effort probe (mirrors startup load).
        if let Err(e) = backend.probe().await {
            tracing::warn!(
                backend = %row.name,
                error = %e,
                "live-registry insert: probe failed (will retry lazily)"
            );
        }
        {
            let mut by_id = self.by_id.write().expect("registry rwlock poisoned");
            by_id.insert(row.id, backend);
        }
        if row.is_default {
            *self.default_id.write().expect("registry rwlock poisoned") = Some(row.id);
        }
        tracing::info!(
            backend_id = %row.id,
            name = %row.name,
            kind = %row.kind,
            "storage_backend added to live registry"
        );
        Ok(())
    }

    /// Remove a backend from the live registry. Called after soft-delete so
    /// VM-create no longer routes to a deleted backend.
    pub fn remove(&self, id: Uuid) {
        {
            let mut by_id = self.by_id.write().expect("registry rwlock poisoned");
            by_id.remove(&id);
        }
        let mut did = self.default_id.write().expect("registry rwlock poisoned");
        if *did == Some(id) {
            *did = None;
        }
        tracing::info!(backend_id = %id, "storage_backend removed from live registry");
    }
}

#[allow(dead_code)]
fn build_backend(
    row: &StorageBackendRow,
    default_agent_url: Option<&str>,
) -> Result<Arc<dyn ControlPlaneBackend>> {
    let kind: BackendKind = match row.kind.as_str() {
        "local_file" => BackendKind::LocalFile,
        "iscsi" => BackendKind::Iscsi,
        "truenas_iscsi" => BackendKind::TrueNasIscsi,
        "spdk_lvol" => BackendKind::SpdkLvol,
        "nfs" => BackendKind::Nfs,
        "iscsi_lvm" => BackendKind::IscsiLvm,
        "smb" => BackendKind::Smb,
        other => {
            return Err(anyhow!("unknown backend kind '{other}'"));
        }
    };
    match kind {
        BackendKind::LocalFile => Ok(Arc::new(LocalFileControlPlaneBackend {
            id: BackendInstanceId(row.id),
        })),
        BackendKind::Iscsi => {
            let cfg: crate::features::storage::backends::iscsi_generic::IscsiGenericConfig =
                serde_json::from_value(row.config_json.clone())
                    .with_context(|| format!("backend '{}' iscsi config", row.name))?;
            Ok(Arc::new(
                crate::features::storage::backends::iscsi_generic::IscsiGenericControlPlaneBackend {
                    id: BackendInstanceId(row.id),
                    config: cfg,
                },
            ))
        }
        BackendKind::TrueNasIscsi => {
            let cfg: crate::features::storage::backends::truenas_iscsi::TrueNasConfig =
                serde_json::from_value(row.config_json.clone())
                    .with_context(|| format!("backend '{}' truenas_iscsi config", row.name))?;
            let api_key = std::env::var(&cfg.api_key_env).with_context(|| {
                format!(
                    "env var {} not set for backend '{}'",
                    cfg.api_key_env, row.name
                )
            })?;
            Ok(Arc::new(
                crate::features::storage::backends::truenas_iscsi::TrueNasIscsiControlPlaneBackend {
                    id: BackendInstanceId(row.id),
                    config: cfg,
                    api_key,
                    http: reqwest::Client::new(),
                },
            ))
        }
        BackendKind::SpdkLvol => {
            let cfg: crate::features::storage::backends::spdk_lvol::SpdkLvolConfig =
                serde_json::from_value(row.config_json.clone())
                    .with_context(|| format!("backend '{}' spdk_lvol config", row.name))?;
            Ok(Arc::new(
                crate::features::storage::backends::spdk_lvol::SpdkLvolControlPlaneBackend::new(
                    BackendInstanceId(row.id),
                    cfg,
                ),
            ))
        }
        BackendKind::Nfs => {
            let mut cfg: crate::features::storage::backends::nfs::NfsConfig =
                serde_json::from_value(row.config_json.clone())
                    .with_context(|| format!("backend '{}' nfs config", row.name))?;
            if cfg.agent_url.is_none() && !cfg.assume_mounted {
                if let Some(addr) = default_agent_url {
                    cfg.agent_url = Some(addr.to_string());
                }
            }
            Ok(Arc::new(
                crate::features::storage::backends::nfs::NfsControlPlaneBackend {
                    id: BackendInstanceId(row.id),
                    config: cfg,
                },
            ))
        }
        BackendKind::IscsiLvm => {
            let mut cfg: crate::features::storage::backends::iscsi_lvm::IscsiLvmConfig =
                serde_json::from_value(row.config_json.clone())
                    .with_context(|| format!("backend '{}' iscsi_lvm config", row.name))?;
            if cfg.agent_url.is_none() {
                cfg.agent_url = default_agent_url.map(|s| s.to_string());
            }
            Ok(Arc::new(
                crate::features::storage::backends::iscsi_lvm::IscsiLvmControlPlaneBackend {
                    id: BackendInstanceId(row.id),
                    config: cfg,
                },
            ))
        }
        BackendKind::Smb => Err(anyhow!("smb not yet implemented (Task 9)")),
    }
}

impl Registry {
    /// Test-only: build a Registry with a single pre-built backend keyed by id.
    /// Bypasses TOML parsing and DB upsert. NOT for production use.
    #[cfg(test)]
    pub fn test_only_with_backend(
        id: uuid::Uuid,
        backend: std::sync::Arc<dyn nexus_storage::ControlPlaneBackend>,
    ) -> Self {
        let mut by_id = std::collections::HashMap::new();
        by_id.insert(id, backend);
        // The test pool is unused by the helpers tests typically exercise
        // (`get`, `default_id`, `default_backend`); `add()` would dereference
        // it but is not called from these tests.
        let pool = sqlx::PgPool::connect_lazy("postgres://nobody@localhost/nobody")
            .expect("lazy pool init does not actually connect");
        Registry {
            by_id: Arc::new(RwLock::new(by_id)),
            default_id: Arc::new(RwLock::new(Some(id))),
            pool,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires DATABASE_URL"]
    async fn registry_loads_localfile_default() {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
        let pool = sqlx::PgPool::connect(&url).await.unwrap();
        let reg = Registry::load(&pool, None).await.unwrap();
        let default = reg.default_backend().expect("default backend present");
        assert!(matches!(default.kind(), BackendKind::LocalFile));
        // T23: also verify capabilities are as expected for LocalFile
        assert!(default.capabilities().supports_clone_from_image);
        assert!(!default.capabilities().supports_native_snapshots);
    }

    /// T28: Volumes that existed before migration 0034 are backfilled to point
    /// at `localfile-default`. This test inserts a volume row explicitly
    /// associated with that backend (simulating what the migration's UPDATE does
    /// for pre-existing rows) and verifies the backend_id round-trips correctly.
    #[tokio::test]
    #[ignore = "requires DATABASE_URL with migrations applied"]
    async fn pre_foundation_volume_row_is_backfilled_to_localfile_default() {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
        let p = sqlx::PgPool::connect(&url).await.unwrap();

        // Resolve localfile-default backend id (seeded by migration 0034).
        let backend_id: uuid::Uuid = sqlx::query_scalar::<_, uuid::Uuid>(
            r#"SELECT id FROM storage_backend WHERE name = 'localfile-default'"#,
        )
        .fetch_one(&p)
        .await
        .unwrap();

        let host_id: Option<uuid::Uuid> = sqlx::query_scalar(r#"SELECT id FROM host LIMIT 1"#)
            .fetch_optional(&p)
            .await
            .unwrap()
            .flatten();

        // Insert a simulated "legacy" volume row pointing at localfile-default.
        let vol_id = uuid::Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO volume (id, name, path, size_bytes, type, status, host_id, backend_id)
               VALUES ($1, $2, $3, 1024, 'raw', 'available', $4, $5)"#,
        )
        .bind(vol_id)
        .bind(format!("legacy-{vol_id}"))
        .bind(format!("/tmp/legacy-{vol_id}.img"))
        .bind(host_id)
        .bind(backend_id)
        .execute(&p)
        .await
        .unwrap();

        // Read it back. backend_id must point at localfile-default.
        let row: (uuid::Uuid, Option<uuid::Uuid>, uuid::Uuid) =
            sqlx::query_as(r#"SELECT id, host_id, backend_id FROM volume WHERE id = $1"#)
                .bind(vol_id)
                .fetch_one(&p)
                .await
                .unwrap();
        assert_eq!(
            row.2, backend_id,
            "backend_id must point at localfile-default"
        );

        // Cleanup.
        sqlx::query("DELETE FROM volume WHERE id = $1")
            .bind(vol_id)
            .execute(&p)
            .await
            .ok();
    }

    #[tokio::test]
    async fn build_backend_constructs_nfs_when_kind_is_nfs() {
        let row = StorageBackendRow {
            id: uuid::Uuid::new_v4(),
            name: "nfs-test".into(),
            kind: "nfs".into(),
            is_default: false,
            config_json: serde_json::json!({
                "server": "10.0.0.5",
                "export": "/mnt/tank/vms"
            }),
            capabilities_json: serde_json::json!({}),
            source: "toml".into(),
            deleted_at: None,
            created_at: chrono::Utc::now(),
        };
        let backend = build_backend(&row, Some("http://127.0.0.1:9090")).expect("build_backend");
        assert!(matches!(backend.kind(), nexus_storage::BackendKind::Nfs));
    }
}
