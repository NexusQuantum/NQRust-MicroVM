/// Audit logging module for tracking all user actions in the system
///
/// This module provides functions to log user actions to the audit_logs table
/// for compliance, security auditing, and debugging purposes.
use anyhow::Result;
use nexus_types::{AuditAction, AuditLog, AuditLogQueryParams, ListAuditLogsResponse};
use sqlx::PgPool;
use uuid::Uuid;

/// Log a user action to the audit trail
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `user_id` - UUID of the user performing the action (None for system actions)
/// * `username` - Username for historical reference
/// * `action` - The action being performed (see AuditAction enum)
/// * `resource_type` - Type of resource (e.g., "vm", "function", "container")
/// * `resource_id` - UUID of the resource being acted upon
/// * `details` - Additional context as JSON
/// * `ip_address` - Client IP address
/// * `success` - Whether the action succeeded
/// * `error_message` - Error message if action failed
#[allow(dead_code, clippy::too_many_arguments)]
pub async fn log_action(
    pool: &PgPool,
    user_id: Option<Uuid>,
    username: &str,
    action: AuditAction,
    resource_type: Option<&str>,
    resource_id: Option<Uuid>,
    details: Option<serde_json::Value>,
    ip_address: Option<&str>,
    success: bool,
    error_message: Option<&str>,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO audit.audit_logs (
            user_id, username, action, resource_type, resource_id,
            details, ip_address, success, error_message
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
    )
    .bind(user_id)
    .bind(username)
    .bind(action.as_str())
    .bind(resource_type)
    .bind(resource_id)
    .bind(details)
    .bind(ip_address)
    .bind(success)
    .bind(error_message)
    .execute(pool)
    .await?;

    Ok(())
}

/// Helper function to log a successful action (success = true, no error message)
#[allow(dead_code, clippy::too_many_arguments)]
pub async fn log_success(
    pool: &PgPool,
    user_id: Uuid,
    username: &str,
    action: AuditAction,
    resource_type: Option<&str>,
    resource_id: Option<Uuid>,
    details: Option<serde_json::Value>,
    ip_address: Option<&str>,
) -> Result<()> {
    log_action(
        pool,
        Some(user_id),
        username,
        action,
        resource_type,
        resource_id,
        details,
        ip_address,
        true,
        None,
    )
    .await
}

/// Helper function to log a failed action
#[allow(dead_code, clippy::too_many_arguments)]
pub async fn log_failure(
    pool: &PgPool,
    user_id: Option<Uuid>,
    username: &str,
    action: AuditAction,
    resource_type: Option<&str>,
    resource_id: Option<Uuid>,
    details: Option<serde_json::Value>,
    ip_address: Option<&str>,
    error: &str,
) -> Result<()> {
    log_action(
        pool,
        user_id,
        username,
        action,
        resource_type,
        resource_id,
        details,
        ip_address,
        false,
        Some(error),
    )
    .await
}

/// Log a login attempt (success or failure)
#[allow(dead_code)]
pub async fn log_login(
    pool: &PgPool,
    username: &str,
    success: bool,
    user_id: Option<Uuid>,
    ip_address: Option<&str>,
    error_message: Option<&str>,
) -> Result<()> {
    let action = if success {
        AuditAction::Login
    } else {
        AuditAction::LoginFailed
    };

    log_action(
        pool,
        user_id,
        username,
        action,
        None,
        None,
        None,
        ip_address,
        success,
        error_message,
    )
    .await
}

/// Query audit logs with filters and pagination
///
/// Uses a fixed query with optional WHERE conditions.
/// All filter params are always bound (using NULL when not provided),
/// so parameter indices are stable.
pub async fn list_audit_logs(
    pool: &PgPool,
    params: AuditLogQueryParams,
) -> Result<ListAuditLogsResponse> {
    let limit = params.limit.unwrap_or(50).min(500);
    let offset = params.offset.unwrap_or(0);

    // Count query — always bind all 3 filter params (NULL = no filter)
    let total: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM audit.audit_logs
        WHERE ($1::uuid IS NULL OR user_id = $1)
          AND ($2::text  IS NULL OR action = $2)
          AND ($3::text  IS NULL OR resource_type = $3)
        "#,
    )
    .bind(params.user_id)
    .bind(&params.action)
    .bind(&params.resource_type)
    .fetch_one(pool)
    .await?;

    let rows = sqlx::query_as::<_, AuditLogRow>(
        r#"
        SELECT id, user_id, username, action, resource_type, resource_id,
               details, ip_address, success, error_message, created_at
        FROM audit.audit_logs
        WHERE ($1::uuid IS NULL OR user_id = $1)
          AND ($2::text  IS NULL OR action = $2)
          AND ($3::text  IS NULL OR resource_type = $3)
        ORDER BY created_at DESC
        LIMIT $4
        OFFSET $5
        "#,
    )
    .bind(params.user_id)
    .bind(&params.action)
    .bind(&params.resource_type)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let items = rows.into_iter().map(Into::into).collect();
    Ok(ListAuditLogsResponse { items, total })
}

/// Database row structure for audit_logs table
#[derive(sqlx::FromRow)]
struct AuditLogRow {
    id: Uuid,
    user_id: Option<Uuid>,
    username: String,
    action: String,
    resource_type: Option<String>,
    resource_id: Option<Uuid>,
    details: Option<serde_json::Value>,
    ip_address: Option<String>,
    success: bool,
    error_message: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<AuditLogRow> for AuditLog {
    fn from(row: AuditLogRow) -> Self {
        Self {
            id: row.id,
            user_id: row.user_id,
            username: row.username,
            action: row.action,
            resource_type: row.resource_type,
            resource_id: row.resource_id,
            details: row.details,
            ip_address: row.ip_address,
            success: row.success,
            error_message: row.error_message,
            created_at: row.created_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_action_as_str() {
        assert_eq!(AuditAction::Login.as_str(), "login");
        assert_eq!(AuditAction::CreateVm.as_str(), "create_vm");
        assert_eq!(AuditAction::DeleteFunction.as_str(), "delete_function");
    }
}
