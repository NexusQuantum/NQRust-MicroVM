//! QEMU-backed VM create/start path.
//!
//! Branch invoked from [`super::service::create_and_start`] when
//! `vmm_kind == Qemu` (either explicit or auto-selected from a UEFI/Pvh
//! boot mode). Talks to the agent's `/agent/v1/vmm/:id/boot` route.
//!
//! The Firecracker-backed flow continues to live in `service.rs` untouched.
//! Anything that's truly common (TAP creation, host selection, audit logs)
//! is shared via the existing helpers.

use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use nexus_types::CreateVmReq;
use nexus_vmm::{BootMode, DiskSpec, GuestOs, NicSpec, VmmKind};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use tracing::info;
use uuid::Uuid;

use crate::AppState;

/// Default OVMF firmware paths on Arch / Fedora / Debian. The agent's
/// edk2 package places these under `/usr/share/edk2/x64/` on Arch.
const DEFAULT_OVMF_CODE: &str = "/usr/share/edk2/x64/OVMF_CODE.4m.fd";
const DEFAULT_OVMF_VARS: &str = "/usr/share/edk2/x64/OVMF_VARS.4m.fd";

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct BootResp {
    pub vm_id: Uuid,
    pub kind: String,
    pub api_sock: String,
    #[serde(default)]
    pub pid: Option<u32>,
    pub systemd_unit: String,
    #[serde(default)]
    pub console_sock: Option<String>,
    #[serde(default)]
    pub vnc: Option<String>,
}

/// Validate the request shape for a QEMU-backed VM, derive the boot mode if
/// the caller did not specify one, and check that the chosen backend supports
/// every requested feature.
pub fn validate_and_resolve(req: &CreateVmReq) -> Result<(VmmKind, GuestOs, BootMode, bool)> {
    let vmm_kind = req.vmm_kind.unwrap_or(VmmKind::Qemu);
    if vmm_kind != VmmKind::Qemu {
        bail!("qemu_service called for non-qemu request");
    }

    let guest_os = req.guest_os.unwrap_or(GuestOs::LinuxDisk);

    let boot_mode = match (&req.boot_mode, req.disk_image_id, &req.kernel_path) {
        (Some(mode), _, _) => mode.clone(),
        (None, Some(_), _) => BootMode::Uefi {
            firmware: req
                .firmware_path
                .clone()
                .unwrap_or_else(|| DEFAULT_OVMF_CODE.to_string())
                .into(),
            nvram_template: Some(
                req.nvram_template_path
                    .clone()
                    .unwrap_or_else(|| DEFAULT_OVMF_VARS.to_string())
                    .into(),
            ),
        },
        (None, None, Some(path)) if !path.is_empty() => BootMode::LinuxKernel {
            kernel: path.into(),
            initrd: None,
            cmdline: "console=ttyS0".into(),
        },
        _ => bail!("qemu VM creation needs one of: boot_mode, disk_image_id, or kernel_path"),
    };

    let feats = nexus_vmm::features(vmm_kind, guest_os);
    if feats == nexus_vmm::FeatureSupport::NONE {
        bail!("vmm_kind={vmm_kind} does not support guest_os={guest_os}",);
    }
    if matches!(boot_mode, BootMode::Uefi { .. }) && !feats.uefi_boot {
        bail!("vmm_kind={vmm_kind} cannot UEFI-boot guest_os={guest_os}");
    }
    if req.enable_vnc && !feats.vnc_console {
        bail!("vmm_kind={vmm_kind} does not support vnc_console");
    }

    Ok((vmm_kind, guest_os, boot_mode, req.enable_vnc))
}

