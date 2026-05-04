//! Pure-logic parsers for LVM tool output (`pvs`, `vgs`, `lvs`) plus iSCSI
//! session lifecycle helpers (discovery, login, logout, block-device resolve).
//!
//! The parser functions are I/O-free; the iSCSI session helpers shell out to
//! `iscsiadm` via `tokio::process::Command` and walk `/dev/disk/by-path/`. The
//! shape mirrors Proxmox VE's `ISCSIPlugin.pm` (`iscsi_login`): discover →
//! login → mark `node.startup=automatic` so the session is restored across
//! reboots.

#![allow(dead_code)]

use std::path::PathBuf;

use async_trait::async_trait;
use axum::{http::StatusCode, response::IntoResponse, routing::post, Json, Router};
use nexus_storage::{
    AttachedPath, BackendKind, HostBackend, StorageError, VolumeHandle, VolumeSnapshotHandle,
};
use serde::{Deserialize, Serialize};

/// One row from `pvs --separator : --noheadings --units k --nosuffix
/// --options pv_name,pv_size,vg_name,pv_uuid`.
#[derive(Debug, Clone)]
pub struct PvInfo {
    pub pv_name: String,
    pub size_kb: u64,
    pub vg_name: Option<String>,
    pub uuid: String,
}

/// One row from `vgs --separator : --noheadings --units b --nosuffix
/// --options vg_name,vg_size,vg_free,lv_count`.
#[derive(Debug, Clone)]
pub struct VgInfo {
    pub name: String,
    pub size_bytes: u64,
    pub free_bytes: u64,
    pub lv_count: u32,
}

/// One row from `lvs --separator : --noheadings
/// --options lv_name,lv_size,lv_tags,lv_attr`.
#[derive(Debug, Clone)]
pub struct LvInfo {
    pub name: String,
    pub size_bytes: u64,
    pub tags: Vec<String>,
    pub is_active: bool,
}

/// Parse a single `pvs` line. Returns `None` if the line is malformed.
pub fn parse_pv_info(line: &str) -> Option<PvInfo> {
    let trimmed = line.trim();
    let parts: Vec<&str> = trimmed.split(':').collect();
    if parts.len() != 4 {
        return None;
    }
    let pv_name = parts[0].trim().to_string();
    let size_kb: u64 = parts[1].trim().parse().ok()?;
    let vg_field = parts[2].trim();
    let vg_name = if vg_field.is_empty() {
        None
    } else {
        Some(vg_field.to_string())
    };
    let uuid = parts[3].trim().to_string();
    if pv_name.is_empty() || uuid.is_empty() {
        return None;
    }
    Some(PvInfo {
        pv_name,
        size_kb,
        vg_name,
        uuid,
    })
}

/// Parse a single `vgs` line. Returns `None` if the line is malformed.
pub fn parse_vg_info(line: &str) -> Option<VgInfo> {
    let trimmed = line.trim();
    let parts: Vec<&str> = trimmed.split(':').collect();
    if parts.len() != 4 {
        return None;
    }
    let name = parts[0].trim().to_string();
    if name.is_empty() {
        return None;
    }
    let size_bytes: u64 = parts[1].trim().parse().ok()?;
    let free_bytes: u64 = parts[2].trim().parse().ok()?;
    let lv_count: u32 = parts[3].trim().parse().ok()?;
    Some(VgInfo {
        name,
        size_bytes,
        free_bytes,
        lv_count,
    })
}

/// Parse a single `lvs` line. Returns `None` if the line is malformed.
pub fn parse_lv_info(line: &str) -> Option<LvInfo> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    let parts: Vec<&str> = trimmed.split(':').collect();
    if parts.len() != 4 {
        return None;
    }
    let name = parts[0].trim().to_string();
    if name.is_empty() {
        return None;
    }
    let size_bytes: u64 = parts[1].trim().parse().ok()?;
    let tags_field = parts[2].trim();
    let tags: Vec<String> = if tags_field.is_empty() {
        Vec::new()
    } else {
        tags_field
            .split(',')
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect()
    };
    let attr = parts[3].trim();
    let is_active = attr.chars().nth(4).map(|c| c == 'a').unwrap_or(false);
    Some(LvInfo {
        name,
        size_bytes,
        tags,
        is_active,
    })
}

#[cfg(test)]
mod parser_tests {
    use super::*;

    #[test]
    fn parse_pvs_extracts_vg_for_device() {
        // Real `pvs` output (with leading whitespace from --noheadings):
        let out = "  /dev/sdb:104857600:vg-nqrust:abcd-1234-uuid";
        let info = parse_pv_info(out).expect("parsed");
        assert_eq!(info.pv_name, "/dev/sdb");
        assert_eq!(info.size_kb, 104857600);
        assert_eq!(info.vg_name.as_deref(), Some("vg-nqrust"));
    }

