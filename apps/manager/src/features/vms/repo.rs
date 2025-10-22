use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone, Serialize, sqlx::FromRow)]
pub struct VmRow {
    pub id: Uuid,
    pub name: String,
    pub state: String,
    pub host_id: Uuid,
    pub template_id: Option<Uuid>,
    pub host_addr: String,
    pub api_sock: String,
    pub tap: String,
    pub log_path: String,
    pub http_port: i32,
    pub fc_unit: String,
    pub vcpu: i32,
    pub mem_mib: i32,
    pub kernel_path: String,
    pub rootfs_path: String,
    pub source_snapshot_id: Option<Uuid>,
    pub guest_ip: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[cfg(not(test))]
pub async fn insert(db: &PgPool, row: &VmRow) -> sqlx::Result<()> {
    sqlx::query(
        r#"INSERT INTO vm (id,name,state,host_id,template_id,api_sock,tap,log_path,http_port,fc_unit,vcpu,mem_mib,kernel_path,rootfs_path,source_snapshot_id)
           VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)"#,
    )
    .bind(row.id)
    .bind(&row.name)
    .bind(&row.state)
    .bind(row.host_id)
    .bind(row.template_id)
    .bind(&row.api_sock)
    .bind(&row.tap)
    .bind(&row.log_path)
    .bind(row.http_port)
    .bind(&row.fc_unit)
    .bind(row.vcpu)
    .bind(row.mem_mib)
    .bind(&row.kernel_path)
    .bind(&row.rootfs_path)
    .bind(row.source_snapshot_id)
    .execute(db)
    .await?;
    Ok(())
}

#[cfg(test)]
pub async fn insert(_: &PgPool, row: &VmRow) -> sqlx::Result<()> {
    store().lock().unwrap().insert(row.id, row.clone());
    Ok(())
}

#[cfg(not(test))]
pub async fn list(db: &PgPool) -> sqlx::Result<Vec<VmRow>> {
    sqlx::query_as::<_, VmRow>(
        r#"
        SELECT vm.id,
               vm.name,
               vm.state,
               vm.host_id,
               vm.template_id,
               host.addr AS host_addr,
               vm.api_sock,
               vm.tap,
               vm.log_path,
               vm.http_port,
               vm.fc_unit,
               vm.vcpu,
               vm.mem_mib,
               vm.kernel_path,
               vm.rootfs_path,
               vm.source_snapshot_id,
               vm.guest_ip,
               vm.created_at,
               vm.updated_at
        FROM vm
        JOIN host ON host.id = vm.host_id
        ORDER BY vm.created_at DESC
        "#,
    )
    .fetch_all(db)
    .await
}

#[cfg(test)]
pub async fn list(_: &PgPool) -> sqlx::Result<Vec<VmRow>> {
    let mut rows: Vec<VmRow> = store().lock().unwrap().values().cloned().collect();
    rows.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(rows)
}

#[cfg(not(test))]
pub async fn list_by_host(db: &PgPool, host_id: Uuid) -> sqlx::Result<Vec<VmRow>> {
    sqlx::query_as::<_, VmRow>(
        r#"
        SELECT vm.id,
               vm.name,
               vm.state,
               vm.host_id,
               vm.template_id,
               host.addr AS host_addr,
               vm.api_sock,
               vm.tap,
               vm.log_path,
               vm.http_port,
               vm.fc_unit,
               vm.vcpu,
               vm.mem_mib,
               vm.kernel_path,
               vm.rootfs_path,
               vm.source_snapshot_id,
               vm.guest_ip,
               vm.created_at,
               vm.updated_at
        FROM vm
        JOIN host ON host.id = vm.host_id
        WHERE vm.host_id = $1
        ORDER BY vm.created_at DESC
        "#,
    )
    .bind(host_id)
    .fetch_all(db)
    .await
}