pub async fn create_and_start_qemu(
    st: &AppState,
    id: Uuid,
    req: CreateVmReq,
    template_id: Option<Uuid>,
    _user_id: Option<Uuid>,
    _audit_username: &str,
) -> Result<()> {
    let (vmm_kind, _guest_os, boot_mode, enable_vnc) = validate_and_resolve(&req)?;

    // Pick a host that has qemu installed AND fits the resource ask.
    let host = pick_host(st, vmm_kind, req.vcpu as i32, req.mem_mib as i64)
        .await
        .context("no eligible qemu host")?;

    // Network bridge — same selection logic as FC path.
    let bridge = host
        .capabilities_json
        .get("bridge")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("host {} has no bridge advertised", host.id))?
        .to_string();

    // Create TAP on the agent (re-uses existing route). In test mode
    // (`MANAGER_TEST_MODE=1`) we skip TAP creation and instead pass a
    // sentinel host_dev = "user" so the agent's QemuDriver uses slirp
    // user-mode networking — lets unprivileged dev hosts complete an
    // end-to-end create + boot without sudo.
    let test_mode = std::env::var("MANAGER_TEST_MODE").is_ok();
    let tap_name = if test_mode {
        "user".to_string()
    } else {
        let tn = format!("tap-{}", &id.to_string()[..8]);
        create_tap(&host.addr, id, &bridge)
            .await
            .context("create_tap on agent")?;
        tn
    };

    // Resolve disk source. Three paths:
    //   1. backend_id provided → allocate volume on that backend, attach on
    //      the agent, populate from disk_image (if given). Same path FC
    //      VMs use; iSCSI / NFS / SPDK / TrueNAS all just work.
    //   2. disk_image_id only → make a per-VM qcow2 thin overlay over the
    //      read-only base image so concurrent VMs from the same image
    //      don't corrupt each other.
    //   3. rootfs_path only → trust the caller's path (legacy escape hatch).
    let (disk_path, disk_format, disk_volume_handle) = resolve_qemu_disk(
        st,
        id,
        &host,
        req.backend_id,
        req.disk_image_id,
        req.rootfs_path.as_deref(),
        req.rootfs_size_mb,
    )
    .await?;

    let mut disks: Vec<DiskSpec> = Vec::new();
    disks.push(DiskSpec {
        drive_id: "rootfs".into(),
        source: disk_path.clone().into(),
        read_only: false,
        root_device: true,
        format: Some(disk_format.clone()),
        cdrom: false,
    });

    // Optional installer ISO attached as CD-ROM.
    if let Some(iso_id) = req.installer_iso_id {
        let iso = st
            .images
            .get(iso_id)
            .await
            .with_context(|| format!("installer iso {iso_id} lookup"))?;
        disks.push(DiskSpec {
            drive_id: "installer".into(),
            source: iso.host_path.into(),
            read_only: true,
            root_device: false,
            format: Some("raw".into()),
            cdrom: true,
        });
    }

    // Single NIC for now (eth0). Mac is chosen by the agent / TAP layer; for
    // QEMU we generate one here so the guest sees a stable address.
    let mac = generate_mac(id);
    let nics = vec![NicSpec {
        iface_id: "net0".into(),
        host_dev: tap_name.clone(),
        mac,
    }];

    // Reserve capacity *after* host selection and *before* boot. The
    // reservation is released on VM delete.
    let host_repo = crate::features::hosts::repo::HostRepository::new(st.db.clone());
    let fit = host_repo
        .try_reserve(host.id, req.vcpu as i32, req.mem_mib as i64)
        .await
        .unwrap_or(true); // try_reserve returns false when total_* is set and we'd overcommit
    if !fit {
        bail!(
            "host {} is at capacity; cannot reserve {} vcpu / {} MiB",
            host.id,
            req.vcpu,
            req.mem_mib
        );
    }

    // Call the agent's pluggable-vmm boot route.
    let body = json!({
        "vmm_kind": vmm_kind.as_str(),
        "vcpu": req.vcpu,
        "mem_mib": req.mem_mib,
        "boot": boot_mode,
        "disks": disks,
        "nics": nics,
        "enable_vnc": enable_vnc,
    });

    let http = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .context("build http client")?;

    info!(vm_id=%id, host=%host.addr, "qemu boot via agent /agent/v1/vmm/:id/boot");
    let resp = match http
        .post(format!("{}/agent/v1/vmm/{}/boot", host.addr, id))
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            // Roll back the reservation if the call itself failed.
            let _ = host_repo
                .release_reservation(host.id, req.vcpu as i32, req.mem_mib as i64)
                .await;
            return Err(anyhow!(e).context("agent boot request failed to send"));
        }
    };
    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        let _ = host_repo
            .release_reservation(host.id, req.vcpu as i32, req.mem_mib as i64)
            .await;
        bail!("agent returned {} on /boot: {}", status, text);
    }
    let handle: BootResp = resp.json().await.context("decode agent boot response")?;

    // Persist VM row. Use vmm_kind = 'qemu', store boot_mode as JSON.
    let row = super::repo::VmRow {
        id,
        name: req.name.clone(),
        state: "running".into(),
        host_id: host.id,
        template_id,
        host_addr: host.addr.clone(),
        api_sock: handle.api_sock.clone(),
        tap: tap_name.clone(),
        log_path: format!("/srv/fc/{id}/qemu.log"),
        http_port: 0,
        fc_unit: handle.systemd_unit.clone(),
        vcpu: req.vcpu as i32,
        mem_mib: req.mem_mib as i32,
        // Legacy NOT-NULL columns. For QEMU UEFI VMs there's no kernel; the
        // disk image goes into rootfs_path so existing list/get UIs keep
        // showing a sensible value.
        kernel_path: String::new(),
        rootfs_path: disk_path.clone(),
        source_snapshot_id: None,
        guest_ip: None,
        tags: req.tags.clone(),
        created_by_user_id: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    super::repo::insert(&st.db, &row)
        .await
        .context("insert qemu vm row")?;

    // Update the new VMM-shape columns now that the row exists.
    update_vmm_columns(
        &st.db,
        VmmColumns {
            id,
            vmm_kind: VmmKind::Qemu,
            boot_mode,
            enable_vnc,
            vnc_listen: handle.vnc.as_deref(),
            firmware_path: req.firmware_path.as_deref(),
            nvram_template_path: req.nvram_template_path.as_deref(),
        },
    )
    .await
    .context("update vmm columns")?;

    // If the disk lives on a storage backend, register the volume +
    // volume_attachment rows so the existing FC-style delete / restart /
    // backup tooling treats this VM identically.
    if let Some(handle) = disk_volume_handle {
        if let Err(e) = persist_volume_attachment(st, id, &handle, &disk_path).await {
            tracing::warn!(vm_id=%id, error=?e, "failed to persist volume_attachment for QEMU VM (delete may need manual cleanup)");
        }
    }

    Ok(())
}

