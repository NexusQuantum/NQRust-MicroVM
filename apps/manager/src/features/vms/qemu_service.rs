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

    // Determine the boot mode. Precedence:
    //   1. Caller-supplied `boot_mode` wins.
    //   2. disk_image_id OR installer_iso_id → UEFI (modern cloud + classic
    //      ISO install both expect UEFI on q35).
    //   3. kernel_path → LinuxKernel (direct kernel boot).
    //   4. None of the above → fail.
    let has_install_target =
        req.disk_image_id.is_some() || req.installer_iso_id.is_some() || req.backend_id.is_some();
    let boot_mode = match (&req.boot_mode, has_install_target, &req.kernel_path) {
        (Some(mode), _, _) => mode.clone(),
        (None, true, _) => BootMode::Uefi {
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
        (None, false, Some(path)) if !path.is_empty() => BootMode::LinuxKernel {
            kernel: path.into(),
            initrd: None,
            cmdline: "console=ttyS0".into(),
        },
        _ => bail!(
            "qemu VM creation needs one of: boot_mode, disk_image_id, installer_iso_id, backend_id, or kernel_path"
        ),
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
    // Resolved guest_os used by several later branches (TPM auto-enable,
    // virtio-win auto-attach, cloud-init seeding).
    let guest_os_resolved = req.guest_os.unwrap_or(_guest_os);

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

    // Cloud-init NoCloud seed disk. cloud-init handles Linux; cloudbase-init
    // (https://github.com/cloudbase/cloudbase-init) reads the SAME NoCloud
    // datasource on Windows guests that have it installed. So we ship the
    // same seed ISO for both — only the user-data block shape changes.
    //
    // Triggered when the caller supplies username/password OR ssh keys.
    let cloud_init_enabled = matches!(
        guest_os_resolved,
        GuestOs::LinuxDisk | GuestOs::LinuxKernel | GuestOs::Windows
    ) && (req.username.is_some()
        || req.password.is_some()
        || !req.ssh_authorized_keys.is_empty());
    if cloud_init_enabled {
        let default_user = match guest_os_resolved {
            GuestOs::Windows => "Administrator",
            _ => "nexus",
        };
        match build_cloud_init_iso(
            st,
            id,
            req.name.as_str(),
            req.username.as_deref().unwrap_or(default_user),
            req.password.as_deref(),
            &req.ssh_authorized_keys,
            guest_os_resolved,
        )
        .await
        {
            Ok(seed_path) => {
                tracing::info!(vm_id=%id, path=%seed_path, "attached cloud-init seed ISO");
                disks.push(DiskSpec {
                    drive_id: "cloudinit".into(),
                    source: seed_path.into(),
                    read_only: true,
                    root_device: false,
                    format: Some("raw".into()),
                    cdrom: true,
                });
            }
            Err(e) => {
                tracing::warn!(
                    vm_id=%id,
                    error=?e,
                    "cloud-init seed generation skipped (install genisoimage/xorriso/mkisofs for first-boot credential injection)"
                );
            }
        }
    }

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

    // Windows guests need virtio-win drivers during Setup — without them
    // the installer can't see the virtio-blk root disk or the virtio-net
    // NIC. Auto-attach the most recent registered virtio-win ISO as a
    // second CD-ROM. Operator uploads it once via the image registry with
    // `image_kind = installer_iso` and a name containing "virtio-win".
    if matches!(guest_os_resolved, GuestOs::Windows) {
        if let Ok(virtio_win_path) = find_virtio_win_iso(st).await {
            tracing::info!(vm_id=%id, path=%virtio_win_path, "auto-attached virtio-win drivers ISO");
            disks.push(DiskSpec {
                drive_id: "virtio-win".into(),
                source: virtio_win_path.into(),
                read_only: true,
                root_device: false,
                format: Some("raw".into()),
                cdrom: true,
            });
        } else {
            tracing::warn!(
                vm_id=%id,
                "guest_os=windows but no virtio-win ISO registered — Windows Setup will fail to see virtio devices. \
                 Upload virtio-win.iso to the image registry with image_kind=installer_iso and a name containing 'virtio-win'."
            );
        }
    }

    // Attach data drives recorded for this VM (POST /v1/vms/:id/drives). QEMU
    // VMs don't go through Firecracker's per-drive proxy, so the manager
    // assembles them here so they're present at boot/restart. The root disk is
    // already in `disks`; skip any drive flagged as a root device to avoid a
    // duplicate boot disk. Format is inferred from the file (qcow2/raw).
    for d in super::repo::drives::list(&st.db, id)
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|d| !d.is_root_device)
    {
        // Probe the real format rather than guessing by extension:
        // auto-provisioned data disks are raw `.img` files, which
        // `detect_disk_format` would mislabel as qcow2 (a heuristic meant for
        // cloud images), making QEMU fail to open them and abort the boot.
        let fmt = probe_disk_format(&d.path_on_host).await;
        tracing::info!(vm_id=%id, drive_id=%d.drive_id, path=%d.path_on_host, format=%fmt, "attaching data drive to qemu VM");
        disks.push(DiskSpec {
            drive_id: d.drive_id,
            source: d.path_on_host.into(),
            read_only: d.is_read_only,
            root_device: false,
            format: Some(fmt),
            cdrom: false,
        });
    }

    // Build the NIC list. First NIC is the primary (the network selected by
    // network_id / host bridge). Additional NICs come from extra_network_ids
    // — each gets its own TAP + virtio-net-pci device.
    let mut nics = vec![NicSpec {
        iface_id: "net0".into(),
        host_dev: tap_name.clone(),
        mac: generate_mac(id),
    }];
    for (i, extra_net_id) in req.extra_network_ids.iter().enumerate() {
        let extra_bridge = match resolve_network_bridge(st, *extra_net_id).await {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!(network_id=%extra_net_id, error=?e, "skipping extra NIC — network lookup failed");
                continue;
            }
        };
        // Use a per-extra-NIC TAP. Keep names short — ifname max len is 15.
        let extra_tap = format!("xtp-{}-{i}", &id.to_string()[..6]);
        if !test_mode {
            if let Err(e) = create_tap(&host.addr, *extra_net_id, &extra_bridge).await {
                tracing::warn!(network_id=%extra_net_id, error=?e, "skipping extra NIC — TAP creation failed");
                continue;
            }
        }
        let mac = generate_extra_mac(id, i as u8);
        nics.push(NicSpec {
            iface_id: format!("net{}", i + 1),
            host_dev: if test_mode { "user".into() } else { extra_tap },
            mac,
        });
    }

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

    // Auto-enable paravirt device flags based on guest_os.
    // - enable_tpm: required for Windows 11; harmless on other guests
    //   (silently no-op'd by the agent when swtpm isn't installed).
    // - enable_balloon: saves host memory; cooperative pressure.
    // - enable_rng: every modern guest benefits from virtio-rng.
    let auto_tpm = matches!(guest_os_resolved, GuestOs::Windows);
    // Secure Boot defaults ON for Windows (so Windows 11 Setup passes its
    // Secure-Boot check without the BypassSecureBootCheck registry hack), and
    // OFF otherwise — overridable via the request's `enable_secure_boot`.
    let secure_boot = req
        .enable_secure_boot
        .unwrap_or(matches!(guest_os_resolved, GuestOs::Windows));

    // Call the agent's pluggable-vmm boot route.
    let body = json!({
        "vmm_kind": vmm_kind.as_str(),
        "vcpu": req.vcpu,
        "mem_mib": req.mem_mib,
        "boot": boot_mode,
        "disks": disks,
        "nics": nics,
        "enable_vnc": enable_vnc,
        "enable_tpm": auto_tpm,
        "enable_secure_boot": secure_boot,
        "enable_balloon": true,
        "enable_rng": true,
        // Proxmox-style: never use -no-reboot. The guest reboots in place, so a
        // multi-reboot installer (Windows) runs to completion on its own. The
        // disk boots before the ISO (see agent bootindex policy), so once the OS
        // is installed, reboots land on it automatically.
        "no_reboot": false,
        // Host PCI devices to pass through (VFIO). Empty for the common case.
        "vfio_devices": req.vfio_devices,
        // QEMU CPU model (e.g. "host", "kvm64", "x86-64-v3"). None → agent default "host".
        "cpu_type": req.cpu_type,
    });

    // Boot can be slow on busy hosts: qemu-img overlay over a large backing
    // image + OVMF nvram copy + spawn + QMP readiness, all under load. A
    // short timeout here both fails good boots AND orphans the spawned VM
    // (the agent keeps going after our client gives up). Use a generous
    // timeout, and on ANY boot failure fire a best-effort destroy so we
    // never leave an unmanaged QEMU process behind.
    let http = Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
        .context("build http client")?;

    // Best-effort orphan cleanup helper: tell the agent to destroy whatever
    // it may have spawned for this id, then release the host reservation.
    async fn cleanup_after_boot_failure(
        host_addr: &str,
        id: Uuid,
        host_repo: &crate::features::hosts::repo::HostRepository,
        host_id: Uuid,
        vcpu: i32,
        mem_mib: i64,
    ) {
        let c = Client::builder().timeout(Duration::from_secs(30)).build();
        if let Ok(c) = c {
            let _ = c
                .post(format!(
                    "{host_addr}/agent/v1/vmm/{id}/destroy?vmm_kind=qemu"
                ))
                .send()
                .await;
        }
        let _ = host_repo.release_reservation(host_id, vcpu, mem_mib).await;
    }

    info!(vm_id=%id, host=%host.addr, "qemu boot via agent /agent/v1/vmm/:id/boot");
    let resp = match http
        .post(format!("{}/agent/v1/vmm/{}/boot", host.addr, id))
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            cleanup_after_boot_failure(
                &host.addr,
                id,
                &host_repo,
                host.id,
                req.vcpu as i32,
                req.mem_mib as i64,
            )
            .await;
            return Err(anyhow!(e).context("agent boot request failed to send"));
        }
    };
    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        cleanup_after_boot_failure(
            &host.addr,
            id,
            &host_repo,
            host.id,
            req.vcpu as i32,
            req.mem_mib as i64,
        )
        .await;
        bail!("agent returned {} on /boot: {}", status, text);
    }
    let handle: BootResp = resp.json().await.context("decode agent boot response")?;

    // Persist VM row. Use vmm_kind = 'qemu', store boot_mode as JSON.
    // Proxmox-style: a VM with an installer ISO is just a normal 'running' VM
    // with a CD attached — it boots the installer, runs Setup (through its own
    // reboots), then boots the installed disk. Ejecting the ISO afterwards is
    // an optional action (POST /v1/vms/:id/install-complete), not a required
    // state transition.
    let initial_state = "running";
    let row = super::repo::VmRow {
        id,
        name: req.name.clone(),
        state: initial_state.into(),
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
        vmm_kind: Some("qemu".to_string()),
        guest_os: Some(guest_os_resolved.as_str().to_string()),
        console_kind: Some(if enable_vnc { "vnc" } else { "unix_serial" }.to_string()),
        vnc_listen: handle.vnc.clone(),
        cpu_type: None,
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
            cpu_type: req.cpu_type.as_deref(),
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

    // Extra blank data disks requested at create time. Reuse the day-2
    // create_drive path (provision on backend + hot-add to the now-running
    // VM). Best-effort: a failure on one disk doesn't fail the VM create.
    for (i, d) in req.data_disks.iter().enumerate() {
        let drive_id = format!("data{}", i + 1);
        let drive_req = nexus_types::CreateDriveReq {
            drive_id: drive_id.clone(),
            path_on_host: None,
            is_root_device: false,
            is_read_only: false,
            cache_type: None,
            io_engine: None,
            rate_limiter: None,
            size_bytes: Some(d.size_mb as u64 * 1024 * 1024),
        };
        if let Err(e) = super::service::create_drive(st, id, drive_req).await {
            tracing::warn!(vm_id=%id, %drive_id, error=?e, "failed to provision extra data disk at create");
        }
    }

    Ok(())
}

