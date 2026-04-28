use nexus_storage::{BackendKind, HostBackend};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Default)]
pub struct HostBackendRegistry {
    by_kind: HashMap<BackendKind, Arc<dyn HostBackend>>,
}

impl HostBackendRegistry {
    pub fn empty() -> Self { Self::default() }

    pub fn register_for(&mut self, kind: BackendKind, backend: Arc<dyn HostBackend>) {
        self.by_kind.insert(kind, backend);
    }

    pub fn get(&self, kind: BackendKind) -> Option<&Arc<dyn HostBackend>> {
        self.by_kind.get(&kind)
    }

    pub fn supported_kinds(&self) -> Vec<BackendKind> {
        let mut v: Vec<_> = self.by_kind.keys().copied().collect();
        v.sort_by_key(|k| k.as_db_str());
        v
    }
}
