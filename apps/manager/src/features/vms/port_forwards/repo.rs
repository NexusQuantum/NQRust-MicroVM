use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone, Serialize, sqlx::FromRow)]
pub struct PortForwardRow {
    pub id: Uuid,
    pub vm_id: Uuid,
    pub host_port: i32,
    pub guest_port: i32,
    pub protocol: String,
    pub description: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<PortForwardRow> for nexus_types::PortForward {
    fn from(row: PortForwardRow) -> Self {
        nexus_types::PortForward {
            id: row.id,
            vm_id: row.vm_id,
            host_port: row.host_port,
            guest_port: row.guest_port,
            protocol: row.protocol,
            description: row.description,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

pub async fn list(db: &PgPool, vm_id: Uuid) -> sqlx::Result<Vec<PortForwardRow>> {
    sqlx::query_as::<_, PortForwardRow>(
        "SELECT * FROM port_forward WHERE vm_id = $1 ORDER BY created_at",
    )
    .bind(vm_id)
    .fetch_all(db)
    .await
}

pub async fn get(db: &PgPool, id: Uuid) -> sqlx::Result<PortForwardRow> {
    sqlx::query_as::<_, PortForwardRow>("SELECT * FROM port_forward WHERE id = $1")
        .bind(id)
        .fetch_one(db)
        .await
}

pub async fn insert(
    db: &PgPool,
    vm_id: Uuid,
    host_port: i32,
    guest_port: i32,
    protocol: &str,
    description: Option<&str>,
) -> sqlx::Result<PortForwardRow> {
    sqlx::query_as::<_, PortForwardRow>(
        r#"INSERT INTO port_forward (id, vm_id, host_port, guest_port, protocol, description)
           VALUES ($1, $2, $3, $4, $5, $6)
           RETURNING *"#,
    )
    .bind(Uuid::new_v4())
    .bind(vm_id)
    .bind(host_port)
    .bind(guest_port)
    .bind(protocol)
    .bind(description)
    .fetch_one(db)
    .await
}

pub async fn delete(db: &PgPool, id: Uuid) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM port_forward WHERE id = $1")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}
