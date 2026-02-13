use crate::features::networks::repo::{NetworkRepository, NetworkRow};
use crate::AppState;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct NetworkSuggestion {
    pub bridge_name: String,
    pub cidr: String,
    pub gateway: String,
    pub dhcp_range_start: String,
    pub dhcp_range_end: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateNetworkParams {
    pub name: String,
    pub description: Option<String>,
    pub network_type: String,
    pub host_id: Uuid,
    pub cidr: Option<String>,
    pub vlan_id: Option<i32>,
    pub dhcp_enabled: Option<bool>,
    pub dhcp_range_start: Option<String>,
    pub dhcp_range_end: Option<String>,
    /// Required for bridged networks: the physical NIC to attach
    pub uplink_interface: Option<String>,
    /// Required for VXLAN networks: the host that runs DHCP + NAT
    pub gateway_host_id: Option<Uuid>,
}

/// Create a network and provision it on the host via the agent.
pub async fn create_network(st: &AppState, params: CreateNetworkParams) -> Result<NetworkRow> {
    if params.network_type != "nat"
        && params.network_type != "isolated"
        && params.network_type != "bridged"
        && params.network_type != "vxlan"
    {
        return Err(anyhow!(
            "network type must be 'nat', 'isolated', 'bridged', or 'vxlan'"
        ));
    }

    if params.network_type == "bridged" && params.uplink_interface.is_none() {
        return Err(anyhow!("uplink_interface is required for bridged networks"));
    }

    // Route VXLAN to its own creation flow
    if params.network_type == "vxlan" {
        return create_vxlan_network(st, params).await;
    }

    let network_repo = NetworkRepository::new(st.db.clone());

    // Get host info to find agent address
    let host = st
        .hosts
        .get(params.host_id)
        .await
        .context("host not found")?;

    // Auto-generate bridge name
    let suggestion = suggest_network(st, params.host_id).await?;
    let bridge_name = suggestion.bridge_name;

    // Bridged networks use the external network — no CIDR/gateway/DHCP from us
    let (cidr, gateway, dhcp_enabled, dhcp_start, dhcp_end) = if params.network_type == "bridged" {
        (
            params.cidr,
            None::<String>,
            false,
            None::<String>,
            None::<String>,
        )
    } else {
        let cidr = params.cidr.unwrap_or(suggestion.cidr);
        let gw = derive_gateway(&cidr)?;
        let (auto_start, auto_end) = derive_dhcp_range(&cidr)?;
        let dhcp_on = params.dhcp_enabled.unwrap_or(true);
        let start = params.dhcp_range_start.unwrap_or(auto_start);
        let end = params.dhcp_range_end.unwrap_or(auto_end);
        (
            Some(cidr),
            Some(gw),
            dhcp_on,
            if dhcp_on { Some(start) } else { None },
            if dhcp_on { Some(end) } else { None },
        )
    };

    info!(
        name = %params.name,
        network_type = %params.network_type,
        bridge = %bridge_name,
        "creating network"
    );

    // Insert DB record with status='provisioning'
    let network = network_repo
        .create(
            &params.name,
            params.description.as_deref(),
            &params.network_type,
            params.vlan_id,
            &bridge_name,
            params.host_id,
            cidr.as_deref(),
            gateway.as_deref(),
            "provisioning",
            true, // managed
            dhcp_enabled,
            dhcp_start.as_deref(),
            dhcp_end.as_deref(),
            params.uplink_interface.as_deref(),
        )
        .await
        .context("failed to insert network record")?;

    // Call agent to provision
    let agent_url = format!(
        "{}/agent/v1/networks/provision",
        host.addr.trim_end_matches('/')
    );

    let mut provision_body = serde_json::json!({
        "network_type": params.network_type,
        "bridge_name": bridge_name,
    });
    if let Some(ref c) = cidr {
        provision_body["cidr"] = serde_json::json!(c);
    }
    if let Some(ref g) = gateway {
        provision_body["gateway"] = serde_json::json!(g);
    }
    provision_body["dhcp_enabled"] = serde_json::json!(dhcp_enabled);
    if let Some(ref s) = dhcp_start {
        provision_body["dhcp_range_start"] = serde_json::json!(s);
    }
    if let Some(ref e) = dhcp_end {
        provision_body["dhcp_range_end"] = serde_json::json!(e);
    }
    if let Some(ref uplink) = params.uplink_interface {
        provision_body["uplink_interface"] = serde_json::json!(uplink);
    }

    let client = reqwest::Client::new();
    let provision_result = client.post(&agent_url).json(&provision_body).send().await;

    match provision_result {
        std::result::Result::Ok(resp) if resp.status().is_success() => {
            info!(network_id = %network.id, bridge = %bridge_name, "network provisioned successfully");
            let updated = network_repo
                .update_status(network.id, "active", None)
                .await
                .context("failed to update network status to active")?;
            Ok(updated)
        }
        std::result::Result::Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            let err_msg = format!("agent returned {}: {}", status, body);
            error!(network_id = %network.id, error = %err_msg, "network provisioning failed");
            let _ = network_repo
                .update_status(network.id, "error", Some(&err_msg))
                .await;
            Err(anyhow!("provisioning failed: {}", err_msg))
        }
        Err(e) => {
            let err_msg = format!("failed to reach agent: {}", e);
            error!(network_id = %network.id, error = %err_msg, "network provisioning failed");
            let _ = network_repo
                .update_status(network.id, "error", Some(&err_msg))
                .await;
            Err(anyhow!("provisioning failed: {}", err_msg))
        }
    }
}