    #[test]
    fn parse_pvs_no_vg_when_uninitialized() {
        // PV exists but no VG yet → vg_name field is empty.
        let out = "  /dev/sdc:104857600::xyz-uuid";
        let info = parse_pv_info(out).expect("parsed");
        assert!(info.vg_name.is_none());
    }

    #[test]
    fn parse_vgs_returns_size_free() {
        let out = "vg-nqrust:107374182400:96636764160:3";
        let info = parse_vg_info(out).expect("parsed");
        assert_eq!(info.name, "vg-nqrust");
        assert_eq!(info.size_bytes, 107374182400);
        assert_eq!(info.free_bytes, 96636764160);
        assert_eq!(info.lv_count, 3);
    }

    #[test]
    fn parse_lvs_extracts_lvs_with_tags() {
        // lv_attr 5th char: 'a' = active, '-' = not. This sample is inactive.
        let out = "vm-100-disk-0:10737418240:nqrust-vm-100:-wi-------";
        let info = parse_lv_info(out).expect("parsed");
        assert_eq!(info.name, "vm-100-disk-0");
        assert_eq!(info.size_bytes, 10737418240);
        assert_eq!(info.tags, vec!["nqrust-vm-100".to_string()]);
        assert!(!info.is_active);
    }

    #[test]
    fn parse_lvs_active_volume_has_a_in_attr() {
        // lv_attr `-wi-ao----` → active.
        let out = "vm-100-disk-0:10737418240:nqrust-vm-100:-wi-ao----";
        let info = parse_lv_info(out).expect("parsed");
        assert!(info.is_active);
    }

    #[test]
    fn parse_lvs_handles_no_tags() {
        let out = "vm-100-disk-0:10737418240::-wi-a-----";
        let info = parse_lv_info(out).expect("parsed");
        assert!(info.tags.is_empty());
    }

    #[test]
    fn parse_lvs_multiple_tags_split_on_comma() {
        let out = "vm-100-disk-0:10737418240:nqrust-vm-100,backup,migrate:-wi-a-----";
        let info = parse_lv_info(out).expect("parsed");
        assert_eq!(info.tags, vec!["nqrust-vm-100", "backup", "migrate"]);
    }

    #[test]
    fn parser_returns_none_on_malformed_input() {
        assert!(parse_pv_info("not enough fields").is_none());
        assert!(parse_vg_info("a:b").is_none());
        assert!(parse_lv_info("").is_none());
    }
}

// -------------------------------------------------------------------------
// iSCSI session lifecycle
// -------------------------------------------------------------------------

/// Build the argv for `iscsiadm` to log in to a single target on a portal.
///
/// Matches the open-iscsi `--mode node --targetname <iqn> --portal <portal>
/// --login` invocation used by Proxmox's `ISCSIPlugin::iscsi_login`.
pub fn build_iscsi_login_args(iqn: &str, portal: &str) -> Vec<String> {
    vec![
        "--mode".into(),
        "node".into(),
        "--targetname".into(),
        iqn.into(),
        "--portal".into(),
        portal.into(),
        "--login".into(),
    ]
}

/// Build the argv for marking a node record as auto-started on boot.
///
/// `iscsiadm --mode node --targetname <iqn> --op update --name node.startup
/// --value automatic`. Should be run after a successful login so the session
/// survives reboots without manual re-login.
pub fn build_iscsi_persistent_args(iqn: &str) -> Vec<String> {
    vec![
        "--mode".into(),
        "node".into(),
        "--targetname".into(),
        iqn.into(),
        "--op".into(),
        "update".into(),
        "--name".into(),
        "node.startup".into(),
        "--value".into(),
        "automatic".into(),
    ]
}

/// Discover, log in, and persist a node record for the given target+portal.
///
/// Discovery and the persistent-config step are best-effort (their failure is
/// swallowed) — only login is treated as load-bearing. `iscsiadm` exit code
/// 15 means "session already exists for this target", which we treat as
/// success since the post-condition (a live session) holds.
pub async fn iscsi_login(iqn: &str, portal: &str) -> Result<(), StorageError> {
    use tokio::process::Command;

    // Discovery first (best-effort).
    let _ = Command::new("iscsiadm")
        .args([
            "--mode",
            "discovery",
            "--type",
            "sendtargets",
            "--portal",
            portal,
        ])
        .status()
        .await;

    // Login.
    let status = Command::new("iscsiadm")
        .args(build_iscsi_login_args(iqn, portal))
        .status()
        .await
        .map_err(|e| {
            StorageError::backend(std::io::Error::other(format!("iscsiadm login spawn: {e}")))
        })?;

    // Exit 15 == already logged in; treat as success.
    if !status.success() && status.code() != Some(15) {
        return Err(StorageError::backend(std::io::Error::other(format!(
            "iscsiadm login failed: exit {:?}",
            status.code()
        ))));
    }

    // Make persistent (best-effort: ignore errors here, the session is up).
    let _ = Command::new("iscsiadm")
        .args(build_iscsi_persistent_args(iqn))
        .status()
        .await;

    Ok(())
}

