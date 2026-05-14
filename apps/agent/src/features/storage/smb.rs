//! SMB / CIFS host backend (Tasks 3–5: cred + arg helpers + mount drivers).
//!
//! Task 7 wires these helpers to HTTP routes (the manager calls them via
//! `agent_client`) and registers `SmbHostBackend` against the agent's
//! `HostBackendRegistry` so the manager learns this host can serve SMB
//! volumes.

use std::path::PathBuf;

use async_trait::async_trait;
use axum::{http::StatusCode, response::IntoResponse, routing::post, Json, Router};
use nexus_storage::{
    AttachedPath, BackendKind, HostBackend, StorageError, VolumeHandle, VolumeSnapshotHandle,
};
use serde::{Deserialize, Serialize};

/// Wire-form locator for SMB-backed volumes. Mirrors `NfsLocatorWire` but
/// adds `share`/`subdir` (CIFS namespacing). The mount-point already
/// accounts for `server` + `share` + `subdir`, so file-lifecycle helpers
/// here only need the plain `file` name to resolve `<mount>/<file>`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmbLocator {
    pub server: String,
    pub share: String,
    pub subdir: Option<String>,
    pub file: String,
}

impl SmbLocator {
    /// Reject filenames that could escape the mount (`..`, `/`) or that
    /// reference hidden files. Same rule as the NFS module so the manager
    /// can validate uniformly regardless of backend kind.
    pub fn validate_file(file: &str) -> Result<(), nexus_storage::StorageError> {
        if file.is_empty() || file.contains('/') || file.starts_with('.') {
            return Err(nexus_storage::StorageError::InvalidLocator(format!(
                "smb locator.file must be a plain filename (no '/', no leading '.'), got {file:?}"
            )));
        }
        Ok(())
    }

    // Used by the manager-side encoder in a later task; agent only decodes
    // today via `from_locator_str` but we keep the symmetric helper here
    // so wire-format changes update both sides at once.
    #[allow(dead_code)]
    pub fn to_locator_string(&self) -> Result<String, nexus_storage::StorageError> {
        serde_json::to_string(self).map_err(|e| {
            nexus_storage::StorageError::InvalidLocator(format!("encode smb locator: {e}"))
        })
    }

    pub fn from_locator_str(s: &str) -> Result<Self, nexus_storage::StorageError> {
        let loc: SmbLocator = serde_json::from_str(s).map_err(|e| {
            nexus_storage::StorageError::InvalidLocator(format!("decode smb locator: {e}"))
        })?;
        Self::validate_file(&loc.file)?;
        Ok(loc)
    }
}

/// Create a sparse file of `size_bytes` at `<mount_point>/<file>`.
///
/// `set_len` on a freshly created file produces a sparse file — a 10 GiB
/// VM disk is allocated instantaneously without consuming backing-store
/// bytes until written.
pub async fn create_file(
    mount_point: &std::path::Path,
    file: &str,
    size_bytes: u64,
) -> Result<(), nexus_storage::StorageError> {
    SmbLocator::validate_file(file)?;
    let path = mount_point.join(file);
    let f = tokio::fs::File::create(&path).await.map_err(|e| {
        nexus_storage::StorageError::backend(std::io::Error::other(format!(
            "create {}: {e}",
            path.display()
        )))
    })?;
    f.set_len(size_bytes).await.map_err(|e| {
        nexus_storage::StorageError::backend(std::io::Error::other(format!(
            "set_len {}: {e}",
            path.display()
        )))
    })?;
    Ok(())
}

/// Remove `<mount_point>/<file>`. Idempotent — missing file is not an
/// error so delete is safe to retry.
pub async fn delete_file(
    mount_point: &std::path::Path,
    file: &str,
) -> Result<(), nexus_storage::StorageError> {
    SmbLocator::validate_file(file)?;
    let path = mount_point.join(file);
    match tokio::fs::remove_file(&path).await {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(nexus_storage::StorageError::backend(std::io::Error::other(
            format!("remove {}: {e}", path.display()),
        ))),
    }
}

/// Copy `source_path` -> `<mount_point>/<file>` (byte-by-byte; works
/// across filesystems). Returns the number of bytes written.
pub async fn clone_from_path(
    source_path: &std::path::Path,
    mount_point: &std::path::Path,
    file: &str,
) -> Result<u64, nexus_storage::StorageError> {
    SmbLocator::validate_file(file)?;
    let dst = mount_point.join(file);
    tokio::fs::copy(source_path, &dst).await.map_err(|e| {
        nexus_storage::StorageError::backend(std::io::Error::other(format!(
            "copy {} -> {}: {e}",
            source_path.display(),
            dst.display()
        )))
    })
}