/// Delete a network, tearing down infrastructure if managed.
pub async fn delete_network(st: &AppState, id: Uuid) -> Result<()> {
    let network_repo = NetworkRepository::new(st.db.clone());

    let network = network_repo.get(id).await.context("network not found")?;

    // Check no VMs attached
    let vm_count = network_repo.get_vm_count(id).await.unwrap_or(0);
    if vm_count > 0 {
        return Err(anyhow!(
            "cannot delete network with {} attached VMs",
            vm_count
        ));
    }

    // VXLAN networks need multi-host teardown
    if network.type_ == "vxlan" {
        return delete_vxlan_network(st, &network).await;
    }

    // If managed, call agent to teardown
    if network.managed {
        if let Some(host_id) = network.host_id {
            let _ = network_repo.update_status(id, "deleting", None).await;

            if let std::result::Result::Ok(host) = st.hosts.get(host_id).await {
                let agent_url = format!(
                    "{}/agent/v1/networks/teardown",
                    host.addr.trim_end_matches('/')
                );

                let client = reqwest::Client::new();
                let result = client
                    .post(&agent_url)
                    .json(&serde_json::json!({
                        "network_type": network.type_,
                        "bridge_name": network.bridge_name,
                        "cidr": network.cidr.unwrap_or_default(),
                    }))
                    .send()
                    .await;

                match result {
                    std::result::Result::Ok(resp) if resp.status().is_success() => {
                        info!(network_id = %id, "network teardown successful");
                    }
                    std::result::Result::Ok(resp) => {
                        let body = resp.text().await.unwrap_or_default();
                        warn!(network_id = %id, error = %body, "agent teardown returned error, deleting record anyway");
                    }
                    Err(e) => {
                        warn!(network_id = %id, error = %e, "failed to reach agent for teardown, deleting record anyway");
                    }
                }
            }
        }
    }

    network_repo
        .delete(id)
        .await
        .context("failed to delete network record")?;
    Ok(())
}

