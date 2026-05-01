//! Raft-replicated SPDK control-plane scaffold.
//!
//! B-II must not claim a production data path before raftblk/Openraft is wired.
//! This backend validates static placement and exposes the future capability
//! shape while returning NotSupported for mutating lifecycle calls.

use nexus_storage::{
    BackendInstanceId, BackendKind, Capabilities, ControlPlaneBackend, CreateOpts, RaftSpdkLocator,
    RaftSpdkReplicaLocator, StorageError, VolumeHandle, VolumeSnapshotHandle,
    RAFT_SPDK_DEFAULT_BLOCK_SIZE, RAFT_SPDK_STATIC_REPLICA_COUNT,
};
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
pub struct RaftSpdkConfig {
    #[serde(default = "default_block_size")]
    pub block_size: u64,
    /// B-II prototype path: `provision` creates raft-block groups on each
    /// agent but does NOT start the Openraft runtime. The locator carries
    /// `prototype_replica: true` so attach refuses to forward guest writes.
    /// Only set this for the harness test.
    #[serde(default)]
    pub prototype_provisioning_enabled: bool,
    /// B-II production path: `provision` creates raft-block groups, starts
    /// an Openraft runtime on each agent with the full peer URL map,
    /// initializes membership on the leader, and waits for the leader to
    /// elect itself. The locator does NOT carry `prototype_replica`, so
    /// attach forwards guest writes through the production raftblk daemon
    /// (when wired). This is the real B-II provisioning path.
    #[serde(default)]
    pub production_provisioning_enabled: bool,
    pub replicas: Vec<RaftSpdkReplicaConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RaftSpdkReplicaConfig {
    pub node_id: u64,
    pub agent_base_url: String,
    pub spdk_backend_id: uuid::Uuid,
}

fn default_block_size() -> u64 {
    RAFT_SPDK_DEFAULT_BLOCK_SIZE
}

pub struct RaftSpdkControlPlaneBackend {
    pub id: BackendInstanceId,
    pub config: RaftSpdkConfig,
    http: reqwest::Client,
}

impl RaftSpdkControlPlaneBackend {
    pub fn new(id: BackendInstanceId, config: RaftSpdkConfig) -> Result<Self, StorageError> {
        validate_config(&config)?;
        Ok(Self {
            id,
            config,
            http: reqwest::Client::new(),
        })
    }

    fn raft_block_url(replica: &RaftSpdkReplicaConfig, path: &str) -> String {
        // The TOML's `agent_base_url` is the FULL base for the raft-block
        // routes — typically `http://host:port/v1/raft_block`. We don't
        // re-add the prefix here. This keeps the value in lockstep with
        // the locator's `agent_base_url` that flows into the agent's
        // RaftBlockNetworkFactory; both the manager (this fn) and the
        // network factory consume it identically.
        let base = replica.agent_base_url.trim_end_matches('/');
        let suffix = path.trim_start_matches('/');
        format!("{base}/{suffix}")
    }

    async fn create_remote_group(
        &self,
        replica: &RaftSpdkReplicaConfig,
        group_id: Uuid,
        size_bytes: u64,
        desired_store_kind: &'static str,
    ) -> Result<(), StorageError> {
        let req = CreateRaftBlockGroupReq {
            group_id,
            node_id: replica.node_id,
            capacity_bytes: size_bytes,
            block_size: self.config.block_size,
            desired_store_kind: Some(desired_store_kind),
        };
        let response = self
            .http
            .post(Self::raft_block_url(replica, "create"))
            .json(&req)
            .send()
            .await
            .map_err(StorageError::backend)?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(StorageError::backend(std::io::Error::other(format!(
                "raft_spdk create group on node {} failed with {status}: {body}",
                replica.node_id
            ))));
        }
        Ok(())
    }

