//! B-III auto-reconciler: drives the planner+executor for two
//! operator-initiated lifecycle events.
//!
//! - **Drain a draining host (Task 6).** When an operator calls
//!   `POST /v1/hosts/{id}/decommission`, the host transitions to
//!   `draining` but the underlying replicas don't move on their own.
//!   This reconciler runs `plan_decommission` for every `draining` host
//!   and dispatches `execute_plan` against the manager itself. On
//!   success the host transitions to `decommissioned`.
//!
//! - **Promote hot-spares on host failure (Task 7).** A host that has
//!   missed heartbeats for [`PROMOTION_THRESHOLD`] is treated as failed;
//!   `plan_hot_spare_promotion` covers its replicas onto a hot-spare
//!   and the executor runs the plan. The failed host is *not*
//!   transitioned automatically — the operator confirms the host is
//!   gone before removing it from the cluster, so a transient blip
//!   doesn't hard-decommission a recoverable host.
//!
//! The reconciler is conservative:
//!
//! - One scan loop, sequential per backend.
//! - Skips backends that already have any `in_progress` row in
//!   `raft_repair_queue` (operator or another reconciler is mid-flight).
//! - On any plan failure: leaves the host in its current state; the
//!   operator inspects the repair queue and re-issues.
//! - Backoff after a failed promotion attempt to avoid thrashing on a
//!   permanently-unfixable host.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use sqlx::PgPool;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::features::storage_backends::executor::{execute, PlanRun, StepStatus};
use crate::features::storage_backends::planner::{
    plan_decommission, plan_hot_spare_promotion, HostView, ReplicaView,
};