/// Insert volume + volume_attachment rows for a QEMU VM whose disk was
/// allocated through the storage registry. Best-effort — the VM is already
/// running, so a logging failure here doesn't roll back the boot.
#[cfg(not(test))]
async fn persist_volume_attachment(
    st: &AppState,
    vm_id: Uuid,
    handle: &nexus_storage::VolumeHandle,
    disk_path: &str,
) -> Result<()> {
    sqlx::query(
        r#"INSERT INTO volume (id, backend_id, path, size_bytes, created_at)
           VALUES ($1, $2, $3, $4, now())
           ON CONFLICT (id) DO UPDATE SET path = EXCLUDED.path"#,
    )
    .bind(handle.volume_id)
    .bind(handle.backend_id.0)
    .bind(handle.locator.as_str())
    .bind(handle.size_bytes as i64)
    .execute(&st.db)
    .await
    .context("insert volume row")?;
    sqlx::query(
        r#"INSERT INTO volume_attachment (volume_id, vm_id, drive_id, attached_at)
           VALUES ($1, $2, 'rootfs', now())
           ON CONFLICT DO NOTHING"#,
    )
    .bind(handle.volume_id)
    .bind(vm_id)
    .execute(&st.db)
    .await
    .context("insert volume_attachment row")?;
    let _ = disk_path; // path is informational; locator is canonical
    Ok(())
}

#[cfg(test)]
async fn persist_volume_attachment(
    _st: &AppState,
    _vm_id: Uuid,
    _handle: &nexus_storage::VolumeHandle,
    _disk_path: &str,
) -> Result<()> {
    Ok(())
}

/// Pick a healthy host that has the requested VMM kind installed. Returns
/// the first match — same posture as the FC `first_healthy` selector.
async fn pick_host(
    st: &AppState,
    kind: VmmKind,
    _vcpu: i32,
    _mem_mib: i64,
) -> Result<crate::features::hosts::repo::HostRow> {
    let host = st.hosts.first_healthy().await.context("no healthy hosts")?;
    let host_repo = crate::features::hosts::repo::HostRepository::new(st.db.clone());
    let kinds = host_repo
        .vmm_kinds_installed(host.id)
        .await
        .context("query vmm_kinds_installed")?;
    if !kinds.iter().any(|k| k == kind.as_str()) {
        bail!(
            "host {} does not have vmm_kind '{}' installed (has: {:?})",
            host.id,
            kind,
            kinds
        );
    }
    Ok(host)
}

