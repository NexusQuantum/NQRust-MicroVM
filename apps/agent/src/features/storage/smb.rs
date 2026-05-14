//! SMB / CIFS host backend (Tasks 3–5: cred + arg helpers + mount drivers).
//!
//! Several functions below are flagged `dead_code` until Task 7 wires
//! them to HTTP routes (the manager calls them via `agent_client`).
//! Until then, suppress the lint at the module level so CI stays green.
#![allow(dead_code)]

use std::path::PathBuf;

/// Compute the canonical cred-file path for a backend.
pub fn cred_file_path(backend_id: &uuid::Uuid) -> std::path::PathBuf {
    std::path::PathBuf::from("/etc/nqrust/storage-creds").join(format!("{backend_id}.cred"))
}

/// Write credentials atomically with mode 0600 + the directory created if
/// missing. Overwrites any existing file (used for rotation).
pub async fn write_cred_file(
    path: &std::path::Path,
    username: &str,
    password: &str,
    domain: Option<&str>,
) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    use tokio::io::AsyncWriteExt;
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
        // Lock down the parent dir too so no other user can list cred files.
        // Best-effort — ignore failures (e.g. when run as non-root in tests).
        let _ = tokio::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700)).await;
    }
    let mut f = tokio::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .mode(0o600)
        .open(path)
        .await?;
    f.write_all(format!("username={username}\npassword={password}\n").as_bytes())
        .await?;
    if let Some(d) = domain.filter(|d| !d.trim().is_empty()) {
        f.write_all(format!("domain={d}\n").as_bytes()).await?;
    }
    Ok(())
}

/// Remove the credential file. Best-effort — missing file is not an error.
pub async fn delete_cred_file(path: &std::path::Path) {
    let _ = tokio::fs::remove_file(path).await;
}

/// Build the argv for `/bin/mount -t cifs ... -o ...`. Returns the args
/// AFTER the `mount` executable name (so the caller does
/// `Command::new("mount").args(&args)`).
#[allow(clippy::too_many_arguments)]
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

/// Build the expected `findmnt` SOURCE string for an SMB mount. This must
/// match what mount.cifs writes into `/proc/self/mountinfo` — namely the
/// `//<server>/<share>[/<subdir>]` form with raw IPv6 wrapped in `[]`.
fn expected_source(server: &str, share: &str, subdir: Option<&str>) -> String {
    let server_h = if server.contains(':') && !server.starts_with('[') {
        format!("[{server}]")
    } else {
        server.to_string()
    };
    let mut s = format!("//{server_h}/{share}");
    if let Some(sub) = subdir.map(str::trim).filter(|s| !s.is_empty()) {
        s.push('/');
        s.push_str(sub.trim_start_matches('/'));
    }
    s
}

/// Idempotent mount. If already mounted with the expected source, returns
/// the mount-point path. If mounted with a different source, errors out.
/// Otherwise issues `mount -t cifs ...` using `build_mount_args`.
#[allow(clippy::too_many_arguments)]
pub async fn ensure_mounted(
    backend_id: uuid::Uuid,
    mount_base: &std::path::Path,
    server: &str,
    share: &str,
    subdir: Option<&str>,
    username: Option<&str>,
    domain: Option<&str>,
    smb_version: Option<&str>,
    extra_options: Option<&str>,
) -> Result<std::path::PathBuf, nexus_storage::StorageError> {
    let base_str = mount_base.to_string_lossy();
    let mp = mount_point_for(&base_str, server, share);
    tokio::fs::create_dir_all(&mp).await?;

    // Probe existing mount via `findmnt --mountpoint` (exact match — not
    // `--target`, which walks up parents and would falsely report the
    // rootfs when our directory isn't itself a mountpoint).
    let want = expected_source(server, share, subdir);
    let probe = tokio::process::Command::new("findmnt")
        .arg("--mountpoint")
        .arg(&mp)
        .arg("--noheadings")
        .arg("--output")
        .arg("SOURCE")
        .output()
        .await;
    let source_line = match probe {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        // findmnt exited non-zero: the path is not a mountpoint.
        Ok(_) => String::new(),
        Err(e) => {
            return Err(nexus_storage::StorageError::backend(std::io::Error::other(
                format!("findmnt not available: {e}"),
            )));
        }
    };
    if source_line == want {
        return Ok(mp);
    }
    if !source_line.is_empty() {
        return Err(nexus_storage::StorageError::backend(std::io::Error::other(
            format!(
                "{} is mounted but as '{}', not '{}'",
                mp.display(),
                source_line,
                want
            ),
        )));
    }

    // Verify the cred file exists when authenticated. The manager's
    // create handler writes it before invoking mount; if it's missing
    // here, fail with a clear message instead of letting mount.cifs
    // fall through to a cryptic "permission denied".
    let cred_path = cred_file_path(&backend_id);
    let cred_arg = if username
        .map(str::trim)
        .map(|u| !u.is_empty())
        .unwrap_or(false)
    {
        if !tokio::fs::try_exists(&cred_path).await.unwrap_or(false) {
            return Err(nexus_storage::StorageError::backend(std::io::Error::other(
                format!(
                    "credentials file missing for backend {backend_id}: {}",
                    cred_path.display()
                ),
            )));
        }
        Some(cred_path.to_string_lossy().into_owned())
    } else {
        None
    };

    let args = build_mount_args(
        server,
        share,
        subdir,
        cred_arg.as_deref(),
        username,
        domain,
        smb_version,
        extra_options,
        &mp.to_string_lossy(),
    );

    let out = tokio::process::Command::new("mount")
        .args(&args)
        .output()
        .await
        .map_err(|e| {
            nexus_storage::StorageError::backend(std::io::Error::other(format!(
                "mount.cifs spawn: {e}"
            )))
        })?;
    if !out.status.success() {
        let code = out
            .status
            .code()
            .map(|c| c.to_string())
            .unwrap_or_else(|| "signal".to_string());
        let stderr_tail = String::from_utf8_lossy(&out.stderr);
        let tail = stderr_tail.trim();
        let tail = if tail.len() > 512 {
            &tail[tail.len() - 512..]
        } else {
            tail
        };
        return Err(nexus_storage::StorageError::backend(std::io::Error::other(
            format!(
                "mount.cifs failed: exit {code}; check credentials, smbversion, options ({tail})"
            ),
        )));
    }
    Ok(mp)
}