/// Retry provisioning for a network in error state.
pub async fn retry_network(st: &AppState, id: Uuid) -> Result<NetworkRow> {
    let network_repo = NetworkRepository::new(st.db.clone());
    let network = network_repo.get(id).await.context("network not found")?;

    if network.status != "error" {
        return Err(anyhow!(
            "can only retry networks in error state, current: {}",
            network.status
        ));
    }

    let host_id = network
        .host_id
        .ok_or_else(|| anyhow!("network has no host"))?;
    let host = st.hosts.get(host_id).await.context("host not found")?;

    let _ = network_repo.update_status(id, "provisioning", None).await;

    let agent_url = format!(
        "{}/agent/v1/networks/provision",
        host.addr.trim_end_matches('/')
    );

    let mut provision_body = serde_json::json!({
        "network_type": network.type_,
        "bridge_name": network.bridge_name,
        "cidr": network.cidr,
        "gateway": network.gateway,
        "dhcp_enabled": network.dhcp_enabled,
        "dhcp_range_start": network.dhcp_range_start,
        "dhcp_range_end": network.dhcp_range_end,
    });
    if let Some(ref uplink) = network.uplink_interface {
        provision_body["uplink_interface"] = serde_json::json!(uplink);
    }

    let client = reqwest::Client::new();
    let result = client.post(&agent_url).json(&provision_body).send().await;

    match result {
        std::result::Result::Ok(resp) if resp.status().is_success() => {
            let updated = network_repo.update_status(id, "active", None).await?;
            Ok(updated)
        }
        std::result::Result::Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            let err_msg = format!("agent returned {}: {}", status, body);
            let _ = network_repo
                .update_status(id, "error", Some(&err_msg))
                .await;
            Err(anyhow!("retry failed: {}", err_msg))
        }
        Err(e) => {
            let err_msg = format!("failed to reach agent: {}", e);
            let _ = network_repo
                .update_status(id, "error", Some(&err_msg))
                .await;
            Err(anyhow!("retry failed: {}", err_msg))
        }
    }
}

/// Suggest next available bridge name and subnet for a host.
pub async fn suggest_network(st: &AppState, host_id: Uuid) -> Result<NetworkSuggestion> {
    let network_repo = NetworkRepository::new(st.db.clone());

    // Find next available bridge name
    let existing_bridges = network_repo
        .list_bridge_names_for_host(host_id)
        .await
        .unwrap_or_default();

    let mut bridge_num = 1u32;
    loop {
        let candidate = format!("nqbr{}", bridge_num);
        if !existing_bridges.contains(&candidate) {
            break;
        }
        bridge_num += 1;
    }
    let bridge_name = format!("nqbr{}", bridge_num);

    // Find next available /24 subnet
    let existing_cidrs = network_repo
        .list_cidrs_for_host(host_id)
        .await
        .unwrap_or_default();

    // Reserved subnets: 10.0.0.0/24 (installer NAT), 10.1.0.0/24 (installer isolated)
    let mut subnet_third = 2u32;
    loop {
        let candidate = format!("10.0.{}.0/24", subnet_third);
        if !existing_cidrs.contains(&candidate) {
            break;
        }
        subnet_third += 1;
        if subnet_third > 254 {
            return Err(anyhow!("no available subnets"));
        }
    }

    let cidr = format!("10.0.{}.0/24", subnet_third);
    let gateway = format!("10.0.{}.1", subnet_third);
    let dhcp_start = format!("10.0.{}.10", subnet_third);
    let dhcp_end = format!("10.0.{}.250", subnet_third);

    Ok(NetworkSuggestion {
        bridge_name,
        cidr,
        gateway,
        dhcp_range_start: dhcp_start,
        dhcp_range_end: dhcp_end,
    })
}

/// List physical network interfaces on a host (proxied from agent).
pub async fn list_host_interfaces(st: &AppState, host_id: Uuid) -> Result<Vec<serde_json::Value>> {
    let host = st.hosts.get(host_id).await.context("host not found")?;

    let agent_url = format!(
        "{}/agent/v1/networks/interfaces",
        host.addr.trim_end_matches('/')
    );

    let client = reqwest::Client::new();
    let resp = client
        .get(&agent_url)
        .send()
        .await
        .context("failed to reach agent")?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!("agent returned error: {}", body));
    }

    let body: serde_json::Value = resp.json().await.context("invalid agent response")?;
    let interfaces = body["interfaces"].as_array().cloned().unwrap_or_default();

    Ok(interfaces)
}