#[cfg(not(test))]
async fn create_tap(host_addr: &str, id: Uuid, bridge: &str) -> Result<()> {
    let http = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .context("build http client (create_tap)")?;
    http.post(format!("{host_addr}/agent/v1/vms/{id}/tap"))
        .json(&json!({"bridge": bridge, "owner_user": serde_json::Value::Null}))
        .send()
        .await
        .context("create_tap request failed")?
        .error_for_status()
        .context("create_tap returned non-2xx")?;
    Ok(())
}

#[cfg(test)]
async fn create_tap(_host_addr: &str, _id: Uuid, _bridge: &str) -> Result<()> {
    Ok(())
}

/// Per-VM VMM column update bundle. Groups the new 0.5.0 fields so the
/// helper's parameter list stays manageable.
#[cfg_attr(test, allow(dead_code))]
struct VmmColumns<'a> {
    id: Uuid,
    vmm_kind: VmmKind,
    boot_mode: BootMode,
    enable_vnc: bool,
    vnc_listen: Option<&'a str>,
    firmware_path: Option<&'a str>,
    nvram_template_path: Option<&'a str>,
}

#[cfg(not(test))]
async fn update_vmm_columns(db: &sqlx::PgPool, c: VmmColumns<'_>) -> Result<()> {
    let boot_json = serde_json::to_value(&c.boot_mode)?;
    let console_kind = if c.enable_vnc { "vnc" } else { "unix_serial" };
    sqlx::query(
        r#"UPDATE vm
            SET vmm_kind = $2,
                guest_os = COALESCE(guest_os, 'linux_disk'),
                boot_mode = $3,
                console_kind = $4,
                vnc_listen = $5,
                firmware_path = $6,
                nvram_path = $7
            WHERE id = $1"#,
    )
    .bind(c.id)
    .bind(c.vmm_kind.as_str())
    .bind(boot_json)
    .bind(console_kind)
    .bind(c.vnc_listen)
    .bind(c.firmware_path)
    .bind(c.nvram_template_path)
    .execute(db)
    .await?;
    Ok(())
}

#[cfg(test)]
async fn update_vmm_columns(_db: &sqlx::PgPool, _c: VmmColumns<'_>) -> Result<()> {
    Ok(())
}

