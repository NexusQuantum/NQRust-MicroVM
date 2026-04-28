use anyhow::{anyhow, Context, Result};
use nexus_storage::{BackendKind, Capabilities};
use serde::Deserialize;
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, Deserialize)]
pub struct StorageBackendsToml {
    #[serde(default, rename = "storage_backend")]
    pub backends: Vec<RawBackendEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawBackendEntry {
    pub name: String,
    pub kind: BackendKind,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default)]
    pub config: JsonValue,
}

/// Validated entry ready to be upserted into the DB and registered as a
/// trait object. Per-kind required fields have already been checked.
#[derive(Debug, Clone)]
pub struct ValidatedBackend {
    pub name: String,
    pub kind: BackendKind,
    pub is_default: bool,
    pub config: JsonValue,
    pub capabilities: Capabilities,
}

/// Parse a TOML string into raw entries.
pub fn parse(toml_str: &str) -> Result<StorageBackendsToml> {
    toml::from_str::<StorageBackendsToml>(toml_str)
        .context("parsing storage_backend TOML")
}

/// Validate per-kind shape and assign capabilities. The capabilities here are
/// the *expected* capabilities for a given kind; the actual backend impl is
/// the source of truth at runtime, but we denormalize here so the DB and UI
/// can show capabilities without instantiating a backend.
pub fn validate(raw: RawBackendEntry) -> Result<ValidatedBackend> {
    if raw.name.is_empty() {
        return Err(anyhow!("storage_backend.name must not be empty"));
    }

    let capabilities = match raw.kind {
        BackendKind::LocalFile => Capabilities {
            supports_native_snapshots: false,
            supports_concurrent_attach: false,
            supports_live_migration: false,
            supports_clone_from_image: true,
        },
        BackendKind::Iscsi => {
            require_str(&raw.config, "target_iqn")
                .map_err(|e| anyhow!("backend '{}' (kind=iscsi): {e}", raw.name))?;
            Capabilities {
                supports_native_snapshots: false,
                supports_concurrent_attach: false,
                supports_live_migration: false,
                supports_clone_from_image: false,
            }
        }
        BackendKind::TrueNasIscsi => {
            require_str(&raw.config, "endpoint")
                .map_err(|e| anyhow!("backend '{}' (kind=truenas_iscsi): {e}", raw.name))?;
            require_str(&raw.config, "api_key_env")
                .map_err(|e| anyhow!("backend '{}' (kind=truenas_iscsi): {e}", raw.name))?;
            require_str(&raw.config, "pool")
                .map_err(|e| anyhow!("backend '{}' (kind=truenas_iscsi): {e}", raw.name))?;
            require_str(&raw.config, "target_iqn_prefix")
                .map_err(|e| anyhow!("backend '{}' (kind=truenas_iscsi): {e}", raw.name))?;
            Capabilities {
                supports_native_snapshots: true,
                supports_concurrent_attach: false,
                supports_live_migration: false,
                supports_clone_from_image: false,
            }
        }
    };

    Ok(ValidatedBackend {
        name: raw.name,
        kind: raw.kind,
        is_default: raw.is_default,
        config: raw.config,
        capabilities,
    })
}

fn require_str(config: &JsonValue, field: &str) -> Result<()> {
    match config.get(field).and_then(|v| v.as_str()) {
        Some(s) if !s.is_empty() => Ok(()),
        Some(_) => Err(anyhow!("config.{field} is empty")),
        None => Err(anyhow!("config.{field} is required")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_localfile_entry() {
        let toml = r#"
            [[storage_backend]]
            name = "localfile-default"
            kind = "local_file"
            is_default = true
        "#;
        let parsed = parse(toml).unwrap();
        assert_eq!(parsed.backends.len(), 1);
        assert_eq!(parsed.backends[0].name, "localfile-default");
        assert_eq!(parsed.backends[0].kind, BackendKind::LocalFile);

        let v = validate(parsed.backends.into_iter().next().unwrap()).unwrap();
        assert!(v.capabilities.supports_clone_from_image);
        assert!(!v.capabilities.supports_native_snapshots);
    }

    #[test]
    fn truenas_missing_endpoint_fails_validation() {
        let raw = RawBackendEntry {
            name: "tn".into(),
            kind: BackendKind::TrueNasIscsi,
            is_default: false,
            config: serde_json::json!({"api_key_env": "X", "pool": "p", "target_iqn_prefix": "iqn.x"}),
        };
        let err = validate(raw).unwrap_err();
        assert!(err.to_string().contains("endpoint"), "got: {err}");
    }

    #[test]
    fn iscsi_requires_target_iqn() {
        let raw = RawBackendEntry {
            name: "i".into(),
            kind: BackendKind::Iscsi,
            is_default: false,
            config: serde_json::json!({}),
        };
        let err = validate(raw).unwrap_err();
        assert!(err.to_string().contains("target_iqn"), "got: {err}");
    }

    /// T27: Malformed TrueNAS iSCSI entry parsed from TOML must fail validation
    /// with an error message naming BOTH the missing field and the backend name.
    #[test]
    fn malformed_truenas_iscsi_entry_fails_fast_with_clear_message() {
        let toml_str = r#"
            [[storage_backend]]
            name = "tn"
            kind = "true_nas_iscsi"
            [storage_backend.config]
            api_key_env = "X"
        "#;
        let parsed = parse(toml_str).unwrap();
        let raw = parsed.backends.into_iter().next().unwrap();
        let err = validate(raw).unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("endpoint"),
            "error should name the missing field 'endpoint': {msg}"
        );
        assert!(
            msg.contains("tn"),
            "error should name the backend 'tn': {msg}"
        );
    }
}
