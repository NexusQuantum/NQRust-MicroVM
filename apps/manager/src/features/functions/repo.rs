use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone, Serialize, sqlx::FromRow)]
pub struct FunctionRow {
    pub id: Uuid,
    pub name: String,
    pub runtime: String,
    pub code: String,
    pub handler: String,
    pub timeout_seconds: i32,
    pub memory_mb: i32,
    pub vcpu: i32,
    pub env_vars: Option<serde_json::Value>,
    pub vm_id: Option<Uuid>,
    pub guest_ip: Option<String>,
    pub port: i32,
    pub state: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub last_invoked_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Clone, Serialize, sqlx::FromRow)]
pub struct FunctionInvocationRow {
    pub id: Uuid,
    pub function_id: Uuid,
    pub status: String,
    pub duration_ms: i64,
    pub memory_used_mb: Option<i32>,
    pub request_id: String,
    pub event: serde_json::Value,
    pub response: Option<serde_json::Value>,
    pub logs: Vec<String>,
    pub error: Option<String>,
    pub invoked_at: chrono::DateTime<chrono::Utc>,
}

// ========================================
// Function CRUD
// ========================================

pub async fn insert(db: &PgPool, row: &FunctionRow) -> sqlx::Result<()> {
    sqlx::query(
        r#"INSERT INTO function (id, name, runtime, code, handler, timeout_seconds, memory_mb, vcpu, env_vars, port, state)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)"#,
    )
    .bind(row.id)
    .bind(&row.name)
    .bind(&row.runtime)
    .bind(&row.code)
    .bind(&row.handler)
    .bind(row.timeout_seconds)
    .bind(row.memory_mb)
    .bind(row.vcpu)
    .bind(&row.env_vars)
    .bind(row.port)
    .bind(&row.state)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn list(db: &PgPool) -> sqlx::Result<Vec<FunctionRow>> {
    sqlx::query_as::<_, FunctionRow>(
        r#"
        SELECT id, name, runtime, code, handler, timeout_seconds, memory_mb, vcpu,
               env_vars, vm_id, guest_ip, port, state, created_at, updated_at, last_invoked_at
        FROM function
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(db)
    .await
}

pub async fn get(db: &PgPool, id: Uuid) -> sqlx::Result<Option<FunctionRow>> {
    sqlx::query_as::<_, FunctionRow>(
        r#"
        SELECT id, name, runtime, code, handler, timeout_seconds, memory_mb, vcpu,
               env_vars, vm_id, guest_ip, port, state, created_at, updated_at, last_invoked_at
        FROM function
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(db)
    .await
}

pub async fn update(
    db: &PgPool,
    id: Uuid,
    name: Option<&str>,
    runtime: Option<&str>,
    code: Option<&str>,
    handler: Option<&str>,
    timeout_seconds: Option<i32>,
    memory_mb: Option<i32>,
    env_vars: Option<&serde_json::Value>,
) -> sqlx::Result<()> {
    let mut query = String::from("UPDATE function SET updated_at = now()");
    let mut bind_count = 1;

    if name.is_some() {
        query.push_str(&format!(", name = ${}", bind_count));
        bind_count += 1;
    }
    if runtime.is_some() {
        query.push_str(&format!(", runtime = ${}", bind_count));
        bind_count += 1;
    }
    if code.is_some() {
        query.push_str(&format!(", code = ${}", bind_count));
        bind_count += 1;
    }
    if handler.is_some() {
        query.push_str(&format!(", handler = ${}", bind_count));
        bind_count += 1;
    }
    if timeout_seconds.is_some() {
        query.push_str(&format!(", timeout_seconds = ${}", bind_count));
        bind_count += 1;
    }
    if memory_mb.is_some() {
        query.push_str(&format!(", memory_mb = ${}", bind_count));
        bind_count += 1;
    }
    if env_vars.is_some() {
        query.push_str(&format!(", env_vars = ${}", bind_count));
        bind_count += 1;
    }

    query.push_str(&format!(" WHERE id = ${}", bind_count));

    let mut q = sqlx::query(&query);

    if let Some(v) = name {
        q = q.bind(v);
    }
    if let Some(v) = runtime {
        q = q.bind(v);
    }
    if let Some(v) = code {
        q = q.bind(v);
    }
    if let Some(v) = handler {
        q = q.bind(v);
    }
    if let Some(v) = timeout_seconds {
        q = q.bind(v);
    }
    if let Some(v) = memory_mb {
        q = q.bind(v);
    }
    if let Some(v) = env_vars {
        q = q.bind(v);
    }

    q = q.bind(id);
    q.execute(db).await?;
    Ok(())
}

pub async fn delete(db: &PgPool, id: Uuid) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM function WHERE id = $1")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn update_last_invoked(db: &PgPool, id: Uuid) -> sqlx::Result<()> {
    sqlx::query("UPDATE function SET last_invoked_at = now() WHERE id = $1")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn update_vm_info(
    db: &PgPool,
    id: Uuid,
    vm_id: Uuid,
    guest_ip: Option<&str>,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE function SET vm_id = $1, guest_ip = $2, updated_at = now() WHERE id = $3"
    )
    .bind(vm_id)
    .bind(guest_ip)
    .bind(id)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn update_state(db: &PgPool, id: Uuid, state: &str) -> sqlx::Result<()> {
    sqlx::query("UPDATE function SET state = $1, updated_at = now() WHERE id = $2")
        .bind(state)
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

// ========================================
// Function Invocations
// ========================================

pub async fn insert_invocation(db: &PgPool, row: &FunctionInvocationRow) -> sqlx::Result<()> {
    sqlx::query(
        r#"INSERT INTO function_invocation
           (id, function_id, status, duration_ms, memory_used_mb, request_id, event, response, logs, error, invoked_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)"#,
    )
    .bind(row.id)
    .bind(row.function_id)
    .bind(&row.status)
    .bind(row.duration_ms)
    .bind(row.memory_used_mb)
    .bind(&row.request_id)
    .bind(&row.event)
    .bind(&row.response)
    .bind(&row.logs)
    .bind(&row.error)
    .bind(row.invoked_at)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn list_invocations(
    db: &PgPool,
    function_id: Uuid,
    status: Option<&str>,
    limit: Option<i64>,
) -> sqlx::Result<Vec<FunctionInvocationRow>> {
    let mut query = String::from(
        r#"
        SELECT id, function_id, status, duration_ms, memory_used_mb, request_id,
               event, response, logs, error, invoked_at
        FROM function_invocation
        WHERE function_id = $1
        "#,
    );

    if status.is_some() {
        query.push_str(" AND status = $2");
    }

    query.push_str(" ORDER BY invoked_at DESC");

    if limit.is_some() {
        if status.is_some() {
            query.push_str(&format!(" LIMIT ${}", 3));
        } else {
            query.push_str(&format!(" LIMIT ${}", 2));
        }
    }

    let mut q = sqlx::query_as::<_, FunctionInvocationRow>(&query).bind(function_id);

    if let Some(s) = status {
        q = q.bind(s);
    }
    if let Some(lim) = limit {
        q = q.bind(lim);
    }

    q.fetch_all(db).await
}