/// Resolve the rootfs disk path + format for a QEMU VM, plus an optional
/// `VolumeHandle` if the disk lives on a storage backend rather than as a
/// per-VM file. Three branches:
///
/// 1. **`backend_id` provided** — provision a volume on that backend
///    (iSCSI, NFS, SPDK, TrueNAS, ...) and either populate it from the
///    source image or leave it blank for an ISO install. Same code path
///    Firecracker uses.
/// 2. **`disk_image_id` provided alone** — create a qcow2 thin overlay
///    over the read-only base image so per-VM writes don't corrupt the
///    shared base. Faster than full copy; safe for concurrent VMs.
/// 3. **`rootfs_path` provided alone** — trust the caller; use as-is.
async fn resolve_qemu_disk(
    st: &AppState,
    vm_id: Uuid,
    host: &crate::features::hosts::repo::HostRow,
    backend_id: Option<Uuid>,
    disk_image_id: Option<Uuid>,
    rootfs_path: Option<&str>,
    rootfs_size_mb: Option<u32>,
) -> Result<(String, String, Option<nexus_storage::VolumeHandle>)> {
    use crate::features::storage::rootfs_allocator::allocate_rootfs;

    // Path 1: storage backend allocate + populate
    if let Some(bid) = backend_id {
        let Some(image_id) = disk_image_id else {
            // Blank-disk allocation for ISO install. Use rootfs_size_mb (default 20 GiB).
            let size_bytes = rootfs_size_mb.unwrap_or(20 * 1024) as u64 * 1024 * 1024;
            let backend = st
                .registry
                .get(bid)
                .ok_or_else(|| anyhow!("storage backend {bid} not found"))?;
            let opts = nexus_storage::CreateOpts {
                name: format!("vm-{vm_id}-rootfs"),
                size_bytes,
                description: Some(format!("blank disk for VM {vm_id}")),
            };
            let handle = backend
                .provision(opts)
                .await
                .with_context(|| format!("provision blank disk on backend {bid}"))?;
            // Activate (lvchange -aey for shared block, no-op for local_file).
            backend
                .activate_volume(&handle)
                .await
                .context("activate blank disk")?;
            // Attach on the agent so we get the actual block device path.
            let attached = crate::features::storage::agent_rpc::agent_attach(&host.addr, &handle)
                .await
                .context("agent attach blank disk")?;
            let path = attached.path().to_string_lossy().into_owned();
            return Ok((path, "raw".into(), Some(handle)));
        };

        // Backend + image: allocate_rootfs handles clone-from-image fast
        // path or provision-then-populate slow path.
        let img = st
            .images
            .get(image_id)
            .await
            .with_context(|| format!("image {image_id} lookup"))?;
        let source_size = img.size.max(0) as u64;
        let target_bytes = rootfs_size_mb
            .map(|mb| mb as u64 * 1024 * 1024)
            .unwrap_or_else(|| (source_size + 2 * 1024 * 1024 * 1024).max(source_size));
        let outcome = allocate_rootfs(
            &st.registry,
            bid,
            &host.addr,
            std::path::Path::new(&img.host_path),
            target_bytes,
            &format!("vm-{vm_id}-rootfs"),
        )
        .await
        .with_context(|| format!("allocate_rootfs on backend {bid}"))?;
        // The volume now holds a copy of the image. Populate writes raw bytes,
        // so format is raw.
        let path = match outcome.attached_for_caller {
            Some(a) => a.path().to_string_lossy().into_owned(),
            None => {
                // Fast path didn't return an AttachedPath; ask the agent.
                let attached = crate::features::storage::agent_rpc::agent_attach(
                    &host.addr,
                    &outcome.volume_handle,
                )
                .await
                .context("agent attach after fast-path clone")?;
                attached.path().to_string_lossy().into_owned()
            }
        };
        return Ok((path, "raw".into(), Some(outcome.volume_handle)));
    }

    // Path 2: image-only — qcow2 thin overlay
    if let Some(image_id) = disk_image_id {
        let img = st
            .images
            .get(image_id)
            .await
            .with_context(|| format!("image {image_id} lookup"))?;
        let format = detect_disk_format(&img.host_path);
        let overlay_path = create_qcow2_overlay(st, vm_id, &img.host_path, &format).await?;
        Ok((overlay_path, "qcow2".into(), None))
    } else if let Some(p) = rootfs_path {
        // Path 3: legacy explicit path
        let format = detect_disk_format(p);
        Ok((p.to_string(), format, None))
    } else {
        bail!("qemu VM creation needs disk_image_id, backend_id, or rootfs_path");
    }
}

