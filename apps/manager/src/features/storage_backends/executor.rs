//! B-III plan executor.
//!
//! Walks a `Plan` produced by `planner` and executes each step against
//! the manager's own HTTP API. Each step is one of:
//!
//! - `AddReplica` → `POST /v1/storage_backends/{id}/groups/{group_id}/replicas`
//! - `RemoveReplica` → `DELETE /v1/storage_backends/{id}/groups/{group_id}/replicas/{node_id}`
//! - `TransferLeader` → not yet wired (Task 4a's endpoint exists; the
//!   planner doesn't currently emit this step but the executor knows
//!   how to dispatch it for future planner versions).
//!
//! Self-HTTP rather than direct function calls keeps the existing route
//! orchestration as the single source of truth for the per-step
//! invariants (advisory locks, repair-queue rows, locator updates).
//! Refactoring into a shared library would duplicate or complicate that
//! contract; HTTP is a clean boundary that already enforces it.
//!
//! Failure semantics: stop on the first failed step. The plan is not
//! transactional — partially-applied plans leave the cluster in a
//! coherent intermediate state (every committed step ran through its
//! own membership-change ratification) and the operator can inspect
//! `/v1/storage_backends/{id}/repair_queue` to see what landed and
//! re-issue the rest.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::features::storage_backends::planner::{Plan, PlanStep};

/// One step's outcome reported back to the operator.
#[derive(Debug, Clone, Serialize)]
pub struct StepReport {
    pub index: usize,
    pub step: PlanStep,
    pub status: StepStatus,
    pub error: Option<String>,
    pub elapsed_ms: u128,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Succeeded,
    Failed,
    /// Skipped because an earlier step failed; the operator decides
    /// whether to re-issue the plan after fixing the underlying cause.
    Skipped,
}

/// Run-level summary the executor returns when finished.
#[derive(Debug, Clone, Serialize)]
pub struct PlanRun {
    pub backend_id: Uuid,
    pub steps: Vec<StepReport>,
    pub total_elapsed_ms: u128,
    /// `true` when every step succeeded.
    pub ok: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AddReplicaSelfBody {
    pub node_id: u64,
    pub agent_base_url: String,
    pub spdk_backend_id: Uuid,
}

/// Execute every step of `plan` against the manager's own HTTP API.
/// `manager_base` is the URL the manager listens on (typically
/// `http://127.0.0.1:18080`); using the loopback URL keeps the
/// transport simple and avoids a second auth round-trip.
pub async fn execute(
    manager_base: &str,
    backend_id: Uuid,
    plan: Plan,
    auth_header: Option<&str>,
) -> PlanRun {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .expect("reqwest client builder always succeeds with these defaults");

    let start = std::time::Instant::now();
    let mut reports: Vec<StepReport> = Vec::with_capacity(plan.steps.len());
    let mut aborted = false;

    for (idx, step) in plan.steps.iter().enumerate() {
        if aborted {
            reports.push(StepReport {
                index: idx,
                step: step.clone(),
                status: StepStatus::Skipped,
                error: None,
                elapsed_ms: 0,
            });
            continue;
        }
        let step_start = std::time::Instant::now();
        let result = run_step(&client, manager_base, backend_id, step, auth_header).await;
        let elapsed_ms = step_start.elapsed().as_millis();
        match result {
            Ok(()) => reports.push(StepReport {
                index: idx,
                step: step.clone(),
                status: StepStatus::Succeeded,
                error: None,
                elapsed_ms,
            }),
            Err(error) => {
                reports.push(StepReport {
                    index: idx,
                    step: step.clone(),
                    status: StepStatus::Failed,
                    error: Some(error),
                    elapsed_ms,
                });
                aborted = true;
            }
        }
    }

    PlanRun {
        backend_id,
        ok: reports.iter().all(|r| r.status == StepStatus::Succeeded),
        steps: reports,
        total_elapsed_ms: start.elapsed().as_millis(),
    }
}

async fn run_step(
    client: &reqwest::Client,
    manager_base: &str,
    backend_id: Uuid,
    step: &PlanStep,
    auth_header: Option<&str>,
) -> Result<(), String> {
    match step {
        PlanStep::AddReplica {
            backend_id: step_backend,
            group_id,
            target_node_id,
            target_agent_base_url,
            target_spdk_backend_id,
            ..
        } => {
            if *step_backend != backend_id {
                return Err(format!(
                    "step targets backend {step_backend} but executor was called for {backend_id}"
                ));
            }
            let url = format!(
                "{}/v1/storage_backends/{backend_id}/groups/{group_id}/replicas",
                manager_base.trim_end_matches('/')
            );
            let body = AddReplicaSelfBody {
                node_id: *target_node_id,
                agent_base_url: target_agent_base_url.clone(),
                spdk_backend_id: *target_spdk_backend_id,
            };
            send_with_auth(client, client.post(&url).json(&body), auth_header).await
        }
        PlanStep::RemoveReplica {
            backend_id: step_backend,
            group_id,
            node_id,
        } => {
            if *step_backend != backend_id {
                return Err(format!(
                    "step targets backend {step_backend} but executor was called for {backend_id}"
                ));
            }
            let url = format!(
                "{}/v1/storage_backends/{backend_id}/groups/{group_id}/replicas/{node_id}",
                manager_base.trim_end_matches('/')
            );
            send_with_auth(client, client.delete(&url), auth_header).await
        }
        PlanStep::TransferLeader { .. } => {
            // Reserved for the future planner that emits this step
            // before a leader-removing RemoveReplica. The endpoint
            // (Task 4a) exists; the wiring is intentionally not enabled
            // yet so callers don't accidentally trigger a leader
            // transfer that the planner shouldn't have asked for.
            Err("TransferLeader step not yet executed by the orchestrator".into())
        }
    }
}

async fn send_with_auth(
    _client: &reqwest::Client,
    mut req: reqwest::RequestBuilder,
    auth_header: Option<&str>,
) -> Result<(), String> {
    if let Some(h) = auth_header {
        req = req.header(reqwest::header::AUTHORIZATION, h);
    }
    let resp = req.send().await.map_err(|e| format!("dispatch: {e}"))?;
    let status = resp.status();
    if status.is_success() {
        return Ok(());
    }
    let body = resp.text().await.unwrap_or_default();
    Err(format!("step returned {status}: {body}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_summary_succeeds_only_when_every_step_succeeded() {
        let mut run = PlanRun {
            backend_id: Uuid::nil(),
            steps: vec![],
            total_elapsed_ms: 0,
            ok: true,
        };
        run.steps.push(StepReport {
            index: 0,
            step: PlanStep::RemoveReplica {
                backend_id: Uuid::nil(),
                group_id: Uuid::nil(),
                node_id: 1,
            },
            status: StepStatus::Succeeded,
            error: None,
            elapsed_ms: 0,
        });
        run.steps.push(StepReport {
            index: 1,
            step: PlanStep::RemoveReplica {
                backend_id: Uuid::nil(),
                group_id: Uuid::nil(),
                node_id: 2,
            },
            status: StepStatus::Failed,
            error: Some("nope".into()),
            elapsed_ms: 0,
        });
        run.ok = run.steps.iter().all(|r| r.status == StepStatus::Succeeded);
        assert!(!run.ok);
    }
}