/// Log out of an iSCSI target. Best-effort: failures are swallowed because
/// logout commonly fails when the kernel still holds device-mapper or LVM
/// references against the session — those callers manage their own teardown.
pub async fn iscsi_logout(iqn: &str) -> Result<(), StorageError> {
    use tokio::process::Command;

    let _ = Command::new("iscsiadm")
        .args(["--mode", "node", "--targetname", iqn, "--logout"])
        .status()
        .await;

    Ok(())
}

/// Find the `/dev/disk/by-path/` symlink for a logged-in iSCSI LUN, if any.
///
/// open-iscsi creates entries of the form
/// `ip-<portal>-iscsi-<iqn>-lun-<N>` once a session is established and udev
/// has finished settling. Returns `None` when the entry is missing (e.g.
/// session not yet up, or wrong LUN number).
pub async fn resolve_iscsi_block_device(iqn: &str, portal: &str, lun: u32) -> Option<PathBuf> {
    let pattern = format!("ip-{portal}-iscsi-{iqn}-lun-{lun}");
    let mut entries = tokio::fs::read_dir("/dev/disk/by-path").await.ok()?;
    while let Ok(Some(e)) = entries.next_entry().await {
        if e.file_name().to_string_lossy() == pattern {
            return Some(e.path());
        }
    }
    None
}

// -------------------------------------------------------------------------
// Volume-group initialization (`pvcreate` + `vgcreate`)
// -------------------------------------------------------------------------

/// Build the argv tail for `pvcreate`. Mirrors Proxmox `LVMPlugin.pm:120`:
/// metadatasize 250k yields `pe_start = 512` (sector 1024), aligned to the
/// 128k boundary preferred by SSD arrays.
fn build_pvcreate_args(device: &str) -> Vec<&str> {
    vec!["--metadatasize", "250k", device]
}

/// Build the argv tail for `vgcreate <vg> <device>`.
fn build_vgcreate_args<'a>(vg: &'a str, device: &'a str) -> Vec<&'a str> {
    vec![vg, device]
}

/// Initialize a volume group on `device`, idempotently.
///
/// Behavior:
/// - If the device already carries a PV that belongs to `vg_name`, returns
///   `Ok(())` immediately (no-op).
/// - If the device carries a PV that belongs to a *different* VG, returns an
///   error and does NOT touch the device — refusing to overwrite another VG's
///   metadata is intentional.
/// - If the device has a PV but no VG, skips the zero+pvcreate step and goes
///   straight to `vgcreate`.
/// - Otherwise: zero the first 512 bytes of the device (mirrors
///   `LVMPlugin.pm:96-103` — `pvcreate` refuses if leftover label data is
///   present), run `pvcreate --metadatasize 250k`, then `vgcreate <vg>
///   <device>`.
pub async fn initialize_vg(device: &std::path::Path, vg_name: &str) -> Result<(), StorageError> {
    use tokio::io::AsyncWriteExt;
    use tokio::process::Command;

    // Canonicalize: `/dev/disk/by-path/...` is a symlink, and `pvs` does not
    // reliably report a PV when given a symlink — even though `pvcreate` does
    // see the signature on the underlying block device. Resolve to the real
    // path (e.g. `/dev/sda`) so the idempotency check and the create command
    // operate on the same identity.
    let canonical = match tokio::fs::canonicalize(device).await {
        Ok(p) => p,
        Err(_) => device.to_path_buf(),
    };
    let device_str = canonical.to_str().ok_or_else(|| {
        StorageError::backend(std::io::Error::other("device path is not valid UTF-8"))
    })?;

    // Idempotency probe: parse `pvs <device>` output. Non-zero exit means "no
    // PV here" → fall through to the create path.
    let probe = Command::new("pvs")
        .args([
            "--separator",
            ":",
            "--noheadings",
            "--units",
            "k",
            "--unbuffered",
            "--nosuffix",
            "--options",
            "pv_name,pv_size,vg_name,pv_uuid",
            device_str,
        ])
        .output()
        .await;

    let mut pv_exists_no_vg = false;
    if let Ok(out) = probe {
        if out.status.success() {
            if let Some(line) = String::from_utf8_lossy(&out.stdout).lines().next() {
                if let Some(info) = parse_pv_info(line) {
                    match info.vg_name.as_deref() {
                        Some(existing) if existing == vg_name => return Ok(()),
                        Some(other) => {
                            return Err(StorageError::backend(std::io::Error::other(format!(
                                "device {} is already part of VG '{}'; refusing to overwrite",
                                device_str, other
                            ))));
                        }
                        None => {
                            // PV exists but no VG — skip zero+pvcreate, jump
                            // straight to vgcreate below.
                            pv_exists_no_vg = true;
                        }
                    }
                }
            }
        }
    }

    if !pv_exists_no_vg {
        // Zero first sector to clear any stale label data.
        let mut f = tokio::fs::OpenOptions::new()
            .write(true)
            .open(device)
            .await
            .map_err(|e| {
                StorageError::backend(std::io::Error::other(format!(
                    "open {} for zero: {e}",
                    device.display()
                )))
            })?;
        f.write_all(&[0u8; 512]).await.map_err(|e| {
            StorageError::backend(std::io::Error::other(format!("zero first sector: {e}")))
        })?;
        drop(f);

        // pvcreate.
        let status = Command::new("pvcreate")
            .args(build_pvcreate_args(device_str))
            .status()
            .await
            .map_err(|e| {
                StorageError::backend(std::io::Error::other(format!("pvcreate spawn: {e}")))
            })?;
        if !status.success() {
            return Err(StorageError::backend(std::io::Error::other(format!(
                "pvcreate failed: exit {:?}",
                status.code()
            ))));
        }
    }

    // vgcreate.
    let status = Command::new("vgcreate")
        .args(build_vgcreate_args(vg_name, device_str))
        .status()
        .await
        .map_err(|e| {
            StorageError::backend(std::io::Error::other(format!("vgcreate spawn: {e}")))
        })?;
    if !status.success() {
        return Err(StorageError::backend(std::io::Error::other(format!(
            "vgcreate failed: exit {:?}",
            status.code()
        ))));
    }

    Ok(())
}