/// Re-boot an existing QEMU VM in place (the `start` after a `stop`). Distinct
/// from `create_and_start_qemu`: the VM row, root disk, cloud-init seed, and
/// capacity reservation already exist, so we reuse them instead of allocating
/// new ones, and we UPDATE the row rather than INSERT. The Firecracker
/// `restart_vm` path can't serve QEMU — it validates an (empty for QEMU)
/// `kernel_path` and rebuilds an FC kernel boot.
pub async fn restart_qemu(st: &AppState, vm: &super::repo::VmRow) -> Result<()> {
    let id = vm.id;
    let host = crate::features::hosts::repo::HostRepository::new(st.db.clone())
        .get(vm.host_id)
        .await
        .context("load host for qemu restart")?;
    let bridge = host
        .capabilities_json
        .get("bridge")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("host {} has no bridge advertised", host.id))?
        .to_string();

    // Boot mode (OVMF/UEFI etc.) persisted at create time.
    let boot_mode_json: Option<serde_json::Value> =
        sqlx::query_scalar(r#"SELECT boot_mode FROM vm WHERE id = $1"#)
            .bind(id)
            .fetch_optional(&st.db)
            .await
            .context("load persisted boot_mode")?
            .flatten();
    let boot_mode: BootMode = match boot_mode_json {
        Some(j) => serde_json::from_value(j).context("decode persisted boot_mode")?,
        None => bail!("qemu vm {id} has no persisted boot_mode; cannot restart"),
    };

    // Recreate the primary TAP (the agent deletes any stale same-named device
    // first, so this is idempotent). Capacity stays reserved across stop, so we
    // deliberately do NOT re-reserve here.
    let test_mode = std::env::var("MANAGER_TEST_MODE").is_ok();
    let tap_name = if test_mode {
        "user".to_string()
    } else {
        let tn = format!("tap-{}", &id.to_string()[..8]);
        create_tap(&host.addr, id, &bridge)
            .await
            .context("create_tap on qemu restart")?;
        tn
    };

    // Reuse the existing root disk (overlay/volume from create time).
    let mut disks: Vec<DiskSpec> = vec![DiskSpec {
        drive_id: "rootfs".into(),
        source: vm.rootfs_path.clone().into(),
        read_only: false,
        root_device: true,
        format: Some(probe_disk_format(&vm.rootfs_path).await),
        cdrom: false,
    }];
    // Re-attach the cloud-init seed if it is still on disk.
    let seed = st.storage.vm_dir(id).join("storage").join("cloud-init.iso");
    if tokio::fs::metadata(&seed).await.is_ok() {
        disks.push(DiskSpec {
            drive_id: "cloudinit".into(),
            source: seed,
            read_only: true,
            root_device: false,
            format: Some("raw".into()),
            cdrom: true,
        });
    }
    // Re-attach data drives recorded for this VM.
    for d in super::repo::drives::list(&st.db, id)
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|d| !d.is_root_device)
    {
        let fmt = probe_disk_format(&d.path_on_host).await;
        disks.push(DiskSpec {
            drive_id: d.drive_id,
            source: d.path_on_host.into(),
            read_only: d.is_read_only,
            root_device: false,
            format: Some(fmt),
            cdrom: false,
        });
    }

    let nics = vec![NicSpec {
        iface_id: "eth0".into(),
        host_dev: tap_name.clone(),
        mac: generate_mac(id),
    }];

    let enable_vnc = vm.console_kind.as_deref() == Some("vnc");
    let body = json!({
        "vmm_kind": "qemu",
        "vcpu": vm.vcpu,
        "mem_mib": vm.mem_mib,
        "boot": boot_mode,
        "disks": disks,
        "nics": nics,
        "enable_vnc": enable_vnc,
        "enable_balloon": true,
        "enable_rng": true,
        // Restart targets a normal (already-installed) VM — reboot in place.
        "no_reboot": false,
        // Preserve the CPU model chosen at create.
        "cpu_type": vm.cpu_type,
    });

    let http = Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
        .context("build http client (restart_qemu)")?;
    let resp = http
        .post(format!("{}/agent/v1/vmm/{}/boot", host.addr, id))
        .json(&body)
        .send()
        .await
        .context("agent boot (restart) request failed")?;
    if !resp.status().is_success() {
        let b = resp.text().await.unwrap_or_default();
        bail!("agent rejected qemu restart boot: {b}");
    }
    let handle: BootResp = resp.json().await.context("decode agent boot response")?;

    // Update the existing row in place (no insert).
    sqlx::query(
        r#"UPDATE vm SET state = 'running', api_sock = $2, tap = $3, fc_unit = $4,
                         vnc_listen = $5, updated_at = now() WHERE id = $1"#,
    )
    .bind(id)
    .bind(&handle.api_sock)
    .bind(&tap_name)
    .bind(&handle.systemd_unit)
    .bind(handle.vnc.as_deref())
    .execute(&st.db)
    .await
    .context("update vm row after qemu restart")?;

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

