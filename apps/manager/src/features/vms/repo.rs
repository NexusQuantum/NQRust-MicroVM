use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone, Serialize, sqlx::FromRow)]
pub struct VmRow {
    pub id: Uuid,
    pub name: String,
    pub state: String,
    pub host_addr: String,
    pub api_sock: String,
    pub tap: String,
    pub log_path: String,
    pub http_port: i32,
    pub fc_unit: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[cfg(not(test))]
pub async fn insert(db: &PgPool, row: &VmRow) -> sqlx::Result<()> {
    sqlx::query(
        r#"INSERT INTO vm (id,name,state,host_addr,api_sock,tap,log_path,http_port,fc_unit)
           VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)"#,
    )
    .bind(row.id)
    .bind(&row.name)
    .bind(&row.state)
    .bind(&row.host_addr)
    .bind(&row.api_sock)
    .bind(&row.tap)
    .bind(&row.log_path)
    .bind(row.http_port)
    .bind(&row.fc_unit)
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
    sqlx::query_as::<_, VmRow>(r#"SELECT * FROM vm ORDER BY created_at DESC"#)
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
pub async fn get(db: &PgPool, id: Uuid) -> sqlx::Result<VmRow> {
    sqlx::query_as::<_, VmRow>(r#"SELECT * FROM vm WHERE id=$1"#)
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
pub fn reset_store() {
    store().lock().unwrap().clear();
}
