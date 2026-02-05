use crate::AppState;
use anyhow::Result;
use tracing::{info, warn};
use uuid::Uuid;

/// Apply all port forwards for a VM (call after guest IP is known)
pub async fn apply_forwards(st: &AppState, vm_id: Uuid) -> Result<()> {
    let vm = super::super::repo::get(&st.db, vm_id).await?;
    let guest_ip = match vm.guest_ip.as_deref() {
        Some(ip) if !ip.is_empty() => ip.to_string(),
        _ => return Ok(()), // No guest IP yet, nothing to apply
    };

    let forwards = super::repo::list(&st.db, vm_id).await?;
    if forwards.is_empty() {
        return Ok(());
    }

    info!(vm_id=%vm_id, count=%forwards.len(), "applying port forwards");

    for fwd in &forwards {
        let resp = reqwest::Client::new()
            .post(format!(
                "{}/agent/v1/vms/{}/port-forward",
                vm.host_addr, vm.id
            ))
            .json(&serde_json::json!({
                "guest_ip": guest_ip,
                "host_port": fwd.host_port as u16,
                "guest_port": fwd.guest_port as u16,
                "protocol": fwd.protocol,
            }))
            .send()
            .await;

        match resp {
            Ok(r) if r.status().is_success() => {
                info!(vm_id=%vm_id, host_port=%fwd.host_port, guest_port=%fwd.guest_port, "port forward applied");
            }
            Ok(r) => {
                warn!(vm_id=%vm_id, host_port=%fwd.host_port, status=%r.status(), "failed to apply port forward");
            }
            Err(e) => {
                warn!(vm_id=%vm_id, host_port=%fwd.host_port, error=?e, "failed to apply port forward");
            }
        }
    }

    Ok(())
}

/// Remove all port forwards for a VM (call before stop)
pub async fn cleanup_forwards(st: &AppState, vm_id: Uuid) -> Result<()> {
    let vm = super::super::repo::get(&st.db, vm_id).await?;
    let guest_ip = match vm.guest_ip.as_deref() {
        Some(ip) if !ip.is_empty() => ip.to_string(),
        _ => return Ok(()), // No guest IP, rules were never applied
    };

    let forwards = super::repo::list(&st.db, vm_id).await?;
    if forwards.is_empty() {
        return Ok(());
    }

    info!(vm_id=%vm_id, count=%forwards.len(), "cleaning up port forwards");

    for fwd in &forwards {
        let resp = reqwest::Client::new()
            .delete(format!(
                "{}/agent/v1/vms/{}/port-forward",
                vm.host_addr, vm.id
            ))
            .json(&serde_json::json!({
                "guest_ip": guest_ip,
                "host_port": fwd.host_port as u16,
                "guest_port": fwd.guest_port as u16,
                "protocol": fwd.protocol,
            }))
            .send()
            .await;

        match resp {
            Ok(r) if r.status().is_success() => {
                info!(vm_id=%vm_id, host_port=%fwd.host_port, "port forward removed");
            }
            Ok(r) => {
                warn!(vm_id=%vm_id, host_port=%fwd.host_port, status=%r.status(), "failed to remove port forward");
            }
            Err(e) => {
                warn!(vm_id=%vm_id, host_port=%fwd.host_port, error=?e, "failed to remove port forward");
            }
        }
    }

    Ok(())
}
