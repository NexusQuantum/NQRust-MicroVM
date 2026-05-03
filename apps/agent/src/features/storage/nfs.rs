//! Agent-side NFS host backend. Each unique (server, export) pair gets
//! its own mount point under `mount_base`. `attach` ensures the export
//! is mounted and returns the path to the volume's file. `detach` is a
//! no-op in v1 — the agent leaves the mount in place across volume
//! lifecycles for two reasons: (1) re-mounting is slow, (2) other
//! volumes on the same export may still be attached.

use std::path::PathBuf;

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone)]
pub struct NfsHostConfig {
    pub mount_base: PathBuf,
}

impl NfsHostConfig {
    /// Deterministic per-(server, export) directory name. The export's
    /// leading slash is stripped and remaining slashes become `_` so the
    /// result is a single path component. Server is appended literally
    /// after a `:`.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn mount_point_for(&self, server: &str, export: &str) -> PathBuf {
        let exp = export.trim_start_matches('/').replace('/', "_");
        let server_safe = server.replace([':', '/'], "_");
        self.mount_base.join(format!("{server_safe}:{exp}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mount_point_is_unique_per_server_export_and_filesystem_safe() {
        let cfg = NfsHostConfig {
            mount_base: PathBuf::from("/var/lib/nqrust/nfs"),
        };
        let a = cfg.mount_point_for("10.0.0.5", "/mnt/tank/vms");
        let b = cfg.mount_point_for("10.0.0.5", "/mnt/tank/iso");
        let c = cfg.mount_point_for("10.0.0.6", "/mnt/tank/vms");
        assert_ne!(a, b);
        assert_ne!(a, c);
        assert_eq!(
            a,
            PathBuf::from("/var/lib/nqrust/nfs/10.0.0.5:mnt_tank_vms")
        );
    }
}