#[cfg(test)]
mod session_tests {
    use super::*;

    #[test]
    fn build_iscsi_login_args_includes_login_flag() {
        let args = build_iscsi_login_args("iqn.foo:bar", "192.168.1.10:3260");
        let s = args.join(" ");
        assert!(s.contains("--mode node"), "{s}");
        assert!(s.contains("--targetname iqn.foo:bar"), "{s}");
        assert!(s.contains("--portal 192.168.1.10:3260"), "{s}");
        assert!(s.contains("--login"), "{s}");
    }

    #[test]
    fn build_iscsi_persistent_args_sets_node_startup_automatic() {
        let args = build_iscsi_persistent_args("iqn.foo:bar");
        let s = args.join(" ");
        assert!(s.contains("--op update"), "{s}");
        assert!(s.contains("--name node.startup"), "{s}");
        assert!(s.contains("--value automatic"), "{s}");
        assert!(s.contains("--targetname iqn.foo:bar"), "{s}");
    }

    // Live test gated on env var: NQRUST_ISCSI_LVM_LIVE_PORTAL=192.168.18.171:3260
    // and NQRUST_ISCSI_LVM_LIVE_IQN=iqn.foo:bar — only run manually.
    #[ignore]
    #[tokio::test]
    async fn live_iscsi_login_against_portal() {
        let portal = match std::env::var("NQRUST_ISCSI_LVM_LIVE_PORTAL") {
            Ok(v) => v,
            Err(_) => return,
        };
        let iqn = match std::env::var("NQRUST_ISCSI_LVM_LIVE_IQN") {
            Ok(v) => v,
            Err(_) => return,
        };
        iscsi_login(&iqn, &portal).await.expect("login");
        // logout to leave host clean for next test
        let _ = iscsi_logout(&iqn).await;
    }
}

// -------------------------------------------------------------------------
// Per-VM LV lifecycle (`lvcreate` / `lvremove` / `lvchange`)
// -------------------------------------------------------------------------

/// Build the argv tail for `lvcreate`. Mirrors Proxmox `LVMPlugin.pm:622-637`
/// — every flag is intentional:
/// - `-aly`: activate locally on creation
/// - `-Wy --yes`: wipe signatures non-interactively
/// - `--setautoactivation n`: do NOT auto-activate at boot. Critical for
///   shared-LUN safety: only the host that needs the LV should activate it.
/// - `--addtag <t>`: attaches ownership tags so we can find LVs later.
fn build_lvcreate_args(vg: &str, name: &str, size: &str, tags: &[&str]) -> Vec<String> {
    let mut a: Vec<String> = vec![
        "-aly".into(),
        "-Wy".into(),
        "--yes".into(),
        "--size".into(),
        size.into(),
        "--name".into(),
        name.into(),
        "--setautoactivation".into(),
        "n".into(),
    ];
    for t in tags {
        a.push("--addtag".into());
        a.push((*t).to_string());
    }
    a.push(vg.into());
    a
}

/// Build the argv for exclusive activation. `-aey` (vs `-aly`) is the
/// cluster-safety mechanism: only one host can hold the LV active at a time,
/// so a single shared LUN can safely back N VMs across N hosts.
/// (Proxmox `LVMPlugin.pm:960`.)
fn build_lvchange_activate_args(vg: &str, lv: &str) -> Vec<String> {
    vec!["-aey".into(), format!("/dev/{vg}/{lv}")]
}