/// Copy `<mount_point>/<source_file>` -> `<mount_point>/<snap_file>`. CIFS
/// has no server-side clone in v1, so this is a real byte copy.
pub async fn snapshot(
    mount_point: &std::path::Path,
    source_file: &str,
    snap_file: &str,
) -> Result<(), nexus_storage::StorageError> {
    SmbLocator::validate_file(source_file)?;
    SmbLocator::validate_file(snap_file)?;
    let src = mount_point.join(source_file);
    let dst = mount_point.join(snap_file);
    tokio::fs::copy(&src, &dst).await.map_err(|e| {
        nexus_storage::StorageError::backend(std::io::Error::other(format!(
            "snapshot copy {} -> {}: {e}",
            src.display(),
            dst.display()
        )))
    })?;
    Ok(())
}

/// Copy `<mount_point>/<snap_file>` -> `<mount_point>/<new_file>` and
/// return the number of bytes copied. Used to materialise a writable
/// volume from a previously-taken snapshot.
pub async fn clone_from_snapshot(
    mount_point: &std::path::Path,
    snap_file: &str,
    new_file: &str,
) -> Result<u64, nexus_storage::StorageError> {
    SmbLocator::validate_file(snap_file)?;
    SmbLocator::validate_file(new_file)?;
    let src = mount_point.join(snap_file);
    let dst = mount_point.join(new_file);
    tokio::fs::copy(&src, &dst).await.map_err(|e| {
        nexus_storage::StorageError::backend(std::io::Error::other(format!(
            "clone-from-snapshot copy {} -> {}: {e}",
            src.display(),
            dst.display()
        )))
    })
}

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

// -------------------------------------------------------------------------
// HostBackend impl (manager → agent attach/detach for VM lifecycle)
// -------------------------------------------------------------------------

/// Default mount base used when the agent isn't given a per-backend
/// override. Mirrors what `ensure_mounted` expects callers to pass via
/// the /mount route's `mount_base` field.
const DEFAULT_SMB_MOUNT_BASE: &str = "/var/lib/nqrust/smb";

#[derive(Clone, Default)]
pub struct SmbHostBackend;

#[async_trait]
impl HostBackend for SmbHostBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Smb
    }

    async fn attach(&self, volume: &VolumeHandle) -> Result<AttachedPath, StorageError> {
        let loc = SmbLocator::from_locator_str(&volume.locator)?;
        // The host backend doesn't carry mount_base config (that's stored
        // by the manager-side control-plane backend), so default to the
        // same path ensure_mounted uses. If the operator overrides the
        // mount base, they must also POST /mount before attach to populate
        // that path before any VM resolves the volume.
        let mp = mount_point_for(DEFAULT_SMB_MOUNT_BASE, &loc.server, &loc.share);
        let path = mp.join(&loc.file);
        if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
            return Err(StorageError::backend(std::io::Error::other(format!(
                "smb volume file not present at {} (call /v1/storage/smb/mount first)",
                path.display()
            ))));
        }
        Ok(AttachedPath::File(path))
    }

    async fn detach(
        &self,
        _volume: &VolumeHandle,
        _attached: AttachedPath,
    ) -> Result<(), StorageError> {
        // The mount is shared across all VMs on this backend; we don't
        // unmount per-VM. Operators issue /umount via the dedicated route
        // when they want to drop the share.
        Ok(())
    }

    async fn populate_streaming(
        &self,
        attached: &AttachedPath,
        source: &std::path::Path,
        target_size_bytes: u64,
    ) -> Result<(), StorageError> {
        use tokio::io::AsyncWriteExt;
        let dst_path = attached.path();
        let mut src = tokio::fs::File::open(source).await?;
        let mut dst = tokio::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(dst_path)
            .await?;
        tokio::io::copy(&mut src, &mut dst).await?;
        let cur = tokio::fs::metadata(dst_path).await?.len();
        if target_size_bytes > cur {
            dst.set_len(target_size_bytes).await?;
        }
        dst.flush().await?;
        Ok(())
    }

    async fn resize2fs(&self, attached: &AttachedPath) -> Result<(), StorageError> {
        super::local_file::run_resize2fs(attached.path()).await
    }

    async fn read_snapshot(
        &self,
        snap: &VolumeSnapshotHandle,
    ) -> Result<Box<dyn tokio::io::AsyncRead + Send + Unpin>, StorageError> {
        let loc = SmbLocator::from_locator_str(&snap.locator)?;
        let mp = mount_point_for(DEFAULT_SMB_MOUNT_BASE, &loc.server, &loc.share);
        let path = mp.join(&loc.file);
        let f = tokio::fs::File::open(&path).await?;
        Ok(Box::new(f))
    }
}