#[cfg(test)]
pub async fn list_by_host(_: &PgPool, host_id: Uuid) -> sqlx::Result<Vec<VmRow>> {
    let mut rows: Vec<VmRow> = store()
        .lock()
        .unwrap()
        .values()
        .filter(|row| row.host_id == host_id)
        .cloned()
        .collect();
    rows.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(rows)
}

#[cfg(not(test))]
pub async fn get(db: &PgPool, id: Uuid) -> sqlx::Result<VmRow> {
    sqlx::query_as::<_, VmRow>(
        r#"
        SELECT vm.id,
               vm.name,
               vm.state,
               vm.host_id,
               vm.template_id,
               host.addr AS host_addr,
               vm.api_sock,
               vm.tap,
               vm.log_path,
               vm.http_port,
               vm.fc_unit,
               vm.vcpu,
               vm.mem_mib,
               vm.kernel_path,
               vm.rootfs_path,
               vm.source_snapshot_id,
                vm.created_at,
                vm.updated_at
        FROM vm
        JOIN host ON host.id = vm.host_id
        WHERE vm.id=$1
        "#,
    )
    .bind(id)
    .fetch_one(db)
    .await
}

#[cfg(test)]
pub async fn get(_: &PgPool, id: Uuid) -> sqlx::Result<VmRow> {
    store()
        .lock()
        .unwrap()
        .get(&id)
        .cloned()
        .ok_or(sqlx::Error::RowNotFound)
}

