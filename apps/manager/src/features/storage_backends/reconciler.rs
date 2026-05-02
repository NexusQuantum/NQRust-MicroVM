//! B-III Task 9: retry reconciler for `raft_repair_queue`.
//!
//! Runs as a background task spawned from `main.rs`. Walks the queue every
//! [`SCAN_INTERVAL`] and:
//!
//! - **Promotes stuck `in_progress` rows to `failed`.** A row that has been
//!   in `in_progress` for more than [`STUCK_THRESHOLD`] is the fingerprint
//!   of a manager that crashed mid-operation. We can't replay arbitrary
//!   ops blind (membership changes need operator review), so we flag it
//!   `failed` with an explicit `last_error` and let an operator decide
//!   whether to retry or cancel.
//!
//! - **Retries idempotent operations on `failed` rows.** Currently only
//!   `repair_replica` qualifies — `runtime_start` on the agent is safe to
//!   re-issue. Add/remove/transfer/decommission stay in `failed` so an
//!   operator can review the partial state before re-issuing through the
//!   normal API.
//!
//! Backoff is plain exponential, capped at [`MAX_BACKOFF`]. After
//! [`MAX_ATTEMPTS`] the row stays in `failed` and stops being retried;
//! the queue listing surfaces it for operator action.

use std::time::Duration;

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// How often the reconciler scans the queue for actionable rows.
const SCAN_INTERVAL: Duration = Duration::from_secs(15);

/// An `in_progress` row older than this is treated as a manager-crash
/// orphan and forced to `failed`.
const STUCK_THRESHOLD: Duration = Duration::from_secs(300);

/// Maximum retries before a `failed` row is left for operator review.
const MAX_ATTEMPTS: i32 = 5;

/// Cap on exponential backoff between retries.
const MAX_BACKOFF: Duration = Duration::from_secs(600);

/// Spawn the reconciler. Returns immediately; the task runs until the
/// process exits.
pub fn spawn(pool: PgPool) {
    tokio::spawn(async move { reconcile_loop(pool).await });
}

async fn reconcile_loop(pool: PgPool) {
    info!("raft repair queue reconciler started");
    loop {
        if let Err(err) = scan_once(&pool).await {
            warn!(error = ?err, "raft repair queue scan failed");
        }
        tokio::time::sleep(SCAN_INTERVAL).await;
    }
}

#[derive(sqlx::FromRow, Debug)]
#[allow(dead_code)]
struct Candidate {
    id: Uuid,
    backend_id: Uuid,
    group_id: Uuid,
    op_type: String,
    /// Retained for future op-specific dispatch; unused in the current
    /// scope (the routes layer owns operation-specific orchestration).
    op_args: serde_json::Value,
    state: String,
    attempts: i32,
    started_at: Option<DateTime<Utc>>,
    updated_at: DateTime<Utc>,
}

async fn scan_once(pool: &PgPool) -> sqlx::Result<()> {
    let rows: Vec<Candidate> = sqlx::query_as(
        r#"
        SELECT id, backend_id, group_id, op_type, op_args, state, attempts,
               started_at, updated_at
          FROM raft_repair_queue
         WHERE state IN ('in_progress', 'failed')
           AND attempts < $1
        "#,
    )
    .bind(MAX_ATTEMPTS)
    .fetch_all(pool)
    .await?;

    for row in rows {
        if row.state == "in_progress" {
            handle_stuck(pool, &row).await;
            continue;
        }
        if row.state == "failed" {
            handle_failed(pool, &row).await;
        }
    }
    Ok(())
}

async fn handle_stuck(pool: &PgPool, row: &Candidate) {
    let started = row.started_at.unwrap_or(row.updated_at);
    let age = Utc::now().signed_duration_since(started);
    if age.num_seconds() < STUCK_THRESHOLD.as_secs() as i64 {
        return;
    }
    warn!(
        operation_id = %row.id,
        op_type = %row.op_type,
        backend_id = %row.backend_id,
        group_id = %row.group_id,
        age_seconds = age.num_seconds(),
        "promoting stuck in_progress row to failed"
    );
    let note = format!(
        "manager interruption: in_progress for {}s without completion",
        age.num_seconds()
    );
    if let Err(err) = sqlx::query(
        r#"
        UPDATE raft_repair_queue
           SET state = 'failed',
               last_error = $2,
               finished_at = now(),
               updated_at = now()
         WHERE id = $1
        "#,
    )
    .bind(row.id)
    .bind(&note)
    .execute(pool)
    .await
    {
        error!(operation_id = %row.id, error = ?err, "failed to mark stuck row failed");
    }
}

async fn handle_failed(pool: &PgPool, row: &Candidate) {
    if !is_retryable(&row.op_type) {
        debug!(operation_id = %row.id, op_type = %row.op_type, "skip retry: op not idempotent");
        return;
    }
    let backoff = backoff_for(row.attempts);
    let age = Utc::now().signed_duration_since(row.updated_at);
    if age.num_seconds() < backoff.as_secs() as i64 {
        debug!(
            operation_id = %row.id,
            op_type = %row.op_type,
            attempts = row.attempts,
            backoff_seconds = backoff.as_secs(),
            "retry not yet due"
        );
        return;
    }
    info!(
        operation_id = %row.id,
        op_type = %row.op_type,
        attempts = row.attempts,
        "re-arming retryable failed operation"
    );
    if let Err(err) = sqlx::query(
        r#"
        UPDATE raft_repair_queue
           SET state = 'pending',
               last_error = NULL,
               started_at = NULL,
               finished_at = NULL,
               updated_at = now()
         WHERE id = $1
           AND state = 'failed'
        "#,
    )
    .bind(row.id)
    .execute(pool)
    .await
    {
        error!(operation_id = %row.id, error = ?err, "failed to re-arm failed row");
    }
    // Note: the actual retry is operator-triggered through the API. This
    // reconciler only re-arms the row to `pending` so the next operator
    // call (or future automatic dispatcher) sees a clean state. We
    // deliberately do not re-issue the agent RPCs here without a leader
    // location and replica config, both of which currently live with the
    // routes handler. A follow-up task can lift those into a shared
    // dispatcher and have this reconciler call it directly.
}

fn is_retryable(op_type: &str) -> bool {
    matches!(op_type, "repair_replica")
}

fn backoff_for(attempts: i32) -> Duration {
    let attempts = attempts.max(0) as u32;
    let secs = 30u64.saturating_mul(1u64.checked_shl(attempts).unwrap_or(u64::MAX));
    Duration::from_secs(secs.min(MAX_BACKOFF.as_secs()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_caps_at_max() {
        assert_eq!(backoff_for(0), Duration::from_secs(30));
        assert_eq!(backoff_for(1), Duration::from_secs(60));
        assert_eq!(backoff_for(2), Duration::from_secs(120));
        assert_eq!(backoff_for(3), Duration::from_secs(240));
        assert_eq!(backoff_for(4), Duration::from_secs(480));
        assert_eq!(backoff_for(5), MAX_BACKOFF);
        assert_eq!(backoff_for(99), MAX_BACKOFF);
    }

    #[test]
    fn only_repair_replica_retries() {
        assert!(is_retryable("repair_replica"));
        assert!(!is_retryable("add_replica"));
        assert!(!is_retryable("remove_replica"));
        assert!(!is_retryable("transfer_leader"));
        assert!(!is_retryable("decommission_host"));
        assert!(!is_retryable("promote_hot_spare"));
        assert!(!is_retryable("rebalance"));
    }
}