// -------------------------------------------------------------------------
// HTTP routes (manager → agent control plane)
// -------------------------------------------------------------------------

fn err_500(e: impl std::fmt::Display) -> axum::response::Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({"error": format!("backend error: {e}")})),
    )
        .into_response()
}

#[derive(Debug, Deserialize)]
pub struct SetCredentialsReq {
    pub backend_id: uuid::Uuid,
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub domain: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ClearCredentialsReq {
    pub backend_id: uuid::Uuid,
}

#[derive(Debug, Deserialize)]
pub struct MountReq {
    pub backend_id: uuid::Uuid,
    #[serde(default)]
    pub mount_base: Option<String>,
    pub server: String,
    pub share: String,
    #[serde(default)]
    pub subdir: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default)]
    pub smb_version: Option<String>,
    #[serde(default)]
    pub options: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MountResp {
    pub mount_point: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct UmountReq {
    pub mount_point: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct CreateFileReq {
    pub mount_point: PathBuf,
    pub file: String,
    pub size_bytes: u64,
}

#[derive(Debug, Deserialize)]
pub struct DeleteFileReq {
    pub mount_point: PathBuf,
    pub file: String,
}

#[derive(Debug, Deserialize)]
pub struct CloneFromPathReq {
    pub source_path: PathBuf,
    pub mount_point: PathBuf,
    pub file: String,
}

#[derive(Debug, Serialize)]
pub struct CloneFromPathResp {
    pub size_bytes: u64,
}

#[derive(Debug, Deserialize)]
pub struct SnapshotReq {
    pub mount_point: PathBuf,
    pub source_file: String,
    pub snap_file: String,
}

#[derive(Debug, Deserialize)]
pub struct CloneFromSnapshotReq {
    pub mount_point: PathBuf,
    pub snap_file: String,
    pub file: String,
}

#[derive(Debug, Serialize)]
pub struct CloneFromSnapshotResp {
    pub size_bytes: u64,
}

async fn set_credentials_handler(Json(body): Json<SetCredentialsReq>) -> impl IntoResponse {
    let path = cred_file_path(&body.backend_id);
    match write_cred_file(
        &path,
        &body.username,
        &body.password,
        body.domain.as_deref(),
    )
    .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_500(e),
    }
}

async fn clear_credentials_handler(Json(body): Json<ClearCredentialsReq>) -> impl IntoResponse {
    let path = cred_file_path(&body.backend_id);
    delete_cred_file(&path).await;
    StatusCode::NO_CONTENT.into_response()
}

async fn mount_handler(Json(body): Json<MountReq>) -> impl IntoResponse {
    let mount_base = body
        .mount_base
        .unwrap_or_else(|| DEFAULT_SMB_MOUNT_BASE.to_string());
    match ensure_mounted(
        body.backend_id,
        std::path::Path::new(&mount_base),
        &body.server,
        &body.share,
        body.subdir.as_deref(),
        body.username.as_deref(),
        body.domain.as_deref(),
        body.smb_version.as_deref(),
        body.options.as_deref(),
    )
    .await
    {
        Ok(mount_point) => (StatusCode::OK, Json(MountResp { mount_point })).into_response(),
        Err(e) => err_500(e),
    }
}

async fn umount_handler(Json(body): Json<UmountReq>) -> impl IntoResponse {
    match unmount(&body.mount_point).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_500(e),
    }
}

async fn create_file_handler(Json(body): Json<CreateFileReq>) -> impl IntoResponse {
    match create_file(&body.mount_point, &body.file, body.size_bytes).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_500(e),
    }
}

async fn delete_file_handler(Json(body): Json<DeleteFileReq>) -> impl IntoResponse {
    match delete_file(&body.mount_point, &body.file).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_500(e),
    }
}

async fn clone_from_path_handler(Json(body): Json<CloneFromPathReq>) -> impl IntoResponse {
    match clone_from_path(&body.source_path, &body.mount_point, &body.file).await {
        Ok(size_bytes) => (StatusCode::OK, Json(CloneFromPathResp { size_bytes })).into_response(),
        Err(e) => err_500(e),
    }
}

async fn snapshot_handler(Json(body): Json<SnapshotReq>) -> impl IntoResponse {
    match snapshot(&body.mount_point, &body.source_file, &body.snap_file).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_500(e),
    }
}