// ========== VXLAN overlay network functions ==========

/// Create a VXLAN overlay network: provision on the gateway host, set up DHCP + NAT.
async fn create_vxlan_network(st: &AppState, params: CreateNetworkParams) -> Result<NetworkRow> {
    let gateway_host_id = params.gateway_host_id.unwrap_or(params.host_id);

    let network_repo = NetworkRepository::new(st.db.clone());

    let gateway_host = st
        .hosts
        .get(gateway_host_id)
        .await
        .context("gateway host not found")?;

    // Auto-assign VNI
    let vni = network_repo
        .next_available_vni()
        .await
        .context("failed to get next VNI")?;

    // Auto-generate bridge name and subnet
    let suggestion = suggest_network(st, gateway_host_id).await?;
    let bridge_name = format!("br-vx{}", vni);

    let cidr = params.cidr.unwrap_or(suggestion.cidr);
    let gateway = derive_gateway(&cidr)?;
    let (auto_start, auto_end) = derive_dhcp_range(&cidr)?;
    let dhcp_on = params.dhcp_enabled.unwrap_or(true);
    let dhcp_start = params.dhcp_range_start.unwrap_or(auto_start);
    let dhcp_end = params.dhcp_range_end.unwrap_or(auto_end);

    let vtep_ip = parse_host_ip(&gateway_host.addr)?;

    info!(
        name = %params.name,
        vni = vni,
        bridge = %bridge_name,
        gateway_host = %gateway_host.name,
        vtep_ip = %vtep_ip,
        "creating VXLAN network"
    );

    // Insert network record
    let network = network_repo
        .create_with_vni(
            &params.name,
            params.description.as_deref(),
            "vxlan",
            params.vlan_id,
            &bridge_name,
            gateway_host_id,
            Some(&cidr),
            Some(&gateway),
            "provisioning",
            true,
            dhcp_on,
            if dhcp_on {
                Some(dhcp_start.as_str())
            } else {
                None
            },
            if dhcp_on {
                Some(dhcp_end.as_str())
            } else {
                None
            },
            vni,
        )
        .await
        .context("failed to insert VXLAN network record")?;

    // Insert network_host record for gateway
    let nh = network_repo
        .add_network_host(network.id, gateway_host_id, &vtep_ip, true)
        .await
        .context("failed to insert network_host record")?;

    // Provision on gateway host
    let agent_url = format!(
        "{}/agent/v1/networks/provision",
        gateway_host.addr.trim_end_matches('/')
    );

    let provision_body = serde_json::json!({
        "network_type": "vxlan",
        "bridge_name": bridge_name,
        "vni": vni,
        "local_ip": vtep_ip,
        "cidr": cidr,
        "gateway": gateway,
        "is_gateway": true,
        "dhcp_enabled": dhcp_on,
        "dhcp_range_start": if dhcp_on { Some(&dhcp_start) } else { None },
        "dhcp_range_end": if dhcp_on { Some(&dhcp_end) } else { None },
    });

    let client = reqwest::Client::new();
    let result = client.post(&agent_url).json(&provision_body).send().await;

    match result {
        std::result::Result::Ok(resp) if resp.status().is_success() => {
            info!(network_id = %network.id, vni = vni, "VXLAN network provisioned on gateway");
            let _ = network_repo
                .update_network_host_status(nh.id, "active", None)
                .await;
            let updated = network_repo
                .update_status(network.id, "active", None)
                .await
                .context("failed to update network status")?;
            Ok(updated)
        }
        std::result::Result::Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            let err_msg = format!("agent returned {}: {}", status, body);
            error!(network_id = %network.id, error = %err_msg, "VXLAN provisioning failed");
            let _ = network_repo
                .update_network_host_status(nh.id, "error", Some(&err_msg))
                .await;
            let _ = network_repo
                .update_status(network.id, "error", Some(&err_msg))
                .await;
            Err(anyhow!("provisioning failed: {}", err_msg))
        }
        Err(e) => {
            let err_msg = format!("failed to reach agent: {}", e);
            error!(network_id = %network.id, error = %err_msg, "VXLAN provisioning failed");
            let _ = network_repo
                .update_network_host_status(nh.id, "error", Some(&err_msg))
                .await;
            let _ = network_repo
                .update_status(network.id, "error", Some(&err_msg))
                .await;
            Err(anyhow!("provisioning failed: {}", err_msg))
        }
    }
}

