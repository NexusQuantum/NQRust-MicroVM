use crate::AppState;
use anyhow::*;
use nexus_types::CreateVmReq;
use reqwest::Client;
use serde_json::json;
use uuid::Uuid;

pub async fn create_and_start(st: &AppState, id: Uuid, req: CreateVmReq) -> Result<()> {
    let host = st
        .hosts
        .first_healthy()
        .await
        .context("no healthy hosts available")?;
    let paths = VmPaths::new(id);

    create_tap(&host.addr, id).await?;
    spawn_firecracker(&host.addr, id, &paths).await?;
    configure_vm(&host.addr, id, &req, &paths).await?;
    start_vm(&host.addr, id, &paths).await?;

    super::repo::insert(
        &st.db,
        &super::repo::VmRow {
            id,
            name: req.name,
            state: "running".into(),
            host_id: host.id,
            host_addr: host.addr.clone(),
            api_sock: paths.sock.clone(),
            tap: paths.tap.clone(),
            log_path: paths.log_path.clone(),
            http_port: 0,
            fc_unit: paths.fc_unit.clone(),
            vcpu: req.vcpu as i32,
            mem_mib: req.mem_mib as i32,
            kernel_path: req.kernel_path,
            rootfs_path: req.rootfs_path,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        },
    )
    .await?;
    Ok(())
}

pub async fn restart_vm(st: &AppState, vm: &super::repo::VmRow) -> Result<()> {
    let host = st.hosts.get(vm.host_id).await?;
    let paths = VmPaths::from_row(vm);
    let req = CreateVmReq {
        name: vm.name.clone(),
        vcpu: vm.vcpu.try_into().context("stored vcpu exceeds u8")?,
        mem_mib: vm.mem_mib.try_into().context("stored mem_mib negative")?,
        kernel_path: vm.kernel_path.clone(),
        rootfs_path: vm.rootfs_path.clone(),
    };

    create_tap(&host.addr, vm.id).await?;
    spawn_firecracker(&host.addr, vm.id, &paths).await?;
    configure_vm(&host.addr, vm.id, &req, &paths).await?;
    start_vm(&host.addr, vm.id, &paths).await?;
    super::repo::update_state(&st.db, vm.id, "running").await?;
    Ok(())
}

pub async fn stop_only(st: &AppState, id: Uuid) -> Result<()> {
    let vm = super::repo::get(&st.db, id).await?;
    super::repo::update_state(&st.db, id, "stopping").await?;

    let response = reqwest::Client::new()
        .post(format!("{}/agent/v1/vms/{}/stop", vm.host_addr, vm.id))
        .json(&serde_json::json!({
            "tap": vm.tap,
            "sock": vm.api_sock,
            "fc_unit": vm.fc_unit
        }))
        .send()
        .await?;

    response.error_for_status()?;
    super::repo::update_state(&st.db, id, "stopped").await?;
    Ok(())
}

pub async fn stop_and_delete(st: &AppState, id: Uuid) -> Result<()> {
    stop_only(st, id).await?;
    super::repo::delete_row(&st.db, id).await?;
    Ok(())
}

struct VmPaths {
    sock: String,
    log_path: String,
    metrics_path: String,
    tap: String,
    fc_unit: String,
}

impl VmPaths {
    fn new(id: Uuid) -> Self {
        Self {
            sock: format!("/srv/fc/vms/{id}/sock/fc.sock"),
            log_path: format!("/srv/fc/vms/{id}/logs/firecracker.log"),
            metrics_path: format!("/srv/fc/vms/{id}/logs/metrics.json"),
            tap: format!("tap-{id}"),
            fc_unit: format!("fc-{id}.scope"),
        }
    }

    fn from_row(vm: &super::repo::VmRow) -> Self {
        Self {
            sock: vm.api_sock.clone(),
            log_path: vm.log_path.clone(),
            metrics_path: format!("/srv/fc/vms/{}/logs/metrics.json", vm.id),
            tap: vm.tap.clone(),
            fc_unit: vm.fc_unit.clone(),
        }
    }
}

async fn create_tap(host_addr: &str, id: Uuid) -> Result<()> {
    reqwest::Client::new()
        .post(format!("{host_addr}/agent/v1/vms/{id}/tap"))
        .json(&json!({"bridge": "fcbr0", "owner_user": serde_json::Value::Null}))
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

async fn spawn_firecracker(host_addr: &str, id: Uuid, paths: &VmPaths) -> Result<()> {
    reqwest::Client::new()
        .post(format!("{host_addr}/agent/v1/vms/{id}/spawn"))
        .json(&json!({"sock": paths.sock, "log_path": paths.log_path}))
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

async fn configure_vm(host_addr: &str, id: Uuid, req: &CreateVmReq, paths: &VmPaths) -> Result<()> {
    let base = format!("{host_addr}/agent/v1/vms/{id}/proxy");
    let qs = format!("?sock={}", urlencoding::encode(&paths.sock));
    let http = Client::new();

    http.put(format!("{base}/machine-config{qs}"))
        .json(&json!({
            "vcpu_count": req.vcpu,
            "mem_size_mib": req.mem_mib,
            "smt": false
        }))
        .send()
        .await?
        .error_for_status()?;

    http.put(format!("{base}/boot-source{qs}"))
        .json(&json!({
            "kernel_image_path": req.kernel_path,
            "boot_args": "console=ttyS0 reboot=k panic=1 pci=off",
        }))
        .send()
        .await?
        .error_for_status()?;

    http.put(format!("{base}/drives/rootfs{qs}"))
        .json(&json!({
            "drive_id": "rootfs",
            "path_on_host": req.rootfs_path,
            "is_root_device": true,
            "is_read_only": false
        }))
        .send()
        .await?
        .error_for_status()?;

    http.put(format!("{base}/network-interfaces/eth0{qs}"))
        .json(&json!({
            "iface_id": "eth0",
            "host_dev_name": paths.tap
        }))
        .send()
        .await?
        .error_for_status()?;

    http.put(format!("{base}/logger{qs}"))
        .json(&json!({
            "log_path": paths.log_path,
            "level": "Info",
            "show_level": true,
            "show_log_origin": false
        }))
        .send()
        .await?
        .error_for_status()?;

    http.put(format!("{base}/metrics{qs}"))
        .json(&json!({
            "metrics_path": paths.metrics_path,
            "level": "Info"
        }))
        .send()
        .await?
        .error_for_status()?;

    Ok(())
}

async fn start_vm(host_addr: &str, id: Uuid, paths: &VmPaths) -> Result<()> {
    let base = format!("{host_addr}/agent/v1/vms/{id}/proxy");
    let qs = format!("?sock={}", urlencoding::encode(&paths.sock));
    Client::new()
        .put(format!("{base}/actions{qs}"))
        .json(&json!({"action_type": "InstanceStart"}))
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}
