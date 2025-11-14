use chrono::DateTime;
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct NetworkRepository {
    pool: PgPool,
}

impl NetworkRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        &self,
        name: &str,
        description: Option<&str>,
        network_type: &str,
        vlan_id: Option<i32>,
        bridge_name: &str,
        host_id: Uuid,
        cidr: Option<&str>,
        gateway: Option<&str>,
    ) -> sqlx::Result<NetworkRow> {
        sqlx::query_as::<_, NetworkRow>(
            r#"
            INSERT INTO network (name, description, type, vlan_id, bridge_name, host_id, cidr, gateway, created_by_user_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(name)
        .bind(description)
        .bind(network_type)
        .bind(vlan_id)
        .bind(bridge_name)
        .bind(host_id)
        .bind(cidr)
        .bind(gateway)
        .bind(None as Option<Uuid>) // created_by_user_id - TODO: Set from authenticated user context
        .fetch_one(&self.pool)
        .await
    }

    pub async fn get(&self, id: Uuid) -> sqlx::Result<NetworkRow> {
        sqlx::query_as::<_, NetworkRow>(r#"SELECT * FROM network WHERE id = $1"#)
            .bind(id)
            .fetch_one(&self.pool)
            .await
    }

    pub async fn list(&self) -> sqlx::Result<Vec<NetworkRow>> {
        sqlx::query_as::<_, NetworkRow>(
            r#"
            SELECT * FROM network
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn list_by_host(&self, host_id: Uuid) -> sqlx::Result<Vec<NetworkRow>> {
        sqlx::query_as::<_, NetworkRow>(
            r#"
            SELECT * FROM network
            WHERE host_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(host_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn update(
        &self,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        cidr: Option<&str>,
        gateway: Option<&str>,
    ) -> sqlx::Result<NetworkRow> {
        // Build dynamic update query based on what's provided
        let mut query = String::from("UPDATE network SET updated_at = now()");
        let mut bind_index = 1;

        if name.is_some() {
            bind_index += 1;
            query.push_str(&format!(", name = ${}", bind_index));
        }
        if description.is_some() {
            bind_index += 1;
            query.push_str(&format!(", description = ${}", bind_index));
        }
        if cidr.is_some() {
            bind_index += 1;
            query.push_str(&format!(", cidr = ${}", bind_index));
        }
        if gateway.is_some() {
            bind_index += 1;
            query.push_str(&format!(", gateway = ${}", bind_index));
        }

        query.push_str(" WHERE id = $1 RETURNING *");

        let mut q = sqlx::query_as::<_, NetworkRow>(&query).bind(id);

        if let Some(n) = name {
            q = q.bind(n);
        }
        if let Some(d) = description {
            q = q.bind(d);
        }
        if let Some(c) = cidr {
            q = q.bind(c);
        }
        if let Some(g) = gateway {
            q = q.bind(g);
        }

        q.fetch_one(&self.pool).await
    }

    pub async fn delete(&self, id: Uuid) -> sqlx::Result<()> {
        sqlx::query(r#"DELETE FROM network WHERE id = $1"#)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_vm_count(&self, network_id: Uuid) -> sqlx::Result<i64> {
        let result: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(DISTINCT vm_id)
            FROM vm_network_interface
            WHERE network_id = $1
            "#,
        )
        .bind(network_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(result.0)
    }

    pub async fn get_vms(&self, network_id: Uuid) -> sqlx::Result<Vec<Uuid>> {
        let rows: Vec<(Uuid,)> = sqlx::query_as(
            r#"
            SELECT DISTINCT vm_id
            FROM vm_network_interface
            WHERE network_id = $1
            "#,
        )
        .bind(network_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|(id,)| id).collect())
    }
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct NetworkRow {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "type")]
    #[sqlx(rename = "type")]
    pub type_: String,
    pub vlan_id: Option<i32>,
    pub bridge_name: String,
    pub host_id: Option<Uuid>,
    pub cidr: Option<String>,
    pub gateway: Option<String>,
    pub created_by_user_id: Option<Uuid>,
    pub created_at: DateTime<chrono::Utc>,
    pub updated_at: DateTime<chrono::Utc>,
}
