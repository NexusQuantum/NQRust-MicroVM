use std::path::Path as StdPath;

use crate::{AppState, DownloadProgress};
use axum::{
    extract::{Multipart, Path, Query},
    http::StatusCode,
    Extension, Json,
};
use nexus_types::{
    CreateImageReq, CreateImageResp, DockerHubSearchReq, DockerHubSearchResp, DockerImageTagsResp,
    DownloadDockerImageReq, DownloadDockerImageResp, GetImageResp, ImageFilter, ImagePathParams,
    ListImagesResp, OkResponse,
};

#[utoipa::path(
    post,
    path = "/v1/images",
    request_body = CreateImageReq,
    responses(
        (status = 200, description = "Image registered", body = CreateImageResp),
        (status = 400, description = "Invalid image path"),
        (status = 500, description = "Failed to store image metadata"),
    ),
    tag = "Images"
)]
pub async fn create(
    Extension(st): Extension<AppState>,
    Json(req): Json<CreateImageReq>,
) -> Result<Json<CreateImageResp>, StatusCode> {
    if !st.images.is_path_allowed(StdPath::new(&req.host_path)) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let image = st.images.insert(&req).await.map_err(map_repo_error)?;

    Ok(Json(CreateImageResp { id: image.id }))
}

#[utoipa::path(
    get,
    path = "/v1/images",
    params(ImageFilter),
    responses(
        (status = 200, description = "Images listed", body = ListImagesResp),
        (status = 500, description = "Failed to list images"),
    ),
    tag = "Images"
)]
pub async fn list(
    Extension(st): Extension<AppState>,
    Query(filter): Query<ImageFilter>,
) -> Result<Json<ListImagesResp>, StatusCode> {
    let items = st.images.list(&filter).await.map_err(map_repo_error)?;
    Ok(Json(ListImagesResp { items }))
}

#[utoipa::path(
    get,
    path = "/v1/images/{id}",
    params(ImagePathParams),
    responses(
        (status = 200, description = "Image fetched", body = GetImageResp),
        (status = 404, description = "Image not found"),
        (status = 500, description = "Failed to fetch image"),
    ),
    tag = "Images"
)]
pub async fn get(
    Extension(st): Extension<AppState>,
    Path(ImagePathParams { id }): Path<ImagePathParams>,
) -> Result<Json<GetImageResp>, StatusCode> {
    let item = st.images.get(id).await.map_err(map_repo_error)?;
    Ok(Json(GetImageResp { item }))
}

#[utoipa::path(
    delete,
    path = "/v1/images/{id}",
    params(ImagePathParams),
    responses(
        (status = 200, description = "Image deleted", body = OkResponse),
        (status = 404, description = "Image not found"),
        (status = 500, description = "Failed to delete image"),
    ),
    tag = "Images"
)]
pub async fn delete(
    Extension(st): Extension<AppState>,
    Path(ImagePathParams { id }): Path<ImagePathParams>,
) -> Result<Json<OkResponse>, StatusCode> {
    st.images.delete(id).await.map_err(map_repo_error)?;
    Ok(Json(OkResponse::default()))
}

