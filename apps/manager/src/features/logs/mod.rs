use crate::AppState;
use axum::{extract::Query, routing::get, Extension, Json, Router};
use nexus_types::{AuditLogQueryParams, ListAuditLogsResponse, TailLogResponse};
use serde::{Deserialize, Serialize};
use sqlx;
use utoipa::{IntoParams, ToSchema};

use super::users::audit;

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct TailLogQuery {
    path: String,
}

pub fn router() -> Router {
    Router::new()
        .route("/tail", get(tail_once))
        .route("/audit", get(list_audit_logs))
        .route("/db-info", get(get_db_info))
        .route("/stats", get(get_system_stats))
}

/// Super simple file read (dev only). Frontend can poll.
#[utoipa::path(
    get,
    path = "/v1/logs/tail",
    params(TailLogQuery),
    responses((status = 200, description = "Log tailed", body = TailLogResponse)),
    tag = "Logs"
)]
pub async fn tail_once(Query(q): Query<TailLogQuery>) -> Json<TailLogResponse> {
    let txt = tokio::fs::read_to_string(q.path).await.unwrap_or_default();
    Json(TailLogResponse { text: txt })
}

/// List audit logs with optional filters and pagination
#[utoipa::path(
    get,
    path = "/v1/logs/audit",
    params(AuditLogQueryParams),
    responses((status = 200, description = "Audit logs listed", body = ListAuditLogsResponse)),
    tag = "Logs"
)]
pub async fn list_audit_logs(
    Extension(st): Extension<AppState>,
    Query(params): Query<AuditLogQueryParams>,
) -> Result<Json<ListAuditLogsResponse>, axum::http::StatusCode> {
    match audit::list_audit_logs(&st.db, params).await {
        Ok(resp) => Ok(Json(resp)),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Database connection info for external tools
#[derive(Serialize, ToSchema)]
pub struct DbInfoResponse {
    pub host: String,
    pub port: String,
    pub database: String,
    pub username: String,
    pub connection_string_masked: String,
}

/// Get database connection info (password masked)
#[utoipa::path(
    get,
    path = "/v1/logs/db-info",
    responses((status = 200, description = "Database connection info", body = DbInfoResponse)),
    tag = "Logs"
)]
pub async fn get_db_info() -> Json<DbInfoResponse> {
    let db_url = std::env::var("DATABASE_URL").unwrap_or_default();

    // Parse postgresql://user:pass@host:port/dbname
    let (host, port, database, username, masked) = parse_database_url(&db_url);

    Json(DbInfoResponse {
        host,
        port,
        database,
        username,
        connection_string_masked: masked,
    })
}

/// System stats for the logging overview
#[derive(Serialize, ToSchema)]
pub struct SystemStatsResponse {
    pub total_hosts: i64,
    pub total_vms: i64,
    pub running_vms: i64,
    pub total_functions: i64,
    pub total_containers: i64,
    pub running_containers: i64,
}

/// Get system-wide resource counts
#[utoipa::path(
    get,
    path = "/v1/logs/stats",
    responses((status = 200, description = "System stats", body = SystemStatsResponse)),
    tag = "Logs"
)]
pub async fn get_system_stats(
    Extension(st): Extension<AppState>,
) -> Result<Json<SystemStatsResponse>, axum::http::StatusCode> {
    let db = &st.db;

    let total_hosts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM host")
        .fetch_one(db)
        .await
        .unwrap_or(0);
    let total_vms: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM vm")
        .fetch_one(db)
        .await
        .unwrap_or(0);
    let running_vms: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM vm WHERE state = 'running'")
        .fetch_one(db)
        .await
        .unwrap_or(0);
    let total_functions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM function")
        .fetch_one(db)
        .await
        .unwrap_or(0);
    let total_containers: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM containers")
        .fetch_one(db)
        .await
        .unwrap_or(0);
    let running_containers: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM containers WHERE state = 'running'")
            .fetch_one(db)
            .await
            .unwrap_or(0);

    Ok(Json(SystemStatsResponse {
        total_hosts,
        total_vms,
        running_vms,
        total_functions,
        total_containers,
        running_containers,
    }))
}

/// Parse postgresql://user:pass@host:port/dbname using simple string ops
fn parse_database_url(url: &str) -> (String, String, String, String, String) {
    let fallback = (
        "unknown".to_string(),
        "5432".to_string(),
        "unknown".to_string(),
        "unknown".to_string(),
        "Unable to parse DATABASE_URL".to_string(),
    );

    // Strip scheme (e.g. "postgresql://")
    let after_scheme = match url.find("://") {
        Some(i) => &url[i + 3..],
        None => return fallback,
    };
    let scheme = &url[..url.find("://").unwrap()];

    // Split user_info@host_and_db
    let (user_info, host_and_db) = match after_scheme.find('@') {
        Some(i) => (&after_scheme[..i], &after_scheme[i + 1..]),
        None => return fallback,
    };

    // Extract username (before ':')
    let username = match user_info.find(':') {
        Some(i) => &user_info[..i],
        None => user_info,
    };

    // Split host:port/database (handle query params with '?')
    let host_and_db = host_and_db.split('?').next().unwrap_or(host_and_db);
    let (host_port, database) = match host_and_db.find('/') {
        Some(i) => (&host_and_db[..i], &host_and_db[i + 1..]),
        None => (host_and_db, ""),
    };

    let (host, port) = match host_port.rfind(':') {
        Some(i) => (&host_port[..i], &host_port[i + 1..]),
        None => (host_port, "5432"),
    };

    let masked = format!(
        "{}://{}:****@{}:{}/{}",
        scheme, username, host, port, database
    );

    (
        host.to_string(),
        port.to_string(),
        database.to_string(),
        username.to_string(),
        masked,
    )
}
