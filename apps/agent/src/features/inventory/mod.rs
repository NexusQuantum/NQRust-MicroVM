use std::path::Path;

use axum::{response::IntoResponse, routing::get, Extension, Json, Router};
use serde::Serialize;
use tokio::process::Command;

use crate::AppState;

pub fn router() -> Router {
    Router::new().route("/agent/v1/inventory", get(inventory))
}

async fn inventory(Extension(state): Extension<AppState>) -> impl IntoResponse {
    let (scopes, taps, sockets) =
        tokio::join!(list_scopes(), list_taps(), list_sockets(&state.run_dir),);

    let response = InventoryResponse {
        scopes: scopes.unwrap_or_default(),
        taps: taps.unwrap_or_default(),
        sockets: sockets.unwrap_or_default(),
    };

    Json(response)
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct InventoryResponse {
    scopes: Vec<String>,
    taps: Vec<String>,
    sockets: Vec<SocketInventory>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct SocketInventory {
    vm_id: String,
    sockets: Vec<String>,
    logs: Vec<String>,
}

async fn list_scopes() -> anyhow::Result<Vec<String>> {
    let output = Command::new("systemctl")
        .args([
            "list-units",
            "fc-*.scope",
            "--all",
            "--plain",
            "--no-legend",
        ])
        .output()
        .await?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_scopes(&stdout))
}

fn parse_scopes(output: &str) -> Vec<String> {
    let mut scopes: Vec<String> = output
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            let unit = line.split_whitespace().next()?;
            if unit.starts_with("fc-") && unit.ends_with(".scope") {
                Some(unit.to_string())
            } else {
                None
            }
        })
        .collect();
    scopes.sort();
    scopes.dedup();
    scopes
}

async fn list_taps() -> anyhow::Result<Vec<String>> {
    let output = Command::new("ip")
        .args(["-o", "link", "show"])
        .output()
        .await?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_taps(&stdout))
}

fn parse_taps(output: &str) -> Vec<String> {
    let mut taps: Vec<String> = output
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            let mut parts = line.splitn(3, ':');
            parts.next()?; // index
            let name = parts.next()?.trim();
            let name = name.split('@').next()?.trim();
            if name.starts_with("tap") {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect();
    taps.sort();
    taps.dedup();
    taps
}

async fn list_sockets(run_dir: &str) -> anyhow::Result<Vec<SocketInventory>> {
    let vms_dir = Path::new(run_dir).join("vms");
    let mut inventories = Vec::new();

    let mut vm_entries = match tokio::fs::read_dir(&vms_dir).await {
        Ok(entries) => entries,
        Err(_) => return Ok(inventories),
    };

    while let Some(entry) = vm_entries.next_entry().await? {
        if !entry.file_type().await?.is_dir() {
            continue;
        }
        let vm_id = entry.file_name().to_string_lossy().to_string();
        let vm_path = entry.path();

        let sockets = collect_dir_files(vm_path.join("sock")).await;
        let logs = collect_dir_files(vm_path.join("log")).await;

        inventories.push(SocketInventory {
            vm_id,
            sockets,
            logs,
        });
    }

    inventories.sort_by(|a, b| a.vm_id.cmp(&b.vm_id));
    Ok(inventories)
}

async fn collect_dir_files(path: impl AsRef<Path>) -> Vec<String> {
    let path = path.as_ref().to_path_buf();
    let mut files = Vec::new();

    if let Ok(mut dir) = tokio::fs::read_dir(&path).await {
        while let Ok(Some(entry)) = dir.next_entry().await {
            if entry
                .file_type()
                .await
                .map(|ft| ft.is_file())
                .unwrap_or(false)
            {
                if let Some(path_str) = entry.path().to_str() {
                    files.push(path_str.to_string());
                }
            }
        }
    }

    files.sort();
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_scopes_from_systemctl() {
        let output = r#"
    fc-123.scope             loaded active running   VM fc-123
    fc-abc.scope             loaded inactive dead
    other.scope              loaded active running   something else
"#;
        let scopes = parse_scopes(output);
        assert_eq!(scopes, vec!["fc-123.scope", "fc-abc.scope"]);
    }

    #[test]
    fn parses_tap_names_from_ip_link() {
        let output = r#"
1: lo: <LOOPBACK> mtu 65536 qdisc noop state DOWN mode DEFAULT group default qlen 1000
2: tap-vm123: <BROADCAST,MULTICAST,UP,LOWER_UP> mtu 1500 qdisc fq_codel state UP mode DEFAULT group default qlen 1000
3: tap-vm456@fcbr0: <BROADCAST,MULTICAST> mtu 1500 qdisc noop state DOWN mode DEFAULT group default qlen 1000
4: eth0@if5: <BROADCAST,MULTICAST> mtu 1500 qdisc noop state DOWN mode DEFAULT group default qlen 1000
"#;
        let taps = parse_taps(output);
        assert_eq!(taps, vec!["tap-vm123", "tap-vm456"]);
    }

    #[tokio::test]
    async fn lists_socket_and_log_files() {
        let tmp = tempfile::tempdir().unwrap();
        let run_dir = tmp.path().join("fc");
        let vm_dir = run_dir.join("vms").join("vm-01");
        let sock_dir = vm_dir.join("sock");
        let log_dir = vm_dir.join("log");
        tokio::fs::create_dir_all(&sock_dir).await.unwrap();
        tokio::fs::create_dir_all(&log_dir).await.unwrap();

        let sock_path = sock_dir.join("fc.sock");
        let log_path = log_dir.join("fc.log");
        tokio::fs::write(&sock_path, b"sock").await.unwrap();
        tokio::fs::write(&log_path, b"log").await.unwrap();

        let sockets = list_sockets(&run_dir.to_string_lossy()).await.unwrap();

        assert_eq!(
            sockets,
            vec![SocketInventory {
                vm_id: "vm-01".into(),
                sockets: vec![sock_path.to_string_lossy().into_owned()],
                logs: vec![log_path.to_string_lossy().into_owned()],
            }]
        );
    }
}
