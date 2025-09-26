use crate::AppState;
use anyhow::{anyhow, bail, Context, Result};
use nexus_types::CreateVmReq;
#[cfg(not(test))]
use reqwest::Client;
#[cfg(not(test))]
use serde_json::json;
use std::path::Path;
use uuid::Uuid;

pub async fn create_and_start(
    st: &AppState,
    id: Uuid,
    req: CreateVmReq,
    template_id: Option<Uuid>,
) -> Result<()> {
    let host = st
        .hosts
        .first_healthy()
        .await
        .context("no healthy hosts available")?;
    let paths = VmPaths::new(id);
    let spec = resolve_vm_spec(st, req).await?;

    create_tap(&host.addr, id).await?;
    spawn_firecracker(&host.addr, id, &paths).await?;
    configure_vm(&host.addr, id, &spec, &paths).await?;
    start_vm(&host.addr, id, &paths).await?;

    super::repo::insert(
        &st.db,
        &super::repo::VmRow {
            id,
            name: spec.name.clone(),
            state: "running".into(),
            host_id: host.id,
            template_id,
            host_addr: host.addr.clone(),
            api_sock: paths.sock.clone(),
            tap: paths.tap.clone(),
            log_path: paths.log_path.clone(),
            http_port: 0,
            fc_unit: paths.fc_unit.clone(),
            vcpu: spec.vcpu as i32,
            mem_mib: spec.mem_mib as i32,
            kernel_path: spec.kernel_path.clone(),
            rootfs_path: spec.rootfs_path.clone(),
            source_snapshot_id: None,
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
    ensure_allowed_path(st, &vm.kernel_path)?;
    ensure_allowed_path(st, &vm.rootfs_path)?;
    let spec = ResolvedVmSpec {
        name: vm.name.clone(),
        vcpu: vm.vcpu.try_into().context("stored vcpu exceeds u8")?,
        mem_mib: vm.mem_mib.try_into().context("stored mem_mib negative")?,
        kernel_path: vm.kernel_path.clone(),
        rootfs_path: vm.rootfs_path.clone(),
    };

    create_tap(&host.addr, vm.id).await?;
    spawn_firecracker(&host.addr, vm.id, &paths).await?;
    configure_vm(&host.addr, vm.id, &spec, &paths).await?;
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
    if let Err(err) = stop_only(st, id).await {
        tracing::warn!(vm_id = %id, error = ?err, "failed to stop vm before deletion");
    }
    super::repo::delete_row(&st.db, id).await?;
    Ok(())
}

#[cfg_attr(test, allow(dead_code))]
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

#[derive(Clone)]
struct ResolvedVmSpec {
    name: String,
    vcpu: u8,
    mem_mib: u32,
    kernel_path: String,
    rootfs_path: String,
}

async fn resolve_vm_spec(st: &AppState, req: CreateVmReq) -> Result<ResolvedVmSpec> {
    let kernel_path =
        resolve_image_path(st, req.kernel_image_id, req.kernel_path, "kernel").await?;
    let rootfs_path =
        resolve_image_path(st, req.rootfs_image_id, req.rootfs_path, "rootfs").await?;

    Ok(ResolvedVmSpec {
        name: req.name,
        vcpu: req.vcpu,
        mem_mib: req.mem_mib,
        kernel_path,
        rootfs_path,
    })
}

async fn resolve_image_path(
    st: &AppState,
    image_id: Option<Uuid>,
    direct_path: Option<String>,
    field: &str,
) -> Result<String> {
    if let Some(id) = image_id {
        let image = st
            .images
            .get(id)
            .await
            .with_context(|| format!("failed to load {field} image {id}"))?;
        ensure_allowed_path(st, &image.host_path)?;
        return Ok(image.host_path);
    }

    if let Some(path) = direct_path {
        if !st.allow_direct_image_paths {
            bail!("{field} path not permitted in production mode");
        }
        ensure_allowed_path(st, &path)?;
        return Ok(path);
    }

    Err(anyhow!("{field} requires an image id or host path"))
}

fn ensure_allowed_path(st: &AppState, path: &str) -> Result<()> {
    let candidate = Path::new(path);
    if !st.images.is_path_allowed(candidate) {
        bail!("path {path} is not within the configured image root");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::hosts::repo::HostRepository;
    use crate::features::vms::repo;
    use nexus_types::CreateImageReq;
    use serde_json::json;

    #[sqlx::test(migrations = "./migrations")]
    async fn create_with_image_ids_resolves_paths(pool: sqlx::PgPool) {
        repo::reset_store();
        let hosts = HostRepository::new(pool.clone());
        let host = hosts
            .register("host", "http://127.0.0.1:1", json!({}))
            .await
            .unwrap();
        let images =
            crate::features::images::repo::ImageRepository::new(pool.clone(), "/srv/images");
        let kernel = images
            .insert(&CreateImageReq {
                kind: "kernel".into(),
                name: "vmlinux".into(),
                host_path: "/srv/images/vmlinux".into(),
                sha256: "abc".into(),
                size: 10,
                project: None,
            })
            .await
            .unwrap();
        let rootfs = images
            .insert(&CreateImageReq {
                kind: "rootfs".into(),
                name: "disk".into(),
                host_path: "/srv/images/rootfs".into(),
                sha256: "def".into(),
                size: 20,
                project: None,
            })
            .await
            .unwrap();

        let snapshots = crate::features::snapshots::repo::SnapshotRepository::new(pool.clone());
        let state = crate::AppState {
            db: pool.clone(),
            hosts: hosts.clone(),
            images: images.clone(),
            snapshots,
            allow_direct_image_paths: false,
        };

        let vm_id = Uuid::new_v4();
        create_and_start(
            &state,
            vm_id,
            CreateVmReq {
                name: "vm".into(),
                vcpu: 1,
                mem_mib: 512,
                kernel_image_id: Some(kernel.id),
                rootfs_image_id: Some(rootfs.id),
                kernel_path: None,
                rootfs_path: None,
            },
            None,
        )
        .await
        .unwrap();

        let stored = repo::get(&state.db, vm_id).await.unwrap();
        assert_eq!(stored.kernel_path, "/srv/images/vmlinux");
        assert_eq!(stored.rootfs_path, "/srv/images/rootfs");
        assert_eq!(stored.host_id, host.id);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn reject_direct_paths_in_prod(pool: sqlx::PgPool) {
        repo::reset_store();
        let hosts = HostRepository::new(pool.clone());
        hosts
            .register("host", "http://127.0.0.1:1", json!({}))
            .await
            .unwrap();
        let images =
            crate::features::images::repo::ImageRepository::new(pool.clone(), "/srv/images");
        let snapshots = crate::features::snapshots::repo::SnapshotRepository::new(pool.clone());
        let state = crate::AppState {
            db: pool,
            hosts,
            images,
            snapshots,
            allow_direct_image_paths: false,
        };

        let err = create_and_start(
            &state,
            Uuid::new_v4(),
            CreateVmReq {
                name: "vm".into(),
                vcpu: 1,
                mem_mib: 512,
                kernel_image_id: None,
                rootfs_image_id: None,
                kernel_path: Some("/srv/images/vmlinux".into()),
                rootfs_path: Some("/srv/images/rootfs".into()),
            },
            None,
        )
        .await
        .unwrap_err();

        assert!(err.to_string().contains("path not permitted"));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn restart_rejects_paths_outside_root(pool: sqlx::PgPool) {
        repo::reset_store();
        let hosts = HostRepository::new(pool.clone());
        let host = hosts
            .register("host", "http://127.0.0.1:1", json!({}))
            .await
            .unwrap();
        let images =
            crate::features::images::repo::ImageRepository::new(pool.clone(), "/srv/images");
        let snapshots = crate::features::snapshots::repo::SnapshotRepository::new(pool.clone());
        let state = crate::AppState {
            db: pool,
            hosts,
            images,
            snapshots,
            allow_direct_image_paths: false,
        };

        let vm = repo::VmRow {
            id: Uuid::new_v4(),
            name: "vm".into(),
            state: "stopped".into(),
            host_id: host.id,
            template_id: None,
            host_addr: host.addr,
            api_sock: "/tmp/sock".into(),
            tap: "tap0".into(),
            log_path: "/tmp/log".into(),
            http_port: 0,
            fc_unit: "fc.scope".into(),
            vcpu: 1,
            mem_mib: 512,
            kernel_path: "/etc/passwd".into(),
            rootfs_path: "/srv/images/rootfs".into(),
            source_snapshot_id: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let err = restart_vm(&state, &vm).await.unwrap_err();
        assert!(err
            .to_string()
            .contains("not within the configured image root"));
    }
}

#[cfg(not(test))]
async fn create_tap(host_addr: &str, id: Uuid) -> Result<()> {
    reqwest::Client::new()
        .post(format!("{host_addr}/agent/v1/vms/{id}/tap"))
        .json(&json!({"bridge": "fcbr0", "owner_user": serde_json::Value::Null}))
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

#[cfg(test)]
async fn create_tap(_: &str, _: Uuid) -> Result<()> {
    Ok(())
}

#[cfg(not(test))]
async fn spawn_firecracker(host_addr: &str, id: Uuid, paths: &VmPaths) -> Result<()> {
    reqwest::Client::new()
        .post(format!("{host_addr}/agent/v1/vms/{id}/spawn"))
        .json(&json!({"sock": paths.sock, "log_path": paths.log_path}))
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

#[cfg(test)]
async fn spawn_firecracker(_: &str, _: Uuid, _: &VmPaths) -> Result<()> {
    Ok(())
}

#[cfg(not(test))]
async fn configure_vm(
    host_addr: &str,
    id: Uuid,
    spec: &ResolvedVmSpec,
    paths: &VmPaths,
) -> Result<()> {
    let base = format!("{host_addr}/agent/v1/vms/{id}/proxy");
    let qs = format!("?sock={}", urlencoding::encode(&paths.sock));
    let http = Client::new();

    http.put(format!("{base}/machine-config{qs}"))
        .json(&json!({
            "vcpu_count": spec.vcpu,
            "mem_size_mib": spec.mem_mib,
            "smt": false
        }))
        .send()
        .await?
        .error_for_status()?;

    http.put(format!("{base}/boot-source{qs}"))
        .json(&json!({
            "kernel_image_path": spec.kernel_path,
            "boot_args": "console=ttyS0 reboot=k panic=1 pci=off",
        }))
        .send()
        .await?
        .error_for_status()?;

    http.put(format!("{base}/drives/rootfs{qs}"))
        .json(&json!({
            "drive_id": "rootfs",
            "path_on_host": spec.rootfs_path,
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

#[cfg(test)]
async fn configure_vm(_: &str, _: Uuid, _: &ResolvedVmSpec, _: &VmPaths) -> Result<()> {
    Ok(())
}

#[cfg(not(test))]
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

#[cfg(test)]
async fn start_vm(_: &str, _: Uuid, _: &VmPaths) -> Result<()> {
    Ok(())
}