#[cfg(not(test))]
pub async fn update_state(db: &PgPool, id: Uuid, state: &str) -> sqlx::Result<()> {
    sqlx::query(r#"UPDATE vm SET state=$2, updated_at=now() WHERE id=$1"#)
        .bind(id)
        .bind(state)
        .execute(db)
        .await?;
    Ok(())
}

#[cfg(test)]
pub async fn update_state(_: &PgPool, id: Uuid, state: &str) -> sqlx::Result<()> {
    let mut guard = store().lock().unwrap();
    let row = guard.get_mut(&id).ok_or(sqlx::Error::RowNotFound)?;
    row.state = state.to_string();
    row.updated_at = chrono::Utc::now();
    Ok(())
}

#[cfg(not(test))]
pub async fn delete_row(db: &PgPool, id: Uuid) -> sqlx::Result<()> {
    sqlx::query(r#"DELETE FROM vm WHERE id=$1"#)
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

#[cfg(test)]
pub async fn delete_row(_: &PgPool, id: Uuid) -> sqlx::Result<()> {
    store().lock().unwrap().remove(&id);
    Ok(())
}

#[cfg(not(test))]
pub async fn insert_event(
    db: &PgPool,
    vm_id: Uuid,
    level: &str,
    message: &str,
) -> sqlx::Result<()> {
    sqlx::query(r#"INSERT INTO vm_event (vm_id, level, message) VALUES ($1,$2,$3)"#)
        .bind(vm_id)
        .bind(level)
        .bind(message)
        .execute(db)
        .await?;
    Ok(())
}

#[cfg(test)]
pub async fn insert_event(_: &PgPool, vm_id: Uuid, level: &str, message: &str) -> sqlx::Result<()> {
    events_store().lock().unwrap().push(TestVmEvent {
        vm_id,
        level: level.to_string(),
        message: message.to_string(),
    });
    Ok(())
}

pub async fn update_guest_ip(db: &PgPool, vm_id: Uuid, guest_ip: Option<&str>) -> sqlx::Result<()> {
    sqlx::query("UPDATE vm SET guest_ip = $1, updated_at = NOW() WHERE id = $2")
        .bind(guest_ip)
        .bind(vm_id)
        .execute(db)
        .await?;
    Ok(())
}

#[derive(Clone, Serialize, sqlx::FromRow)]
pub struct VmDrive {
    pub id: Uuid,
    pub vm_id: Uuid,
    pub drive_id: String,
    pub path_on_host: String,
    pub size_bytes: Option<i64>,
    pub is_root_device: bool,
    pub is_read_only: bool,
    pub cache_type: Option<String>,
    pub io_engine: Option<String>,
    pub rate_limiter: Option<serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Serialize, sqlx::FromRow)]
pub struct VmNic {
    pub id: Uuid,
    pub vm_id: Uuid,
    pub iface_id: String,
    pub host_dev_name: String,
    pub guest_mac: Option<String>,
    pub rx_rate_limiter: Option<serde_json::Value>,
    pub tx_rate_limiter: Option<serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<VmDrive> for nexus_types::VmDrive {
    fn from(row: VmDrive) -> Self {
        nexus_types::VmDrive {
            id: row.id,
            vm_id: row.vm_id,
            drive_id: row.drive_id,
            path_on_host: row.path_on_host,
            size_bytes: row.size_bytes,
            is_root_device: row.is_root_device,
            is_read_only: row.is_read_only,
            cache_type: row.cache_type,
            io_engine: row.io_engine,
            rate_limiter: row.rate_limiter,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<VmNic> for nexus_types::VmNic {
    fn from(row: VmNic) -> Self {
        nexus_types::VmNic {
            id: row.id,
            vm_id: row.vm_id,
            iface_id: row.iface_id,
            host_dev_name: row.host_dev_name,
            guest_mac: row.guest_mac,
            rx_rate_limiter: row.rx_rate_limiter,
            tx_rate_limiter: row.tx_rate_limiter,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use std::sync::{Mutex, OnceLock};

#[cfg(test)]
fn store() -> &'static Mutex<HashMap<Uuid, VmRow>> {
    static STORE: OnceLock<Mutex<HashMap<Uuid, VmRow>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(test)]
fn drive_store() -> &'static Mutex<HashMap<Uuid, VmDrive>> {
    static STORE: OnceLock<Mutex<HashMap<Uuid, VmDrive>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(test)]
fn nic_store() -> &'static Mutex<HashMap<Uuid, VmNic>> {
    static STORE: OnceLock<Mutex<HashMap<Uuid, VmNic>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub mod drives {
    #[cfg(test)]
    use super::drive_store;
    use super::{PgPool, Uuid, VmDrive};

    #[allow(unused_variables)]
    pub async fn list(db: &PgPool, vm_id: Uuid) -> sqlx::Result<Vec<VmDrive>> {
        #[cfg(not(test))]
        {
            sqlx::query_as::<_, VmDrive>(
                r#"
                SELECT *
                FROM vm_drive
                WHERE vm_id = $1
                ORDER BY created_at
                "#,
            )
            .bind(vm_id)
            .fetch_all(db)
            .await
        }
        #[cfg(test)]
        {
            let store = drive_store().lock().unwrap();
            Ok(store
                .values()
                .filter(|d| d.vm_id == vm_id)
                .cloned()
                .collect())
        }
    }

    #[allow(unused_variables)]
    pub async fn get(db: &PgPool, id: Uuid) -> sqlx::Result<VmDrive> {
        #[cfg(not(test))]
        {
            sqlx::query_as::<_, VmDrive>(
                r#"
                SELECT *
                FROM vm_drive
                WHERE id = $1
                "#,
            )
            .bind(id)
            .fetch_one(db)
            .await
        }
        #[cfg(test)]
        {
            drive_store()
                .lock()
                .unwrap()
                .get(&id)
                .cloned()
                .ok_or(sqlx::Error::RowNotFound)
        }
    }

    #[allow(unused_variables)]
    pub async fn insert(
        db: &PgPool,
        vm_id: Uuid,
        drive_id: &str,
        path_on_host: &str,
        size_bytes: Option<i64>,
        is_root_device: bool,
        is_read_only: bool,
        cache_type: Option<&str>,
        io_engine: Option<&str>,
        rate_limiter: Option<&serde_json::Value>,
    ) -> sqlx::Result<VmDrive> {
        #[cfg(not(test))]
        {
            sqlx::query_as::<_, VmDrive>(
                r#"
                INSERT INTO vm_drive
                    (id, vm_id, drive_id, path_on_host, size_bytes, is_root_device, is_read_only, cache_type, io_engine, rate_limiter)
                VALUES
                    ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                RETURNING *
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(vm_id)
            .bind(drive_id)
            .bind(path_on_host)
            .bind(size_bytes)
            .bind(is_root_device)
            .bind(is_read_only)
            .bind(cache_type)
            .bind(io_engine)
            .bind(rate_limiter)
            .fetch_one(db)
            .await
        }
        #[cfg(test)]
        {
            let now = chrono::Utc::now();
            let drive = VmDrive {
                id: Uuid::new_v4(),
                vm_id,
                drive_id: drive_id.to_string(),
                path_on_host: path_on_host.to_string(),
                size_bytes,
                is_root_device,
                is_read_only,
                cache_type: cache_type.map(|s| s.to_string()),
                io_engine: io_engine.map(|s| s.to_string()),
                rate_limiter: rate_limiter.cloned(),
                created_at: now,
                updated_at: now,
            };
            drive_store()
                .lock()
                .unwrap()
                .insert(drive.id, drive.clone());
            Ok(drive)
        }
    }

    #[allow(unused_variables)]
    pub async fn update(
        db: &PgPool,
        id: Uuid,
        path_on_host: &str,
        rate_limiter: Option<&serde_json::Value>,
    ) -> sqlx::Result<VmDrive> {
        #[cfg(not(test))]
        {
            sqlx::query_as::<_, VmDrive>(
                r#"
                UPDATE vm_drive
                SET path_on_host = $2,
                    rate_limiter = $3,
                    updated_at = now()
                WHERE id = $1
                RETURNING *
                "#,
            )
            .bind(id)
            .bind(path_on_host)
            .bind(rate_limiter)
            .fetch_one(db)
            .await
        }
        #[cfg(test)]
        {
            let mut store = drive_store().lock().unwrap();
            let entry = store.get_mut(&id).ok_or(sqlx::Error::RowNotFound)?;
            entry.path_on_host = path_on_host.to_string();
            entry.rate_limiter = rate_limiter.cloned();
            entry.updated_at = chrono::Utc::now();
            Ok(entry.clone())
        }
    }

    #[allow(unused_variables)]
    pub async fn delete(db: &PgPool, id: Uuid) -> sqlx::Result<()> {
        #[cfg(not(test))]
        {
            sqlx::query(
                r#"
                DELETE FROM vm_drive
                WHERE id = $1
                "#,
            )
            .bind(id)
            .execute(db)
            .await?
            .rows_affected();
            Ok(())
        }
        #[cfg(test)]
        {
            drive_store().lock().unwrap().remove(&id);
            Ok(())
        }
    }
}

pub mod nics {
    #[cfg(test)]
    use super::nic_store;
    use super::{PgPool, Uuid, VmNic};

    #[allow(unused_variables)]
    pub async fn list(db: &PgPool, vm_id: Uuid) -> sqlx::Result<Vec<VmNic>> {
        #[cfg(not(test))]
        {
            sqlx::query_as::<_, VmNic>(
                r#"
                SELECT *
                FROM vm_network_interface
                WHERE vm_id = $1
                ORDER BY created_at
                "#,
            )
            .bind(vm_id)
            .fetch_all(db)
            .await
        }
        #[cfg(test)]
        {
            let store = nic_store().lock().unwrap();
            Ok(store
                .values()
                .filter(|n| n.vm_id == vm_id)
                .cloned()
                .collect())
        }
    }

    #[allow(unused_variables)]
    pub async fn get(db: &PgPool, id: Uuid) -> sqlx::Result<VmNic> {
        #[cfg(not(test))]
        {
            sqlx::query_as::<_, VmNic>(
                r#"
                SELECT *
                FROM vm_network_interface
                WHERE id = $1
                "#,
            )
            .bind(id)
            .fetch_one(db)
            .await
        }
        #[cfg(test)]
        {
            nic_store()
                .lock()
                .unwrap()
                .get(&id)
                .cloned()
                .ok_or(sqlx::Error::RowNotFound)
        }
    }

    #[allow(unused_variables)]
    pub async fn insert(
        db: &PgPool,
        vm_id: Uuid,
        iface_id: &str,
        host_dev_name: &str,
        guest_mac: Option<&str>,
        rx_rate_limiter: Option<&serde_json::Value>,
        tx_rate_limiter: Option<&serde_json::Value>,
    ) -> sqlx::Result<VmNic> {
        #[cfg(not(test))]
        {
            sqlx::query_as::<_, VmNic>(
                r#"
                INSERT INTO vm_network_interface
                    (id, vm_id, iface_id, host_dev_name, guest_mac, rx_rate_limiter, tx_rate_limiter)
                VALUES
                    ($1, $2, $3, $4, $5, $6, $7)
                RETURNING *
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(vm_id)
            .bind(iface_id)
            .bind(host_dev_name)
            .bind(guest_mac)
            .bind(rx_rate_limiter)
            .bind(tx_rate_limiter)
            .fetch_one(db)
            .await
        }
        #[cfg(test)]
        {
            let now = chrono::Utc::now();
            let nic = VmNic {
                id: Uuid::new_v4(),
                vm_id,
                iface_id: iface_id.to_string(),
                host_dev_name: host_dev_name.to_string(),
                guest_mac: guest_mac.map(|s| s.to_string()),
                rx_rate_limiter: rx_rate_limiter.cloned(),
                tx_rate_limiter: tx_rate_limiter.cloned(),
                created_at: now,
                updated_at: now,
            };
            nic_store().lock().unwrap().insert(nic.id, nic.clone());
            Ok(nic)
        }
    }

    #[allow(unused_variables)]
    pub async fn update_rate_limiters(
        db: &PgPool,
        id: Uuid,
        rx: Option<&serde_json::Value>,
        tx: Option<&serde_json::Value>,
    ) -> sqlx::Result<VmNic> {
        #[cfg(not(test))]
        {
            sqlx::query_as::<_, VmNic>(
                r#"
                UPDATE vm_network_interface
                SET rx_rate_limiter = $2,
                    tx_rate_limiter = $3,
                    updated_at = now()
                WHERE id = $1
                RETURNING *
                "#,
            )
            .bind(id)
            .bind(rx)
            .bind(tx)
            .fetch_one(db)
            .await
        }
        #[cfg(test)]
        {
            let mut store = nic_store().lock().unwrap();
            let entry = store.get_mut(&id).ok_or(sqlx::Error::RowNotFound)?;
            entry.rx_rate_limiter = rx.cloned();
            entry.tx_rate_limiter = tx.cloned();
            entry.updated_at = chrono::Utc::now();
            Ok(entry.clone())
        }
    }

    #[allow(unused_variables)]
    pub async fn delete(db: &PgPool, id: Uuid) -> sqlx::Result<()> {
        #[cfg(not(test))]
        {
            sqlx::query(
                r#"
                DELETE FROM vm_network_interface
                WHERE id = $1
                "#,
            )
            .bind(id)
            .execute(db)
            .await?
            .rows_affected();
            Ok(())
        }
        #[cfg(test)]
        {
            nic_store().lock().unwrap().remove(&id);
            Ok(())
        }
    }
}

#[cfg(test)]
#[allow(dead_code)]
pub fn reset_store() {
    store().lock().unwrap().clear();
    events_store().lock().unwrap().clear();
}

#[cfg(test)]
#[derive(Clone)]
#[allow(dead_code)]
pub struct TestVmEvent {
    pub vm_id: Uuid,
    pub level: String,
    pub message: String,
}

#[cfg(test)]
fn events_store() -> &'static Mutex<Vec<TestVmEvent>> {
    static STORE: OnceLock<Mutex<Vec<TestVmEvent>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(Vec::new()))
}

#[cfg(test)]
#[allow(dead_code)]
pub fn event_store_snapshot() -> Vec<TestVmEvent> {
    events_store().lock().unwrap().clone()
}
