use std::collections::HashMap;
use std::time::Duration;

use crate::features::hosts::repo::HostRow;
use crate::features::networks;
use crate::features::vms;
use crate::features::vms::repo::{VmDrive, VmNic};
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

    reconcile_devices(state, host, &vm_map, &inventory).await?;
    reconcile_networks(state, host).await?;

    Ok(())
}

async fn reconcile_networks(state: &AppState, host: &HostRow) -> Result<()> {
    let network_repo = networks::repo::NetworkRepository::new(state.db.clone());
    let client = reqwest::Client::new();

    // --- Single-host networks (NAT, isolated, bridged) ---
    let managed_networks = network_repo
        .list_active_managed_for_host(host.id)
        .await
        .unwrap_or_default();

    for network in &managed_networks {
        reconcile_single_network(&client, &network_repo, host, network).await;
    }

    // --- VXLAN overlay networks ---
    let vxlan_hosts = network_repo
        .list_active_vxlan_hosts_for_host(host.id)
        .await
        .unwrap_or_default();

    for nh in &vxlan_hosts {
        let network = match network_repo.get(nh.network_id).await {
            Ok(n) => n,
            Err(_) => continue,
        };

        // Check bridge exists on this host
        let status_url = format!(
            "{}/agent/v1/networks/status/{}",
            host.addr.trim_end_matches('/'),
            network.bridge_name
        );

        let bridge_exists = match client.get(&status_url).send().await {
            Ok(resp) if resp.status().is_success() => resp
                .json::<serde_json::Value>()
                .await
                .ok()
                .and_then(|v| v["bridge_exists"].as_bool())
                .unwrap_or(false),
            _ => {
                debug!(
                    network_id = %network.id,
                    bridge = %network.bridge_name,
                    "could not check VXLAN network status, skipping"
                );
                continue;
            }
        };

        if bridge_exists {
            continue;
        }

        // Re-provision VXLAN bridge
        info!(
            network_id = %network.id,
            bridge = %network.bridge_name,
            vni = ?network.vni,
            is_gateway = nh.is_gateway,
            "VXLAN network bridge missing, re-provisioning"
        );
        metrics::counter!("manager_reconciler_network_reprovision_attempts", 1);

        let provision_url = format!(
            "{}/agent/v1/networks/provision",
            host.addr.trim_end_matches('/')
        );

        let body = serde_json::json!({
            "network_type": "vxlan",
            "bridge_name": network.bridge_name,
            "cidr": network.cidr,
            "gateway": network.gateway,
            "dhcp_enabled": network.dhcp_enabled,
            "dhcp_range_start": network.dhcp_range_start,
            "dhcp_range_end": network.dhcp_range_end,
            "vni": network.vni,
            "local_ip": nh.vtep_ip,
            "is_gateway": nh.is_gateway,
        });

        match client.post(&provision_url).json(&body).send().await {
            Ok(resp) if resp.status().is_success() => {
                metrics::counter!("manager_reconciler_network_reprovision_success", 1);
                info!(
                    network_id = %network.id,
                    bridge = %network.bridge_name,
                    "VXLAN network re-provisioned successfully"
                );
            }
            Ok(resp) => {
                let err_body = resp.text().await.unwrap_or_default();
                metrics::counter!("manager_reconciler_network_reprovision_failure", 1);
                warn!(
                    network_id = %network.id,
                    bridge = %network.bridge_name,
                    error = %err_body,
                    "VXLAN network re-provisioning failed"
                );
                continue; // don't attempt peers if provisioning failed
            }
            Err(err) => {
                metrics::counter!("manager_reconciler_network_reprovision_failure", 1);
                warn!(
                    network_id = %network.id,
                    error = ?err,
                    "failed to reach agent for VXLAN re-provisioning"
                );
                continue;
            }
        }

        // Re-add FDB peers: get all other hosts for this network
        let all_hosts = network_repo
            .list_network_hosts(network.id)
            .await
            .unwrap_or_default();

        for peer in &all_hosts {
            if peer.host_id == host.id {
                continue; // skip self
            }
            let peer_url = format!(
                "{}/agent/v1/networks/peers/add",
                host.addr.trim_end_matches('/')
            );
            let peer_body = serde_json::json!({
                "vni": network.vni,
                "peer_ip": peer.vtep_ip,
            });
            if let Err(err) = client.post(&peer_url).json(&peer_body).send().await {
                warn!(
                    network_id = %network.id,
                    peer_ip = %peer.vtep_ip,
                    error = ?err,
                    "failed to re-add VXLAN peer"
                );
            }
        }
    }

    Ok(())
}