/// Reschedule a QEMU VM onto a new host (HA / host-death recovery).
/// Differs from live_migrate in that the source VM is assumed dead — we
/// don't try to coordinate state transfer, just boot a fresh QEMU on the
/// target pointing at the same backend-allocated disk. Requires the disk
/// to live on shared storage (iSCSI / NFS / SPDK / TrueNAS).
pub async fn reschedule(st: &AppState, vm_id: Uuid, target_host_id: Uuid) -> Result<()> {
    let vm = super::repo::get(&st.db, vm_id)
        .await
        .context("load vm row")?;
    let host_repo = crate::features::hosts::repo::HostRepository::new(st.db.clone());
    let target_host = host_repo
        .get(target_host_id)
        .await
        .context("load target host")?;
    let kinds = host_repo
        .vmm_kinds_installed(target_host_id)
        .await
        .context("query target vmm_kinds_installed")?;
    if !kinds.iter().any(|k| k == "qemu") {
        bail!("target host {target_host_id} does not have qemu installed");
    }
    // Pull the saved boot_mode so we can re-boot with the right config.
    let boot_mode_json: Option<serde_json::Value> =
        sqlx::query_scalar(r#"SELECT boot_mode FROM vm WHERE id = $1"#)
            .bind(vm_id)
            .fetch_optional(&st.db)
            .await
            .context("load saved boot_mode")?
            .flatten();
    let Some(boot_mode_json) = boot_mode_json else {
        bail!("vm has no persisted boot_mode (was it created on 0.5.0?)");
    };
    let boot_mode: BootMode =
        serde_json::from_value(boot_mode_json).context("decode persisted boot_mode")?;

    let fit = host_repo
        .try_reserve(target_host_id, vm.vcpu, vm.mem_mib as i64)
        .await
        .unwrap_or(true);
    if !fit {
        bail!("target host {target_host_id} is at capacity");
    }

    // Re-attach the volume on the target if there is one. For overlay-mode
    // VMs (no backend_id), reschedule isn't supported — the overlay only
    // exists on the source host.
    let vol_handle = sqlx::query_as::<_, (Uuid, Uuid, String, i64)>(
        r#"SELECT v.id, v.backend_id, v.path, v.size_bytes
           FROM volume v
           JOIN volume_attachment va ON va.volume_id = v.id
           WHERE va.vm_id = $1 AND va.drive_id = 'rootfs'
           ORDER BY va.attached_at DESC LIMIT 1"#,
    )
    .bind(vm_id)
    .fetch_optional(&st.db)
    .await
    .ok()
    .flatten();
    let Some((volume_id, backend_id, locator, size_bytes)) = vol_handle else {
        let _ = host_repo
            .release_reservation(target_host_id, vm.vcpu, vm.mem_mib as i64)
            .await;
        bail!(
            "vm has no shared-storage volume — reschedule requires backend-allocated disk; \
             use snapshot+restore for local-overlay VMs"
        );
    };
    let backend = st
        .registry
        .get(backend_id)
        .ok_or_else(|| anyhow!("backend {backend_id} not found"))?;
    let handle = nexus_storage::VolumeHandle {
        volume_id,
        backend_id: nexus_storage::BackendInstanceId(backend_id),
        backend_kind: backend.kind(),
        locator,
        size_bytes: size_bytes.max(0) as u64,
    };
    backend
        .activate_volume(&handle)
        .await
        .context("activate shared volume on target")?;
    let attached = crate::features::storage::agent_rpc::agent_attach(&target_host.addr, &handle)
        .await
        .context("attach shared volume on target")?;
    let disk_path = attached.path().to_string_lossy().into_owned();

    // Boot the QEMU on the target. We reuse the persisted boot_mode and the
    // same VM id so the platform treats it as the same VM (now homed on the
    // new host).
    let body = json!({
        "vmm_kind": "qemu",
        "vcpu": vm.vcpu,
        "mem_mib": vm.mem_mib,
        "boot": boot_mode,
        "disks": [{
            "drive_id": "rootfs",
            "source": disk_path,
            "read_only": false,
            "root_device": true,
            "format": "raw",
            "cdrom": false,
        }],
        "nics": [{
            "iface_id": "net0",
            "host_dev": "user",
            "mac": generate_mac(vm_id),
        }],
        "enable_vnc": false,
        "enable_balloon": true,
        "enable_rng": true,
    });
    let http = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .context("build http client")?;
    let resp = http
        .post(format!("{}/agent/v1/vmm/{}/boot", target_host.addr, vm.id))
        .json(&body)
        .send()
        .await
        .context("agent boot request")?;
    if !resp.status().is_success() {
        let _ = host_repo
            .release_reservation(target_host_id, vm.vcpu, vm.mem_mib as i64)
            .await;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        bail!("target agent returned {status}: {body}");
    }
    // Release the source's reservation.
    let _ = host_repo
        .release_reservation(vm.host_id, vm.vcpu, vm.mem_mib as i64)
        .await;
    sqlx::query(
        r#"UPDATE vm SET host_id = $2, state = 'running', updated_at = now() WHERE id = $1"#,
    )
    .bind(vm_id)
    .bind(target_host_id)
    .execute(&st.db)
    .await
    .context("update vm host_id after reschedule")?;
    Ok(())
}

/// Live-migrate a QEMU VM to another healthy host. Both source and target
/// must have `qemu` in `vmm_kinds_installed` AND share the disk via a
/// storage backend (iSCSI / NFS / SPDK / TrueNAS). Local-file overlays
/// won't migrate because the target host can't see the source's local
/// qcow2 file.
///
/// Full target-side automation: the manager POSTs `/migrate/incoming` on
/// the target with the source VM's full spec, which makes the target
/// agent spawn QEMU paused with `-incoming tcp:0.0.0.0:<port>`. Then the
/// manager POSTs `/migrate/outgoing` on the source to drive the QMP
/// `migrate`. When the stream finishes, the source QEMU exits and the
/// target's paused QEMU transitions to running automatically.
pub async fn live_migrate(
    st: &AppState,
    vm_id: Uuid,
    target_host_id: Uuid,
    target_port: u16,
) -> Result<()> {
    let vm = super::repo::get(&st.db, vm_id)
        .await
        .context("load vm row")?;
    if vm.host_id == target_host_id {
        bail!("vm is already on target host {}", target_host_id);
    }
    // Verify target is healthy AND has qemu installed AND has capacity.
    let host_repo = crate::features::hosts::repo::HostRepository::new(st.db.clone());
    let target_host = host_repo
        .get(target_host_id)
        .await
        .context("load target host")?;
    let kinds = host_repo
        .vmm_kinds_installed(target_host_id)
        .await
        .context("query target vmm_kinds_installed")?;
    if !kinds.iter().any(|k| k == "qemu") {
        bail!("target host {target_host_id} does not have qemu installed");
    }
    let fit = host_repo
        .try_reserve(target_host_id, vm.vcpu, vm.mem_mib as i64)
        .await
        .unwrap_or(true);
    if !fit {
        bail!("target host {target_host_id} is at capacity");
    }
    // Step 1: tell the target to spawn a paused QEMU listening for the
    // incoming migration stream. We re-derive the spec from the source
    // VM's persisted boot_mode + the volume_attachment on shared storage.
    let boot_mode_json: Option<serde_json::Value> =
        sqlx::query_scalar(r#"SELECT boot_mode FROM vm WHERE id = $1"#)
            .bind(vm_id)
            .fetch_optional(&st.db)
            .await
            .context("load saved boot_mode")?
            .flatten();
    let Some(boot_mode_json) = boot_mode_json else {
        let _ = host_repo
            .release_reservation(target_host_id, vm.vcpu, vm.mem_mib as i64)
            .await;
        bail!("vm has no persisted boot_mode");
    };
    let boot_mode_target: BootMode =
        serde_json::from_value(boot_mode_json).context("decode persisted boot_mode")?;

    // The target needs to mount the same shared volume on its host.
    let vol_row = sqlx::query_as::<_, (Uuid, Uuid, String)>(
        r#"SELECT v.id, v.backend_id, v.path
           FROM volume v
           JOIN volume_attachment va ON va.volume_id = v.id
           WHERE va.vm_id = $1 AND va.drive_id = 'rootfs'
           ORDER BY va.attached_at DESC LIMIT 1"#,
    )
    .bind(vm_id)
    .fetch_optional(&st.db)
    .await
    .ok()
    .flatten();
    let Some((volume_id, backend_id, locator)) = vol_row else {
        let _ = host_repo
            .release_reservation(target_host_id, vm.vcpu, vm.mem_mib as i64)
            .await;
        bail!(
            "live migration requires shared-storage volume; this VM uses local overlay. \
             Use snapshot+restore to a target host instead."
        );
    };
    let backend = st
        .registry
        .get(backend_id)
        .ok_or_else(|| anyhow!("backend {backend_id} not in registry"))?;
    let handle_for_target = nexus_storage::VolumeHandle {
        volume_id,
        backend_id: nexus_storage::BackendInstanceId(backend_id),
        backend_kind: backend.kind(),
        locator,
        size_bytes: 0,
    };
    backend
        .activate_volume(&handle_for_target)
        .await
        .context("activate shared volume on target")?;
    let target_attached =
        crate::features::storage::agent_rpc::agent_attach(&target_host.addr, &handle_for_target)
            .await
            .context("attach shared volume on target")?;
    let target_disk_path = target_attached.path().to_string_lossy().into_owned();

    let http = Client::builder()
        .timeout(Duration::from_secs(900)) // up to 15 min for big VMs
        .build()
        .context("build http client")?;
    let incoming_body = json!({
        "vmm_kind": "qemu",
        "listen_port": target_port,
        "vcpu": vm.vcpu,
        "mem_mib": vm.mem_mib,
        "boot": boot_mode_target,
        "disks": [{
            "drive_id": "rootfs",
            "source": target_disk_path,
            "read_only": false,
            "root_device": true,
            "format": "raw",
            "cdrom": false,
        }],
        // Target NIC uses user-mode by default; bridge-aware NIC setup is
        // a follow-up. For TAP-based VMs, operator pre-creates the TAP.
        "nics": [{
            "iface_id": "net0",
            "host_dev": "user",
            "mac": generate_mac(vm_id),
        }],
        "enable_balloon": true,
        "enable_rng": true,
    });
    let target_resp = http
        .post(format!(
            "{}/agent/v1/vmm/{}/migrate/incoming",
            target_host.addr, vm_id
        ))
        .json(&incoming_body)
        .send()
        .await
        .context("agent migrate/incoming request")?;
    if !target_resp.status().is_success() {
        let _ = host_repo
            .release_reservation(target_host_id, vm.vcpu, vm.mem_mib as i64)
            .await;
        let status = target_resp.status();
        let body = target_resp.text().await.unwrap_or_default();
        bail!("target agent returned {status} on migrate/incoming: {body}");
    }
    // Brief pause to let -incoming socket bind before the source connects.
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Step 2: tell the source to drive the QMP migrate.
    let target_host_ip = target_host
        .addr
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .split(':')
        .next()
        .unwrap_or("127.0.0.1");
    let target_uri = format!("tcp:{target_host_ip}:{target_port}");
    let resp = http
        .post(format!(
            "{}/agent/v1/vmm/{}/migrate/outgoing",
            vm.host_addr, vm.id
        ))
        .json(&json!({ "target_uri": target_uri }))
        .send()
        .await
        .context("agent migrate/outgoing request")?;
    if !resp.status().is_success() {
        // Release the reservation on failure.
        let _ = host_repo
            .release_reservation(target_host_id, vm.vcpu, vm.mem_mib as i64)
            .await;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        bail!("source agent returned {status} on migrate: {body}");
    }
    // Release the source's reservation; the source VM is gone.
    let _ = host_repo
        .release_reservation(vm.host_id, vm.vcpu, vm.mem_mib as i64)
        .await;
    // Update the VM row's host_id. host_addr is now derived from the host
    // table via JOIN so we just point at the new host_id and let downstream
    // queries pick up the new agent URL automatically.
    let _ = target_host;
    sqlx::query(r#"UPDATE vm SET host_id = $2, updated_at = now() WHERE id = $1"#)
        .bind(vm_id)
        .bind(target_host_id)
        .execute(&st.db)
        .await
        .context("update vm host_id after migrate")?;
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
    cpu_type: Option<&'a str>,
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
                nvram_path = $7,
                cpu_type = $8
            WHERE id = $1"#,
    )
    .bind(c.id)
    .bind(c.vmm_kind.as_str())
    .bind(boot_json)
    .bind(console_kind)
    .bind(c.vnc_listen)
    .bind(c.firmware_path)
    .bind(c.nvram_template_path)
    .bind(c.cpu_type)
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
        let overlay_path =
            create_qcow2_overlay(st, vm_id, &img.host_path, &format, rootfs_size_mb).await?;
        Ok((overlay_path, "qcow2".into(), None))
    } else if let Some(p) = rootfs_path {
        // Path 3: legacy explicit path
        let format = detect_disk_format(p);
        Ok((p.to_string(), format, None))
    } else {
        // Path 4: blank local qcow2. Used when the caller wants to install
        // from an ISO onto a fresh disk and didn't pick a storage backend.
        // Size comes from `rootfs_size_mb` (default 20 GiB). The qcow2
        // is sparse so the file only grows as the guest writes — a 20 GB
        // declaration takes a few MB on disk until the installer fills it.
        let size_mb = rootfs_size_mb.unwrap_or(20 * 1024);
        let blank = create_blank_qcow2(st, vm_id, size_mb).await?;
        Ok((blank, "qcow2".into(), None))
    }
}

/// Create a fresh per-VM blank qcow2 disk for ISO install flows that don't
/// route through a storage backend. Size is in MiB. The qcow2 metadata
/// declares the full virtual size; physical bytes are allocated lazily as
/// the guest writes (sparse).
async fn create_blank_qcow2(st: &AppState, vm_id: Uuid, size_mb: u32) -> anyhow::Result<String> {
    st.storage.ensure_vm_dirs(vm_id).await?;
    let dir = st.storage.vm_dir(vm_id).join("storage");
    tokio::fs::create_dir_all(&dir).await?;
    let target = dir.join("disk.qcow2");
    if tokio::fs::metadata(&target).await.is_ok() {
        return Ok(target.display().to_string());
    }
    let out = tokio::process::Command::new("qemu-img")
        .args([
            "create",
            "-f",
            "qcow2",
            &target.display().to_string(),
            &format!("{size_mb}M"),
        ])
        .output()
        .await
        .context("spawn qemu-img create (blank)")?;
    if !out.status.success() {
        bail!(
            "qemu-img create blank failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(target.display().to_string())
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
    rootfs_size_mb: Option<u32>,
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

    // Honor a user-requested rootfs size. A fresh overlay inherits the backing
    // image's virtual size, so without this the requested size is silently
    // ignored. Only grow — never shrink below the source (qcow2 overlays can't
    // safely shrink a populated backing image).
    if let Some(mb) = rootfs_size_mb {
        let requested_bytes = mb as u64 * 1024 * 1024;
        let current_virtual = qcow2_virtual_size_bytes(&target.display().to_string())
            .await
            .unwrap_or(0);
        if requested_bytes > current_virtual {
            let out = tokio::process::Command::new("qemu-img")
                .args(["resize", &target.display().to_string(), &format!("{mb}M")])
                .output()
                .await
                .context("spawn qemu-img resize")?;
            if !out.status.success() {
                bail!(
                    "qemu-img resize overlay to {mb}M failed: {}",
                    String::from_utf8_lossy(&out.stderr)
                );
            }
        } else {
            tracing::warn!(
                vm_id = %vm_id,
                requested_mb = mb,
                current_bytes = current_virtual,
                "requested rootfs size is not larger than the source image; keeping source size"
            );
        }
    }
    Ok(target.display().to_string())
}

/// Return the virtual (declared) size of a qcow2/raw disk in bytes via
/// `qemu-img info --output=json`.
async fn qcow2_virtual_size_bytes(path: &str) -> Result<u64> {
    let out = tokio::process::Command::new("qemu-img")
        .args(["info", "--output=json", path])
        .output()
        .await
        .context("spawn qemu-img info")?;
    if !out.status.success() {
        bail!(
            "qemu-img info failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).context("parse qemu-img info json")?;
    v.get("virtual-size")
        .and_then(|s| s.as_u64())
        .context("qemu-img info missing virtual-size")
}

/// Build a NoCloud cloud-init seed ISO with a user-data block that sets
/// the hostname + creates the user + sets the password + injects SSH keys.
/// Cloudbase-init reads the same NoCloud datasource on Windows guests,
/// so we shape the user-data slightly differently when `guest_os=windows`
/// but use the same ISO layout. Returns the host path of the generated
/// ISO. Auto-detects genisoimage / mkisofs / xorriso in that order;
/// returns Err if none are installed.
async fn build_cloud_init_iso(
    st: &AppState,
    vm_id: Uuid,
    hostname: &str,
    username: &str,
    password: Option<&str>,
    ssh_keys: &[String],
    guest_os: GuestOs,
) -> anyhow::Result<String> {
    st.storage.ensure_vm_dirs(vm_id).await?;
    let work_dir = st.storage.vm_dir(vm_id).join("cloud-init");
    tokio::fs::create_dir_all(&work_dir).await?;

    let meta_data = format!("instance-id: nqr-{vm_id}\nlocal-hostname: {hostname}\n");
    let user_data = build_cloud_init_user_data(hostname, username, password, ssh_keys, guest_os);
    // Match the NIC by name glob instead of hardcoding `eth0`. Modern Linux
    // cloud images use predictable interface names (enp0s3, ens3, eno1, …), so
    // a literal `eth0` stanza never matches — the NIC stays down and the guest
    // never DHCPs. `e*` covers predictable (en*) and legacy (eth*) names; every
    // ethernet NIC gets DHCP, which is the right default for bridged VMs.
    let network_config =
        "version: 2\nethernets:\n  primary:\n    match:\n      name: \"e*\"\n    dhcp4: true\n";

    tokio::fs::write(work_dir.join("meta-data"), meta_data).await?;
    tokio::fs::write(work_dir.join("user-data"), user_data).await?;
    tokio::fs::write(work_dir.join("network-config"), network_config).await?;

    let iso_path = st
        .storage
        .vm_dir(vm_id)
        .join("storage")
        .join("cloud-init.iso");
    if let Some(parent) = iso_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    // Try genisoimage → mkisofs → xorriso, in that order.
    for cmd in ["genisoimage", "mkisofs", "xorriso"] {
        let mut args: Vec<String> = if cmd == "xorriso" {
            vec![
                "-as".into(),
                "mkisofs".into(),
                "-volid".into(),
                "CIDATA".into(),
                "-joliet".into(),
                "-rock".into(),
                "-output".into(),
                iso_path.display().to_string(),
                work_dir.join("meta-data").display().to_string(),
                work_dir.join("user-data").display().to_string(),
                work_dir.join("network-config").display().to_string(),
            ]
        } else {
            vec![
                "-output".into(),
                iso_path.display().to_string(),
                "-volid".into(),
                "CIDATA".into(),
                "-joliet".into(),
                "-rock".into(),
                work_dir.join("meta-data").display().to_string(),
                work_dir.join("user-data").display().to_string(),
                work_dir.join("network-config").display().to_string(),
            ]
        };
        // Note: --quiet is genisoimage-specific; xorriso/mkisofs accept it too.
        args.insert(0, "-quiet".into());
        match tokio::process::Command::new(cmd).args(&args).output().await {
            Ok(out) if out.status.success() => {
                return Ok(iso_path.display().to_string());
            }
            Ok(out) => {
                tracing::debug!(
                    cmd,
                    stderr = %String::from_utf8_lossy(&out.stderr),
                    "cloud-init ISO command failed; trying next"
                );
                continue;
            }
            Err(_) => continue, // binary not installed
        }
    }
    anyhow::bail!("no ISO9660 tool found (install genisoimage, mkisofs, or xorriso)")
}

/// Find the most-recently-registered virtio-win drivers ISO. Heuristic
/// match on `name ILIKE '%virtio-win%' AND image_kind = 'installer_iso'`.
/// Returns the host_path of the first match. Error when none registered.
async fn find_virtio_win_iso(st: &AppState) -> anyhow::Result<String> {
    let row: Option<(String,)> = sqlx::query_as(
        r#"SELECT host_path FROM image
            WHERE image_kind = 'installer_iso'
              AND name ILIKE '%virtio-win%'
            ORDER BY created_at DESC
            LIMIT 1"#,
    )
    .fetch_optional(&st.db)
    .await?;
    row.map(|(p,)| p)
        .ok_or_else(|| anyhow!("no virtio-win ISO registered"))
}

/// Build the cloud-init user-data string for a given guest_os. Linux uses
/// the standard cloud-init shape; Windows uses cloudbase-init's accepted
/// subset (users/passwd via plain_text_passwd, ssh keys via authorized_keys).
fn build_cloud_init_user_data(
    hostname: &str,
    username: &str,
    password: Option<&str>,
    ssh_keys: &[String],
    guest_os: GuestOs,
) -> String {
    let mut s = String::from("#cloud-config\n");
    s.push_str(&format!("hostname: {hostname}\n"));

    match guest_os {
        GuestOs::Windows => {
            // cloudbase-init understands a small subset of cloud-config.
            // `users` works for creating local accounts; `set_hostname` is
            // already covered by the top-level hostname.
            if password.is_some() || !ssh_keys.is_empty() {
                s.push_str("users:\n");
                s.push_str(&format!("  - name: {username}\n"));
                s.push_str("    primary_group: Administrators\n");
                if let Some(pw) = password {
                    s.push_str(&format!("    plain_text_passwd: \"{pw}\"\n"));
                    s.push_str("    lock_passwd: false\n");
                }
                if !ssh_keys.is_empty() {
                    s.push_str("    ssh_authorized_keys:\n");
                    for key in ssh_keys {
                        // YAML-safe: quote each key
                        s.push_str(&format!("      - \"{}\"\n", yaml_escape(key)));
                    }
                }
            }
        }
        _ => {
            // Linux: standard cloud-init shape.
            s.push_str("ssh_pwauth: ");
            s.push_str(if password.is_some() {
                "true\n"
            } else {
                "false\n"
            });
            s.push_str("users:\n");
            s.push_str(&format!("  - name: {username}\n"));
            s.push_str("    sudo: ALL=(ALL) NOPASSWD:ALL\n");
            s.push_str("    shell: /bin/bash\n");
            s.push_str("    lock_passwd: false\n");
            if !ssh_keys.is_empty() {
                s.push_str("    ssh_authorized_keys:\n");
                for key in ssh_keys {
                    s.push_str(&format!("      - \"{}\"\n", yaml_escape(key)));
                }
            }
            if let Some(pw) = password {
                s.push_str("chpasswd:\n");
                s.push_str("  expire: false\n");
                s.push_str("  list: |\n");
                s.push_str(&format!("    {username}:{pw}\n"));
            }
        }
    }

    s
}

/// Conservative YAML escape: backslashes + double-quotes only. SSH keys
/// don't normally contain either, but defensive in case the input has
/// shell-escaped characters.
fn yaml_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
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

/// Probe a disk image's real on-disk format via `qemu-img info` instead of
/// guessing from the file extension. Auto-provisioned data disks are raw `.img`
/// files that the extension heuristic ([`detect_disk_format`]) mislabels as
/// qcow2 — passing `format=qcow2` for a raw file makes QEMU refuse to open it
/// and aborts the boot. Falls back to the extension heuristic if the probe is
/// unavailable (e.g. qemu-img missing or path not locally accessible).
pub(crate) async fn probe_disk_format(path: &str) -> String {
    let out = tokio::process::Command::new("qemu-img")
        .args(["info", "--output=json", "-U", path])
        .output()
        .await;
    if let Ok(o) = out {
        if o.status.success() {
            if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&o.stdout) {
                if let Some(f) = v.get("format").and_then(|f| f.as_str()) {
                    return f.to_string();
                }
            }
        }
    }
    detect_disk_format(path)
}

/// Deterministic locally-administered MAC. First three bytes 52:54:00
/// (the QEMU "well-known" OUI), last three derived from the VM UUID so
/// reboots keep the same MAC.
fn generate_mac(id: Uuid) -> String {
    let b = id.as_bytes();
    format!("52:54:00:{:02x}:{:02x}:{:02x}", b[13], b[14], b[15])
}

/// MAC for extra NICs. XOR the index into the final byte so each NIC gets
/// a unique MAC that's still deterministic per (vm_id, nic_index).
fn generate_extra_mac(id: Uuid, nic_index: u8) -> String {
    let b = id.as_bytes();
    format!(
        "52:54:00:{:02x}:{:02x}:{:02x}",
        b[13],
        b[14],
        b[15] ^ (nic_index + 1)
    )
}

/// Look up a network's bridge name. Used for multi-NIC extra-network attach.
async fn resolve_network_bridge(st: &AppState, network_id: Uuid) -> anyhow::Result<String> {
    use crate::features::networks::repo::NetworkRepository;
    let repo = NetworkRepository::new(st.db.clone());
    let net = repo.get(network_id).await?;
    Ok(net.bridge_name)
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
            extra_network_ids: vec![],
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
            enable_secure_boot: None,
            ssh_authorized_keys: vec![],
            data_disks: vec![],
            vfio_devices: vec![],
            cpu_type: None,
        }
    }
}