fn map_repo_error(err: super::repo::ImageRepoError) -> StatusCode {
    match err {
        super::repo::ImageRepoError::InvalidPath(_) => StatusCode::BAD_REQUEST,
        super::repo::ImageRepoError::Sql(sqlx::Error::RowNotFound) => StatusCode::NOT_FOUND,
        super::repo::ImageRepoError::Sql(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

// Docker Hub API routes

#[utoipa::path(
    post,
    path = "/v1/images/dockerhub/search",
    request_body = DockerHubSearchReq,
    responses(
        (status = 200, description = "Docker Hub search results", body = DockerHubSearchResp),
        (status = 500, description = "Failed to search Docker Hub"),
    ),
    tag = "Images"
)]
pub async fn dockerhub_search(
    Extension(st): Extension<AppState>,
    Json(req): Json<DockerHubSearchReq>,
) -> Result<Json<DockerHubSearchResp>, StatusCode> {
    let dockerhub = super::dockerhub::DockerHubClient::new(st.images.root().to_path_buf());

    let items = dockerhub.search(&req.query, req.limit).await.map_err(|e| {
        tracing::error!("Docker Hub search failed: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(DockerHubSearchResp { items }))
}

#[utoipa::path(
    post,
    path = "/v1/images/dockerhub/tags",
    request_body = inline(String),
    responses(
        (status = 200, description = "Docker image tags", body = DockerImageTagsResp),
        (status = 500, description = "Failed to get image tags"),
    ),
    tag = "Images"
)]
pub async fn dockerhub_tags(
    Extension(st): Extension<AppState>,
    Json(image_name): Json<String>,
) -> Result<Json<DockerImageTagsResp>, StatusCode> {
    let dockerhub = super::dockerhub::DockerHubClient::new(st.images.root().to_path_buf());

    let items = dockerhub.get_tags(&image_name).await.map_err(|e| {
        tracing::error!("Failed to get Docker image tags: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(DockerImageTagsResp { items }))
}

#[utoipa::path(
    post,
    path = "/v1/images/dockerhub/download",
    request_body = DownloadDockerImageReq,
    responses(
        (status = 200, description = "Docker image downloaded and cached", body = DownloadDockerImageResp),
        (status = 500, description = "Failed to download image"),
    ),
    tag = "Images"
)]
pub async fn dockerhub_download(
    Extension(st): Extension<AppState>,
    Json(req): Json<DownloadDockerImageReq>,
) -> Result<Json<DownloadDockerImageResp>, StatusCode> {
    tracing::info!("Starting Docker image download: {}", req.image);

    // Initialize progress tracking
    {
        let mut progress_map = st.download_progress.lock().await;
        progress_map.insert(
            req.image.clone(),
            DownloadProgress {
                image: req.image.clone(),
                status: "Initializing...".to_string(),
                current_bytes: 0,
                total_bytes: 0,
                completed: false,
                error: None,
            },
        );
    }

    let dockerhub = super::dockerhub::DockerHubClient::new(st.images.root().to_path_buf());

    // Download the image and save as tarball
    let download_result = dockerhub
        .download_image(
            &req.image,
            req.registry_auth.as_ref(),
            st.download_progress.clone(),
        )
        .await;

    let (tarball_path, sha256, size) = match download_result {
        Ok(result) => result,
        Err(e) => {
            tracing::error!("Failed to download Docker image '{}': {}", req.image, e);
            // Update progress with error
            let mut progress_map = st.download_progress.lock().await;
            if let Some(progress) = progress_map.get_mut(&req.image) {
                progress.error = Some(e.to_string());
                progress.completed = true;
                progress.status = "Failed".to_string();
            }
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Register the image in the database
    let image_req = CreateImageReq {
        kind: "docker".to_string(),
        name: req.image.clone(),
        host_path: tarball_path.to_string_lossy().to_string(),
        sha256: sha256.clone(),
        size,
        project: Some("dockerhub".to_string()),
    };

    let image = st.images.insert(&image_req).await.map_err(map_repo_error)?;

    // Mark download as completed
    {
        let mut progress_map = st.download_progress.lock().await;
        if let Some(progress) = progress_map.get_mut(&req.image) {
            progress.completed = true;
            progress.status = "Completed".to_string();
            progress.current_bytes = size;
            progress.total_bytes = size;
        }
    }

    Ok(Json(DownloadDockerImageResp {
        id: image.id,
        path: tarball_path.to_string_lossy().to_string(),
    }))
}

#[utoipa::path(
    get,
    path = "/v1/images/dockerhub/download/progress/{image_name}",
    params(
        ("image_name" = String, Path, description = "Docker image name (e.g., nginx:latest)")
    ),
    responses(
        (status = 200, description = "Download progress", body = DownloadProgress),
        (status = 404, description = "Download not found"),
    ),
    tag = "Images"
)]
pub async fn dockerhub_download_progress(
    Extension(st): Extension<AppState>,
    Path(image_name): Path<String>,
) -> Result<Json<DownloadProgress>, StatusCode> {
    // Decode URL-encoded image name (e.g., "postgres%3Alatest" -> "postgres:latest")
    let decoded_name = urlencoding::decode(&image_name)
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .to_string();

    let progress_map = st.download_progress.lock().await;

    // Try both encoded and decoded names for compatibility
    if let Some(progress) = progress_map
        .get(&decoded_name)
        .or_else(|| progress_map.get(&image_name))
    {
        Ok(Json(progress.clone()))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

#[utoipa::path(
    post,
    path = "/v1/images/dockerhub/preload",
    responses(
        (status = 200, description = "Default images pre-loaded", body = inline(Vec<String>)),
        (status = 500, description = "Failed to pre-load images"),
    ),
    tag = "Images"
)]
pub async fn dockerhub_preload(
    Extension(st): Extension<AppState>,
) -> Result<Json<Vec<String>>, StatusCode> {
    let loaded_ids = super::preload::preload_default_images(
        st.images.root().to_path_buf(),
        &st.images,
        st.download_progress.clone(),
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to pre-load default images: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(
        loaded_ids.into_iter().map(|id| id.to_string()).collect(),
    ))
}

/// Import a VMware VMDK (or any qemu-img-readable disk) as a registered
/// image, optionally running virt-v2v to adapt the guest drivers from
/// VMware's vmxnet3/pvscsi to virtio. Pure server-side operation: the
/// VMDK must already live somewhere the manager can read.
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct ImportVmdkRequest {
    /// Filesystem path of the source VMDK / qcow2 / raw disk. Must be
    /// readable by the manager process.
    pub source_path: String,
    /// Name for the resulting image. Defaults to the source filename.
    #[serde(default)]
    pub name: Option<String>,
    /// If true, run `virt-v2v -i disk` to convert the guest's drivers from
    /// VMware paravirt to virtio. Recommended when importing Windows or
    /// older Linux guests. Pure-format conversion (qemu-img convert) is
    /// used when false — faster, but the guest may not boot if it was
    /// using vmxnet3 / pvscsi inside.
    #[serde(default = "default_true")]
    pub run_virt_v2v: bool,
}

fn default_true() -> bool {
    true
}

#[utoipa::path(
    post,
    path = "/v1/images/import/vmdk",
    request_body = ImportVmdkRequest,
    responses(
        (status = 200, description = "VMDK imported", body = CreateImageResp),
        (status = 400, description = "Source not readable"),
        (status = 500, description = "virt-v2v / qemu-img conversion failed"),
    ),
    tag = "Images"
)]
pub async fn import_vmdk(
    Extension(st): Extension<AppState>,
    Json(req): Json<ImportVmdkRequest>,
) -> Result<Json<CreateImageResp>, StatusCode> {
    let source = std::path::Path::new(&req.source_path);
    if tokio::fs::metadata(source).await.is_err() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let name = req
        .name
        .clone()
        .or_else(|| {
            source
                .file_stem()
                .and_then(|s| s.to_str())
                .map(String::from)
        })
        .unwrap_or_else(|| "imported".to_string());
    let dest_dir = st.images.root().join("imported");
    if let Err(e) = tokio::fs::create_dir_all(&dest_dir).await {
        tracing::error!(?e, "create import dest dir");
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    let dest = dest_dir.join(format!("{name}.qcow2"));

    if req.run_virt_v2v {
        // virt-v2v -i disk <source> -o local -of qcow2 -os <dest_dir> -on <name>
        let out = tokio::process::Command::new("virt-v2v")
            .args([
                "-i",
                "disk",
                source.to_str().unwrap_or(""),
                "-o",
                "local",
                "-of",
                "qcow2",
                "-os",
            ])
            .arg(&dest_dir)
            .arg("-on")
            .arg(&name)
            .output()
            .await
            .map_err(|e| {
                tracing::error!(?e, "virt-v2v spawn (is libguestfs-tools installed?)");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        if !out.status.success() {
            tracing::error!(stderr=%String::from_utf8_lossy(&out.stderr), "virt-v2v failed");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
        // virt-v2v `-o local` writes the converted disk as "<name>-sda" (qcow2)
        // plus a libvirt "<name>.xml" — NOT "<name>.qcow2". Rename the disk to
        // the path the rest of this handler expects, and drop the XML.
        let v2v_disk = dest_dir.join(format!("{name}-sda"));
        if let Err(e) = tokio::fs::rename(&v2v_disk, &dest).await {
            tracing::error!(?e, src=%v2v_disk.display(), dst=%dest.display(),
                "virt-v2v output rename failed (expected <name>-sda)");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
        let _ = tokio::fs::remove_file(dest_dir.join(format!("{name}.xml"))).await;
    } else {
        // Plain qemu-img convert. Faster, but no driver adaptation.
        let out = tokio::process::Command::new("qemu-img")
            .arg("convert")
            .arg("-O")
            .arg("qcow2")
            .arg(source)
            .arg(&dest)
            .output()
            .await
            .map_err(|e| {
                tracing::error!(?e, "qemu-img spawn");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        if !out.status.success() {
            tracing::error!(stderr=%String::from_utf8_lossy(&out.stderr), "qemu-img convert failed");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }
    let meta = tokio::fs::metadata(&dest)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let sha = sha256_file(&dest).await.unwrap_or_default();
    let image_req = nexus_types::CreateImageReq {
        kind: "rootfs".to_string(),
        name,
        host_path: dest.display().to_string(),
        sha256: sha,
        size: meta.len() as i64,
        project: Some("imported".to_string()),
    };
    let image = st.images.insert(&image_req).await.map_err(map_repo_error)?;
    // Tag as uefi_disk — modern VMware exports are typically UEFI; operator
    // can adjust via image PATCH if needed.
    let _ = sqlx::query(
        r#"UPDATE image
            SET image_kind = 'uefi_disk',
                guest_os_hint = 'linux',
                disk_format = 'qcow2',
                nvram_template_path = '/usr/share/edk2/x64/OVMF_VARS.4m.fd'
            WHERE id = $1"#,
    )
    .bind(image.id)
    .execute(&st.db)
    .await;
    Ok(Json(CreateImageResp { id: image.id }))
}

/// Agentless P2V / B2V (baremetal-to-VM): the manager opens an SSH session to a
/// reachable physical machine, streams the chosen block device off it, writes a
/// local qcow2, and (optionally) runs virt-v2v to adapt the guest's drivers to
/// virtio. No agent is installed on the source. For a consistent image the
/// source should be quiesced or live-USB-booted — imaging a live root disk is
/// crash-consistent only (like a hard reset), which most Linux/Windows guests
/// recover from but is not transactional.
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct ImportP2vRequest {
    /// SSH host (IP or DNS) of the physical machine to image.
    pub ssh_host: String,
    /// SSH port. Defaults to 22.
    #[serde(default = "default_ssh_port")]
    pub ssh_port: u16,
    /// SSH username. Must be root, or a user with passwordless sudo, so it can
    /// read the raw block device.
    pub ssh_user: String,
    /// SSH password. Provide this or `ssh_key_path` (one is required).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ssh_password: Option<String>,
    /// Path to an SSH private key readable by the manager. Alternative to
    /// `ssh_password`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ssh_key_path: Option<String>,
    /// Source block device on the remote host, e.g. `/dev/sda` or
    /// `/dev/nvme0n1`. Image the whole disk (not a partition) so the result is
    /// bootable.
    pub source_disk: String,
    /// Name for the resulting image.
    pub name: String,
    /// Run `virt-v2v -i disk` to adapt the guest's drivers to virtio
    /// (recommended — physical machines use vendor/AHCI/NVMe drivers a VM lacks).
    #[serde(default = "default_true")]
    pub run_virt_v2v: bool,
}

fn default_ssh_port() -> u16 {
    22
}

/// Single-quote a value for safe interpolation into a remote shell command.
fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

#[utoipa::path(
    post,
    path = "/v1/images/import/p2v",
    request_body = ImportP2vRequest,
    responses(
        (status = 200, description = "Physical disk imported", body = CreateImageResp),
        (status = 400, description = "Missing credentials"),
        (status = 500, description = "SSH stream / virt-v2v / qemu-img failed"),
    ),
    tag = "Images"
)]
pub async fn import_p2v(
    Extension(st): Extension<AppState>,
    Json(req): Json<ImportP2vRequest>,
) -> Result<Json<CreateImageResp>, StatusCode> {
    if req.ssh_password.is_none() && req.ssh_key_path.is_none() {
        tracing::error!("p2v: ssh_password or ssh_key_path is required");
        return Err(StatusCode::BAD_REQUEST);
    }
    let name = req.name.clone();
    let dest_dir = st.images.root().join("imported");
    if let Err(e) = tokio::fs::create_dir_all(&dest_dir).await {
        tracing::error!(?e, "create import dest dir");
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    // Raw disk streamed off the physical host, before any conversion. `dd` is
    // universally present on the source (no qemu-img dependency on baremetal).
    let staged = dest_dir.join(format!("{name}-p2v-src.raw"));
    let dest = dest_dir.join(format!("{name}.qcow2"));

    // Build the ssh invocation. sshpass wraps ssh for password auth; key auth
    // uses `ssh -i` directly.
    let mut cmd;
    let mut args: Vec<String> = Vec::new();
    if let Some(pw) = req.ssh_password.as_deref() {
        cmd = tokio::process::Command::new("sshpass");
        args.push("-p".into());
        args.push(pw.into());
        args.push("ssh".into());
    } else {
        cmd = tokio::process::Command::new("ssh");
    }
    args.extend([
        "-o".into(),
        "StrictHostKeyChecking=no".into(),
        "-o".into(),
        "UserKnownHostsFile=/dev/null".into(),
        "-o".into(),
        "ConnectTimeout=20".into(),
        "-p".into(),
        req.ssh_port.to_string(),
    ]);
    if let Some(key) = req.ssh_key_path.as_deref() {
        args.push("-i".into());
        args.push(key.into());
    }
    args.push(format!("{}@{}", req.ssh_user, req.ssh_host));
    // Remote: stream the raw block device to stdout. Non-root users need
    // passwordless sudo to read the device.
    let sudo = if req.ssh_user == "root" {
        ""
    } else {
        "sudo -n "
    };
    args.push(format!(
        "{sudo}dd if={} bs=4M status=none",
        sh_quote(&req.source_disk)
    ));

    // Redirect the SSH stdout straight into the staging file.
    let file = match std::fs::File::create(&staged) {
        Ok(f) => f,
        Err(e) => {
            tracing::error!(?e, "p2v: create staging file");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    let spawn = cmd
        .args(&args)
        .stdout(std::process::Stdio::from(file))
        .stderr(std::process::Stdio::piped())
        .spawn();
    let child = match spawn {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(?e, "p2v: ssh/sshpass spawn (is sshpass installed?)");
            let _ = tokio::fs::remove_file(&staged).await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    let out = match child.wait_with_output().await {
        Ok(o) => o,
        Err(e) => {
            tracing::error!(?e, "p2v: ssh stream wait");
            let _ = tokio::fs::remove_file(&staged).await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    if !out.status.success() {
        tracing::error!(stderr=%String::from_utf8_lossy(&out.stderr), "p2v: ssh disk stream failed");
        let _ = tokio::fs::remove_file(&staged).await;
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Convert the staged raw image to the registered qcow2, adapting drivers
    // with virt-v2v when requested. virt-v2v reads a whole-disk raw image via
    // `-i disk` and inspects its partitions directly.
    if req.run_virt_v2v {
        let v2v = tokio::process::Command::new("virt-v2v")
            .args(["-i", "disk"])
            .arg(&staged)
            .args(["-o", "local", "-of", "qcow2", "-os"])
            .arg(&dest_dir)
            .arg("-on")
            .arg(&name)
            .output()
            .await
            .map_err(|e| {
                tracing::error!(?e, "p2v: virt-v2v spawn");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        if !v2v.status.success() {
            tracing::error!(stderr=%String::from_utf8_lossy(&v2v.stderr), "p2v: virt-v2v failed");
            let _ = tokio::fs::remove_file(&staged).await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
        // virt-v2v `-o local` writes "<name>-sda" + "<name>.xml" (see import_vmdk).
        let v2v_disk = dest_dir.join(format!("{name}-sda"));
        if let Err(e) = tokio::fs::rename(&v2v_disk, &dest).await {
            tracing::error!(?e, src=%v2v_disk.display(), "p2v: virt-v2v output rename failed");
            let _ = tokio::fs::remove_file(&staged).await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
        let _ = tokio::fs::remove_file(dest_dir.join(format!("{name}.xml"))).await;
    } else {
        let conv = tokio::process::Command::new("qemu-img")
            .args(["convert", "-f", "raw", "-O", "qcow2"])
            .arg(&staged)
            .arg(&dest)
            .output()
            .await
            .map_err(|e| {
                tracing::error!(?e, "p2v: qemu-img spawn");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        if !conv.status.success() {
            tracing::error!(stderr=%String::from_utf8_lossy(&conv.stderr), "p2v: qemu-img convert failed");
            let _ = tokio::fs::remove_file(&staged).await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }
    // The raw intermediate is large; drop it now that the qcow2 exists.
    let _ = tokio::fs::remove_file(&staged).await;

    let meta = tokio::fs::metadata(&dest)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let sha = sha256_file(&dest).await.unwrap_or_default();
    let image_req = nexus_types::CreateImageReq {
        kind: "rootfs".to_string(),
        name,
        host_path: dest.display().to_string(),
        sha256: sha,
        size: meta.len() as i64,
        project: Some("imported".to_string()),
    };
    let image = st.images.insert(&image_req).await.map_err(map_repo_error)?;
    let _ = sqlx::query(
        r#"UPDATE image
            SET image_kind = 'uefi_disk',
                guest_os_hint = 'linux',
                disk_format = 'qcow2',
                nvram_template_path = '/usr/share/edk2/x64/OVMF_VARS.4m.fd'
            WHERE id = $1"#,
    )
    .bind(image.id)
    .execute(&st.db)
    .await;
    Ok(Json(CreateImageResp { id: image.id }))
}

async fn sha256_file(path: &std::path::Path) -> Option<String> {
    use sha2::{Digest, Sha256};
    use tokio::io::AsyncReadExt;
    let mut f = tokio::fs::File::open(path).await.ok()?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 65536];
    loop {
        let n = f.read(&mut buf).await.ok()?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Some(hex::encode(hasher.finalize()))
}

#[utoipa::path(
    post,
    path = "/v1/images/upload",
    request_body(content = inline(String), description = "Multipart form data with 'file' and 'kind' fields", content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Image uploaded successfully", body = CreateImageResp),
        (status = 400, description = "Invalid file or missing fields"),
        (status = 500, description = "Upload failed"),
    ),
    tag = "Images"
)]
pub async fn upload_image(
    Extension(st): Extension<AppState>,
    mut multipart: Multipart,
) -> Result<Json<CreateImageResp>, StatusCode> {
    let mut kind: Option<String> = None;
    let mut name: Option<String> = None;
    let mut project: Option<String> = None;
    // 0.5.0+ VMM-aware fields.
    let mut image_kind: Option<String> = None;
    let mut nvram_template_path: Option<String> = None;

    // Multipart fields are processed in arrival order, but the handler is
    // order-independent: the `file` part is streamed to a staging directory in
    // this same pass (we must NOT break and re-iterate, or the file part gets
    // skipped), and the final destination is resolved from `kind` after the
    // loop. `file_path` below holds the staged path until then.
    let mut file_path: Option<std::path::PathBuf> = None;
    let mut sha256: Option<String> = None;
    let mut size: Option<i64> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
    {
        let field_name = field.name().unwrap_or("").to_string();

        match field_name.as_str() {
            "kind" => {
                kind = Some(field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?);
            }
            "name" => {
                name = Some(field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?);
            }
            "project" => {
                project = Some(field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?);
            }
            "image_kind" => {
                image_kind = Some(field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?);
            }
            "nvram_template_path" => {
                nvram_template_path =
                    Some(field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?);
            }
            "file" => {
                // Stream the file to a staging dir without requiring `kind` to
                // have arrived yet — browsers send the `file` part before the
                // `kind` text field, so resolving the destination here would
                // wrongly 400. The final directory is resolved after the loop.
                let staging = st.images.root().join(".staging");
                let (p, s, sz) = super::upload::write_field_to_disk(field, staging, "upload")
                    .await
                    .map_err(|e| {
                        tracing::error!("File upload failed: {}", e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?;
                file_path = Some(p);
                sha256 = Some(s);
                size = Some(sz);
            }
            _ => {}
        }
    }

    let kind = kind.ok_or(StatusCode::BAD_REQUEST)?;
    let staged_path = file_path.ok_or(StatusCode::BAD_REQUEST)?;
    // Resolve the destination now that every text field has been parsed, then
    // move the staged file into place. This makes the handler independent of
    // multipart field ordering.
    let upload_dir = match kind.as_str() {
        "docker" => st.images.root().join("docker"),
        "kernel" | "rootfs" => st.images.root().to_path_buf(),
        _ => {
            let _ = tokio::fs::remove_file(&staged_path).await;
            return Err(StatusCode::BAD_REQUEST);
        }
    };
    let file_path = super::upload::move_into_dir(&staged_path, upload_dir)
        .await
        .map_err(|e| {
            tracing::error!("Failed to finalize uploaded file: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    let sha256 = sha256.unwrap_or_default();
    let size = size.unwrap_or(0);

    // If Docker image, load it to get the actual image name
    let default_name = || {
        file_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    };
    let image_name = if kind == "docker" {
        match super::upload::load_docker_image_from_tarball(&file_path).await {
            Ok(name) => name,
            Err(e) => {
                tracing::warn!("Failed to load Docker image, using filename: {}", e);
                name.unwrap_or_else(default_name)
            }
        }
    } else {
        name.unwrap_or_else(default_name)
    };

    // Register in database
    let image_req = CreateImageReq {
        kind,
        name: image_name,
        host_path: file_path.to_string_lossy().to_string(),
        sha256,
        size,
        project: project.or(Some("uploaded".to_string())),
    };

    let image = st.images.insert(&image_req).await.map_err(map_repo_error)?;

    // Persist the VMM-aware fields if the client supplied them. Defaults
    // ('linux_kernel' for the strict enum) are already set by the migration.
    if image_kind.is_some() || nvram_template_path.is_some() {
        let effective_kind = image_kind.as_deref().unwrap_or("linux_kernel");
        // Validate against the strict enum so the CHECK constraint won't
        // bounce us at INSERT time.
        let allowed = matches!(
            effective_kind,
            "linux_kernel" | "linux_disk" | "uefi_disk" | "installer_iso"
        );
        if !allowed {
            return Err(StatusCode::BAD_REQUEST);
        }
        if let Err(e) = sqlx::query(
            r#"UPDATE image
                SET image_kind = $2,
                    nvram_template_path = COALESCE($3, nvram_template_path)
                WHERE id = $1"#,
        )
        .bind(image.id)
        .bind(effective_kind)
        .bind(nvram_template_path.as_deref())
        .execute(&st.db)
        .await
        {
            tracing::warn!(image_id=%image.id, error=?e, "failed to set image_kind / nvram_template_path");
        }
    }

    Ok(Json(CreateImageResp { id: image.id }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::hosts::repo::HostRepository;
    use crate::features::images::repo::ImageRepository;
    use axum::{extract::Path, Extension};
    use nexus_types::CreateImageReq;

    async fn test_registry(pool: &sqlx::PgPool) -> crate::features::storage::registry::Registry {
        crate::features::storage::registry::Registry::load(pool, None)
            .await
            .expect("registry")
    }

    #[ignore]
    #[sqlx::test(migrations = "./migrations")]
    async fn create_and_list_images(pool: sqlx::PgPool) {
        let hosts = HostRepository::new(pool.clone());
        let images = ImageRepository::new(pool.clone(), "/srv/images");
        let snapshots = crate::features::snapshots::repo::SnapshotRepository::new(pool.clone());
        let storage = crate::features::storage::LocalStorage::new();
        storage.init().await.unwrap();
        let shell_repo = crate::features::vms::shell::ShellRepository::new(pool.clone());
        let download_progress =
            std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
        let users = crate::features::users::repo::UserRepository::new(pool.clone());
        let registry = test_registry(&pool).await;
        let state = crate::AppState {
            db: pool.clone(),
            hosts,
            images: images.clone(),
            snapshots,
            users,
            shell_repo,
            licensing: crate::features::licensing::repo::LicensingRepository::new(pool.clone()),
            allow_direct_image_paths: true,
            storage,
            registry,
            download_progress,
            license_state: std::sync::Arc::new(tokio::sync::RwLock::new(
                nexus_types::LicenseState::default(),
            )),
            license_config: crate::features::licensing::license_service::LicenseConfig::from_env(),
            sso_providers: crate::features::sso::repo::SsoProviderRepository::new(pool.clone()),
            user_identities: crate::features::sso::repo::UserIdentityRepository::new(pool.clone()),
            auth_states: crate::features::sso::repo::AuthStateRepository::new(pool.clone()),
            sso_base_url: "http://localhost:18080".to_string(),
            sso_frontend_url: "http://localhost:3000".to_string(),
            sso_encryption_key: crate::features::sso::crypto::derive_key("test-key"),
        };

        let req = CreateImageReq {
            kind: "kernel".into(),
            name: "linux".into(),
            host_path: "/srv/images/vmlinux".into(),
            sha256: "deadbeef".into(),
            size: 1234,
            project: Some("default".into()),
        };

        let Json(resp) = super::create(Extension(state.clone()), Json(req.clone()))
            .await
            .unwrap();

        let Json(list) = super::list(Extension(state.clone()), Query(ImageFilter::default()))
            .await
            .unwrap();
        assert_eq!(list.items.len(), 1);
        assert_eq!(list.items[0].id, resp.id);

        let Json(item) = super::get(
            Extension(state.clone()),
            Path(ImagePathParams { id: resp.id }),
        )
        .await
        .unwrap();
        assert_eq!(item.item.name, req.name);

        let Json(ok) = super::delete(
            Extension(state.clone()),
            Path(ImagePathParams { id: resp.id }),
        )
        .await
        .unwrap();
        assert_eq!(ok, OkResponse::default());
    }

    #[ignore]
    #[sqlx::test(migrations = "./migrations")]
    async fn reject_out_of_root_path(pool: sqlx::PgPool) {
        let hosts = HostRepository::new(pool.clone());
        let images = ImageRepository::new(pool.clone(), "/srv/images");
        let snapshots = crate::features::snapshots::repo::SnapshotRepository::new(pool.clone());
        let shell_repo = crate::features::vms::shell::ShellRepository::new(pool.clone());
        let storage = crate::features::storage::LocalStorage::new();
        storage.init().await.unwrap();
        let download_progress =
            std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
        let users = crate::features::users::repo::UserRepository::new(pool.clone());
        let registry = test_registry(&pool).await;
        let state = crate::AppState {
            db: pool.clone(),
            hosts,
            images,
            snapshots,
            users,
            shell_repo,
            licensing: crate::features::licensing::repo::LicensingRepository::new(pool.clone()),
            allow_direct_image_paths: true,
            storage,
            registry,
            download_progress,
            license_state: std::sync::Arc::new(tokio::sync::RwLock::new(
                nexus_types::LicenseState::default(),
            )),
            license_config: crate::features::licensing::license_service::LicenseConfig::from_env(),
            sso_providers: crate::features::sso::repo::SsoProviderRepository::new(pool.clone()),
            user_identities: crate::features::sso::repo::UserIdentityRepository::new(pool.clone()),
            auth_states: crate::features::sso::repo::AuthStateRepository::new(pool.clone()),
            sso_base_url: "http://localhost:18080".to_string(),
            sso_frontend_url: "http://localhost:3000".to_string(),
            sso_encryption_key: crate::features::sso::crypto::derive_key("test-key"),
        };

        let req = CreateImageReq {
            kind: "kernel".into(),
            name: "bad".into(),
            host_path: "/etc/passwd".into(),
            sha256: "deadbeef".into(),
            size: 1234,
            project: None,
        };

        let result = super::create(Extension(state), Json(req)).await;
        assert_eq!(result.unwrap_err(), StatusCode::BAD_REQUEST);
    }
}
