//! Pluggable VMM backends.
//!
//! The agent holds a [`VmmRegistry`] populated at startup with one driver
//! per installed VMM kind. Per-VM routes dispatch on `vmm_kind` (passed in
//! the request body) and call into the corresponding driver.
//!
//! Adding a new backend: implement [`VmmDriver`] in a new module, register
//! it from [`VmmRegistry::probe_installed`].

use std::collections::HashMap;
use std::sync::Arc;

use nexus_vmm::{VmmDriver, VmmKind};
use tracing::{info, warn};

pub mod firecracker;
pub mod qemu;
pub mod qmp;
pub mod resource;

/// Registry of installed VMM drivers, keyed by [`VmmKind`].
#[derive(Clone, Default)]
pub struct VmmRegistry {
    drivers: HashMap<VmmKind, Arc<dyn VmmDriver>>,
    versions: HashMap<VmmKind, String>,
}

impl VmmRegistry {
    pub fn empty() -> Self {
        Self::default()
    }

    /// Probe each known backend by invoking `--version` on the binary. Only
    /// successfully-probed kinds are added to the registry. Returns the
    /// registry with whatever was found; an empty registry is a valid
    /// (albeit useless) state — the agent will report no kinds installed.
    pub async fn probe_installed() -> Self {
        let mut reg = Self::empty();

        let fc = Arc::new(firecracker::FirecrackerDriver::new());
        match fc.probe().await {
            Ok(ver) => {
                info!(version = %ver, "firecracker installed");
                reg.versions.insert(VmmKind::Firecracker, ver);
                reg.drivers.insert(VmmKind::Firecracker, fc);
            }
            Err(err) => {
                warn!(
                    ?err,
                    "firecracker not installed; skipping driver registration"
                );
            }
        }

        let q = Arc::new(qemu::QemuDriver::new());
        match q.probe().await {
            Ok(ver) => {
                info!(version = %ver, "qemu installed");
                reg.versions.insert(VmmKind::Qemu, ver);
                reg.drivers.insert(VmmKind::Qemu, q);
            }
            Err(err) => {
                warn!(?err, "qemu not installed; skipping driver registration");
            }
        }

        reg
    }

    /// All installed VMM kinds. Sorted for stable JSON serialization.
    pub fn installed_kinds(&self) -> Vec<VmmKind> {
        let mut kinds: Vec<VmmKind> = self.drivers.keys().copied().collect();
        kinds.sort_by_key(|k| k.as_str());
        kinds
    }

    /// Version string for an installed kind, if any.
    pub fn version(&self, kind: VmmKind) -> Option<&str> {
        self.versions.get(&kind).map(|s| s.as_str())
    }

    pub fn get(&self, kind: VmmKind) -> Option<Arc<dyn VmmDriver>> {
        self.drivers.get(&kind).cloned()
    }

    /// True if the given kind has a registered driver.
    pub fn has(&self, kind: VmmKind) -> bool {
        self.drivers.contains_key(&kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_registry_has_no_kinds() {
        let r = VmmRegistry::empty();
        assert!(r.installed_kinds().is_empty());
        assert!(!r.has(VmmKind::Firecracker));
        assert!(!r.has(VmmKind::Qemu));
    }
}
