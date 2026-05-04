//! Pure-logic parsers for LVM tool output (`pvs`, `vgs`, `lvs`).
//!
//! These functions parse lines produced by the LVM tools when invoked with
//! `--separator : --noheadings` and a fixed column set. They are intentionally
//! free of any I/O so they can be unit-tested without LVM/iSCSI installed on
//! the host.

#![allow(dead_code)]

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