/// Reconcile a single non-VXLAN network (NAT, isolated, bridged).
async fn reconcile_single_network(
    client: &reqwest::Client,
    network_repo: &networks::repo::NetworkRepository,
    host: &HostRow,
    network: &networks::repo::NetworkRow,
) {
    let status_url = format!(
        "{}/agent/v1/networks/status/{}",
        host.addr.trim_end_matches('/'),
        network.bridge_name
    );

    let bridge_exists = match client.get(&status_url).send().await {
        Ok(resp) if resp.status().is_success() => resp
            .json::<serde_json::Value>()
            .await
            .ok()
            .and_then(|v| v["bridge_exists"].as_bool())
            .unwrap_or(false),
        _ => {
            debug!(
                network_id = %network.id,
                bridge = %network.bridge_name,
                "could not check network status, skipping"
            );
            return;
        }
    };

    if bridge_exists {
        return;
    }

    info!(
        network_id = %network.id,
        bridge = %network.bridge_name,
        network_type = %network.type_,
        "network bridge missing, re-provisioning"
    );
    metrics::counter!("manager_reconciler_network_reprovision_attempts", 1);

    let provision_url = format!(
        "{}/agent/v1/networks/provision",
        host.addr.trim_end_matches('/')
    );

    let mut body = serde_json::json!({
        "network_type": network.type_,
        "bridge_name": network.bridge_name,
        "cidr": network.cidr,
        "gateway": network.gateway,
        "dhcp_enabled": network.dhcp_enabled,
        "dhcp_range_start": network.dhcp_range_start,
        "dhcp_range_end": network.dhcp_range_end,
    });
    if let Some(ref uplink) = network.uplink_interface {
        body["uplink_interface"] = serde_json::json!(uplink);
    }

    match client.post(&provision_url).json(&body).send().await {
        Ok(resp) if resp.status().is_success() => {
            metrics::counter!("manager_reconciler_network_reprovision_success", 1);
            info!(
                network_id = %network.id,
                bridge = %network.bridge_name,
                "network re-provisioned successfully"
            );
        }
        Ok(resp) => {
            let err_body = resp.text().await.unwrap_or_default();
            metrics::counter!("manager_reconciler_network_reprovision_failure", 1);
            warn!(
                network_id = %network.id,
                bridge = %network.bridge_name,
                error = %err_body,
                "network re-provisioning failed"
            );
            let _ = network_repo
                .update_status(network.id, "error", Some(&err_body))
                .await;
        }
        Err(err) => {
            metrics::counter!("manager_reconciler_network_reprovision_failure", 1);
            warn!(
                network_id = %network.id,
                error = ?err,
                "failed to reach agent for network re-provisioning"
            );
        }
    }
}

async fn reconcile_devices(
    state: &AppState,
    host: &HostRow,
    vm_map: &HashMap<Uuid, vms::repo::VmRow>,
    _inventory: &AgentInventory,
) -> Result<()> {
    for (vm_id, vm_row) in vm_map {
        let desired_drives = vms::repo::drives::list(&state.db, *vm_id).await?;
        reconcile_vm_drives(state, host, vm_row, &desired_drives).await?;

        let desired_nics = vms::repo::nics::list(&state.db, *vm_id).await?;
        reconcile_vm_nics(state, host, vm_row, &desired_nics).await?;
    }
    Ok(())
}

async fn reconcile_vm_drives(
    _state: &AppState,
    _host: &HostRow,
    vm: &vms::repo::VmRow,
    desired: &[VmDrive],
) -> Result<()> {
    let base = format!("{}/agent/v1/vms/{}/proxy", vm.host_addr, vm.id);
    let qs = format!("?sock={}", urlencoding::encode(&vm.api_sock));
    let client = reqwest::Client::new();

    for drive in desired {
        let body = serde_json::json!({
            "drive_id": drive.drive_id,
            "path_on_host": drive.path_on_host,
            "is_root_device": drive.is_root_device,
            "is_read_only": drive.is_read_only,
            "cache_type": drive.cache_type,
            "io_engine": drive.io_engine,
            "rate_limiter": drive.rate_limiter,
        });

        if let Err(err) = client
            .put(format!("{base}/drives/{}{}", drive.drive_id, qs))
            .json(&body)
            .send()
            .await
            .and_then(|resp| resp.error_for_status())
        {
            warn!(vm_id = %vm.id, drive_id = %drive.drive_id, error = %err, "failed to reconcile drive");
        }
    }

    Ok(())
}

async fn reconcile_vm_nics(
    _state: &AppState,
    _host: &HostRow,
    vm: &vms::repo::VmRow,
    desired: &[VmNic],
) -> Result<()> {
    let base = format!("{}/agent/v1/vms/{}/proxy", vm.host_addr, vm.id);
    let qs = format!("?sock={}", urlencoding::encode(&vm.api_sock));
    let client = reqwest::Client::new();

    for nic in desired {
        let put_body = serde_json::json!({
            "iface_id": nic.iface_id,
            "host_dev_name": nic.host_dev_name,
            "guest_mac": nic.guest_mac,
        });

        if let Err(err) = client
            .put(format!("{base}/network-interfaces/{}{}", nic.iface_id, qs))
            .json(&put_body)
            .send()
            .await
            .and_then(|resp| resp.error_for_status())
        {
            warn!(vm_id = %vm.id, iface_id = %nic.iface_id, error = %err, "failed to reconcile nic");
            continue;
        }

        let patch_body = serde_json::json!({
            "iface_id": nic.iface_id,
            "rx_rate_limiter": nic.rx_rate_limiter,
            "tx_rate_limiter": nic.tx_rate_limiter,
        });

        if let Err(err) = client
            .patch(format!("{base}/network-interfaces/{}{}", nic.iface_id, qs))
            .json(&patch_body)
            .send()
            .await
            .and_then(|resp| resp.error_for_status())
        {
            warn!(vm_id = %vm.id, iface_id = %nic.iface_id, error = %err, "failed to reconcile nic rate limiters");
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
            created_by_user_id: None,
            guest_ip: None,
            tags: vec![],
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
