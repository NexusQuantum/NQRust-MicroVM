use std::collections::HashMap;
use std::time::Duration;

use crate::features::hosts::repo::HostRow;
use crate::features::vms;
use crate::AppState;
use anyhow::{anyhow, Result};
use reqwest::StatusCode;
use serde::Deserialize;
use tokio::time::{interval, MissedTickBehavior};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

const INTERVAL_SECS: u64 = 15;

pub fn spawn(state: AppState) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(INTERVAL_SECS));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            if let Err(err) = reconcile_once(&state).await {
                error!(error = ?err, "reconciler iteration failed");
            }
            ticker.tick().await;
        }
    })
}

async fn reconcile_once(state: &AppState) -> Result<()> {
    let hosts = state.hosts.list_healthy().await?;
    for host in hosts {
        match fetch_inventory(&host).await {
            Ok(inventory) => {
                reconcile_host(state, &host, inventory).await?;
            }
            Err(err) => {
                warn!(host_id = %host.id, host_addr = %host.addr, error = ?err, "failed to fetch inventory");
            }
        }
    }
    Ok(())
}

async fn reconcile_host(state: &AppState, host: &HostRow, inventory: AgentInventory) -> Result<()> {
    let vms = vms::repo::list_by_host(&state.db, host.id).await?;
    let plan = diff_host(&vms, &inventory);
    let vm_map: HashMap<Uuid, vms::repo::VmRow> =
        vms.into_iter().map(|row| (row.id, row)).collect();

    for vm_id in plan.restart {
        if let Some(vm) = vm_map.get(&vm_id) {
            metrics::counter!("manager_reconciler_restart_attempts", 1);
            info!(vm_id = %vm.id, host_id = %host.id, "attempting restart for vm missing resources");
            match vms::service::restart_vm(state, vm).await {
                Ok(()) => {
                    metrics::counter!("manager_reconciler_restart_success", 1);
                    info!(vm_id = %vm.id, host_id = %host.id, "vm restart succeeded");
                }
                Err(err) => {
                    metrics::counter!("manager_reconciler_restart_failure", 1);
                    error!(vm_id = %vm.id, host_id = %host.id, error = ?err, "vm restart failed");
                    vms::repo::update_state(&state.db, vm.id, "stopped").await?;
                    let message = format!("reconciler restart failed: {err:#}");
                    let _ = vms::repo::insert_event(&state.db, vm.id, "error", &message).await;
                }
            }
        }
    }

    for orphan in plan.orphans {
        metrics::counter!("manager_reconciler_orphan_cleanup_attempts", 1);
        match cleanup_orphan(&host.addr, &orphan).await {
            Ok(()) => {
                metrics::counter!("manager_reconciler_orphan_cleanup_success", 1);
                info!(vm_id = %orphan.vm_id, host_id = %host.id, "cleaned orphan artifacts");
            }
            Err(err) => {
                metrics::counter!("manager_reconciler_orphan_cleanup_failure", 1);
                warn!(vm_id = %orphan.vm_id, host_id = %host.id, error = ?err, "failed to cleanup orphan artifacts");
            }
        }
    }

    Ok(())
}

async fn fetch_inventory(host: &HostRow) -> Result<AgentInventory> {
    let response = reqwest::Client::new()
        .get(format!("{}/agent/v1/inventory", host.addr))
        .send()
        .await?;

    if response.status() == StatusCode::NOT_FOUND {
        return Err(anyhow!("inventory endpoint missing"));
    }

    let inv = response
        .error_for_status()?
        .json::<AgentInventory>()
        .await?;
    Ok(inv)
}

