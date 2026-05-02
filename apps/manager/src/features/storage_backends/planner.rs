//! B-III placement planner.
//!
//! Pure functions that compute the *plan* for membership changes. The
//! planner does not call any agent or Openraft RPC. It takes a snapshot
//! of the current cluster state (hosts, replicas) and returns a list of
//! ordered operations (`add_replica` / `remove_replica`) that an
//! operator (or the reconciler) executes through the existing routes.
//!
//! Splitting compute from execute lets the same logic power three
//! different operator surfaces:
//!
//! - **Decommission preview** (Task 6): "show me everything that has to
//!   move before host H can drain."
//! - **Hot-spare promotion preview** (Task 7): "host H is unhealthy;
//!   here's what failure recovery would do."
//! - **Rebalance preview** (Task 8): "load is skewed; here's how I'd
//!   move groups around to even it out."
//!
//! The planner is deliberately conservative: when in doubt, refuse to
//! emit a plan (operator sees an error, fixes the constraint, retries).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// One step in a plan. Order matters — execute top-to-bottom. Each step
/// must complete before the next begins because membership changes hold
/// a per-group advisory lock.
///
/// `TransferLeader` is reserved for the case where a `RemoveReplica`
/// targets the current leader; the current planner functions don't emit
/// it (operator removes the leader manually after a `transfer_leader`
/// API call), but the variant is here so future planner versions can.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[allow(dead_code)]
pub enum PlanStep {
    /// Add a new voter to a group on a target host. Used by all three
    /// surfaces — decommission and rebalance always add the replacement
    /// before they remove the old replica so the group's voter count
    /// stays at >= n/2 + 1 throughout.
    AddReplica {
        backend_id: Uuid,
        group_id: Uuid,
        target_host_id: Uuid,
        target_node_id: u64,
        target_agent_base_url: String,
        target_spdk_backend_id: Uuid,
    },
    /// Remove a voter from a group. The route layer already refuses to
    /// remove the leader without an explicit transfer, and refuses to
    /// drop below a 3-voter shape; the planner doesn't duplicate those
    /// checks but does ensure it never emits a remove without a paired
    /// add.
    RemoveReplica {
        backend_id: Uuid,
        group_id: Uuid,
        node_id: u64,
    },
    /// Transfer leadership before a `RemoveReplica`. Emitted only when
    /// the target of removal is the current leader.
    TransferLeader {
        backend_id: Uuid,
        group_id: Uuid,
        from_node_id: u64,
        to_node_id: u64,
    },
}

/// A planner output bundles the steps with the reasoning, so the
/// operator-facing surface can show *why* this plan was chosen.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub steps: Vec<PlanStep>,
    pub notes: Vec<String>,
}

/// View of a host the planner consumes. Decoupled from `HostRow` so
/// tests don't have to fabricate a full DB row.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct HostView {
    pub id: Uuid,
    pub addr: String,
    pub is_hot_spare: bool,
    pub lifecycle_state: String,
    pub healthy: bool,
    /// Number of raft_spdk replicas currently placed on this host.
    /// Used by the rebalance planner to pick the least-loaded target.
    /// (Currently unused; the planner re-computes from the replica list
    /// because that's the source of truth — kept here so future callers
    /// can pre-compute and pass through.)
    pub replica_count: usize,
}

impl HostView {
    /// Eligible as a placement target. Mirrors `list_healthy` semantics
    /// plus the rebalance constraint that hot-spares stay reserved for
    /// failure recovery, not normal placement.
    pub fn is_placement_target(&self) -> bool {
        self.healthy && !self.is_hot_spare && self.lifecycle_state == "active"
    }

    /// Eligible as a hot-spare promotion target.
    pub fn is_promotion_target(&self) -> bool {
        self.healthy && self.is_hot_spare && self.lifecycle_state == "active"
    }
}

/// View of a replica the planner consumes.
#[derive(Debug, Clone)]
pub struct ReplicaView {
    pub backend_id: Uuid,
    pub group_id: Uuid,
    pub node_id: u64,
    /// The host this replica's agent runs on. Resolved by the caller
    /// from `agent_base_url` against the host registry.
    pub host_id: Uuid,
}

