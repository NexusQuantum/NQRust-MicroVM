use chrono::DateTime;
use serde::Serialize;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct HostRepository {
    pool: PgPool,
}

impl HostRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn register(
        &self,
        name: &str,
        addr: &str,
        capabilities: Value,
    ) -> sqlx::Result<HostRow> {
        sqlx::query_as::<_, HostRow>(
            r#"
            INSERT INTO host (id, name, addr, capabilities_json, last_seen_at)
            VALUES ($1, $2, $3, $4, now())
            ON CONFLICT (addr) DO UPDATE
            SET name = EXCLUDED.name,
                capabilities_json = EXCLUDED.capabilities_json,
                last_seen_at = now()
            RETURNING *
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(name)
        .bind(addr)
        .bind(capabilities)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn heartbeat(&self, id: Uuid, capabilities: Option<Value>) -> sqlx::Result<HostRow> {
        match capabilities {
            Some(value) => {
                sqlx::query_as::<_, HostRow>(
                    r#"UPDATE host SET capabilities_json=$2, last_seen_at=now() WHERE id=$1 RETURNING *"#,
                )
                .bind(id)
                .bind(value)
                .fetch_one(&self.pool)
                .await
            }
            None => {
                sqlx::query_as::<_, HostRow>(
                    r#"UPDATE host SET last_seen_at=now() WHERE id=$1 RETURNING *"#,
                )
                .bind(id)
                .fetch_one(&self.pool)
                .await
            }
        }
    }

    pub async fn get(&self, id: Uuid) -> sqlx::Result<HostRow> {
        sqlx::query_as::<_, HostRow>(r#"SELECT * FROM host WHERE id=$1"#)
            .bind(id)
            .fetch_one(&self.pool)
            .await
    }

    pub async fn first_healthy(&self) -> sqlx::Result<HostRow> {
        sqlx::query_as::<_, HostRow>(
            r#"
            SELECT * FROM host
            WHERE last_seen_at > now() - INTERVAL '30 seconds'
            ORDER BY last_seen_at DESC
            LIMIT 1
            "#,
        )
        .fetch_one(&self.pool)
        .await
    }
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct HostRow {
    pub id: Uuid,
    pub name: String,
    pub addr: String,
    pub capabilities_json: Value,
    pub last_seen_at: DateTime<chrono::Utc>,
}