    async fn stop_remote_group(&self, replica: &RaftSpdkReplicaConfig, group_id: Uuid) {
        let _ = self
            .stop_remote_group_url(replica.node_id, &replica.agent_base_url, group_id)
            .await;
    }

    async fn stop_remote_group_url(
        &self,
        node_id: u64,
        agent_base_url: &str,
        group_id: Uuid,
    ) -> Result<(), StorageError> {
        let url = format!("{}/{}", agent_base_url.trim_end_matches('/'), "stop");
        let response = self
            .http
            .post(url)
            .json(&StopRaftBlockGroupReq { group_id })
            .send()
            .await
            .map_err(StorageError::backend)?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(StorageError::backend(std::io::Error::other(format!(
                "raft_spdk stop group on node {node_id} failed with {status}: {body}"
            ))));
        }
        Ok(())
    }

    async fn destroy_remote_group_url(
        &self,
        node_id: u64,
        agent_base_url: &str,
        group_id: Uuid,
    ) -> Result<(), StorageError> {
        let url = format!("{}/{}", agent_base_url.trim_end_matches('/'), "destroy");
        let response = self
            .http
            .post(url)
            .json(&DestroyRaftBlockGroupReq { group_id })
            .send()
            .await
            .map_err(StorageError::backend)?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(StorageError::backend(std::io::Error::other(format!(
                "raft_spdk destroy group on node {node_id} failed with {status}: {body}"
            ))));
        }
        Ok(())
    }

    /// Start an Openraft runtime on `replica` for `group_id`, with the full
    /// peer URL map. Followers learn membership from the leader's
    /// initialize call; this just gets the runtime registered atop the
    /// pre-existing storage so it can receive append_entries/vote RPCs.
    async fn start_remote_runtime(
        &self,
        replica: &RaftSpdkReplicaConfig,
        group_id: Uuid,
        peers: &std::collections::HashMap<u64, String>,
    ) -> Result<(), StorageError> {
        let req = serde_json::json!({
            "group_id": group_id,
            "peers": peers,
        });
        let response = self
            .http
            .post(Self::raft_block_url(replica, "runtime_start"))
            .json(&req)
            .send()
            .await
            .map_err(StorageError::backend)?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(StorageError::backend(std::io::Error::other(format!(
                "raft_spdk runtime_start on node {} failed with {status}: {body}",
                replica.node_id
            ))));
        }
        Ok(())
    }

    /// Bootstrap the cluster's membership on `replica`. Must only be called
    /// on the chosen leader (typically `replicas[0]`); followers learn
    /// membership through subsequent append_entries.
    async fn initialize_remote_membership(
        &self,
        replica: &RaftSpdkReplicaConfig,
        group_id: Uuid,
        members: &[u64],
    ) -> Result<(), StorageError> {
        let req = serde_json::json!({
            "group_id": group_id,
            "members": members,
        });
        let response = self
            .http
            .post(Self::raft_block_url(replica, "runtime_initialize"))
            .json(&req)
            .send()
            .await
            .map_err(StorageError::backend)?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(StorageError::backend(std::io::Error::other(format!(
                "raft_spdk runtime_initialize on node {} failed with {status}: {body}",
                replica.node_id
            ))));
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl ControlPlaneBackend for RaftSpdkControlPlaneBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::RaftSpdk
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            supports_native_snapshots: true,
            supports_concurrent_attach: false,
            supports_live_migration: false,
            supports_clone_from_image: false,
        }
    }

    async fn provision(&self, opts: CreateOpts) -> Result<VolumeHandle, StorageError> {
        let prototype = self.config.prototype_provisioning_enabled;
        let production = self.config.production_provisioning_enabled;
        if !prototype && !production {
            return Err(StorageError::NotSupported(format!(
                "raft_spdk backend {} with {} replicas awaits provisioning; set production_provisioning_enabled to bootstrap a real Openraft group, or prototype_provisioning_enabled for the B-II harness path",
                self.id.0,
                self.config.replicas.len()
            )));
        }
        if prototype && production {
            return Err(StorageError::InvalidLocator(
                "raft_spdk: prototype_provisioning_enabled and production_provisioning_enabled are mutually exclusive".into(),
            ));
        }
        if opts.size_bytes == 0 || !opts.size_bytes.is_multiple_of(self.config.block_size) {
            return Err(StorageError::InvalidLocator(format!(
                "raft_spdk volume size must be a nonzero multiple of block_size {}",
                self.config.block_size
            )));
        }

        let group_id = Uuid::new_v4();
        let mut created: Vec<&RaftSpdkReplicaConfig> = Vec::new();
        for replica in &self.config.replicas {
            if let Err(err) = self
                .create_remote_group(
                    replica,
                    group_id,
                    opts.size_bytes,
                    if production { "spdk_lvol" } else { "sidecar" },
                )
                .await
            {
                for created_replica in &created {
                    self.stop_remote_group(created_replica, group_id).await;
                }
                return Err(err);
            }
            created.push(replica);
        }

        // Production path: also bootstrap the Openraft runtime + membership.
        if production {
            let peers: std::collections::HashMap<u64, String> = self
                .config
                .replicas
                .iter()
                .map(|r| (r.node_id, r.agent_base_url.clone()))
                .collect();
            for replica in &self.config.replicas {
                if let Err(err) = self.start_remote_runtime(replica, group_id, &peers).await {
                    for created_replica in &created {
                        self.stop_remote_group(created_replica, group_id).await;
                    }
                    return Err(err);
                }
            }
            // Bootstrap membership on the first replica (node_id is whatever
            // the operator put first in the TOML config). Followers learn
            // through subsequent append_entries.
            let leader = &self.config.replicas[0];
            let members: Vec<u64> = self.config.replicas.iter().map(|r| r.node_id).collect();
            if let Err(err) = self
                .initialize_remote_membership(leader, group_id, &members)
                .await
            {
                for created_replica in &created {
                    self.stop_remote_group(created_replica, group_id).await;
                }
                return Err(err);
            }
        }

        let prototype_marker = prototype;
        let locator = RaftSpdkLocator::new(
            group_id,
            opts.size_bytes,
            self.config.block_size,
            self.config
                .replicas
                .iter()
                .map(|replica| RaftSpdkReplicaLocator {
                    node_id: replica.node_id,
                    agent_base_url: replica.agent_base_url.clone(),
                    spdk_lvol_locator: if prototype_marker {
                        serde_json::json!({
                            "spdk_backend_id": replica.spdk_backend_id,
                            "prototype_replica": true
                        })
                        .to_string()
                    } else {
                        serde_json::json!({
                            "spdk_backend_id": replica.spdk_backend_id,
                            "production_replica": true
                        })
                        .to_string()
                    },
                })
                .collect(),
            self.config.replicas.first().map(|replica| replica.node_id),
        )?;

        Ok(VolumeHandle {
            volume_id: Uuid::new_v4(),
            backend_id: self.id,
            backend_kind: BackendKind::RaftSpdk,
            locator: locator.to_locator_string()?,
            size_bytes: opts.size_bytes,
        })
    }

    async fn destroy(&self, handle: VolumeHandle) -> Result<(), StorageError> {
        let locator = RaftSpdkLocator::from_locator_str(&handle.locator)?;
        let mut errors = Vec::new();
        for replica in &locator.replicas {
            if let Err(err) = self
                .destroy_remote_group_url(
                    replica.node_id,
                    &replica.agent_base_url,
                    locator.group_id,
                )
                .await
            {
                errors.push(err.to_string());
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(StorageError::backend(std::io::Error::other(format!(
                "raft_spdk destroy stopped with replica errors: {}",
                errors.join("; ")
            ))))
        }
    }

    async fn clone_from_image(
        &self,
        _source_image: &Path,
        _opts: CreateOpts,
    ) -> Result<VolumeHandle, StorageError> {
        Err(StorageError::NotSupported(
            "raft_spdk clone_from_image must write through Raft".into(),
        ))
    }

    async fn snapshot(
        &self,
        _volume: &VolumeHandle,
        _name: &str,
    ) -> Result<VolumeSnapshotHandle, StorageError> {
        Err(StorageError::NotSupported(
            "raft_spdk snapshot awaits consistent Raft snapshot export".into(),
        ))
    }

    async fn clone_from_snapshot(
        &self,
        _snap: &VolumeSnapshotHandle,
    ) -> Result<VolumeHandle, StorageError> {
        Err(StorageError::NotSupported(
            "raft_spdk clone_from_snapshot awaits Raft snapshot import".into(),
        ))
    }

    async fn delete_snapshot(&self, _snap: VolumeSnapshotHandle) -> Result<(), StorageError> {
        Err(StorageError::NotSupported(
            "raft_spdk delete_snapshot awaits Raft snapshot metadata".into(),
        ))
    }
}

