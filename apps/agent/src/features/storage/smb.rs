//! SMB / CIFS host backend (Task 3 — arg builders only).

use std::path::PathBuf;

/// Build the argv for `/bin/mount -t cifs ... -o ...`. Returns the args
/// AFTER the `mount` executable name (so the caller does
/// `Command::new("mount").args(&args)`).
pub fn build_mount_args(
    server: &str,
    share: &str,
    subdir: Option<&str>,
    cred_file: Option<&str>,
    username: Option<&str>,
    domain: Option<&str>,
    smb_version: Option<&str>,
    extra_options: Option<&str>,
    mount_point: &str,
) -> Vec<String> {
    // Wrap raw IPv6 server addresses in [] so mount.cifs parses them.
    let server_h = if server.contains(':') && !server.starts_with('[') {
        format!("[{server}]")
    } else {
        server.to_string()
    };

    let mut source = format!("//{server_h}/{share}");
    if let Some(s) = subdir.map(str::trim).filter(|s| !s.is_empty()) {
        source.push('/');
        source.push_str(s.trim_start_matches('/'));
    }

    let mut opts: Vec<String> = vec!["soft".into()];
    if let Some(u) = username.map(str::trim).filter(|u| !u.is_empty()) {
        opts.push(format!("username={u}"));
        if let Some(cf) = cred_file {
            opts.push(format!("credentials={cf}"));
        }
    } else {
        // Anonymous / guest mount — no creds file.
        opts.push("guest".into());
        opts.push("username=guest".into());
    }
    if let Some(d) = domain.map(str::trim).filter(|d| !d.is_empty()) {
        opts.push(format!("domain={d}"));
    }
    if let Some(v) = smb_version
        .map(str::trim)
        .filter(|v| !v.is_empty() && *v != "default")
    {
        opts.push(format!("vers={v}"));
    }
    if let Some(extra) = extra_options.map(str::trim).filter(|e| !e.is_empty()) {
        opts.push(extra.to_string());
    }

    vec![
        "-t".into(),
        "cifs".into(),
        source,
        mount_point.into(),
        "-o".into(),
        opts.join(","),
    ]
}

/// Deterministic per-`(server, share)` mount point under `base`. Same shape
/// idea as the NFS module: slugify the share, replace separators in the
/// server, join with `<server>:<share>`.
pub fn mount_point_for(base: &str, server: &str, share: &str) -> PathBuf {
    let share_safe = share.trim_start_matches('/').replace('/', "_");
    let server_safe = server.replace([':', '/'], "_");
    PathBuf::from(base).join(format!("{server_safe}:{share_safe}"))
}

#[cfg(test)]
mod arg_tests {
    use super::*;

    #[test]
    fn build_mount_args_authenticated() {
        let args = build_mount_args(
            "fileserver.local",
            "vms",
            None,
            Some("/etc/nqrust/storage-creds/abc.cred"),
            Some("vm-admin"),
            Some("CORP"),
            Some("3.0"),
            None,
            "/var/lib/nqrust/smb/abc",
        );
        let s = args.join(" ");
        assert!(s.contains("//fileserver.local/vms"), "{s}");
        assert!(s.contains("/var/lib/nqrust/smb/abc"), "{s}");
        assert!(s.contains("-t cifs"), "{s}");
        assert!(s.contains("username=vm-admin"), "{s}");
        assert!(
            s.contains("credentials=/etc/nqrust/storage-creds/abc.cred"),
            "{s}"
        );
        assert!(s.contains("domain=CORP"), "{s}");
        assert!(s.contains("vers=3.0"), "{s}");
    }

    #[test]
    fn build_mount_args_anonymous() {
        let args = build_mount_args(
            "srv",
            "public",
            None,
            None,
            None,
            None,
            None,
            None,
            "/var/lib/nqrust/smb/x",
        );
        let s = args.join(" ");
        assert!(s.contains("guest"), "{s}");
        assert!(!s.contains("credentials="), "{s}");
    }

    #[test]
    fn build_mount_args_with_subdir() {
        let args = build_mount_args(
            "srv",
            "share",
            Some("tenant-a"),
            None,
            None,
            None,
            None,
            None,
            "/var/lib/nqrust/smb/x",
        );
        let s = args.join(" ");
        assert!(s.contains("//srv/share/tenant-a"), "{s}");
    }

    #[test]
    fn build_mount_args_ipv6_wraps_in_brackets() {
        let args = build_mount_args(
            "fe80::1",
            "share",
            None,
            None,
            None,
            None,
            None,
            None,
            "/var/lib/nqrust/smb/x",
        );
        assert!(
            args.iter().any(|a| a.contains("//[fe80::1]/share")),
            "{args:?}"
        );
    }

    #[test]
    fn build_mount_args_appends_extra_options() {
        let args = build_mount_args(
            "srv",
            "s",
            None,
            None,
            None,
            None,
            None,
            Some("uid=33,gid=33,file_mode=0660"),
            "/m",
        );
        let s = args.join(" ");
        assert!(s.contains("uid=33,gid=33,file_mode=0660"), "{s}");
    }

    #[test]
    fn mount_point_for_uses_safe_share_chars() {
        let mp = mount_point_for("/var/lib/nqrust/smb", "192.168.1.5", "vm/data");
        assert_eq!(
            mp.to_string_lossy(),
            "/var/lib/nqrust/smb/192.168.1.5:vm_data"
        );
    }
}