/// Plan a host decommission: every group that has a replica on `host_id`
/// gets an add+remove pair, with the add targeting the best-available
/// hot-spare. If no hot-spare is available, returns an error so the
/// operator must add capacity before draining.
pub fn plan_decommission(
    host_id: Uuid,
    hosts: &[HostView],
    replicas: &[ReplicaView],
    pick_node_id: impl Fn(&[ReplicaView]) -> u64,
    spdk_backend_id_for_host: impl Fn(Uuid) -> Option<Uuid>,
) -> Result<Plan, String> {
    let target_replicas: Vec<&ReplicaView> =
        replicas.iter().filter(|r| r.host_id == host_id).collect();
    if target_replicas.is_empty() {
        return Ok(Plan {
            steps: vec![],
            notes: vec!["host has no raft_spdk replicas; lifecycle move is a no-op".into()],
        });
    }
    let spares: Vec<&HostView> = hosts.iter().filter(|h| h.is_promotion_target()).collect();
    if spares.is_empty() {
        return Err(
            "decommission refused: host has raft_spdk replicas and no healthy hot-spare is available"
                .into(),
        );
    }

    let mut steps = Vec::new();
    let mut notes = Vec::new();
    let mut spare_replica_count: Vec<(Uuid, usize)> = spares
        .iter()
        .map(|h| (h.id, count_for(replicas, h.id)))
        .collect();

    for replica in &target_replicas {
        // Pick the spare with the lightest current load so we don't
        // pile every drained replica onto the first spare.
        spare_replica_count.sort_by_key(|(_, count)| *count);
        let (target_host_id, _) = spare_replica_count[0];
        let target_host = spares
            .iter()
            .find(|h| h.id == target_host_id)
            .expect("spare in list");
        let new_node_id = pick_node_id(replicas);
        let spdk_backend_id = spdk_backend_id_for_host(target_host.id).ok_or_else(|| {
            format!(
                "host {target_host_id} has no spdk_backend_id configured; cannot host raft_spdk replicas"
            )
        })?;
        steps.push(PlanStep::AddReplica {
            backend_id: replica.backend_id,
            group_id: replica.group_id,
            target_host_id,
            target_node_id: new_node_id,
            target_agent_base_url: target_host.addr.clone(),
            target_spdk_backend_id: spdk_backend_id,
        });
        steps.push(PlanStep::RemoveReplica {
            backend_id: replica.backend_id,
            group_id: replica.group_id,
            node_id: replica.node_id,
        });
        // Update the running count so the next iteration picks a fresh spare
        // when this one fills up.
        if let Some(entry) = spare_replica_count
            .iter_mut()
            .find(|(id, _)| *id == target_host_id)
        {
            entry.1 += 1;
        }
    }
    notes.push(format!(
        "draining {} replica(s) from host {host_id} onto {} hot-spare(s)",
        target_replicas.len(),
        spares.len()
    ));
    Ok(Plan { steps, notes })
}

/// Plan a hot-spare promotion: same shape as `plan_decommission` but
/// triggered by health, not operator action. The failed host remains
/// in the locator until an operator removes it (so post-recovery
/// the original replica is still discoverable), but a hot-spare is
/// added to keep quorum alive.
pub fn plan_hot_spare_promotion(
    failed_host_id: Uuid,
    hosts: &[HostView],
    replicas: &[ReplicaView],
    pick_node_id: impl Fn(&[ReplicaView]) -> u64,
    spdk_backend_id_for_host: impl Fn(Uuid) -> Option<Uuid>,
) -> Result<Plan, String> {
    let affected: Vec<&ReplicaView> = replicas
        .iter()
        .filter(|r| r.host_id == failed_host_id)
        .collect();
    if affected.is_empty() {
        return Ok(Plan {
            steps: vec![],
            notes: vec!["failed host has no raft_spdk replicas; nothing to promote".into()],
        });
    }
    let spares: Vec<&HostView> = hosts.iter().filter(|h| h.is_promotion_target()).collect();
    if spares.is_empty() {
        return Err("hot-spare promotion refused: no healthy hot-spare available".into());
    }

    let mut steps = Vec::new();
    let mut spare_replica_count: Vec<(Uuid, usize)> = spares
        .iter()
        .map(|h| (h.id, count_for(replicas, h.id)))
        .collect();

    for replica in &affected {
        spare_replica_count.sort_by_key(|(_, count)| *count);
        let (target_host_id, _) = spare_replica_count[0];
        let target_host = spares
            .iter()
            .find(|h| h.id == target_host_id)
            .expect("spare in list");
        let new_node_id = pick_node_id(replicas);
        let spdk_backend_id = spdk_backend_id_for_host(target_host.id)
            .ok_or_else(|| format!("host {target_host_id} has no spdk_backend_id configured"))?;
        steps.push(PlanStep::AddReplica {
            backend_id: replica.backend_id,
            group_id: replica.group_id,
            target_host_id,
            target_node_id: new_node_id,
            target_agent_base_url: target_host.addr.clone(),
            target_spdk_backend_id: spdk_backend_id,
        });
        // Note: we deliberately do NOT emit a RemoveReplica for the
        // failed host. The host might come back; the operator decides
        // to remove the orphan via the manual API once recovery is done.
        if let Some(entry) = spare_replica_count
            .iter_mut()
            .find(|(id, _)| *id == target_host_id)
        {
            entry.1 += 1;
        }
    }
    Ok(Plan {
        steps,
        notes: vec![format!(
            "promoting hot-spare to cover {} replica(s) lost on host {failed_host_id}",
            affected.len()
        )],
    })
}