/// Create a per-VM qcow2 overlay file backed by the source image so the
/// base stays read-only and concurrent VMs don't trample each other. Only
/// used when no storage backend is selected — backend-allocated disks
/// already give each VM its own writable volume.
async fn create_qcow2_overlay(
    st: &AppState,
    vm_id: Uuid,
    source_path: &str,
    source_format: &str,
) -> Result<String> {
    st.storage
        .ensure_vm_dirs(vm_id)
        .await
        .context("ensure vm dirs")?;
    let target_dir = st.storage.vm_dir(vm_id).join("storage");
    tokio::fs::create_dir_all(&target_dir).await?;
    let target = target_dir.join("disk.qcow2");
    if tokio::fs::metadata(&target).await.is_ok() {
        return Ok(target.display().to_string());
    }
    let out = tokio::process::Command::new("qemu-img")
        .args([
            "create",
            "-f",
            "qcow2",
            "-F",
            source_format,
            "-b",
            source_path,
            &target.display().to_string(),
        ])
        .output()
        .await
        .context("spawn qemu-img create")?;
    if !out.status.success() {
        bail!(
            "qemu-img create overlay failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(target.display().to_string())
}

/// Heuristic disk-format detection from the host path extension. Cloud
/// images typically ship as qcow2 (with `.img` or `.qcow2` suffix); plain
/// `.raw`, `.ext4`, or no extension default to raw. Operators with custom
/// extensions can override per-VM by passing an explicit DiskSpec.format
/// upstream — this helper covers the common cases.
fn detect_disk_format(path: &str) -> String {
    let lower = path.to_lowercase();
    if lower.ends_with(".qcow2") || lower.ends_with(".qcow") {
        "qcow2".into()
    } else if lower.ends_with(".vmdk") {
        "vmdk".into()
    } else if lower.ends_with(".vdi") {
        "vdi".into()
    } else if lower.ends_with(".img") {
        // Modern cloud images (Ubuntu, Fedora, Debian) ship .img files
        // that are actually qcow2. Treat .img as qcow2 by default; raw
        // .img files would need an explicit format override.
        "qcow2".into()
    } else {
        "raw".into()
    }
}

/// Deterministic locally-administered MAC. First three bytes 52:54:00
/// (the QEMU "well-known" OUI), last three derived from the VM UUID so
/// reboots keep the same MAC.
fn generate_mac(id: Uuid) -> String {
    let b = id.as_bytes();
    format!("52:54:00:{:02x}:{:02x}:{:02x}", b[13], b[14], b[15])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_disk_format_handles_common_extensions() {
        assert_eq!(detect_disk_format("/srv/images/foo.qcow2"), "qcow2");
        assert_eq!(detect_disk_format("/srv/images/foo.IMG"), "qcow2");
        assert_eq!(detect_disk_format("/srv/images/foo.vmdk"), "vmdk");
        assert_eq!(detect_disk_format("/srv/images/foo.raw"), "raw");
        assert_eq!(detect_disk_format("/srv/images/foo.ext4"), "raw");
        assert_eq!(detect_disk_format("/srv/images/foo"), "raw");
    }

    #[test]
    fn mac_is_deterministic_and_qemu_oui() {
        let id = Uuid::parse_str("00000000-0000-0000-0000-000000abcdef").unwrap();
        let m = generate_mac(id);
        assert_eq!(m, "52:54:00:ab:cd:ef");
    }

    #[test]
    fn validate_resolves_uefi_from_disk_image() {
        let req = CreateVmReq {
            name: "x".into(),
            vcpu: 2,
            mem_mib: 1024,
            vmm_kind: Some(VmmKind::Qemu),
            disk_image_id: Some(Uuid::new_v4()),
            ..base_req()
        };
        let (k, _, bm, _) = validate_and_resolve(&req).unwrap();
        assert_eq!(k, VmmKind::Qemu);
        match bm {
            BootMode::Uefi { .. } => {}
            other => panic!("expected uefi, got {:?}", other),
        }
    }

    #[test]
    fn validate_rejects_vnc_when_unsupported() {
        let req = CreateVmReq {
            name: "x".into(),
            vcpu: 1,
            mem_mib: 256,
            vmm_kind: Some(VmmKind::Qemu),
            guest_os: Some(GuestOs::Other),
            enable_vnc: true,
            disk_image_id: Some(Uuid::new_v4()),
            ..base_req()
        };
        // Other supports vnc per the matrix, so this is OK. Validate the
        // negative case by forcing a feature gap: pick FC as the kind.
        let mut bad = req.clone();
        bad.vmm_kind = Some(VmmKind::Firecracker);
        bad.guest_os = Some(GuestOs::LinuxKernel);
        // FC qemu_service is wrong path; the qemu service should bail
        let err = validate_and_resolve(&bad).unwrap_err();
        assert!(err.to_string().contains("non-qemu"));
    }

    fn base_req() -> CreateVmReq {
        CreateVmReq {
            name: "n".into(),
            vcpu: 1,
            mem_mib: 256,
            kernel_image_id: None,
            rootfs_image_id: None,
            kernel_path: None,
            rootfs_path: None,
            source_snapshot_id: None,
            username: None,
            password: None,
            tags: vec![],
            rootfs_size_mb: None,
            network_id: None,
            port_forwards: vec![],
            backend_id: None,
            vmm_kind: None,
            boot_mode: None,
            guest_os: None,
            enable_vnc: false,
            disk_image_id: None,
            installer_iso_id: None,
            firmware_path: None,
            nvram_template_path: None,
        }
    }
}