/// Delete a VXLAN network: teardown on all participating hosts.
async fn delete_vxlan_network(st: &AppState, network: &NetworkRow) -> Result<()> {
    let network_repo = NetworkRepository::new(st.db.clone());
    let _ = network_repo
        .update_status(network.id, "deleting", None)
        .await;

    let network_hosts = network_repo
        .list_network_hosts(network.id)
        .await
        .unwrap_or_default();

    let client = reqwest::Client::new();
    let vni = network.vni.unwrap_or(0);

    for nh in &network_hosts {
        if let std::result::Result::Ok(host) = st.hosts.get(nh.host_id).await {
            let agent_url = format!(
                "{}/agent/v1/networks/teardown",
                host.addr.trim_end_matches('/')
            );

            let result = client
                .post(&agent_url)
                .json(&serde_json::json!({
                    "network_type": "vxlan",
                    "bridge_name": network.bridge_name,
                    "cidr": network.cidr.as_deref().unwrap_or_default(),
                    "vni": vni,
                    "is_gateway": nh.is_gateway,
                }))
                .send()
                .await;

            match result {
                std::result::Result::Ok(resp) if resp.status().is_success() => {
                    info!(network_id = %network.id, host_id = %nh.host_id, "VXLAN teardown successful");
                }
                std::result::Result::Ok(resp) => {
                    let body = resp.text().await.unwrap_or_default();
                    warn!(network_id = %network.id, host_id = %nh.host_id, error = %body, "VXLAN teardown error, continuing");
                }
                Err(e) => {
                    warn!(network_id = %network.id, host_id = %nh.host_id, error = %e, "failed to reach agent for VXLAN teardown, continuing");
                }
            }
        }
    }

    network_repo
        .delete_network_hosts(network.id)
        .await
        .context("failed to delete network_host records")?;
    network_repo
        .delete(network.id)
        .await
        .context("failed to delete VXLAN network record")?;

    Ok(())
}