/// Plan a rebalance: minimize variance of replica count across active
/// (non-spare, non-draining) hosts. Each move is an add+remove pair on
/// the same group so quorum is never reduced.
pub fn plan_rebalance(
    backend_id: Uuid,
    hosts: &[HostView],
    replicas: &[ReplicaView],
    pick_node_id: impl Fn(&[ReplicaView]) -> u64,
    spdk_backend_id_for_host: impl Fn(Uuid) -> Option<Uuid>,
) -> Result<Plan, String> {
    let placeable: Vec<&HostView> = hosts.iter().filter(|h| h.is_placement_target()).collect();
    if placeable.len() < 2 {
        return Ok(Plan {
            steps: vec![],
            notes: vec![format!(
                "rebalance no-op: only {} placeable host(s)",
                placeable.len()
            )],
        });
    }

    let mut counts: Vec<(Uuid, String, usize)> = placeable
        .iter()
        .map(|h| (h.id, h.addr.clone(), count_for(replicas, h.id)))
        .collect();
    counts.sort_by_key(|(_, _, count)| *count);

    let total: usize = counts.iter().map(|(_, _, c)| c).sum();
    let target = total / counts.len();
    let max_observed = counts.last().map(|(_, _, c)| *c).unwrap_or(0);
    if max_observed.saturating_sub(target) <= 1 {
        return Ok(Plan {
            steps: vec![],
            notes: vec![format!(
                "rebalance no-op: per-host load already balanced (max {max_observed}, target {target})"
            )],
        });
    }

    let mut steps = Vec::new();
    // For each over-loaded host, move one replica per iteration to the
    // currently-least-loaded host until the variance is acceptable.
    let mut iterations = 0;
    let max_iterations = (counts.len() * counts.len()).max(8);
    loop {
        if iterations >= max_iterations {
            break;
        }
        iterations += 1;
        counts.sort_by_key(|(_, _, count)| *count);
        let min_idx = 0;
        let max_idx = counts.len() - 1;
        let (max_host, _, max_count) = &counts[max_idx];
        let (min_host, min_addr, min_count) = &counts[min_idx];
        if max_count.saturating_sub(*min_count) <= 1 {
            break;
        }

        // Pick a replica on max_host that the min_host doesn't already
        // host (no two replicas of the same group on the same host).
        let groups_on_min: std::collections::HashSet<Uuid> = replicas
            .iter()
            .filter(|r| r.host_id == *min_host && r.backend_id == backend_id)
            .map(|r| r.group_id)
            .collect();
        let candidate = replicas.iter().find(|r| {
            r.host_id == *max_host
                && r.backend_id == backend_id
                && !groups_on_min.contains(&r.group_id)
        });
        let Some(replica) = candidate else { break };

        let target_host_id = *min_host;
        let target_addr = min_addr.clone();
        let new_node_id = pick_node_id(replicas);
        let spdk_backend_id = spdk_backend_id_for_host(target_host_id)
            .ok_or_else(|| format!("host {target_host_id} has no spdk_backend_id configured"))?;
        steps.push(PlanStep::AddReplica {
            backend_id: replica.backend_id,
            group_id: replica.group_id,
            target_host_id,
            target_node_id: new_node_id,
            target_agent_base_url: target_addr,
            target_spdk_backend_id: spdk_backend_id,
        });
        steps.push(PlanStep::RemoveReplica {
            backend_id: replica.backend_id,
            group_id: replica.group_id,
            node_id: replica.node_id,
        });
        counts[min_idx].2 += 1;
        counts[max_idx].2 -= 1;
    }
    let notes = if steps.is_empty() {
        vec!["rebalance no-op: no compatible move found (every replica is co-located with min-load host)".into()]
    } else {
        vec![format!(
            "rebalance: {} migration(s), {} hosts affected",
            steps.len() / 2,
            counts.len()
        )]
    };
    Ok(Plan { steps, notes })
}