/// Best-effort unmount. Missing mount point is not an error — we always
/// return Ok(()).
pub async fn unmount(mp: &std::path::Path) -> Result<(), nexus_storage::StorageError> {
    let _ = tokio::process::Command::new("umount")
        .arg(mp)
        .status()
        .await;
    Ok(())
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

    #[test]
    fn expected_source_matches_mount_cifs_source_string() {
        assert_eq!(expected_source("srv", "share", None), "//srv/share");
        assert_eq!(
            expected_source("srv", "share", Some("tenant-a")),
            "//srv/share/tenant-a"
        );
        assert_eq!(
            expected_source("srv", "share", Some("/tenant-a")),
            "//srv/share/tenant-a"
        );
        // IPv6 servers are bracketed so findmnt's SOURCE column matches
        // what mount.cifs wrote.
        assert_eq!(
            expected_source("fe80::1", "share", None),
            "//[fe80::1]/share"
        );
        // Pre-bracketed IPv6 isn't double-wrapped.
        assert_eq!(
            expected_source("[fe80::1]", "share", None),
            "//[fe80::1]/share"
        );
        // Empty subdir is treated as None.
        assert_eq!(expected_source("srv", "share", Some("   ")), "//srv/share");
    }
}

#[cfg(test)]
mod cred_tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[tokio::test]
    async fn cred_file_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.cred");
        write_cred_file(&path, "user", "pass", Some("DOM"))
            .await
            .unwrap();
        let perms = tokio::fs::metadata(&path).await.unwrap().permissions();
        assert_eq!(perms.mode() & 0o777, 0o600);
        let content = tokio::fs::read_to_string(&path).await.unwrap();
        assert!(content.contains("username=user"), "{content}");
        assert!(content.contains("password=pass"), "{content}");
        assert!(content.contains("domain=DOM"), "{content}");
    }

    #[tokio::test]
    async fn cred_file_no_domain() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("nodomain.cred");
        write_cred_file(&path, "u", "p", None).await.unwrap();
        let content = tokio::fs::read_to_string(&path).await.unwrap();
        assert!(!content.contains("domain="), "{content}");
    }

    #[tokio::test]
    async fn cred_file_empty_domain_treated_as_none() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("emptydom.cred");
        write_cred_file(&path, "u", "p", Some("   ")).await.unwrap();
        let content = tokio::fs::read_to_string(&path).await.unwrap();
        assert!(!content.contains("domain="), "{content}");
    }

    #[tokio::test]
    async fn delete_cred_file_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("ghost.cred");
        delete_cred_file(&path).await; // not present, should not panic
        write_cred_file(&path, "u", "p", None).await.unwrap();
        assert!(tokio::fs::try_exists(&path).await.unwrap());
        delete_cred_file(&path).await;
        assert!(!tokio::fs::try_exists(&path).await.unwrap());
    }

    #[test]
    fn cred_file_path_uses_uuid_under_storage_creds() {
        let id = uuid::Uuid::nil();
        let p = cred_file_path(&id);
        assert_eq!(
            p.to_string_lossy(),
            "/etc/nqrust/storage-creds/00000000-0000-0000-0000-000000000000.cred"
        );
    }
}