/// Build the argv for local deactivation (`-aln`). Releases the exclusive
/// lock so another host can activate the same LV next.
fn build_lvchange_deactivate_args(vg: &str, lv: &str) -> Vec<String> {
    vec!["-aln".into(), format!("/dev/{vg}/{lv}")]
}

/// Create a logical volume of `size_bytes` for `vmid` in `vg`. Tags the LV
/// with `nqrust-vm-<vmid>` for ownership tracking. Returns the device path
/// `/dev/<vg>/<name>` on success.
pub async fn lvcreate(
    vg: &str,
    name: &str,
    size_bytes: u64,
    vmid: &str,
) -> Result<PathBuf, StorageError> {
    use tokio::process::Command;
    let size_arg = format!("{size_bytes}B");
    let tag = format!("nqrust-vm-{vmid}");
    let tags = [tag.as_str()];
    let args = build_lvcreate_args(vg, name, &size_arg, &tags);
    let status = Command::new("lvcreate")
        .args(&args)
        .status()
        .await
        .map_err(|e| {
            StorageError::backend(std::io::Error::other(format!("lvcreate spawn: {e}")))
        })?;
    if !status.success() {
        return Err(StorageError::backend(std::io::Error::other(format!(
            "lvcreate failed: exit {:?}",
            status.code()
        ))));
    }
    Ok(PathBuf::from(format!("/dev/{vg}/{name}")))
}

/// Remove a logical volume. Uses `-f` to skip the interactive confirmation.
pub async fn lvremove(vg: &str, name: &str) -> Result<(), StorageError> {
    use tokio::process::Command;
    let path = format!("{vg}/{name}");
    let status = Command::new("lvremove")
        .args(["-f", &path])
        .status()
        .await
        .map_err(|e| {
            StorageError::backend(std::io::Error::other(format!("lvremove spawn: {e}")))
        })?;
    if !status.success() {
        return Err(StorageError::backend(std::io::Error::other(format!(
            "lvremove failed: exit {:?}",
            status.code()
        ))));
    }
    Ok(())
}

/// Activate an LV exclusively on this host (`-aey`), then refresh device-mapper
/// state (`--refresh`, best-effort) to mirror `LVMPlugin.pm:970`.
pub async fn lvchange_activate(vg: &str, lv: &str) -> Result<(), StorageError> {
    use tokio::process::Command;
    let args = build_lvchange_activate_args(vg, lv);
    let status = Command::new("lvchange")
        .args(&args)
        .status()
        .await
        .map_err(|e| {
            StorageError::backend(std::io::Error::other(format!(
                "lvchange activate spawn: {e}"
            )))
        })?;
    if !status.success() {
        return Err(StorageError::backend(std::io::Error::other(format!(
            "lvchange activate failed: exit {:?}",
            status.code()
        ))));
    }
    // --refresh after activate (LVMPlugin.pm:970). Best-effort.
    let _ = Command::new("lvchange")
        .args(["--refresh", &format!("/dev/{vg}/{lv}")])
        .status()
        .await;
    Ok(())
}

/// Deactivate an LV locally (`-aln`). Best-effort: failures are logged but not
/// propagated, since another process holding the LV (e.g. firecracker still
/// flushing) is the common case during shutdown.
pub async fn lvchange_deactivate(vg: &str, lv: &str) -> Result<(), StorageError> {
    use tokio::process::Command;
    let args = build_lvchange_deactivate_args(vg, lv);
    let status = Command::new("lvchange")
        .args(&args)
        .status()
        .await
        .map_err(|e| {
            StorageError::backend(std::io::Error::other(format!(
                "lvchange deactivate spawn: {e}"
            )))
        })?;
    if !status.success() {
        // Deactivate is best-effort: another process holding the LV is common
        // (e.g. firecracker still writing). Log via tracing::warn but return Ok.
        tracing::warn!(
            vg = %vg,
            lv = %lv,
            exit = ?status.code(),
            "lvchange deactivate failed (best-effort)"
        );
    }
    Ok(())
}

#[cfg(test)]
mod lv_tests {
    use super::*;

    #[test]
    fn lvcreate_args_match_proxmox_shape() {
        let tags = ["nqrust-vm-100"];
        let args = build_lvcreate_args("vg-nqrust", "vm-100-disk-0", "10737418240B", &tags);
        let s = args.join(" ");
        // From LVMPlugin.pm:622-637 — every flag is intentional.
        assert!(s.contains("-aly"), "{s}"); // activate immediately
        assert!(s.contains("-Wy"), "{s}"); // wipe signatures
        assert!(s.contains("--yes"), "{s}"); // assume yes
        assert!(s.contains("--size 10737418240B"), "{s}");
        assert!(s.contains("--name vm-100-disk-0"), "{s}");
        assert!(s.contains("--setautoactivation n"), "{s}");
        assert!(s.contains("--addtag nqrust-vm-100"), "{s}");
        assert!(s.ends_with("vg-nqrust"), "{s}");
    }