fn count_for(replicas: &[ReplicaView], host_id: Uuid) -> usize {
    replicas.iter().filter(|r| r.host_id == host_id).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host(id_byte: u8, hot_spare: bool, lifecycle: &str) -> HostView {
        let mut bytes = [0u8; 16];
        bytes[0] = id_byte;
        HostView {
            id: Uuid::from_bytes(bytes),
            addr: format!("http://10.0.0.{id_byte}:9090"),
            is_hot_spare: hot_spare,
            lifecycle_state: lifecycle.into(),
            healthy: true,
            replica_count: 0,
        }
    }

    fn replica(group_byte: u8, node_id: u64, host_id_byte: u8) -> ReplicaView {
        let mut group_bytes = [0u8; 16];
        group_bytes[0] = group_byte;
        let mut host_bytes = [0u8; 16];
        host_bytes[0] = host_id_byte;
        ReplicaView {
            backend_id: Uuid::from_u128(1),
            group_id: Uuid::from_bytes(group_bytes),
            node_id,
            host_id: Uuid::from_bytes(host_bytes),
        }
    }

    fn pick_const(value: u64) -> impl Fn(&[ReplicaView]) -> u64 {
        move |_replicas| value
    }

    fn const_spdk_backend(id: Uuid) -> impl Fn(Uuid) -> Option<Uuid> {
        move |_host| Some(id)
    }

    #[test]
    fn decommission_with_no_replicas_is_noop() {
        let hosts = vec![host(1, false, "draining"), host(9, true, "active")];
        let replicas = vec![replica(0xAA, 1, 5)]; // not on host 1
        let plan = plan_decommission(
            hosts[0].id,
            &hosts,
            &replicas,
            pick_const(99),
            const_spdk_backend(Uuid::from_u128(2)),
        )
        .unwrap();
        assert!(plan.steps.is_empty());
    }

    #[test]
    fn decommission_with_no_spare_refuses() {
        let hosts = vec![host(1, false, "draining"), host(2, false, "active")];
        let replicas = vec![replica(0xAA, 1, 1)]; // on host 1
        let err = plan_decommission(
            hosts[0].id,
            &hosts,
            &replicas,
            pick_const(99),
            const_spdk_backend(Uuid::from_u128(2)),
        )
        .unwrap_err();
        assert!(err.contains("no healthy hot-spare"));
    }

    #[test]
    fn decommission_emits_add_then_remove_paired_per_group() {
        let hosts = vec![
            host(1, false, "draining"),
            host(2, false, "active"),
            host(9, true, "active"), // hot spare
        ];
        let replicas = vec![replica(0xAA, 1, 1), replica(0xBB, 1, 1)];
        let plan = plan_decommission(
            hosts[0].id,
            &hosts,
            &replicas,
            pick_const(99),
            const_spdk_backend(Uuid::from_u128(2)),
        )
        .unwrap();
        assert_eq!(plan.steps.len(), 4);
        assert!(matches!(plan.steps[0], PlanStep::AddReplica { .. }));
        assert!(matches!(plan.steps[1], PlanStep::RemoveReplica { .. }));
        assert!(matches!(plan.steps[2], PlanStep::AddReplica { .. }));
        assert!(matches!(plan.steps[3], PlanStep::RemoveReplica { .. }));
    }

    #[test]
    fn promotion_does_not_remove_failed_replica() {
        let hosts = vec![
            host(1, false, "active"), // failed host (still listed)
            host(2, false, "active"),
            host(9, true, "active"),
        ];
        let replicas = vec![replica(0xAA, 1, 1)];
        let plan = plan_hot_spare_promotion(
            hosts[0].id,
            &hosts,
            &replicas,
            pick_const(99),
            const_spdk_backend(Uuid::from_u128(2)),
        )
        .unwrap();
        assert_eq!(plan.steps.len(), 1);
        assert!(matches!(plan.steps[0], PlanStep::AddReplica { .. }));
        assert!(plan
            .steps
            .iter()
            .all(|s| !matches!(s, PlanStep::RemoveReplica { .. })));
    }

    #[test]
    fn rebalance_balanced_cluster_is_noop() {
        let hosts = vec![
            host(1, false, "active"),
            host(2, false, "active"),
            host(3, false, "active"),
        ];
        // 3 replicas, one per host: balanced
        let replicas = vec![
            replica(0xAA, 1, 1),
            replica(0xAA, 2, 2),
            replica(0xAA, 3, 3),
        ];
        let plan = plan_rebalance(
            Uuid::from_u128(1),
            &hosts,
            &replicas,
            pick_const(99),
            const_spdk_backend(Uuid::from_u128(2)),
        )
        .unwrap();
        assert!(plan.steps.is_empty());
    }

    #[test]
    fn rebalance_skewed_cluster_emits_moves() {
        let hosts = vec![
            host(1, false, "active"),
            host(2, false, "active"),
            host(3, false, "active"),
        ];
        // host 1 has 3 groups, hosts 2 and 3 have 0 each: needs moves
        let replicas = vec![
            replica(0xAA, 1, 1),
            replica(0xBB, 2, 1),
            replica(0xCC, 3, 1),
        ];
        let plan = plan_rebalance(
            Uuid::from_u128(1),
            &hosts,
            &replicas,
            pick_const(99),
            const_spdk_backend(Uuid::from_u128(2)),
        )
        .unwrap();
        // Expect at least 2 add+remove pairs to drop host 1 from 3 -> 1.
        assert!(plan.steps.len() >= 4, "got: {:?}", plan.steps);
    }
}