#[derive(Debug, Serialize)]
struct CreateRaftBlockGroupReq {
    group_id: Uuid,
    node_id: u64,
    capacity_bytes: u64,
    block_size: u64,
    desired_store_kind: Option<&'static str>,
}

#[derive(Debug, Serialize)]
struct StopRaftBlockGroupReq {
    group_id: Uuid,
}

#[derive(Debug, Serialize)]
struct DestroyRaftBlockGroupReq {
    group_id: Uuid,
}

pub fn validate_config(config: &RaftSpdkConfig) -> Result<(), StorageError> {
    if config.block_size == 0 {
        return Err(StorageError::InvalidLocator(
            "raft_spdk config.block_size must be nonzero".into(),
        ));
    }
    let n = config.replicas.len();
    if n != 1 && n != RAFT_SPDK_STATIC_REPLICA_COUNT {
        return Err(StorageError::InvalidLocator(format!(
            "raft_spdk requires 1 or {RAFT_SPDK_STATIC_REPLICA_COUNT} static replicas (got {n})"
        )));
    }
    let mut node_ids = std::collections::BTreeSet::new();
    for replica in &config.replicas {
        if replica.node_id == 0 {
            return Err(StorageError::InvalidLocator(
                "raft_spdk replica node_id must be nonzero".into(),
            ));
        }
        if !node_ids.insert(replica.node_id) {
            return Err(StorageError::InvalidLocator(format!(
                "raft_spdk duplicate replica node_id {}",
                replica.node_id
            )));
        }
        if replica.agent_base_url.trim().is_empty() {
            return Err(StorageError::InvalidLocator(
                "raft_spdk replica agent_base_url must not be empty".into(),
            ));
        }
        if replica.spdk_backend_id.is_nil() {
            return Err(StorageError::InvalidLocator(
                "raft_spdk replica spdk_backend_id must not be nil".into(),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> RaftSpdkConfig {
        RaftSpdkConfig {
            block_size: 512,
            prototype_provisioning_enabled: false,
            production_provisioning_enabled: false,
            replicas: vec![
                RaftSpdkReplicaConfig {
                    node_id: 1,
                    agent_base_url: "http://agent-1:19090".into(),
                    spdk_backend_id: uuid::Uuid::new_v4(),
                },
                RaftSpdkReplicaConfig {
                    node_id: 2,
                    agent_base_url: "http://agent-2:19090".into(),
                    spdk_backend_id: uuid::Uuid::new_v4(),
                },
                RaftSpdkReplicaConfig {
                    node_id: 3,
                    agent_base_url: "http://agent-3:19090".into(),
                    spdk_backend_id: uuid::Uuid::new_v4(),
                },
            ],
        }
    }

    #[test]
    fn validates_three_static_replicas() {
        validate_config(&cfg()).unwrap();
    }

    #[test]
    fn rejects_duplicate_replica_node_ids() {
        let mut cfg = cfg();
        cfg.replicas[2].node_id = 2;
        let err = validate_config(&cfg).unwrap_err();
        assert!(err.to_string().contains("duplicate"));
    }

    #[tokio::test]
    async fn provision_is_guarded_until_data_path_exists() {
        let backend =
            RaftSpdkControlPlaneBackend::new(BackendInstanceId(uuid::Uuid::new_v4()), cfg())
                .unwrap();
        let err = backend
            .provision(CreateOpts {
                name: "vol".into(),
                size_bytes: 4096,
                description: None,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, StorageError::NotSupported(_)));
    }

    #[tokio::test]
    async fn prototype_provisioning_creates_static_agent_groups_and_locator() {
        async fn record(
            axum::extract::State(calls): axum::extract::State<
                std::sync::Arc<tokio::sync::Mutex<Vec<serde_json::Value>>>,
            >,
            axum::Json(body): axum::Json<serde_json::Value>,
        ) -> axum::Json<serde_json::Value> {
            calls.lock().await.push(body);
            axum::Json(serde_json::json!({}))
        }

        async fn spawn_agent() -> (
            String,
            std::sync::Arc<tokio::sync::Mutex<Vec<serde_json::Value>>>,
            tokio::task::JoinHandle<()>,
        ) {
            let calls = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new()));
            let app = axum::Router::new()
                .route("/v1/raft_block/create", axum::routing::post(record))
                .route("/v1/raft_block/stop", axum::routing::post(record))
                .with_state(calls.clone());
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            let handle = tokio::spawn(async move {
                axum::serve(listener, app).await.unwrap();
            });
            (format!("http://{addr}"), calls, handle)
        }

        let (url1, calls1, server1) = spawn_agent().await;
        let (url2, calls2, server2) = spawn_agent().await;
        let (url3, calls3, server3) = spawn_agent().await;
        let mut cfg = cfg();
        cfg.prototype_provisioning_enabled = true;
        // Mock servers expose routes under /v1/raft_block; the production
        // TOML convention is the same (`agent_base_url` is the full base
        // for the raft-block routes, not just the host:port).
        cfg.replicas[0].agent_base_url = format!("{url1}/v1/raft_block");
        cfg.replicas[1].agent_base_url = format!("{url2}/v1/raft_block");
        cfg.replicas[2].agent_base_url = format!("{url3}/v1/raft_block");
        let backend =
            RaftSpdkControlPlaneBackend::new(BackendInstanceId(uuid::Uuid::new_v4()), cfg).unwrap();

        let handle = backend
            .provision(CreateOpts {
                name: "vol".into(),
                size_bytes: 4096,
                description: None,
            })
            .await
            .unwrap();

        assert_eq!(handle.backend_kind, BackendKind::RaftSpdk);
        let locator = RaftSpdkLocator::from_locator_str(&handle.locator).unwrap();
        assert_eq!(locator.replicas.len(), RAFT_SPDK_STATIC_REPLICA_COUNT);
        assert_eq!(locator.leader_hint, Some(1));
        assert_eq!(calls1.lock().await[0]["node_id"], 1);
        assert_eq!(calls2.lock().await[0]["node_id"], 2);
        assert_eq!(calls3.lock().await[0]["node_id"], 3);

        server1.abort();
        server2.abort();
        server3.abort();
    }

    /// Production provisioning calls create -> runtime_start (on each
    /// replica) -> runtime_initialize (on the leader, with the full
    /// membership). The locator does NOT carry `prototype_replica`.
    type CallLog = std::sync::Arc<tokio::sync::Mutex<Vec<(String, serde_json::Value)>>>;

    #[tokio::test]
    async fn production_provisioning_creates_groups_starts_runtimes_initializes_leader() {
        async fn record(
            axum::extract::State(calls): axum::extract::State<CallLog>,
            uri: axum::extract::OriginalUri,
            axum::Json(body): axum::Json<serde_json::Value>,
        ) -> axum::Json<serde_json::Value> {
            calls.lock().await.push((uri.0.path().to_string(), body));
            axum::Json(serde_json::json!({}))
        }

        async fn spawn_agent() -> (String, CallLog, tokio::task::JoinHandle<()>) {
            let calls = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new()));
            let app = axum::Router::new()
                .route("/v1/raft_block/create", axum::routing::post(record))
                .route("/v1/raft_block/stop", axum::routing::post(record))
                .route("/v1/raft_block/runtime_start", axum::routing::post(record))
                .route(
                    "/v1/raft_block/runtime_initialize",
                    axum::routing::post(record),
                )
                .with_state(calls.clone());
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            let handle = tokio::spawn(async move {
                axum::serve(listener, app).await.unwrap();
            });
            (format!("http://{addr}"), calls, handle)
        }

        let (url1, calls1, server1) = spawn_agent().await;
        let (url2, calls2, server2) = spawn_agent().await;
        let (url3, calls3, server3) = spawn_agent().await;
        let mut cfg = cfg();
        cfg.production_provisioning_enabled = true;
        // Mock servers expose routes under /v1/raft_block; the production
        // TOML convention is the same (`agent_base_url` is the full base
        // for the raft-block routes, not just the host:port).
        cfg.replicas[0].agent_base_url = format!("{url1}/v1/raft_block");
        cfg.replicas[1].agent_base_url = format!("{url2}/v1/raft_block");
        cfg.replicas[2].agent_base_url = format!("{url3}/v1/raft_block");
        let backend =
            RaftSpdkControlPlaneBackend::new(BackendInstanceId(uuid::Uuid::new_v4()), cfg).unwrap();

        let handle = backend
            .provision(CreateOpts {
                name: "vol".into(),
                size_bytes: 4096,
                description: None,
            })
            .await
            .unwrap();

        assert_eq!(handle.backend_kind, BackendKind::RaftSpdk);
        let locator = RaftSpdkLocator::from_locator_str(&handle.locator).unwrap();
        assert_eq!(locator.replicas.len(), RAFT_SPDK_STATIC_REPLICA_COUNT);
        assert_eq!(locator.leader_hint, Some(1));

        // Locator must NOT carry prototype_replica in production mode.
        for replica in &locator.replicas {
            let parsed: serde_json::Value =
                serde_json::from_str(&replica.spdk_lvol_locator).unwrap();
            assert!(parsed.get("prototype_replica").is_none());
            assert_eq!(parsed["production_replica"], true);
        }

        // Each replica saw create + runtime_start.
        for calls in [&calls1, &calls2, &calls3] {
            let recorded = calls.lock().await;
            let paths: Vec<String> = recorded.iter().map(|(p, _)| p.clone()).collect();
            assert!(
                paths.contains(&"/v1/raft_block/create".to_string()),
                "missing create call: {paths:?}"
            );
            assert!(
                paths.contains(&"/v1/raft_block/runtime_start".to_string()),
                "missing runtime_start call: {paths:?}"
            );
        }
        // Only the leader (replica 0) saw runtime_initialize.
        let calls1_recorded = calls1.lock().await;
        let leader_paths: Vec<String> = calls1_recorded.iter().map(|(p, _)| p.clone()).collect();
        assert!(
            leader_paths.contains(&"/v1/raft_block/runtime_initialize".to_string()),
            "leader missing runtime_initialize: {leader_paths:?}"
        );
        let initialize_body = calls1_recorded
            .iter()
            .find(|(p, _)| p == "/v1/raft_block/runtime_initialize")
            .map(|(_, b)| b.clone())
            .unwrap();
        let members: Vec<u64> = serde_json::from_value(initialize_body["members"].clone()).unwrap();
        assert_eq!(members, vec![1, 2, 3]);
        drop(calls1_recorded);

        // Followers should NOT have received runtime_initialize.
        for calls in [&calls2, &calls3] {
            let recorded = calls.lock().await;
            let paths: Vec<String> = recorded.iter().map(|(p, _)| p.clone()).collect();
            assert!(
                !paths.contains(&"/v1/raft_block/runtime_initialize".to_string()),
                "follower wrongly saw runtime_initialize: {paths:?}"
            );
        }

        server1.abort();
        server2.abort();
        server3.abort();
    }

    #[tokio::test]
    async fn destroy_stops_every_locator_replica() {
        async fn record(
            axum::extract::State(calls): axum::extract::State<CallLog>,
            uri: axum::extract::OriginalUri,
            axum::Json(body): axum::Json<serde_json::Value>,
        ) -> axum::Json<serde_json::Value> {
            calls.lock().await.push((uri.0.path().to_string(), body));
            axum::Json(serde_json::json!({}))
        }

        async fn spawn_agent() -> (String, CallLog, tokio::task::JoinHandle<()>) {
            let calls = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new()));
            let app = axum::Router::new()
                .route("/v1/raft_block/destroy", axum::routing::post(record))
                .with_state(calls.clone());
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            let handle = tokio::spawn(async move {
                axum::serve(listener, app).await.unwrap();
            });
            (format!("http://{addr}/v1/raft_block"), calls, handle)
        }

        let (url1, calls1, server1) = spawn_agent().await;
        let (url2, calls2, server2) = spawn_agent().await;
        let (url3, calls3, server3) = spawn_agent().await;
        let mut cfg = cfg();
        cfg.replicas[0].agent_base_url = url1.clone();
        cfg.replicas[1].agent_base_url = url2.clone();
        cfg.replicas[2].agent_base_url = url3.clone();
        let backend =
            RaftSpdkControlPlaneBackend::new(BackendInstanceId(uuid::Uuid::new_v4()), cfg).unwrap();
        let group_id = Uuid::new_v4();
        let locator = RaftSpdkLocator::new(
            group_id,
            4096,
            512,
            vec![
                RaftSpdkReplicaLocator {
                    node_id: 1,
                    agent_base_url: url1,
                    spdk_lvol_locator: "{}".into(),
                },
                RaftSpdkReplicaLocator {
                    node_id: 2,
                    agent_base_url: url2,
                    spdk_lvol_locator: "{}".into(),
                },
                RaftSpdkReplicaLocator {
                    node_id: 3,
                    agent_base_url: url3,
                    spdk_lvol_locator: "{}".into(),
                },
            ],
            Some(1),
        )
        .unwrap();

        backend
            .destroy(VolumeHandle {
                volume_id: Uuid::new_v4(),
                backend_id: backend.id,
                backend_kind: BackendKind::RaftSpdk,
                locator: locator.to_locator_string().unwrap(),
                size_bytes: 4096,
            })
            .await
            .unwrap();

        for calls in [&calls1, &calls2, &calls3] {
            let recorded = calls.lock().await;
            assert_eq!(recorded.len(), 1);
            assert_eq!(recorded[0].0, "/v1/raft_block/destroy");
            assert_eq!(recorded[0].1["group_id"], group_id.to_string());
        }

        server1.abort();
        server2.abort();
        server3.abort();
    }

    /// Setting both prototype and production flags is rejected up front.
    #[tokio::test]
    async fn provisioning_rejects_both_flags_set() {
        let mut cfg = cfg();
        cfg.prototype_provisioning_enabled = true;
        cfg.production_provisioning_enabled = true;
        let backend =
            RaftSpdkControlPlaneBackend::new(BackendInstanceId(uuid::Uuid::new_v4()), cfg).unwrap();
        let err = backend
            .provision(CreateOpts {
                name: "vol".into(),
                size_bytes: 4096,
                description: None,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, StorageError::InvalidLocator(_)));
        assert!(err.to_string().contains("mutually exclusive"));
    }
}
