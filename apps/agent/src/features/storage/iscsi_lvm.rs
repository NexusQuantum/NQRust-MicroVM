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

use nexus_storage::StorageError;

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

    let device_str = device.to_str().ok_or_else(|| {
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
