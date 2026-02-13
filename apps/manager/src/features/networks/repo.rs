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
        status: &str,
        managed: bool,
        dhcp_enabled: bool,
        dhcp_range_start: Option<&str>,
        dhcp_range_end: Option<&str>,
        uplink_interface: Option<&str>,
    ) -> sqlx::Result<NetworkRow> {
        sqlx::query_as::<_, NetworkRow>(
            r#"
            INSERT INTO network (name, description, type, vlan_id, bridge_name, host_id, cidr, gateway,
                                 status, managed, dhcp_enabled, dhcp_range_start, dhcp_range_end,
                                 created_by_user_id, uplink_interface)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
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
        .bind(status)
        .bind(managed)
        .bind(dhcp_enabled)
        .bind(dhcp_range_start)
        .bind(dhcp_range_end)
        .bind(None as Option<Uuid>) // created_by_user_id - TODO: Set from authenticated user context
        .bind(uplink_interface)
        .fetch_one(&self.pool)
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_with_vni(
        &self,
        name: &str,
        description: Option<&str>,
        network_type: &str,
        vlan_id: Option<i32>,
        bridge_name: &str,
        host_id: Uuid,
        cidr: Option<&str>,
        gateway: Option<&str>,
        status: &str,
        managed: bool,
        dhcp_enabled: bool,
        dhcp_range_start: Option<&str>,
        dhcp_range_end: Option<&str>,
        vni: i32,
    ) -> sqlx::Result<NetworkRow> {
        sqlx::query_as::<_, NetworkRow>(
            r#"
            INSERT INTO network (name, description, type, vlan_id, bridge_name, host_id, cidr, gateway,
                                 status, managed, dhcp_enabled, dhcp_range_start, dhcp_range_end, created_by_user_id, vni)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
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
        .bind(status)
        .bind(managed)
        .bind(dhcp_enabled)
        .bind(dhcp_range_start)
        .bind(dhcp_range_end)
        .bind(None as Option<Uuid>) // created_by_user_id
        .bind(vni)
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

    /// List active VXLAN network_host entries for a host (for reconciliation).
    pub async fn list_active_vxlan_hosts_for_host(
        &self,
        host_id: Uuid,
    ) -> sqlx::Result<Vec<NetworkHostRow>> {
        sqlx::query_as::<_, NetworkHostRow>(
            r#"
            SELECT nh.* FROM network_host nh
            JOIN network n ON n.id = nh.network_id
            WHERE nh.host_id = $1
              AND nh.status = 'active'
              AND n.managed = true
              AND n.status = 'active'
              AND n.type = 'vxlan'
            ORDER BY nh.created_at
            "#,
        )
        .bind(host_id)
        .fetch_all(&self.pool)
        .await
    }

    /// List active, managed networks bound to a specific host (for reconciliation).
    /// Excludes VXLAN networks which use the network_host junction table.
    pub async fn list_active_managed_for_host(
        &self,
        host_id: Uuid,
    ) -> sqlx::Result<Vec<NetworkRow>> {
        sqlx::query_as::<_, NetworkRow>(
            r#"
            SELECT * FROM network
            WHERE host_id = $1
              AND managed = true
              AND status = 'active'
              AND type IN ('nat', 'isolated', 'bridged')
            ORDER BY created_at
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

    pub async fn update_status(
        &self,
        id: Uuid,
        status: &str,
        error_message: Option<&str>,
    ) -> sqlx::Result<NetworkRow> {
        sqlx::query_as::<_, NetworkRow>(
            r#"
            UPDATE network
            SET status = $2, error_message = $3, updated_at = now()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(status)
        .bind(error_message)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn list_bridge_names_for_host(&self, host_id: Uuid) -> sqlx::Result<Vec<String>> {
        let rows: Vec<(String,)> =
            sqlx::query_as(r#"SELECT bridge_name FROM network WHERE host_id = $1"#)
                .bind(host_id)
                .fetch_all(&self.pool)
                .await?;
        Ok(rows.into_iter().map(|(n,)| n).collect())
    }

    pub async fn list_cidrs_for_host(&self, host_id: Uuid) -> sqlx::Result<Vec<String>> {
        let rows: Vec<(String,)> =
            sqlx::query_as(r#"SELECT cidr FROM network WHERE host_id = $1 AND cidr IS NOT NULL"#)
                .bind(host_id)
                .fetch_all(&self.pool)
                .await?;
        Ok(rows.into_iter().map(|(c,)| c).collect())
    }

    #[allow(dead_code)]
    pub async fn find_by_bridge_and_host(
        &self,
        bridge_name: &str,
        host_id: Uuid,
    ) -> sqlx::Result<Option<NetworkRow>> {
        sqlx::query_as::<_, NetworkRow>(
            r#"SELECT * FROM network WHERE bridge_name = $1 AND host_id = $2 LIMIT 1"#,
        )
        .bind(bridge_name)
        .bind(host_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Get the next available VNI (VXLAN Network Identifier), starting from 100.
    pub async fn next_available_vni(&self) -> sqlx::Result<i32> {
        let result: (Option<i32>,) =
            sqlx::query_as(r#"SELECT MAX(vni) FROM network WHERE vni IS NOT NULL"#)
                .fetch_one(&self.pool)
                .await?;
        Ok(result.0.map_or(100, |max| max + 1))
    }

    // --- network_host junction table methods ---

    pub async fn add_network_host(
        &self,
        network_id: Uuid,
        host_id: Uuid,
        vtep_ip: &str,
        is_gateway: bool,
    ) -> sqlx::Result<NetworkHostRow> {
        sqlx::query_as::<_, NetworkHostRow>(
            r#"
            INSERT INTO network_host (network_id, host_id, vtep_ip, is_gateway, status)
            VALUES ($1, $2, $3, $4, 'provisioning')
            RETURNING *
            "#,
        )
        .bind(network_id)
        .bind(host_id)
        .bind(vtep_ip)
        .bind(is_gateway)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn list_network_hosts(&self, network_id: Uuid) -> sqlx::Result<Vec<NetworkHostRow>> {
        sqlx::query_as::<_, NetworkHostRow>(
            r#"SELECT * FROM network_host WHERE network_id = $1 ORDER BY created_at"#,
        )
        .bind(network_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get_network_host(
        &self,
        network_id: Uuid,
        host_id: Uuid,
    ) -> sqlx::Result<Option<NetworkHostRow>> {
        sqlx::query_as::<_, NetworkHostRow>(
            r#"SELECT * FROM network_host WHERE network_id = $1 AND host_id = $2"#,
        )
        .bind(network_id)
        .bind(host_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn update_network_host_status(
        &self,
        id: Uuid,
        status: &str,
        error_message: Option<&str>,
    ) -> sqlx::Result<()> {
        sqlx::query(r#"UPDATE network_host SET status = $2, error_message = $3 WHERE id = $1"#)
            .bind(id)
            .bind(status)
            .bind(error_message)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_network_hosts(&self, network_id: Uuid) -> sqlx::Result<()> {
        sqlx::query(r#"DELETE FROM network_host WHERE network_id = $1"#)
            .bind(network_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn count_network_hosts(&self, network_id: Uuid) -> sqlx::Result<i64> {
        let result: (i64,) =
            sqlx::query_as(r#"SELECT COUNT(*) FROM network_host WHERE network_id = $1"#)
                .bind(network_id)
                .fetch_one(&self.pool)
                .await?;
        Ok(result.0)
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
    pub status: String,
    pub error_message: Option<String>,
    pub managed: bool,
    pub dhcp_enabled: bool,
    pub dhcp_range_start: Option<String>,
    pub dhcp_range_end: Option<String>,
    pub created_by_user_id: Option<Uuid>,
    pub vni: Option<i32>,
    pub uplink_interface: Option<String>,
    pub created_at: DateTime<chrono::Utc>,
    pub updated_at: DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct NetworkHostRow {
    pub id: Uuid,
    pub network_id: Uuid,
    pub host_id: Uuid,
    pub vtep_ip: String,
    pub is_gateway: bool,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: DateTime<chrono::Utc>,
}