    #[test]
    fn lvcreate_args_with_multiple_tags() {
        let tags = ["nqrust-vm-100", "backup"];
        let args = build_lvcreate_args("vg-x", "lv-y", "1G", &tags);
        let s = args.join(" ");
        assert!(s.contains("--addtag nqrust-vm-100"));
        assert!(s.contains("--addtag backup"));
    }

    #[test]
    fn lvchange_activate_uses_exclusive_mode() {
        // From LVMPlugin.pm:960 — `-aey` (exclusive). `-aly` would allow
        // multiple hosts active = corruption risk. Pinning the exact flag.
        let args = build_lvchange_activate_args("vg-nqrust", "vm-100-disk-0");
        let s = args.join(" ");
        assert!(s.contains("-aey"), "{s}");
        assert!(s.contains("/dev/vg-nqrust/vm-100-disk-0"), "{s}");
    }

    #[test]
    fn lvchange_deactivate_uses_local_mode() {
        let args = build_lvchange_deactivate_args("vg-nqrust", "vm-100-disk-0");
        let s = args.join(" ");
        assert!(s.contains("-aln"), "{s}");
        assert!(s.contains("/dev/vg-nqrust/vm-100-disk-0"), "{s}");
    }
}

#[cfg(test)]
mod init_tests {
    use super::*;

    #[test]
    fn pvcreate_args_use_proxmox_metadata_size() {
        let args = build_pvcreate_args("/dev/sdb");
        assert_eq!(args, vec!["--metadatasize", "250k", "/dev/sdb"]);
    }

    #[test]
    fn vgcreate_args_minimal() {
        let args = build_vgcreate_args("vg-nqrust", "/dev/sdb");
        assert_eq!(args, vec!["vg-nqrust", "/dev/sdb"]);
    }

    // Live test: requires a real block device the test runner is allowed to
    // wipe. Set NQRUST_LVM_LIVE_DEVICE=/dev/sdX (and optionally
    // NQRUST_LVM_LIVE_VG=name) and run with `--ignored`.
    #[ignore]
    #[tokio::test]
    async fn live_initialize_vg_idempotent() {
        let device = match std::env::var("NQRUST_LVM_LIVE_DEVICE") {
            Ok(v) => v,
            Err(_) => return,
        };
        let vg = std::env::var("NQRUST_LVM_LIVE_VG").unwrap_or_else(|_| "test-vg-nqrust".into());
        initialize_vg(std::path::Path::new(&device), &vg)
            .await
            .expect("first init");
        // Second call should be a no-op.
        initialize_vg(std::path::Path::new(&device), &vg)
            .await
            .expect("idempotent");
    }
}

// -------------------------------------------------------------------------
// Locator + HostBackend impl
// -------------------------------------------------------------------------

/// Wire-format locator JSON for an iscsi_lvm volume. The manager-side
/// control-plane backend (Task 9) serializes this into `VolumeHandle.locator`
/// so the agent knows which LV to activate when attaching the volume.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IscsiLvmLocator {
    pub vg: String,
    pub lv: String,
}

/// Zero-sized agent-side host-backend for `BackendKind::IscsiLvm`. The iSCSI
/// session is shared per-LUN across all VMs on this host and lives outside any
/// individual volume's lifecycle, so attach/detach only manage LV activation.
#[derive(Clone, Default)]
pub struct IscsiLvmHostBackend;

impl IscsiLvmHostBackend {
    fn parse_locator(raw: &str) -> Result<IscsiLvmLocator, StorageError> {
        serde_json::from_str(raw)
            .map_err(|e| StorageError::InvalidLocator(format!("decode iscsi_lvm locator: {e}")))
    }
}

#[async_trait]
impl HostBackend for IscsiLvmHostBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::IscsiLvm
    }

    async fn attach(&self, volume: &VolumeHandle) -> Result<AttachedPath, StorageError> {
        let loc = Self::parse_locator(&volume.locator)?;
        lvchange_activate(&loc.vg, &loc.lv).await?;
        let dev = PathBuf::from(format!("/dev/{}/{}", loc.vg, loc.lv));
        Ok(AttachedPath::BlockDevice(dev))
    }

    async fn detach(
        &self,
        volume: &VolumeHandle,
        _attached: AttachedPath,
    ) -> Result<(), StorageError> {
        let loc = Self::parse_locator(&volume.locator)?;
        lvchange_deactivate(&loc.vg, &loc.lv).await
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
            .open(dst_path)
            .await?;
        tokio::io::copy(&mut src, &mut dst).await?;
        dst.flush().await?;
        // LV size is fixed at lvcreate time; target_size_bytes is informational
        // for block backends.
        let _ = target_size_bytes;
        Ok(())
    }

    async fn resize2fs(&self, attached: &AttachedPath) -> Result<(), StorageError> {
        super::local_file::run_resize2fs(attached.path()).await
    }

    async fn read_snapshot(
        &self,
        snap: &VolumeSnapshotHandle,
    ) -> Result<Box<dyn tokio::io::AsyncRead + Send + Unpin>, StorageError> {
        // Snapshot locator carries the same {vg, lv} shape but referencing the
        // snapshot LV. Activate exclusively, then open as a regular file.
        let loc = Self::parse_locator(&snap.locator)?;
        lvchange_activate(&loc.vg, &loc.lv).await?;
        let dev = PathBuf::from(format!("/dev/{}/{}", loc.vg, loc.lv));
        let f = tokio::fs::File::open(&dev).await?;
        Ok(Box::new(f))
    }
}

