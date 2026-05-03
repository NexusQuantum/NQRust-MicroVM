//! Discovery helpers for storage backends. Wraps `showmount -e` for NFS
//! and `iscsiadm -m discovery` for iSCSI so operators don't have to
//! remember exports or target IQNs — same UX shape as Proxmox VE's
//! `pvesm nfsscan`.

// Public items are wired into REST routes in Tasks 2 & 3.
#![allow(dead_code)]

use serde::Serialize;
use std::time::Duration;

const SHELL_TIMEOUT: Duration = Duration::from_secs(8);

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NfsExport {
    pub path: String,
    /// Raw access spec from `showmount -e` (e.g. "10.0.0.0/24" or "*"),
    /// shown to operators as a hint about who's allowed to mount.
    pub allowed: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct IscsiTarget {
    pub portal: String,
    pub iqn: String,
}

/// Parse the body of `showmount -e <server>` output. Skips the header
/// line ("Export list for ...") and any blank lines.
pub fn parse_showmount(stdout: &str) -> Vec<NfsExport> {
    stdout
        .lines()
        .skip_while(|l| !l.trim_start().starts_with('/'))
        .filter(|l| !l.trim().is_empty())
        .filter_map(|line| {
            let line = line.trim();
            // showmount -e prints `<path>   <allowed>` separated by tabs
            // or whitespace. Split on first run of whitespace.
            let mut parts = line.splitn(2, char::is_whitespace);
            let path = parts.next()?.to_string();
            let allowed = parts.next().unwrap_or("").trim().to_string();
            Some(NfsExport { path, allowed })
        })
        .collect()
}

/// Parse `iscsiadm -m discovery -t st -p <portal>` output. Each line
/// is `<ip>:<port>,<tag> <iqn>`.
pub fn parse_iscsiadm_discovery(stdout: &str) -> Vec<IscsiTarget> {
    stdout
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let portal_with_tag = parts.next()?;
            let iqn = parts.next()?.to_string();
            // Strip the `,<tag>` suffix from the portal field.
            let portal = portal_with_tag
                .split(',')
                .next()
                .unwrap_or(portal_with_tag)
                .to_string();
            Some(IscsiTarget { portal, iqn })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_showmount_handles_typical_truenas_output() {
        let raw = r#"Export list for 10.0.0.5:
/mnt/NQRust/harvester-nfs    10.0.0.0/24
/mnt/NQRust/iso              *
"#;
        let exports = parse_showmount(raw);
        assert_eq!(exports.len(), 2);
        assert_eq!(
            exports[0],
            NfsExport {
                path: "/mnt/NQRust/harvester-nfs".into(),
                allowed: "10.0.0.0/24".into(),
            }
        );
        assert_eq!(
            exports[1],
            NfsExport {
                path: "/mnt/NQRust/iso".into(),
                allowed: "*".into(),
            }
        );
    }

    #[test]
    fn parse_showmount_returns_empty_when_server_has_no_exports() {
        let raw = "Export list for 10.0.0.5:\n";
        assert_eq!(parse_showmount(raw), vec![]);
    }

    #[test]
    fn parse_showmount_handles_path_only_without_allowed_field() {
        let raw = "Export list for s:\n/srv/share\n";
        assert_eq!(
            parse_showmount(raw),
            vec![NfsExport {
                path: "/srv/share".into(),
                allowed: "".into(),
            }]
        );
    }

    #[test]
    fn parse_iscsiadm_discovery_handles_typical_truenas_output() {
        let raw = "10.0.0.5:3260,1 iqn.2005-10.org.freenas.ctl:nqrust-v-myvm-12345678\n10.0.0.5:3260,1 iqn.2005-10.org.freenas.ctl:csi-pvc-1b19dc9e-harvester\n";
        let targets = parse_iscsiadm_discovery(raw);
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].portal, "10.0.0.5:3260");
        assert!(targets[0].iqn.contains("nqrust-v-myvm-12345678"));
        assert!(targets[1].iqn.contains("csi-pvc-1b19dc9e-harvester"));
    }

    #[test]
    fn parse_iscsiadm_discovery_skips_blank_lines() {
        let raw = "\n10.0.0.5:3260,1 iqn.x:y\n\n";
        assert_eq!(parse_iscsiadm_discovery(raw).len(), 1);
    }
}