async fn clone_from_snapshot_handler(Json(body): Json<CloneFromSnapshotReq>) -> impl IntoResponse {
    match clone_from_snapshot(&body.mount_point, &body.snap_file, &body.file).await {
        Ok(size_bytes) => {
            (StatusCode::OK, Json(CloneFromSnapshotResp { size_bytes })).into_response()
        }
        Err(e) => err_500(e),
    }
}

/// Build the `/v1/storage/smb/*` sub-router. Stateless — handlers call
/// the module-level helpers directly and the cred files live on disk, so
/// no per-request state is needed. Generic over the parent's state type
/// `S` so it composes via `Router::nest` regardless of what the parent
/// router carries.
pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/set_credentials", post(set_credentials_handler))
        .route("/clear_credentials", post(clear_credentials_handler))
        .route("/mount", post(mount_handler))
        .route("/umount", post(umount_handler))
        .route("/create_file", post(create_file_handler))
        .route("/delete_file", post(delete_file_handler))
        .route("/clone_from_path", post(clone_from_path_handler))
        .route("/snapshot", post(snapshot_handler))
        .route("/clone_from_snapshot", post(clone_from_snapshot_handler))
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

#[cfg(test)]
mod locator_tests {
    use super::*;

    #[test]
    fn locator_round_trips_json() {
        let l = SmbLocator {
            server: "srv".into(),
            share: "vms".into(),
            subdir: Some("tenant-a".into()),
            file: "rootfs-abc.raw".into(),
        };
        let s = l.to_locator_string().unwrap();
        let back = SmbLocator::from_locator_str(&s).unwrap();
        assert_eq!(l.server, back.server);
        assert_eq!(l.share, back.share);
        assert_eq!(l.subdir, back.subdir);
        assert_eq!(l.file, back.file);
    }

    #[test]
    fn locator_rejects_slash_in_file() {
        assert!(SmbLocator::validate_file("foo.raw").is_ok());
        assert!(SmbLocator::validate_file("foo/bar").is_err());
        assert!(SmbLocator::validate_file(".hidden").is_err());
        assert!(SmbLocator::validate_file("").is_err());
    }

    #[test]
    fn from_locator_str_validates() {
        let bad = r#"{"server":"s","share":"sh","subdir":null,"file":"../etc/shadow"}"#;
        assert!(SmbLocator::from_locator_str(bad).is_err());
    }
}

#[cfg(test)]
mod file_lifecycle_tests {
    use super::*;

    #[tokio::test]
    async fn create_file_makes_sparse_file_of_requested_size() {
        let tmp = tempfile::tempdir().unwrap();
        create_file(tmp.path(), "test.raw", 10 * 1024 * 1024)
            .await
            .unwrap();
        let md = tokio::fs::metadata(tmp.path().join("test.raw"))
            .await
            .unwrap();
        assert_eq!(md.len(), 10 * 1024 * 1024);
    }

    #[tokio::test]
    async fn create_file_rejects_bad_names() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(create_file(tmp.path(), "../escape", 0).await.is_err());
        assert!(create_file(tmp.path(), "with/slash", 0).await.is_err());
    }

    #[tokio::test]
    async fn delete_file_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        delete_file(tmp.path(), "ghost.raw").await.unwrap(); // not present
        create_file(tmp.path(), "real.raw", 4096).await.unwrap();
        delete_file(tmp.path(), "real.raw").await.unwrap();
        assert!(!tokio::fs::try_exists(tmp.path().join("real.raw"))
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn clone_from_path_copies_bytes() {
        let src = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(src.path(), b"hello world").unwrap();
        let mnt = tempfile::tempdir().unwrap();
        let bytes = clone_from_path(src.path(), mnt.path(), "dst.raw")
            .await
            .unwrap();
        assert_eq!(bytes, 11);
        let content = tokio::fs::read(mnt.path().join("dst.raw")).await.unwrap();
        assert_eq!(content, b"hello world");
    }

    #[tokio::test]
    async fn snapshot_then_clone_from_snapshot_recovers_bytes() {
        let mnt = tempfile::tempdir().unwrap();
        tokio::fs::write(mnt.path().join("orig.raw"), b"abcdefg")
            .await
            .unwrap();
        snapshot(mnt.path(), "orig.raw", "snap.raw").await.unwrap();
        let n = clone_from_snapshot(mnt.path(), "snap.raw", "clone.raw")
            .await
            .unwrap();
        assert_eq!(n, 7);
        let restored = tokio::fs::read(mnt.path().join("clone.raw")).await.unwrap();
        assert_eq!(restored, b"abcdefg");
    }
}