// -------------------------------------------------------------------------
// HTTP routes (manager → agent)
// -------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct LoginReq {
    pub iqn: String,
    pub portal: String,
}

#[derive(Debug, Deserialize)]
pub struct LogoutReq {
    pub iqn: String,
}

#[derive(Debug, Deserialize)]
pub struct InitVgReq {
    pub iqn: String,
    pub portal: String,
    pub lun: u32,
    pub vg_name: String,
}

#[derive(Debug, Deserialize)]
pub struct VgStatusReq {
    pub vg: String,
}

#[derive(Debug, Serialize)]
pub struct VgStatusResp {
    pub size_bytes: u64,
    pub free_bytes: u64,
    pub lv_count: u32,
}

#[derive(Debug, Deserialize)]
pub struct LvCreateReq {
    pub vg: String,
    pub name: String,
    pub size_bytes: u64,
    pub vm_id: String,
}

#[derive(Debug, Serialize)]
pub struct LvCreateResp {
    pub device: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct LvNameReq {
    pub vg: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct CloneFromPathReq {
    pub source_path: PathBuf,
    pub vg: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct LvSnapshotReq {
    pub vg: String,
    pub source_lv: String,
    pub snap_name: String,
    pub size_bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct LvSnapshotResp {
    pub snap_name: String,
}

fn err_500(e: impl std::fmt::Display) -> axum::response::Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({"error": format!("backend error: {e}")})),
    )
        .into_response()
}

async fn login_handler(Json(body): Json<LoginReq>) -> impl IntoResponse {
    match iscsi_login(&body.iqn, &body.portal).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_500(e),
    }
}

async fn logout_handler(Json(body): Json<LogoutReq>) -> impl IntoResponse {
    match iscsi_logout(&body.iqn).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_500(e),
    }
}

async fn init_vg_handler(Json(body): Json<InitVgReq>) -> impl IntoResponse {
    // Ensure session, then locate the block device, then init the VG.
    if let Err(e) = iscsi_login(&body.iqn, &body.portal).await {
        return err_500(e);
    }
    // Wait briefly for udev to populate /dev/disk/by-path.
    let mut device: Option<PathBuf> = None;
    for _ in 0..30 {
        if let Some(p) = resolve_iscsi_block_device(&body.iqn, &body.portal, body.lun).await {
            device = Some(p);
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    let dev = match device {
        Some(p) => p,
        None => {
            return err_500(format!(
                "iscsi block device for {}@{} lun={} did not appear",
                body.iqn, body.portal, body.lun
            ))
        }
    };
    match initialize_vg(&dev, &body.vg_name).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_500(e),
    }
}

async fn vg_status_handler(Json(body): Json<VgStatusReq>) -> impl IntoResponse {
    use tokio::process::Command;
    let out = match Command::new("vgs")
        .args([
            "--separator",
            ":",
            "--noheadings",
            "--units",
            "b",
            "--unbuffered",
            "--nosuffix",
            "--options",
            "vg_name,vg_size,vg_free,lv_count",
            &body.vg,
        ])
        .output()
        .await
    {
        Ok(o) => o,
        Err(e) => return err_500(format!("vgs spawn: {e}")),
    };
    if !out.status.success() {
        return err_500(format!(
            "vgs failed: exit {:?}, stderr={}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let line = match stdout.lines().next() {
        Some(l) => l,
        None => return err_500(format!("vgs returned no rows for vg='{}'", body.vg)),
    };
    let info = match parse_vg_info(line) {
        Some(i) => i,
        None => return err_500(format!("could not parse vgs row: {line:?}")),
    };
    (
        StatusCode::OK,
        Json(VgStatusResp {
            size_bytes: info.size_bytes,
            free_bytes: info.free_bytes,
            lv_count: info.lv_count,
        }),
    )
        .into_response()
}

async fn lv_create_handler(Json(body): Json<LvCreateReq>) -> impl IntoResponse {
    match lvcreate(&body.vg, &body.name, body.size_bytes, &body.vm_id).await {
        Ok(device) => (StatusCode::OK, Json(LvCreateResp { device })).into_response(),
        Err(e) => err_500(e),
    }
}

async fn lv_remove_handler(Json(body): Json<LvNameReq>) -> impl IntoResponse {
    match lvremove(&body.vg, &body.name).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_500(e),
    }
}

async fn lv_activate_handler(Json(body): Json<LvNameReq>) -> impl IntoResponse {
    match lvchange_activate(&body.vg, &body.name).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_500(e),
    }
}

async fn lv_deactivate_handler(Json(body): Json<LvNameReq>) -> impl IntoResponse {
    match lvchange_deactivate(&body.vg, &body.name).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_500(e),
    }
}

