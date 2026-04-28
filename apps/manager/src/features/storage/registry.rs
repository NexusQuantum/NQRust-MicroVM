use crate::features::storage::backends::local_file::LocalFileControlPlaneBackend;
use crate::features::storage::config::{parse, validate, ValidatedBackend};
use crate::features::storage_backends::repo::{StorageBackendRepository, StorageBackendRow};
use anyhow::{anyhow, Context, Result};
use nexus_storage::{BackendInstanceId, BackendKind, ControlPlaneBackend};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Manager-side registry. Holds one trait object per active backend instance,
/// keyed by `backend_id`. Built once at startup; immutable thereafter.
/// Task 15 wires this into AppState — suppress dead-code warnings until then.
#[allow(dead_code)]
#[derive(Clone)]
pub struct Registry {
    by_id: HashMap<Uuid, Arc<dyn ControlPlaneBackend>>,
    default_id: Option<Uuid>,
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

        // Don't soft-delete localfile-default — it's the migration-seeded fallback.
        for existing in repo.list_active().await? {
            if existing.name == "localfile-default" { continue; }
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
            )
            .await
            .with_context(|| format!("upserting storage_backend '{}'", v.name))?;
        }

        // 3. Build the in-memory map. Walk active rows from the DB (post-upsert).
        let mut by_id: HashMap<Uuid, Arc<dyn ControlPlaneBackend>> = HashMap::new();
        let mut default_id: Option<Uuid> = None;
        for row in repo.list_active().await? {
            let trait_obj = match build_backend(&row) {
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
            by_id.insert(row.id, trait_obj);
        }

        if by_id.is_empty() {
            return Err(anyhow!("no active storage backends — migration should have seeded localfile-default"));
        }

        Ok(Registry { by_id, default_id })
    }

    pub fn get(&self, id: Uuid) -> Option<&Arc<dyn ControlPlaneBackend>> {
        self.by_id.get(&id)
    }

    pub fn default_id(&self) -> Option<Uuid> {
        self.default_id
    }

    pub fn default_backend(&self) -> Option<&Arc<dyn ControlPlaneBackend>> {
        self.default_id.and_then(|id| self.by_id.get(&id))
    }
}

#[allow(dead_code)]
fn build_backend(row: &StorageBackendRow) -> Result<Arc<dyn ControlPlaneBackend>> {
    let kind: BackendKind = match row.kind.as_str() {
        "local_file" => BackendKind::LocalFile,
        "iscsi" => BackendKind::Iscsi,
        "truenas_iscsi" => BackendKind::TrueNasIscsi,
        other => {
            return Err(anyhow!("unknown backend kind '{other}'"));
        }
    };
    match kind {
        BackendKind::LocalFile => Ok(Arc::new(LocalFileControlPlaneBackend {
            id: BackendInstanceId(row.id),
        })),
        BackendKind::Iscsi | BackendKind::TrueNasIscsi => {
            // Implemented in Plan 2. For now, refuse to register.
            Err(anyhow!(
                "backend kind '{}' not implemented in this plan — use Plan 2",
                kind.as_db_str()
            ))
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
}
