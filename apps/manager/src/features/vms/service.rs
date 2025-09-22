use anyhow::*;
use reqwest::Client;
use uuid::Uuid;
use crate::AppState;
use nexus_types::CreateVmReq;

pub async fn create_and_start(st: &AppState, id: Uuid, req: CreateVmReq) -> Result<()> {
    let host = st.agent_base.clone();
    let vm_dir = format!("/srv/fc/vms/{id}");
    let sock = format!("{vm_dir}/sock/fc.sock");
    let log_path = format!("{vm_dir}/logs/firecracker.log");
    let tap = format!("tap-{id}");

    // 1) create tap on agent
    reqwest::Client::new()
        .post(format!("{host}/agent/v1/vms/{id}/tap"))
        .json(&serde_json::json!({"bridge": "fcbr0", "owner_user": serde_json::Value::Null}))
        .send()
        .await?
        .error_for_status()?;

    // 2) spawn firecracker
    reqwest::Client::new()
        .post(format!("{host}/agent/v1/vms/{id}/spawn"))
        .json(&serde_json::json!({"sock": sock, "log_path": log_path}))
        .send()
        .await?
        .error_for_status()?;

    // 3) configure via proxy
    let base = format!("{host}/agent/v1/vms/{id}/proxy");
    let qs = format!("?sock={}", urlencoding::encode(&sock));
    let http = Client::new();

    http
        .put(format!("{base}/machine-config{qs}"))
        .json(&serde_json::json!({
            "vcpu_count": req.vcpu,
            "mem_size_mib": req.mem_mib,
            "smt": false
        }))
        .send()
        .await?
        .error_for_status()?;

    http
        .put(format!("{base}/boot-source{qs}"))
        .json(&serde_json::json!({
            "kernel_image_path": req.kernel_path,
            "boot_args": "console=ttyS0 reboot=k panic=1 pci=off"
        }))
        .send()
        .await?
        .error_for_status()?;

    http
        .put(format!("{base}/drives/rootfs{qs}"))
        .json(&serde_json::json!({
            "drive_id": "rootfs",
            "path_on_host": req.rootfs_path,
            "is_root_device": true,
            "is_read_only": false
        }))
        .send()
        .await?
        .error_for_status()?;

    http
        .put(format!("{base}/network-interfaces/eth0{qs}"))
        .json(&serde_json::json!({
            "iface_id": "eth0",
            "host_dev_name": tap
        }))
        .send()
        .await?
        .error_for_status()?;

    // 4) Logger & Metrics
    http
        .put(format!("{base}/logger{qs}"))
        .json(&serde_json::json!({
            "log_path": log_path,
            "level": "Info",
            "show_level": true,
            "show_log_origin": false
        }))
        .send()
        .await?
        .error_for_status()?;
    http
        .put(format!("{base}/metrics{qs}"))
        .json(&serde_json::json!({
            "metrics_path": format!("{vm_dir}/logs/metrics.json"),
            "level": "Info"
        }))
        .send()
        .await?
        .error_for_status()?;

    // 5) Start
    http
        .put(format!("{base}/actions{qs}"))
        .json(&serde_json::json!({"action_type": "InstanceStart"}))
        .send()
        .await?
        .error_for_status()?;

    // 6) Persist row
    super::repo::insert(
        &st.db,
        &super::repo::VmRow {
            id,
            name: req.name,
            state: "running".into(),
            host_addr: host.clone(),
            api_sock: sock,
            tap: format!("tap-{id}"),
            log_path,
            http_port: 0,
            fc_unit: format!("fc-{id}.scope"),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        },
    )
    .await?;
    Ok(())
}

pub async fn stop_only(st: &AppState, id: Uuid) -> Result<()> {
    let vm = super::repo::get(&st.db, id).await?;
    reqwest::Client::new()
        .post(format!("{}/agent/v1/vms/{}/stop", vm.host_addr, vm.id))
        .json(&serde_json::json!({
            "tap": vm.tap,
            "sock": vm.api_sock,
            "fc_unit": vm.fc_unit
        }))
        .send()
        .await?
        .error_for_status()?;
    super::repo::update_state(&st.db, id, "stopping").await?;
    Ok(())
}

pub async fn stop_and_delete(st: &AppState, id: Uuid) -> Result<()> {
    let _ = stop_only(st, id).await; // best effort
    super::repo::delete_row(&st.db, id).await?;
    Ok(())
}