async fn cleanup_orphan(host_addr: &str, orphan: &OrphanArtifacts) -> Result<()> {
    let tap = orphan
        .tap
        .clone()
        .unwrap_or_else(|| format!("tap-{}", &orphan.vm_id.to_string()[..8]));
    let fc_unit = orphan
        .scope
        .clone()
        .unwrap_or_else(|| format!("fc-{}.scope", orphan.vm_id));
    let sock = orphan
        .sockets
        .iter()
        .find(|path| path.ends_with(".sock"))
        .cloned()
        .unwrap_or_else(|| format!("/srv/fc/vms/{}/sock/fc.sock", orphan.vm_id));

    let body = serde_json::json!({
        "tap": tap,
        "sock": sock,
        "fc_unit": fc_unit,
    });

    reqwest::Client::new()
        .post(format!("{host_addr}/agent/v1/vms/{}/stop", orphan.vm_id))
        .json(&body)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AgentInventory {
    pub scopes: Vec<String>,
    pub taps: Vec<String>,
    pub sockets: Vec<SocketInventory>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SocketInventory {
    pub vm_id: String,
    pub sockets: Vec<String>,
    pub logs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HostPlan {
    pub restart: Vec<Uuid>,
    pub orphans: Vec<OrphanArtifacts>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OrphanArtifacts {
    pub vm_id: Uuid,
    pub scope: Option<String>,
    pub tap: Option<String>,
    pub sockets: Vec<String>,
}

struct VmPresence {
    has_scope: bool,
    has_socket: bool,
}

pub fn diff_host(vms: &[vms::repo::VmRow], inventory: &AgentInventory) -> HostPlan {
    let mut status: HashMap<Uuid, VmPresence> = vms
        .iter()
        .map(|vm| {
            (
                vm.id,
                VmPresence {
                    has_scope: false,
                    has_socket: false,
                },
            )
        })
        .collect();
    let mut orphans: HashMap<Uuid, OrphanArtifacts> = HashMap::new();

    for scope in &inventory.scopes {
        match parse_scope(scope) {
            Some(vm_id) if status.contains_key(&vm_id) => {
                if let Some(presence) = status.get_mut(&vm_id) {
                    presence.has_scope = true;
                }
            }
            Some(vm_id) => {
                let entry = orphans.entry(vm_id).or_insert_with(|| OrphanArtifacts {
                    vm_id,
                    ..Default::default()
                });
                entry.scope = Some(scope.clone());
            }
            None => debug!(%scope, "ignoring unknown scope"),
        }
    }

    for tap in &inventory.taps {
        match parse_tap(tap) {
            Some(vm_id) if status.contains_key(&vm_id) => {
                // taps are best-effort; restarts focus on scope/socket
            }
            Some(vm_id) => {
                let entry = orphans.entry(vm_id).or_insert_with(|| OrphanArtifacts {
                    vm_id,
                    ..Default::default()
                });
                entry.tap = Some(tap.clone());
            }
            None => debug!(%tap, "ignoring tap without vm id"),
        }
    }

    for sock_inv in &inventory.sockets {
        match Uuid::parse_str(&sock_inv.vm_id) {
            Ok(vm_id) if status.contains_key(&vm_id) => {
                if let Some(presence) = status.get_mut(&vm_id) {
                    // sockets vector contains fully qualified paths
                    if let Some(vm) = vms.iter().find(|vm| vm.id == vm_id) {
                        if sock_inv.sockets.iter().any(|s| s == &vm.api_sock) {
                            presence.has_socket = true;
                        }
                    }
                }
            }
            Ok(vm_id) => {
                let entry = orphans.entry(vm_id).or_insert_with(|| OrphanArtifacts {
                    vm_id,
                    ..Default::default()
                });
                entry.sockets.extend(sock_inv.sockets.clone());
            }
            Err(_) => debug!(vm_id = %sock_inv.vm_id, "ignoring socket inventory without uuid"),
        }
    }

    let mut restart = Vec::new();
    for vm in vms {
        if vm.state == "running" {
            if let Some(presence) = status.get(&vm.id) {
                if !presence.has_scope || !presence.has_socket {
                    restart.push(vm.id);
                }
            } else {
                restart.push(vm.id);
            }
        }
    }

    HostPlan {
        restart,
        orphans: orphans.into_values().collect(),
    }
}

fn parse_scope(scope: &str) -> Option<Uuid> {
    scope
        .strip_prefix("fc-")
        .and_then(|rest| rest.strip_suffix(".scope"))
        .and_then(|id| Uuid::parse_str(id).ok())
}

fn parse_tap(tap: &str) -> Option<Uuid> {
    tap.strip_prefix("tap-")
        .and_then(|id| Uuid::parse_str(id).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_vm(id: Uuid) -> vms::repo::VmRow {
        vms::repo::VmRow {
            id,
            name: format!("vm-{id}"),
            state: "running".into(),
            host_id: Uuid::new_v4(),
            template_id: None,
            host_addr: "http://127.0.0.1".into(),
            api_sock: format!("/srv/fc/vms/{id}/sock/fc.sock"),
            tap: format!("tap-{}", &id.to_string()[..8]),
            log_path: format!("/srv/fc/vms/{id}/logs/firecracker.log"),
            http_port: 0,
            fc_unit: format!("fc-{id}.scope"),
            vcpu: 1,
            mem_mib: 512,
            kernel_path: "/tmp/kernel".into(),
            rootfs_path: "/tmp/rootfs".into(),
            source_snapshot_id: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn diff_marks_restart_when_scope_missing() {
        let vm_id = Uuid::new_v4();
        let vm = make_vm(vm_id);
        let inv = AgentInventory {
            scopes: vec![],
            taps: vec![format!("tap-{}", &vm_id.to_string()[..8])],
            sockets: vec![SocketInventory {
                vm_id: vm_id.to_string(),
                sockets: vec![vm.api_sock.clone()],
                logs: vec![],
            }],
        };

        let plan = diff_host(&[vm], &inv);
        assert_eq!(plan.restart, vec![vm_id]);
        assert!(plan.orphans.is_empty());
    }

    #[test]
    fn diff_marks_restart_when_socket_missing() {
        let vm_id = Uuid::new_v4();
        let vm = make_vm(vm_id);
        let inv = AgentInventory {
            scopes: vec![format!("fc-{vm_id}.scope")],
            taps: vec![format!("tap-{}", &vm_id.to_string()[..8])],
            sockets: vec![],
        };

        let plan = diff_host(&[vm], &inv);
        assert_eq!(plan.restart, vec![vm_id]);
    }

    #[test]
    fn diff_detects_orphan_scope() {
        let vm_id = Uuid::new_v4();
        let inv = AgentInventory {
            scopes: vec![format!("fc-{vm_id}.scope")],
            taps: vec![],
            sockets: vec![],
        };

        let plan = diff_host(&[], &inv);
        assert_eq!(plan.restart.len(), 0);
        assert_eq!(plan.orphans.len(), 1);
        assert_eq!(plan.orphans[0].vm_id, vm_id);
        assert_eq!(plan.orphans[0].scope, Some(format!("fc-{vm_id}.scope")));
    }

    #[test]
    fn diff_ignores_invalid_artifacts() {
        let vm_id = Uuid::new_v4();
        let vm = make_vm(vm_id);
        let inv = AgentInventory {
            scopes: vec!["weird.scope".into()],
            taps: vec!["tap-not-a-uuid".into()],
            sockets: vec![SocketInventory {
                vm_id: "not-a-uuid".into(),
                sockets: vec!["/tmp/foo.sock".into()],
                logs: vec![],
            }],
        };

        let plan = diff_host(&[vm], &inv);
        assert!(plan.orphans.is_empty());
        assert_eq!(plan.restart, vec![vm_id]);
    }
}