async fn clone_from_path_handler(Json(body): Json<CloneFromPathReq>) -> impl IntoResponse {
    use tokio::process::Command;
    // Caller is expected to have already created+activated the LV via
    // /lv_create. qemu-img convert handles raw and qcow2 sources transparently.
    let dst = format!("/dev/{}/{}", body.vg, body.name);
    let status = match Command::new("qemu-img")
        .arg("convert")
        .arg("-O")
        .arg("raw")
        .arg(&body.source_path)
        .arg(&dst)
        .status()
        .await
    {
        Ok(s) => s,
        Err(e) => return err_500(format!("qemu-img spawn: {e}")),
    };
    if !status.success() {
        return err_500(format!(
            "qemu-img convert {} -> {} failed: exit {:?}",
            body.source_path.display(),
            dst,
            status.code()
        ));
    }
    StatusCode::NO_CONTENT.into_response()
}

async fn lv_snapshot_handler(Json(body): Json<LvSnapshotReq>) -> impl IntoResponse {
    use tokio::process::Command;
    let size_arg = format!("{}b", body.size_bytes);
    let source = format!("{}/{}", body.vg, body.source_lv);
    let status = match Command::new("lvcreate")
        .args([
            "--snapshot",
            "--name",
            &body.snap_name,
            "--size",
            &size_arg,
            &source,
        ])
        .status()
        .await
    {
        Ok(s) => s,
        Err(e) => return err_500(format!("lvcreate --snapshot spawn: {e}")),
    };
    if !status.success() {
        return err_500(format!(
            "lvcreate --snapshot failed: exit {:?}",
            status.code()
        ));
    }
    (
        StatusCode::OK,
        Json(LvSnapshotResp {
            snap_name: body.snap_name,
        }),
    )
        .into_response()
}

/// Mounted under `/v1/storage/iscsi_lvm` by the parent storage router. Handlers
/// are stateless — they call the iSCSI/LVM helper functions directly. The
/// router is generic over the parent's state type `S` so it composes via
/// `Router::nest` regardless of what state the parent carries.
pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/login", post(login_handler))
        .route("/logout", post(logout_handler))
        .route("/init_vg", post(init_vg_handler))
        .route("/vg_status", post(vg_status_handler))
        .route("/lv_create", post(lv_create_handler))
        .route("/lv_remove", post(lv_remove_handler))
        .route("/lv_activate", post(lv_activate_handler))
        .route("/lv_deactivate", post(lv_deactivate_handler))
        .route("/clone_from_path", post(clone_from_path_handler))
        .route("/lv_snapshot", post(lv_snapshot_handler))
}

#[cfg(test)]
mod backend_tests {
    use super::*;
    use nexus_storage::{BackendInstanceId, BackendKind};
    use uuid::Uuid;

    #[test]
    fn host_backend_kind_is_iscsi_lvm() {
        let b = IscsiLvmHostBackend;
        assert!(matches!(b.kind(), BackendKind::IscsiLvm));
    }

    #[test]
    fn parse_locator_round_trip() {
        let raw = serde_json::json!({"vg":"vg-nqrust","lv":"vm-100-disk-0"}).to_string();
        let loc = IscsiLvmHostBackend::parse_locator(&raw).expect("parsed");
        assert_eq!(loc.vg, "vg-nqrust");
        assert_eq!(loc.lv, "vm-100-disk-0");
    }

    #[test]
    fn parse_locator_rejects_garbage() {
        let err = IscsiLvmHostBackend::parse_locator("not json").unwrap_err();
        assert!(matches!(err, StorageError::InvalidLocator(_)));
    }

    #[test]
    fn volume_handle_locator_path_format() {
        // Verify the device path format an agent would derive from a locator —
        // pinning the wire-format → /dev/<vg>/<lv> contract used by attach.
        let raw = serde_json::json!({"vg":"vg-x","lv":"vm-7"}).to_string();
        let loc = IscsiLvmHostBackend::parse_locator(&raw).unwrap();
        assert_eq!(format!("/dev/{}/{}", loc.vg, loc.lv), "/dev/vg-x/vm-7");

        // Make sure VolumeHandle/VolumeSnapshotHandle still build with this kind.
        let _ = VolumeHandle {
            volume_id: Uuid::new_v4(),
            backend_id: BackendInstanceId(Uuid::new_v4()),
            backend_kind: BackendKind::IscsiLvm,
            locator: raw,
            size_bytes: 1024,
        };
    }
}