/// How often the auto-reconciler scans the cluster. Overridable via
/// `MANAGER_AUTO_RECONCILER_SCAN_SECS` for smoke/integration tests.
fn scan_interval() -> Duration {
    Duration::from_secs(
        std::env::var("MANAGER_AUTO_RECONCILER_SCAN_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(60),
    )
}

/// A host that has missed heartbeats for this long is treated as failed
/// for hot-spare promotion. Overridable via
/// `MANAGER_PROMOTION_THRESHOLD_SECS` for smoke/integration tests.
fn promotion_threshold() -> Duration {
    Duration::from_secs(
        std::env::var("MANAGER_PROMOTION_THRESHOLD_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(600),
    )
}

/// Don't re-attempt promotion against the same failed host within this
/// window. Overridable via `MANAGER_PROMOTION_BACKOFF_SECS`.
fn promotion_backoff() -> Duration {
    Duration::from_secs(
        std::env::var("MANAGER_PROMOTION_BACKOFF_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(900),
    )
}

#[derive(Clone)]
struct AutoReconcilerCtx {
    pool: PgPool,
    manager_base: String,
    /// `Bearer <token>` value the executor passes when calling back into
    /// the manager's own HTTP API. Minted once at spawn time against the
    /// `root` admin user so the executor isn't rejected by the auth
    /// layer guarding `/v1/storage_backends/*`.
    auth_header: Option<String>,
    /// In-memory record of "we tried to promote spare for this host at
    /// time T" so we can apply [`promotion_backoff`] without an extra
    /// DB column. Lost on manager restart, which is fine — the
    /// startup race resolves naturally as the loop runs again.
    last_promotion_attempt: Arc<std::sync::Mutex<HashMap<Uuid, std::time::Instant>>>,
}

pub fn spawn(pool: PgPool, manager_base: String) {
    let ctx_pool = pool.clone();
    tokio::spawn(async move {
        let auth_header = match mint_service_token(&ctx_pool).await {
            Ok(t) => Some(format!("Bearer {t}")),
            Err(err) => {
                warn!(?err, "auto-reconciler: failed to mint service token; executor calls will fail with 401");
                None
            }
        };
        let ctx = AutoReconcilerCtx {
            pool: ctx_pool,
            manager_base,
            auth_header,
            last_promotion_attempt: Arc::new(std::sync::Mutex::new(HashMap::new())),
        };
        run_loop(ctx).await;
    });
}

async fn mint_service_token(pool: &PgPool) -> anyhow::Result<String> {
    let users = crate::features::users::repo::UserRepository::new(pool.clone());
    let user = users.get_by_username("root").await?;
    let token = users.create_token(user.id, None).await?;
    Ok(token)
}

async fn run_loop(ctx: AutoReconcilerCtx) {
    info!("storage auto-reconciler started");
    loop {
        if let Err(err) = scan_once(&ctx).await {
            warn!(error = ?err, "storage auto-reconciler scan failed");
        }
        tokio::time::sleep(scan_interval()).await;
    }
}

async fn scan_once(ctx: &AutoReconcilerCtx) -> sqlx::Result<()> {
    // Each raft_spdk backend gets its own scan pass.
    let backends: Vec<Uuid> = sqlx::query_scalar(
        r#"SELECT id FROM storage_backend WHERE kind = 'raft_spdk' AND deleted_at IS NULL"#,
    )
    .fetch_all(&ctx.pool)
    .await?;
    for backend_id in backends {
        if let Err(err) = scan_backend(ctx, backend_id).await {
            warn!(backend_id = %backend_id, error = ?err, "scan_backend failed");
        }
    }
    Ok(())
}

async fn scan_backend(ctx: &AutoReconcilerCtx, backend_id: Uuid) -> sqlx::Result<()> {
    if has_in_progress_repair(&ctx.pool, backend_id).await? {
        debug!(backend_id = %backend_id, "skip scan: in_progress repair queue row");
        return Ok(());
    }

    let (hosts, replicas, spdk_by_host) = collect_state(ctx, backend_id).await?;
    drain_draining_hosts(ctx, backend_id, &hosts, &replicas, &spdk_by_host).await?;
    promote_failed_hosts(ctx, backend_id, &hosts, &replicas, &spdk_by_host).await?;
    Ok(())
}

async fn has_in_progress_repair(pool: &PgPool, backend_id: Uuid) -> sqlx::Result<bool> {
    let count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
          FROM raft_repair_queue
         WHERE backend_id = $1
           AND state = 'in_progress'
        "#,
    )
    .bind(backend_id)
    .fetch_one(pool)
    .await?;
    Ok(count > 0)
}

#[derive(sqlx::FromRow)]
struct HostRow {
    id: Uuid,
    addr: String,
    is_hot_spare: bool,
    lifecycle_state: String,
    last_seen_at: chrono::DateTime<chrono::Utc>,
    spdk_backend_id: Option<Uuid>,
}

#[derive(sqlx::FromRow)]
struct ReplicaRow {
    group_id: Uuid,
    node_id: i64,
    agent_base_url: String,
}

async fn collect_state(
    ctx: &AutoReconcilerCtx,
    backend_id: Uuid,
) -> sqlx::Result<(Vec<HostView>, Vec<ReplicaView>, HashMap<Uuid, Uuid>)> {
    let host_rows: Vec<HostRow> = sqlx::query_as(
        r#"SELECT id, addr, is_hot_spare, lifecycle_state, last_seen_at, spdk_backend_id
             FROM host"#,
    )
    .fetch_all(&ctx.pool)
    .await?;
    let now = chrono::Utc::now();
    let host_views: Vec<HostView> = host_rows
        .iter()
        .map(|h| HostView {
            id: h.id,
            addr: h.addr.clone(),
            is_hot_spare: h.is_hot_spare,
            lifecycle_state: h.lifecycle_state.clone(),
            healthy: now.signed_duration_since(h.last_seen_at).num_seconds() <= 30,
            replica_count: 0,
        })
        .collect();
    let spdk_by_host: HashMap<Uuid, Uuid> = host_rows
        .iter()
        .filter_map(|h| h.spdk_backend_id.map(|id| (h.id, id)))
        .collect();

    let replica_rows: Vec<ReplicaRow> = sqlx::query_as(
        r#"SELECT group_id, node_id, agent_base_url
             FROM raft_spdk_replica
            WHERE backend_id = $1 AND removed_at IS NULL"#,
    )
    .bind(backend_id)
    .fetch_all(&ctx.pool)
    .await?;
    let host_by_addr: HashMap<String, Uuid> =
        host_rows.iter().map(|h| (h.addr.clone(), h.id)).collect();
    let replicas: Vec<ReplicaView> = replica_rows
        .into_iter()
        .filter_map(|r| {
            let host_addr = r
                .agent_base_url
                .rsplit_once("/v1/raft_block")
                .map(|(prefix, _)| prefix.to_string())
                .unwrap_or_else(|| r.agent_base_url.clone());
            let host_id = host_by_addr.get(&host_addr).copied()?;
            Some(ReplicaView {
                backend_id,
                group_id: r.group_id,
                node_id: r.node_id as u64,
                host_id,
            })
        })
        .collect();

    Ok((host_views, replicas, spdk_by_host))
}

async fn drain_draining_hosts(
    ctx: &AutoReconcilerCtx,
    backend_id: Uuid,
    hosts: &[HostView],
    replicas: &[ReplicaView],
    spdk_by_host: &HashMap<Uuid, Uuid>,
) -> sqlx::Result<()> {
    let draining: Vec<&HostView> = hosts
        .iter()
        .filter(|h| h.lifecycle_state == "draining")
        .collect();
    if draining.is_empty() {
        return Ok(());
    }
    info!(
        backend_id = %backend_id,
        draining_count = draining.len(),
        "draining hosts found; computing plans"
    );
    for host in draining {
        let plan = match plan_decommission(
            host.id,
            hosts,
            replicas,
            |rs| rs.iter().map(|r| r.node_id).max().unwrap_or(0) + 1,
            |target| spdk_by_host.get(&target).copied(),
        ) {
            Ok(p) => p,
            Err(err) => {
                warn!(host_id = %host.id, error = %err, "drain plan refused; leaving host in 'draining' for operator");
                continue;
            }
        };
        if plan.steps.is_empty() {
            // Host had no replicas; safe to mark decommissioned.
            info!(host_id = %host.id, "drain plan empty; marking host decommissioned");
            mark_decommissioned(&ctx.pool, host.id).await?;
            continue;
        }
        info!(
            host_id = %host.id,
            steps = plan.steps.len(),
            "executing drain plan"
        );
        let run = execute(&ctx.manager_base, backend_id, plan, ctx.auth_header.as_deref()).await;
        log_run(host.id, &run);
        if run.ok {
            mark_decommissioned(&ctx.pool, host.id).await?;
        }
    }
    Ok(())
}

async fn promote_failed_hosts(
    ctx: &AutoReconcilerCtx,
    backend_id: Uuid,
    hosts: &[HostView],
    replicas: &[ReplicaView],
    spdk_by_host: &HashMap<Uuid, Uuid>,
) -> sqlx::Result<()> {
    // A host is a promotion candidate when:
    //   - it carries one or more raft_spdk replicas in this backend,
    //   - it has been unhealthy for >= PROMOTION_THRESHOLD,
    //   - its lifecycle_state is `active` (we don't auto-promote
    //     against draining/decommissioned hosts; the drain path
    //     handles those).
    //
    // We re-derive `unhealthy_for` from the host row's last_seen_at
    // because `HostView::healthy` is the binary 30s-threshold view.
    let now = chrono::Utc::now();
    let last_seen: HashMap<Uuid, chrono::DateTime<chrono::Utc>> =
        sqlx::query_as::<_, (Uuid, chrono::DateTime<chrono::Utc>)>(
            r#"SELECT id, last_seen_at FROM host"#,
        )
        .fetch_all(&ctx.pool)
        .await?
        .into_iter()
        .collect();
    let replicas_by_host: HashSet<Uuid> = replicas.iter().map(|r| r.host_id).collect();

    for host in hosts {
        if host.lifecycle_state != "active" {
            continue;
        }
        if !replicas_by_host.contains(&host.id) {
            continue;
        }
        let Some(last_ts) = last_seen.get(&host.id) else {
            continue;
        };
        let unhealthy_for = now.signed_duration_since(*last_ts);
        if unhealthy_for.num_seconds() < promotion_threshold().as_secs() as i64 {
            continue;
        }
        // Backoff check (tight scope so the std::sync::Mutex guard
        // never crosses an await — Send-safety constraint for the
        // tokio task this runs in).
        {
            let last_attempt = ctx
                .last_promotion_attempt
                .lock()
                .expect("auto-reconciler mutex poisoned");
            if let Some(prev_attempt) = last_attempt.get(&host.id) {
                if prev_attempt.elapsed() < promotion_backoff() {
                    debug!(host_id = %host.id, "skip promotion: still in backoff window");
                    continue;
                }
            }
        }

        let plan = match plan_hot_spare_promotion(
            host.id,
            hosts,
            replicas,
            |rs| rs.iter().map(|r| r.node_id).max().unwrap_or(0) + 1,
            |target| spdk_by_host.get(&target).copied(),
        ) {
            Ok(p) => p,
            Err(err) => {
                warn!(host_id = %host.id, error = %err, "promotion plan refused");
                ctx.last_promotion_attempt
                    .lock()
                    .expect("auto-reconciler mutex poisoned")
                    .insert(host.id, std::time::Instant::now());
                continue;
            }
        };
        if plan.steps.is_empty() {
            continue;
        }
        warn!(
            host_id = %host.id,
            unhealthy_for_seconds = unhealthy_for.num_seconds(),
            steps = plan.steps.len(),
            "host unhealthy past promotion threshold; promoting hot-spare"
        );
        ctx.last_promotion_attempt
            .lock()
            .expect("auto-reconciler mutex poisoned")
            .insert(host.id, std::time::Instant::now());

        let run = execute(&ctx.manager_base, backend_id, plan, ctx.auth_header.as_deref()).await;
        log_run(host.id, &run);
    }
    Ok(())
}

async fn mark_decommissioned(pool: &PgPool, host_id: Uuid) -> sqlx::Result<()> {
    sqlx::query(
        r#"
        UPDATE host
           SET lifecycle_state = 'decommissioned',
               lifecycle_changed_at = now()
         WHERE id = $1
           AND lifecycle_state = 'draining'
        "#,
    )
    .bind(host_id)
    .execute(pool)
    .await?;
    info!(host_id = %host_id, "host transitioned to decommissioned");
    Ok(())
}

fn log_run(host_id: Uuid, run: &PlanRun) {
    let succeeded = run
        .steps
        .iter()
        .filter(|s| s.status == StepStatus::Succeeded)
        .count();
    let failed = run
        .steps
        .iter()
        .filter(|s| s.status == StepStatus::Failed)
        .count();
    let skipped = run
        .steps
        .iter()
        .filter(|s| s.status == StepStatus::Skipped)
        .count();
    if run.ok {
        info!(
            host_id = %host_id,
            succeeded,
            elapsed_ms = run.total_elapsed_ms,
            "plan executed successfully"
        );
    } else {
        let first_error = run
            .steps
            .iter()
            .find(|s| s.status == StepStatus::Failed)
            .and_then(|s| s.error.clone())
            .unwrap_or_else(|| "unknown".into());
        error!(
            host_id = %host_id,
            succeeded,
            failed,
            skipped,
            first_error,
            elapsed_ms = run.total_elapsed_ms,
            "plan execution stopped on first failed step"
        );
    }
}