/// Expand a VXLAN network to a new host. Called during VM creation when the VM's
/// host doesn't yet participate in the overlay.
pub async fn expand_vxlan_to_host(
    st: &AppState,
    network: &NetworkRow,
    new_host_id: Uuid,
) -> Result<()> {
    let network_repo = NetworkRepository::new(st.db.clone());
    let new_host = st
        .hosts
        .get(new_host_id)
        .await
        .context("new host not found")?;
    let new_vtep_ip = parse_host_ip(&new_host.addr)?;
    let vni = network
        .vni
        .ok_or_else(|| anyhow!("VXLAN network has no VNI"))?;

    info!(
        network_id = %network.id,
        vni = vni,
        new_host = %new_host.name,
        new_vtep = %new_vtep_ip,
        "expanding VXLAN to new host"
    );

    // Get all existing peers
    let existing_peers = network_repo
        .list_network_hosts(network.id)
        .await
        .context("failed to list network hosts")?;

    // 1. Provision VXLAN on new host (non-gateway)
    let agent_url = format!(
        "{}/agent/v1/networks/provision",
        new_host.addr.trim_end_matches('/')
    );

    let client = reqwest::Client::new();
    let result = client
        .post(&agent_url)
        .json(&serde_json::json!({
            "network_type": "vxlan",
            "bridge_name": network.bridge_name,
            "vni": vni,
            "local_ip": new_vtep_ip,
            "cidr": network.cidr,
            "gateway": network.gateway,
            "is_gateway": false,
            "dhcp_enabled": false,
        }))
        .send()
        .await;

    match result {
        std::result::Result::Ok(resp) if resp.status().is_success() => {}
        std::result::Result::Ok(resp) => {
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!(
                "failed to provision VXLAN on {}: {}",
                new_host.name,
                body
            ));
        }
        Err(e) => {
            return Err(anyhow!("failed to reach agent on {}: {}", new_host.name, e));
        }
    }

    // 2. Add FDB peers on new host → point to all existing hosts
    for peer in &existing_peers {
        let _ = client
            .post(format!(
                "{}/agent/v1/networks/peers/add",
                new_host.addr.trim_end_matches('/')
            ))
            .json(&serde_json::json!({
                "vni": vni,
                "peer_ip": peer.vtep_ip,
            }))
            .send()
            .await;
    }

    // 3. Add FDB peer on all existing hosts → point to new host
    for peer in &existing_peers {
        if let std::result::Result::Ok(host) = st.hosts.get(peer.host_id).await {
            let _ = client
                .post(format!(
                    "{}/agent/v1/networks/peers/add",
                    host.addr.trim_end_matches('/')
                ))
                .json(&serde_json::json!({
                    "vni": vni,
                    "peer_ip": new_vtep_ip,
                }))
                .send()
                .await;
        }
    }

    // 4. Record in DB
    let nh = network_repo
        .add_network_host(network.id, new_host_id, &new_vtep_ip, false)
        .await
        .context("failed to insert network_host record")?;
    let _ = network_repo
        .update_network_host_status(nh.id, "active", None)
        .await;

    info!(
        network_id = %network.id,
        new_host = %new_host.name,
        "VXLAN expanded to new host"
    );

    Ok(())
}

/// Get the count of hosts participating in a VXLAN network.
#[allow(dead_code)]
pub async fn get_network_host_count(st: &AppState, network_id: Uuid) -> i64 {
    let network_repo = NetworkRepository::new(st.db.clone());
    network_repo
        .count_network_hosts(network_id)
        .await
        .unwrap_or(0)
}

/// Check if a host already participates in a VXLAN network.
pub async fn network_host_exists(st: &AppState, network_id: Uuid, host_id: Uuid) -> bool {
    let network_repo = NetworkRepository::new(st.db.clone());
    network_repo
        .get_network_host(network_id, host_id)
        .await
        .ok()
        .flatten()
        .is_some()
}

/// Parse IP address from a host addr like "http://10.0.0.5:9090".
fn parse_host_ip(addr: &str) -> Result<String> {
    let addr = addr
        .trim_start_matches("http://")
        .trim_start_matches("https://");
    // addr is now like "10.0.0.5:9090" or "10.0.0.5"
    let ip = addr.split(':').next().unwrap_or(addr);
    if ip.is_empty() {
        return Err(anyhow!("could not parse IP from host addr"));
    }
    Ok(ip.to_string())
}

/// Derive gateway IP (first usable) from CIDR
fn derive_gateway(cidr: &str) -> Result<String> {
    // Parse "10.0.2.0/24" → "10.0.2.1"
    let parts: Vec<&str> = cidr.split('/').collect();
    if parts.len() != 2 {
        return Err(anyhow!("invalid CIDR: {}", cidr));
    }
    let octets: Vec<&str> = parts[0].split('.').collect();
    if octets.len() != 4 {
        return Err(anyhow!("invalid CIDR: {}", cidr));
    }
    Ok(format!("{}.{}.{}.1", octets[0], octets[1], octets[2]))
}

/// Derive DHCP range from CIDR
fn derive_dhcp_range(cidr: &str) -> Result<(String, String)> {
    let parts: Vec<&str> = cidr.split('/').collect();
    let octets: Vec<&str> = parts[0].split('.').collect();
    if octets.len() != 4 {
        return Err(anyhow!("invalid CIDR: {}", cidr));
    }
    let start = format!("{}.{}.{}.10", octets[0], octets[1], octets[2]);
    let end = format!("{}.{}.{}.250", octets[0], octets[1], octets[2]);
    Ok((start, end))
}
