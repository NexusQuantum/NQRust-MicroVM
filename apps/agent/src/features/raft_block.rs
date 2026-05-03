use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use nexus_raft_block::{
    openraft_entry, BlockCommand, BlockRaftTypeConfig, BlockResponse, BlockSnapshot,
    FileReplicaStore, InMemoryOpenraftBlockStore, RaftBlockError, VoteOutcome,
};
use nexus_storage::RaftBlockStoreKind;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SpdkGroupManifest {
    version: u32,
    group_id: Uuid,
    node_id: u64,
    capacity_bytes: u64,
    block_size: u64,
}

#[derive(Debug, Clone)]
enum RaftBlockStoreConfig {
    Sidecar,
    SpdkLvol { template: String },
    InMemory,
}

impl RaftBlockStoreConfig {
    fn detect() -> Self {
        if let Ok(template) = std::env::var("RAFT_BLOCK_SPDK_NBD_TEMPLATE") {
            Self::SpdkLvol { template }
        } else if std::env::var("AGENT_RAFTBLK_IN_MEMORY")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
        {
            Self::InMemory
        } else {
            Self::Sidecar
        }
    }

    fn kind(&self) -> RaftBlockStoreKind {
        match self {
            Self::Sidecar => RaftBlockStoreKind::Sidecar,
            Self::SpdkLvol { .. } => RaftBlockStoreKind::SpdkLvol,
            Self::InMemory => RaftBlockStoreKind::InMemory,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RaftBlockState {
    base_dir: PathBuf,
    store_config: RaftBlockStoreConfig,
    groups: Arc<Mutex<HashMap<Uuid, InMemoryOpenraftBlockStore>>>,
    /// Per-group Openraft runtimes. A group present in `runtimes` is in
    /// real-Raft mode: the openraft_* routes dispatch incoming RPCs through
    /// `Raft::append_entries`/`Raft::vote`/`Raft::install_snapshot` and writes
    /// flow through `Raft::client_write`. A group present in `groups` but
    /// not `runtimes` is in legacy storage-only mode (existing prototype
    /// tests, `populate_streaming` direct-replica path).
    runtimes: Arc<Mutex<HashMap<Uuid, RaftBlockRuntime>>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RaftBlockStatus {
    pub group_id: Uuid,
    pub state: String,
    pub data_path: String,
    pub transport: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raft_state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_term: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_leader: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_log_index: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub millis_since_quorum_ack: Option<u64>,
    pub store_kind: RaftBlockStoreKind,
    pub store_path: Option<String>,
    pub node_id: Option<u64>,
    pub capacity_bytes: Option<u64>,
    pub block_size: Option<u64>,
    pub last_applied_index: Option<u64>,
    pub compacted_through: Option<u64>,
    pub retained_log_entries: u64,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RaftBlockHttpClient {
    client: reqwest::Client,
    base_url: String,
}

#[allow(dead_code)]
impl RaftBlockHttpClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: normalize_base_url(base_url.into()),
        }
    }

    pub fn with_client(client: reqwest::Client, base_url: impl Into<String>) -> Self {
        Self {
            client,
            base_url: normalize_base_url(base_url.into()),
        }
    }

    pub async fn create_group(&self, req: &CreateGroupReq) -> Result<(), RaftBlockTransportError> {
        self.post_empty("create", req).await
    }

    pub async fn append_entries(
        &self,
        req: &AppendEntriesReq,
    ) -> Result<Vec<BlockResponse>, RaftBlockTransportError> {
        self.post_json("append_entries", req).await
    }

    pub async fn openraft_append_entries(
        &self,
        group_id: Uuid,
        req: &openraft::raft::AppendEntriesRequest<BlockRaftTypeConfig>,
    ) -> Result<openraft::raft::AppendEntriesResponse<u64>, RaftBlockTransportError> {
        self.post_json(&format!("{group_id}/openraft/append_entries"), req)
            .await
    }

    pub async fn vote(&self, req: &VoteReq) -> Result<VoteOutcome, RaftBlockTransportError> {
        self.post_json("vote", req).await
    }

    pub async fn openraft_vote(
        &self,
        group_id: Uuid,
        req: &openraft::raft::VoteRequest<u64>,
    ) -> Result<openraft::raft::VoteResponse<u64>, RaftBlockTransportError> {
        self.post_json(&format!("{group_id}/openraft/vote"), req)
            .await
    }

    pub async fn install_snapshot(
        &self,
        req: &InstallSnapshotReq,
    ) -> Result<(), RaftBlockTransportError> {
        self.post_empty("install_snapshot", req).await
    }

    pub async fn openraft_install_snapshot(
        &self,
        group_id: Uuid,
        req: &openraft::raft::InstallSnapshotRequest<BlockRaftTypeConfig>,
    ) -> Result<openraft::raft::InstallSnapshotResponse<u64>, RaftBlockTransportError> {
        self.post_json(&format!("{group_id}/openraft/install_snapshot"), req)
            .await
    }

    pub async fn snapshot(&self, group_id: Uuid) -> Result<BlockSnapshot, RaftBlockTransportError> {
        let url = self.url(&format!("{group_id}/snapshot"));
        self.decode_response(self.client.get(url).send().await?)
            .await
    }

    pub async fn heartbeat(
        &self,
        req: &HeartbeatReq,
    ) -> Result<serde_json::Value, RaftBlockTransportError> {
        self.post_json("heartbeat", req).await
    }

    pub async fn status(&self, group_id: Uuid) -> Result<RaftBlockStatus, RaftBlockTransportError> {
        let url = self.url(&format!("{group_id}/status"));
        self.decode_response(self.client.get(url).send().await?)
            .await
    }

    pub async fn read(&self, req: &ReadReq) -> Result<ReadResp, RaftBlockTransportError> {
        self.post_json("read", req).await
    }

    fn url(&self, path: &str) -> String {
        format!("{}/{}", self.base_url, path.trim_start_matches('/'))
    }

    async fn post_empty<T: Serialize + ?Sized>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<(), RaftBlockTransportError> {
        let _: serde_json::Value = self.post_json(path, body).await?;
        Ok(())
    }

    async fn post_json<T, R>(&self, path: &str, body: &T) -> Result<R, RaftBlockTransportError>
    where
        T: Serialize + ?Sized,
        R: for<'de> Deserialize<'de>,
    {
        let url = self.url(path);
        let response = self.client.post(url).json(body).send().await?;
        self.decode_response(response).await
    }

    async fn decode_response<R>(
        &self,
        response: reqwest::Response,
    ) -> Result<R, RaftBlockTransportError>
    where
        R: for<'de> Deserialize<'de>,
    {
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(RaftBlockTransportError::Remote { status, body });
        }
        Ok(response.json().await?)
    }
}

#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum RaftBlockTransportError {
    #[error("raft block transport request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("raft block remote returned {status}: {body}")]
    Remote {
        status: reqwest::StatusCode,
        body: String,
    },
}

#[allow(dead_code)]
fn normalize_base_url(mut base_url: String) -> String {
    while base_url.ends_with('/') {
        base_url.pop();
    }
    base_url
}

#[allow(dead_code)]
/// Openraft `RaftNetworkFactory` for `BlockRaftTypeConfig`.
///
/// Holds a static peer table mapping `NodeId -> base_url` and constructs a
/// per-target `RaftBlockNetworkConnection` that forwards Openraft RPCs to
/// the existing `/:group_id/openraft/{append_entries,vote,install_snapshot}`
/// agent routes via `RaftBlockHttpClient`.
///
/// Each Raft group spins up its own factory. A factory is built with the
/// current group_id baked in so connections it produces can address the
/// remote agent's group-scoped routes without the call sites needing to
/// thread the group id through Openraft's network trait surface.
#[derive(Debug, Clone)]
pub struct RaftBlockNetworkFactory {
    group_id: Uuid,
    peers: Arc<std::sync::RwLock<HashMap<u64, String>>>,
    client: reqwest::Client,
}

#[allow(dead_code)]
impl RaftBlockNetworkFactory {
    /// Build a factory for `group_id` whose peer node-id->url map is `peers`.
    /// The local node's own id should be included; Openraft's runtime never
    /// constructs a network client targeting itself, but the storage harness
    /// validates that the local node id is in the membership.
    pub fn new(group_id: Uuid, peers: HashMap<u64, String>) -> Self {
        Self {
            group_id,
            peers: Arc::new(std::sync::RwLock::new(
                peers
                    .into_iter()
                    .map(|(node_id, url)| (node_id, normalize_base_url(url)))
                    .collect(),
            )),
            client: reqwest::Client::new(),
        }
    }

    /// Same as `new` but reuses an existing `reqwest::Client` (test pools,
    /// custom timeouts, etc.).
    pub fn with_client(
        group_id: Uuid,
        peers: HashMap<u64, String>,
        client: reqwest::Client,
    ) -> Self {
        Self {
            group_id,
            peers: Arc::new(std::sync::RwLock::new(
                peers
                    .into_iter()
                    .map(|(node_id, url)| (node_id, normalize_base_url(url)))
                    .collect(),
            )),
            client,
        }
    }

    fn lookup(&self, target: u64) -> Option<String> {
        self.peers
            .read()
            .expect("RaftBlockNetworkFactory peers RwLock poisoned")
            .get(&target)
            .cloned()
    }

    /// Replace the peer map. Used by `update_peers` so add_replica can
    /// teach the existing leader/followers the URL of a newly-added
    /// learner before openraft tries to send append_entries to it.
    pub fn update_peers(&self, peers: HashMap<u64, String>) {
        let mut guard = self
            .peers
            .write()
            .expect("RaftBlockNetworkFactory peers RwLock poisoned");
        *guard = peers
            .into_iter()
            .map(|(node_id, url)| (node_id, normalize_base_url(url)))
            .collect();
    }
}

impl openraft::network::RaftNetworkFactory<BlockRaftTypeConfig> for RaftBlockNetworkFactory {
    type Network = RaftBlockNetworkConnection;

    async fn new_client(&mut self, target: u64, _node: &openraft::BasicNode) -> Self::Network {
        // If the peer is unknown the connection still constructs successfully;
        // every RPC then returns Unreachable, matching Openraft's contract that
        // a missing-peer error must not panic the network factory.
        let base_url = self.lookup(target).unwrap_or_default();
        RaftBlockNetworkConnection {
            target,
            group_id: self.group_id,
            base_url,
            client: self.client.clone(),
        }
    }
}

#[allow(dead_code)]
/// One outgoing Raft channel toward a single peer node, scoped to a group.
///
/// Wraps `RaftBlockHttpClient::openraft_*` so its reqwest-shaped errors are
/// translated into Openraft's `RPCError` taxonomy.
#[derive(Debug)]
pub struct RaftBlockNetworkConnection {
    target: u64,
    group_id: Uuid,
    base_url: String,
    client: reqwest::Client,
}

impl RaftBlockNetworkConnection {
    fn http_client(&self) -> Option<RaftBlockHttpClient> {
        if self.base_url.is_empty() {
            None
        } else {
            Some(RaftBlockHttpClient::with_client(
                self.client.clone(),
                self.base_url.clone(),
            ))
        }
    }

    fn transport_to_rpc<E>(
        &self,
        err: RaftBlockTransportError,
    ) -> openraft::error::RPCError<u64, openraft::BasicNode, E>
    where
        E: std::error::Error,
    {
        use openraft::error::{NetworkError, RPCError, Unreachable};
        match err {
            // Connection-level failures: the remote did not respond, treat as
            // unreachable so Openraft schedules a backoff retry.
            RaftBlockTransportError::Request(req_err) => {
                if req_err.is_connect() || req_err.is_timeout() {
                    let std_err: std::io::Error = std::io::Error::other(req_err.to_string());
                    RPCError::Unreachable(Unreachable::new(&std_err))
                } else {
                    let std_err: std::io::Error = std::io::Error::other(req_err.to_string());
                    RPCError::Network(NetworkError::new(&std_err))
                }
            }
            // HTTP-level failures (5xx etc.) are surfaced as a generic network
            // error rather than RemoteError because the agent routes do not
            // currently serialize structured Raft errors back; a future PR
            // will tighten this once the routes return RaftError JSON.
            RaftBlockTransportError::Remote { status, body } => {
                let std_err: std::io::Error =
                    std::io::Error::other(format!("status {status}: {body}"));
                RPCError::Network(NetworkError::new(&std_err))
            }
        }
    }

    fn unreachable<E>(&self) -> openraft::error::RPCError<u64, openraft::BasicNode, E>
    where
        E: std::error::Error,
    {
        use openraft::error::{RPCError, Unreachable};
        let std_err: std::io::Error =
            std::io::Error::other(format!("no peer URL for node {}", self.target));
        RPCError::Unreachable(Unreachable::new(&std_err))
    }
}

impl openraft::network::RaftNetwork<BlockRaftTypeConfig> for RaftBlockNetworkConnection {
    async fn append_entries(
        &mut self,
        rpc: openraft::raft::AppendEntriesRequest<BlockRaftTypeConfig>,
        _option: openraft::network::RPCOption,
    ) -> Result<
        openraft::raft::AppendEntriesResponse<u64>,
        openraft::error::RPCError<u64, openraft::BasicNode, openraft::error::RaftError<u64>>,
    > {
        let Some(client) = self.http_client() else {
            return Err(self.unreachable());
        };
        client
            .openraft_append_entries(self.group_id, &rpc)
            .await
            .map_err(|e| self.transport_to_rpc(e))
    }

    async fn vote(
        &mut self,
        rpc: openraft::raft::VoteRequest<u64>,
        _option: openraft::network::RPCOption,
    ) -> Result<
        openraft::raft::VoteResponse<u64>,
        openraft::error::RPCError<u64, openraft::BasicNode, openraft::error::RaftError<u64>>,
    > {
        let Some(client) = self.http_client() else {
            return Err(self.unreachable());
        };
        client
            .openraft_vote(self.group_id, &rpc)
            .await
            .map_err(|e| self.transport_to_rpc(e))
    }

    async fn install_snapshot(
        &mut self,
        rpc: openraft::raft::InstallSnapshotRequest<BlockRaftTypeConfig>,
        _option: openraft::network::RPCOption,
    ) -> Result<
        openraft::raft::InstallSnapshotResponse<u64>,
        openraft::error::RPCError<
            u64,
            openraft::BasicNode,
            openraft::error::RaftError<u64, openraft::error::InstallSnapshotError>,
        >,
    > {
        let Some(client) = self.http_client() else {
            return Err(self.unreachable());
        };
        client
            .openraft_install_snapshot(self.group_id, &rpc)
            .await
            .map_err(|e| self.transport_to_rpc(e))
    }
}

/// A live Openraft node bound to a `BlockRaftTypeConfig` group.
///
/// This is the bridge between the agent's HTTP routes (which still call into
/// the storage harness directly for the prototype path) and a real Raft
/// runtime that performs leader election, log replication, and state machine
/// application via Openraft.
///
/// Construction is `start_single_node` for tests and `start` for production
/// three-node groups. The runtime owns the network factory and the storage,
/// so the caller only needs to keep the `RaftBlockRuntime` alive.
#[allow(dead_code)]
#[derive(Clone)]
pub struct RaftBlockRuntime {
    pub group_id: Uuid,
    pub node_id: u64,
    pub raft: openraft::Raft<BlockRaftTypeConfig>,
    pub store: InMemoryOpenraftBlockStore,
    /// Peer agent base URLs (NodeId -> base_url). Used to forward
    /// client_write requests to the leader when a follower receives one.
    /// Wrapped in RwLock so add_replica can teach existing nodes the
    /// URL of a newly-joining learner without restarting the runtime.
    pub peers: Arc<std::sync::RwLock<HashMap<u64, String>>>,
    /// Cloned reference to the network factory's peer map so
    /// `update_peers` can broadcast the new map to both leader-forward
    /// (`peers`) and openraft network factory (`network_factory.peers`)
    /// in a single call site.
    pub network_factory: RaftBlockNetworkFactory,
    /// Shared HTTP client for leader-forwarding.
    pub http: reqwest::Client,
}

impl std::fmt::Debug for RaftBlockRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RaftBlockRuntime")
            .field("group_id", &self.group_id)
            .field("node_id", &self.node_id)
            .field("raft", &"<openraft::Raft handle>")
            .field("store", &self.store)
            .finish()
    }
}

#[allow(dead_code)]
impl RaftBlockRuntime {
    /// Build a runtime that talks to a static set of peers via HTTP.
    ///
    /// `peers` maps `NodeId -> base_url`. The local node id MUST be present
    /// in the map (Openraft's storage validates that the local id is in the
    /// membership when initializing); the local entry's URL is unused by
    /// `RaftBlockNetworkFactory` because Openraft never sends RPCs to itself.
    pub async fn start(
        group_id: Uuid,
        node_id: u64,
        capacity_bytes: u64,
        block_size: u64,
        store_path: PathBuf,
        peers: HashMap<u64, String>,
    ) -> Result<Self, RaftBlockError> {
        let store = InMemoryOpenraftBlockStore::open_or_create(
            FileReplicaStore::new(store_path),
            node_id,
            capacity_bytes,
            block_size,
        )?;
        let peers_arc = Arc::new(std::sync::RwLock::new(peers.clone()));
        let factory = RaftBlockNetworkFactory::new(group_id, peers);
        let config = nexus_raft_block::default_openraft_config()?;
        let (log_store, state_machine) = openraft::storage::Adaptor::new(store.clone());
        let raft = openraft::Raft::new(node_id, config, factory.clone(), log_store, state_machine)
            .await
            .map_err(|e| RaftBlockError::Store(format!("Raft::new: {e}")))?;
        Ok(Self {
            group_id,
            node_id,
            raft,
            store,
            peers: peers_arc,
            network_factory: factory,
            http: reqwest::Client::new(),
        })
    }

    /// Build a runtime from a pre-existing storage handle (the agent's
    /// `RaftBlockState` already created and persisted the replica via the
    /// `create` route, and the runtime registers atop that same storage so
    /// the existing prototype data path is preserved). The storage handle is
    /// `Arc`-backed and cloned cheaply; both the runtime and the legacy
    /// `RaftBlockState::groups` map share the same backing replica.
    pub async fn from_existing(
        group_id: Uuid,
        node_id: u64,
        store: InMemoryOpenraftBlockStore,
        peers: HashMap<u64, String>,
    ) -> Result<Self, RaftBlockError> {
        let peers_arc = Arc::new(std::sync::RwLock::new(peers.clone()));
        let factory = RaftBlockNetworkFactory::new(group_id, peers);
        let config = nexus_raft_block::default_openraft_config()?;
        let (log_store, state_machine) = openraft::storage::Adaptor::new(store.clone());
        let raft = openraft::Raft::new(node_id, config, factory.clone(), log_store, state_machine)
            .await
            .map_err(|e| RaftBlockError::Store(format!("Raft::new: {e}")))?;
        Ok(Self {
            group_id,
            node_id,
            raft,
            store,
            peers: peers_arc,
            network_factory: factory,
            http: reqwest::Client::new(),
        })
    }

    /// Replace the peer URL map in both the leader-forward path and the
    /// openraft network factory. Add-replica calls this on every existing
    /// node before `add_learner` so the leader can immediately route
    /// append_entries / install_snapshot to the new node.
    pub fn update_peers(&self, peers: HashMap<u64, String>) {
        {
            let mut guard = self
                .peers
                .write()
                .expect("RaftBlockRuntime peers RwLock poisoned");
            *guard = peers.clone();
        }
        self.network_factory.update_peers(peers);
    }

    /// Initialize this runtime as the sole member of the cluster (single-node
    /// path used by tests and by the leader of a fresh three-node group).
    /// After `initialize` returns, the node will elect itself leader within
    /// one heartbeat interval and accept `client_write`.
    pub async fn initialize_single_node(&self) -> Result<(), RaftBlockError> {
        let mut members: std::collections::BTreeMap<u64, openraft::BasicNode> =
            std::collections::BTreeMap::new();
        members.insert(self.node_id, openraft::BasicNode::default());
        self.raft
            .initialize(members)
            .await
            .map_err(|e| RaftBlockError::Store(format!("Raft::initialize: {e}")))
    }

    /// Initialize this runtime as the bootstrap leader of a static membership.
    /// All node ids must be present in the peer URL map.
    pub async fn initialize_membership(
        &self,
        members: std::collections::BTreeMap<u64, openraft::BasicNode>,
    ) -> Result<(), RaftBlockError> {
        self.raft
            .initialize(members)
            .await
            .map_err(|e| RaftBlockError::Store(format!("Raft::initialize: {e}")))
    }

    /// Commit a membership replacement through Openraft. This drives
    /// Openraft's joint-consensus path when the current and next voter sets
    /// differ, and must be called on the current leader.
    pub async fn change_membership(
        &self,
        voters: std::collections::BTreeSet<u64>,
        retain: bool,
    ) -> Result<String, RaftBlockError> {
        let response = self
            .raft
            .change_membership(openraft::ChangeMembers::ReplaceAllVoters(voters), retain)
            .await
            .map_err(|e| RaftBlockError::Store(format!("Raft::change_membership: {e}")))?;
        Ok(openraft::MessageSummary::summary(&response))
    }

    /// Add a non-voting learner. Must be called before promoting the node
    /// to voter via `change_membership` — Openraft refuses to promote a
    /// node that isn't already in the cluster as a learner. The leader
    /// replicates log entries to learners but they don't count toward
    /// quorum.
    pub async fn add_learner(&self, node_id: u64) -> Result<String, RaftBlockError> {
        let response = self
            .raft
            .add_learner(node_id, openraft::BasicNode::default(), true)
            .await
            .map_err(|e| RaftBlockError::Store(format!("Raft::add_learner: {e}")))?;
        Ok(openraft::MessageSummary::summary(&response))
    }

    /// Submit a block command through the Raft pipeline. Returns once the
    /// command is committed and applied. Only the leader accepts writes;
    /// followers return a `ForwardToLeader`-shaped error which is mapped to
    /// `RaftBlockError::Store` for the prototype.
    pub async fn client_write(
        &self,
        command: BlockCommand,
    ) -> Result<BlockResponse, RaftBlockError> {
        // Try local; if Openraft says we're not the leader, look up the
        // leader's URL in `peers` and forward the request to its
        // `runtime_write` endpoint. Without this, a daemon attached on a
        // follower replica cannot serve writes — every write would block
        // forever on a non-leader Raft handle.
        match self.raft.client_write(command.clone()).await {
            Ok(result) => Ok(result.data),
            Err(openraft::error::RaftError::APIError(
                openraft::error::ClientWriteError::ForwardToLeader(fwd),
            )) => {
                let leader_id = fwd.leader_id.ok_or_else(|| {
                    RaftBlockError::Store(
                        "ForwardToLeader without a known leader (election in progress)".into(),
                    )
                })?;
                let leader_url = self
                    .peers
                    .read()
                    .expect("RaftBlockRuntime peers RwLock poisoned")
                    .get(&leader_id)
                    .cloned()
                    .ok_or_else(|| {
                        RaftBlockError::Store(format!(
                            "ForwardToLeader: no peer URL for node {leader_id}"
                        ))
                    })?;
                let url = format!("{}/runtime_write", leader_url.trim_end_matches('/'));
                let body = serde_json::json!({
                    "group_id": self.group_id,
                    "command": command,
                });
                let resp = self.http.post(&url).json(&body).send().await.map_err(|e| {
                    RaftBlockError::Store(format!("forward to leader {leader_id}: {e}"))
                })?;
                if !resp.status().is_success() {
                    let status = resp.status();
                    let body_text = resp.text().await.unwrap_or_default();
                    return Err(RaftBlockError::Store(format!(
                        "forwarded write rejected by leader {leader_id}: {status}: {body_text}"
                    )));
                }
                let resp_json: BlockResponse = resp.json().await.map_err(|e| {
                    RaftBlockError::Store(format!("forwarded write response decode: {e}"))
                })?;
                Ok(resp_json)
            }
            Err(e) => Err(RaftBlockError::Store(format!("Raft::client_write: {e}"))),
        }
    }

    /// Read the current cluster metrics. Useful for `is_leader()` checks
    /// and for surfacing Raft state through `/v1/raft_block/:id/status` in a
    /// follow-up PR.
    pub fn metrics(
        &self,
    ) -> tokio::sync::watch::Receiver<openraft::RaftMetrics<u64, openraft::BasicNode>> {
        self.raft.metrics()
    }

    /// Block until this node observes itself as leader, or `timeout` elapses.
    /// Returns `Ok(())` if leadership was reached, `Err` otherwise.
    pub async fn await_leader(&self, timeout: std::time::Duration) -> Result<(), RaftBlockError> {
        let deadline = tokio::time::Instant::now() + timeout;
        let mut metrics = self.raft.metrics();
        while tokio::time::Instant::now() < deadline {
            let snapshot = metrics.borrow().clone();
            if snapshot.current_leader == Some(self.node_id) {
                return Ok(());
            }
            tokio::select! {
                _ = tokio::time::sleep_until(deadline) => break,
                changed = metrics.changed() => {
                    if changed.is_err() {
                        break;
                    }
                }
            }
        }
        Err(RaftBlockError::Store(
            "timed out waiting for leadership".into(),
        ))
    }

    /// Gracefully shut the runtime down. Idempotent.
    pub async fn shutdown(&self) -> Result<(), RaftBlockError> {
        self.raft
            .shutdown()
            .await
            .map_err(|e| RaftBlockError::Store(format!("Raft::shutdown: {e}")))
    }
}

impl RaftBlockState {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
            store_config: RaftBlockStoreConfig::detect(),
            groups: Arc::new(Mutex::new(HashMap::new())),
            runtimes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    #[cfg(test)]
    fn new_with_store_config(
        base_dir: impl Into<PathBuf>,
        store_config: RaftBlockStoreConfig,
    ) -> Self {
        Self {
            base_dir: base_dir.into(),
            store_config,
            groups: Arc::new(Mutex::new(HashMap::new())),
            runtimes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Start an Openraft runtime for an existing group. The group's storage
    /// must already exist (created via `create_group`/`ensure_group`). Once a
    /// runtime is started, the openraft_* routes dispatch through it; calling
    /// it twice is a no-op.
    pub async fn start_runtime(
        &self,
        group_id: Uuid,
        peers: HashMap<u64, String>,
    ) -> Result<(), RaftBlockError> {
        {
            let runtimes = self.runtimes.lock().await;
            if runtimes.contains_key(&group_id) {
                return Ok(());
            }
        }
        let store = {
            let groups = self.groups.lock().await;
            groups
                .get(&group_id)
                .cloned()
                .ok_or_else(|| RaftBlockError::Store(format!("group {group_id} not started")))?
        };
        let node_id = store.node_id()?;
        let runtime = RaftBlockRuntime::from_existing(group_id, node_id, store, peers).await?;
        self.runtimes.lock().await.insert(group_id, runtime);
        Ok(())
    }

    /// Initialize this node as the bootstrap member of the cluster. For
    /// single-node groups pass a single-entry membership; for static
    /// three-node groups pass all three node ids. Only the bootstrap leader
    /// calls this; followers learn membership via append_entries.
    pub async fn initialize_runtime(
        &self,
        group_id: Uuid,
        members: std::collections::BTreeMap<u64, openraft::BasicNode>,
    ) -> Result<(), RaftBlockError> {
        let runtime = self
            .runtime_for(group_id)
            .await
            .ok_or_else(|| RaftBlockError::Store(format!("runtime for {group_id} not started")))?;
        runtime.initialize_membership(members).await
    }

    pub async fn change_membership(
        &self,
        group_id: Uuid,
        voters: std::collections::BTreeSet<u64>,
        retain: bool,
    ) -> Result<String, RaftBlockError> {
        let runtime = self
            .runtime_for(group_id)
            .await
            .ok_or_else(|| RaftBlockError::Store(format!("runtime for {group_id} not started")))?;
        runtime.change_membership(voters, retain).await
    }

    pub async fn add_learner(
        &self,
        group_id: Uuid,
        node_id: u64,
    ) -> Result<String, RaftBlockError> {
        let runtime = self
            .runtime_for(group_id)
            .await
            .ok_or_else(|| RaftBlockError::Store(format!("runtime for {group_id} not started")))?;
        runtime.add_learner(node_id).await
    }

    pub async fn update_runtime_peers(
        &self,
        group_id: Uuid,
        peers: HashMap<u64, String>,
    ) -> Result<(), RaftBlockError> {
        let runtime = self
            .runtime_for(group_id)
            .await
            .ok_or_else(|| RaftBlockError::Store(format!("runtime for {group_id} not started")))?;
        runtime.update_peers(peers);
        Ok(())
    }

    /// Submit a `BlockCommand` through Raft. Returns once the command is
    /// committed and applied. Only the leader accepts writes.
    pub async fn runtime_client_write(
        &self,
        group_id: Uuid,
        command: BlockCommand,
    ) -> Result<BlockResponse, RaftBlockError> {
        let runtime = self
            .runtime_for(group_id)
            .await
            .ok_or_else(|| RaftBlockError::Store(format!("runtime for {group_id} not started")))?;
        runtime.client_write(command).await
    }

    /// Stop a runtime, leaving the underlying storage intact. Used by
    /// `RaftSpdkHostBackend::detach`.
    pub async fn stop_runtime(&self, group_id: Uuid) -> Result<bool, RaftBlockError> {
        let removed = self.runtimes.lock().await.remove(&group_id);
        if let Some(runtime) = removed {
            runtime.shutdown().await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Cheap snapshot of a runtime handle (Raft is Arc-backed).
    pub async fn runtime_for(&self, group_id: Uuid) -> Option<RaftBlockRuntime> {
        self.runtimes.lock().await.get(&group_id).cloned()
    }

    /// Block until this node is observed as leader for `group_id`, or
    /// `timeout` elapses. Convenience wrapper for tests and the bootstrap
    /// flow.
    pub async fn await_leader(
        &self,
        group_id: Uuid,
        timeout: std::time::Duration,
    ) -> Result<(), RaftBlockError> {
        let runtime = self
            .runtime_for(group_id)
            .await
            .ok_or_else(|| RaftBlockError::Store(format!("runtime for {group_id} not started")))?;
        runtime.await_leader(timeout).await
    }

    fn store_for(&self, group_id: Uuid, node_id: u64) -> FileReplicaStore {
        // Operator opt-in to the SPDK-backed replica store. When the
        // env var is set, every replica state is persisted through an
        // NBD device exposed by SPDK rather than a JSON file under
        // base_dir. The template is a printf-style string with
        // `{node_id}` and optional `{group_id}` interpolation, e.g.
        // `/dev/nbd{node_id}` or `/var/lib/raftblk/{group_id}-{node_id}.dev`.
        //
        // Default (env var unset) persists through the filesystem store
        // under <base_dir>/raft-block/<group_id>/node-<node_id>.json.d:
        // metadata, block bytes, and append-only log are split so normal
        // writes do not rewrite the whole replica image.
        if let RaftBlockStoreConfig::SpdkLvol { template } = &self.store_config {
            let nbd_path = self.render_spdk_template(template, group_id, node_id);
            let impl_obj = std::sync::Arc::new(
                crate::features::storage::spdk_replica_store::SpdkLvolReplicaStore::new(nbd_path),
            );
            return FileReplicaStore::external(impl_obj);
        }
        // Smoke-test / ephemeral mode: skip on-disk persistence entirely.
        // Kept for tests and emergency smokes only. Crash recovery is
        // forfeited in exchange.
        if matches!(self.store_config, RaftBlockStoreConfig::InMemory) {
            return FileReplicaStore::in_memory();
        }
        FileReplicaStore::new(
            self.base_dir
                .join("raft-block")
                .join(group_id.to_string())
                .join(format!("node-{node_id}.json")),
        )
    }

    fn store_descriptor(
        &self,
        group_id: Uuid,
        node_id: u64,
    ) -> (RaftBlockStoreKind, Option<String>) {
        if let RaftBlockStoreConfig::SpdkLvol { template } = &self.store_config {
            return (
                RaftBlockStoreKind::SpdkLvol,
                Some(self.render_spdk_template(template, group_id, node_id)),
            );
        }
        if matches!(self.store_config, RaftBlockStoreConfig::InMemory) {
            return (RaftBlockStoreKind::InMemory, None);
        }
        let path = self
            .base_dir
            .join("raft-block")
            .join(group_id.to_string())
            .join(format!("node-{node_id}.json"));
        (
            RaftBlockStoreKind::Sidecar,
            Some(path.to_string_lossy().into_owned()),
        )
    }

    fn render_spdk_template(&self, template: &str, group_id: Uuid, node_id: u64) -> String {
        template
            .replace("{group_id}", &group_id.to_string())
            .replace("{node_id}", &node_id.to_string())
    }

    fn spdk_manifest_dir(&self, group_id: Uuid) -> PathBuf {
        self.base_dir
            .join("raft-block-spdk")
            .join(group_id.to_string())
    }

    fn spdk_manifest_path(&self, group_id: Uuid, node_id: u64) -> PathBuf {
        self.spdk_manifest_dir(group_id)
            .join(format!("node-{node_id}.json"))
    }

    fn save_spdk_manifest(
        &self,
        group_id: Uuid,
        node_id: u64,
        capacity_bytes: u64,
        block_size: u64,
    ) -> Result<(), RaftBlockError> {
        if self.current_store_kind() != RaftBlockStoreKind::SpdkLvol {
            return Ok(());
        }
        let dir = self.spdk_manifest_dir(group_id);
        std::fs::create_dir_all(&dir)
            .map_err(|e| RaftBlockError::Store(format!("create {dir:?}: {e}")))?;
        let path = self.spdk_manifest_path(group_id, node_id);
        let manifest = SpdkGroupManifest {
            version: 1,
            group_id,
            node_id,
            capacity_bytes,
            block_size,
        };
        let encoded = serde_json::to_vec_pretty(&manifest)
            .map_err(|e| RaftBlockError::Store(format!("encode {path:?}: {e}")))?;
        let tmp_path = path.with_extension("json.tmp");
        std::fs::write(&tmp_path, encoded)
            .map_err(|e| RaftBlockError::Store(format!("write {tmp_path:?}: {e}")))?;
        std::fs::rename(&tmp_path, &path)
            .map_err(|e| RaftBlockError::Store(format!("rename {tmp_path:?} -> {path:?}: {e}")))?;
        Ok(())
    }

    fn remove_spdk_manifest(
        &self,
        group_id: Uuid,
        node_id: Option<u64>,
    ) -> Result<(), RaftBlockError> {
        let Some(node_id) = node_id else {
            return Ok(());
        };
        let path = self.spdk_manifest_path(group_id, node_id);
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => {
                return Err(RaftBlockError::Store(format!(
                    "remove SPDK manifest {path:?}: {err}"
                )));
            }
        }
        let dir = self.spdk_manifest_dir(group_id);
        let _ = std::fs::remove_dir(&dir);
        Ok(())
    }

    fn current_store_kind(&self) -> RaftBlockStoreKind {
        self.store_config.kind()
    }

    pub async fn ensure_group(
        &self,
        group_id: Uuid,
        node_id: u64,
        capacity_bytes: u64,
        block_size: u64,
    ) -> Result<(), RaftBlockError> {
        self.create_group(CreateGroupReq {
            group_id,
            node_id,
            capacity_bytes,
            block_size,
            desired_store_kind: None,
        })
        .await
    }

    pub async fn stop_group(&self, group_id: Uuid) -> Result<bool, RaftBlockError> {
        let runtime_stopped = self.stop_runtime(group_id).await?;
        let group_stopped = self.groups.lock().await.remove(&group_id).is_some();
        Ok(runtime_stopped || group_stopped)
    }

    pub async fn destroy_group(&self, group_id: Uuid) -> Result<bool, RaftBlockError> {
        let node_id_from_groups = {
            let groups = self.groups.lock().await;
            groups.get(&group_id).and_then(|group| group.node_id().ok())
        };
        // If the group has already been stop-removed from the in-memory map
        // (idempotent destroy retry, or a runtime-only registration), fall
        // back to the on-disk SPDK manifest so we still know which node-id
        // owns this group and can clean its store + manifest.
        let node_id = node_id_from_groups.or_else(|| self.spdk_manifest_node_id(group_id));
        tracing::info!(
            target: "agent::raft_block",
            group_id = %group_id,
            node_id_from_groups = ?node_id_from_groups,
            node_id_resolved = ?node_id,
            store_kind = %self.current_store_kind(),
            "destroy_group: resolving cleanup target"
        );
        let store_descriptor = node_id.map(|node_id| self.store_descriptor(group_id, node_id));
        let stopped = self.stop_group(group_id).await?;
        let sidecar_dir = self.base_dir.join("raft-block").join(group_id.to_string());
        if sidecar_dir.exists() {
            std::fs::remove_dir_all(&sidecar_dir)
                .map_err(|e| RaftBlockError::Store(format!("remove {sidecar_dir:?}: {e}")))?;
        }
        if let Some((store_kind, Some(store_path))) = store_descriptor {
            tracing::info!(
                target: "agent::raft_block",
                group_id = %group_id,
                ?store_kind,
                store_path = %store_path,
                "destroy_group: clearing store"
            );
            if store_kind == RaftBlockStoreKind::SpdkLvol {
                destroy_spdk_store_path(&store_path)?;
            }
        }
        self.remove_spdk_manifest(group_id, node_id)?;
        Ok(stopped || !sidecar_dir.exists())
    }

    /// Read the on-disk SPDK manifest for `group_id` and return its
    /// `node_id` if a valid manifest exists. Used by `destroy_group` to
    /// recover the cleanup target after the in-memory `groups` map has
    /// already evicted the entry.
    fn spdk_manifest_node_id(&self, group_id: Uuid) -> Option<u64> {
        let dir = self.spdk_manifest_dir(group_id);
        let entries = std::fs::read_dir(&dir).ok()?;
        for entry in entries.flatten() {
            if entry.file_type().ok()?.is_file() {
                let bytes = std::fs::read(entry.path()).ok()?;
                if let Ok(manifest) = serde_json::from_slice::<SpdkGroupManifest>(&bytes) {
                    if manifest.version == 1 && manifest.group_id == group_id {
                        return Some(manifest.node_id);
                    }
                }
            }
        }
        None
    }

    pub async fn load_existing_groups(&self) -> Result<usize, RaftBlockError> {
        let spdk_loaded = self.load_existing_spdk_groups().await?;
        let root = self.base_dir.join("raft-block");
        if !root.exists() {
            return Ok(spdk_loaded);
        }
        let mut loaded = spdk_loaded;
        let mut groups = self.groups.lock().await;
        let dirs = std::fs::read_dir(&root)
            .map_err(|e| RaftBlockError::Store(format!("read {root:?}: {e}")))?;
        for dir in dirs {
            let dir = dir.map_err(|e| RaftBlockError::Store(format!("read {root:?}: {e}")))?;
            if !dir
                .file_type()
                .map_err(|e| RaftBlockError::Store(format!("stat {:?}: {e}", dir.path())))?
                .is_dir()
            {
                continue;
            }
            let Some(group_id) = dir
                .file_name()
                .to_str()
                .and_then(|raw| Uuid::parse_str(raw).ok())
            else {
                continue;
            };
            if groups.contains_key(&group_id) {
                continue;
            }
            let files = std::fs::read_dir(dir.path())
                .map_err(|e| RaftBlockError::Store(format!("read {:?}: {e}", dir.path())))?;
            for file in files {
                let file =
                    file.map_err(|e| RaftBlockError::Store(format!("read {:?}: {e}", dir.path())))?;
                let file_name = file.file_name().to_string_lossy().to_string();
                if !file_name.starts_with("node-") {
                    continue;
                }
                let store_path = if let Some(raw) = file_name.strip_suffix(".d") {
                    file.path().with_file_name(raw)
                } else if file
                    .file_type()
                    .map_err(|e| RaftBlockError::Store(format!("stat {:?}: {e}", file.path())))?
                    .is_file()
                {
                    file.path()
                } else {
                    continue;
                };
                let Some(store) =
                    InMemoryOpenraftBlockStore::open_existing(FileReplicaStore::new(store_path))?
                else {
                    continue;
                };
                groups.insert(group_id, store);
                loaded += 1;
                break;
            }
        }
        Ok(loaded)
    }

    async fn load_existing_spdk_groups(&self) -> Result<usize, RaftBlockError> {
        if self.current_store_kind() != RaftBlockStoreKind::SpdkLvol {
            return Ok(0);
        }
        let root = self.base_dir.join("raft-block-spdk");
        if !root.exists() {
            return Ok(0);
        }
        let mut loaded = 0;
        let mut groups = self.groups.lock().await;
        let dirs = std::fs::read_dir(&root)
            .map_err(|e| RaftBlockError::Store(format!("read {root:?}: {e}")))?;
        for dir in dirs {
            let dir = dir.map_err(|e| RaftBlockError::Store(format!("read {root:?}: {e}")))?;
            if !dir
                .file_type()
                .map_err(|e| RaftBlockError::Store(format!("stat {:?}: {e}", dir.path())))?
                .is_dir()
            {
                continue;
            }
            let Some(group_id) = dir
                .file_name()
                .to_str()
                .and_then(|raw| Uuid::parse_str(raw).ok())
            else {
                continue;
            };
            if groups.contains_key(&group_id) {
                continue;
            }
            let files = std::fs::read_dir(dir.path())
                .map_err(|e| RaftBlockError::Store(format!("read {:?}: {e}", dir.path())))?;
            for file in files {
                let file =
                    file.map_err(|e| RaftBlockError::Store(format!("read {:?}: {e}", dir.path())))?;
                if !file
                    .file_type()
                    .map_err(|e| RaftBlockError::Store(format!("stat {:?}: {e}", file.path())))?
                    .is_file()
                {
                    continue;
                }
                let bytes = std::fs::read(file.path()).map_err(|e| {
                    RaftBlockError::Store(format!("read manifest {:?}: {e}", file.path()))
                })?;
                let manifest: SpdkGroupManifest = serde_json::from_slice(&bytes).map_err(|e| {
                    RaftBlockError::Store(format!("decode manifest {:?}: {e}", file.path()))
                })?;
                if manifest.version != 1 || manifest.group_id != group_id {
                    continue;
                }
                let Some(store) = InMemoryOpenraftBlockStore::open_existing(
                    self.store_for(group_id, manifest.node_id),
                )?
                else {
                    continue;
                };
                groups.insert(group_id, store);
                loaded += 1;
                break;
            }
        }
        Ok(loaded)
    }

    async fn create_group(&self, req: CreateGroupReq) -> Result<(), RaftBlockError> {
        if let Some(desired) = req.desired_store_kind {
            let actual = self.current_store_kind();
            if desired != actual {
                return Err(RaftBlockError::Store(format!(
                    "raft block store kind mismatch: requested {desired}, agent is using {actual}"
                )));
            }
        }
        let mut groups = self.groups.lock().await;
        if let Some(existing) = groups.get(&req.group_id) {
            validate_existing_group(existing, &req)?;
            return Ok(());
        }
        let store = self.store_for(req.group_id, req.node_id);
        let replica = InMemoryOpenraftBlockStore::open_or_create(
            store,
            req.node_id,
            req.capacity_bytes,
            req.block_size,
        )?;
        self.save_spdk_manifest(
            req.group_id,
            req.node_id,
            req.capacity_bytes,
            req.block_size,
        )?;
        groups.insert(req.group_id, replica);
        Ok(())
    }

    async fn append(&self, req: AppendReq) -> Result<BlockResponse, RaftBlockError> {
        let mut groups = self.groups.lock().await;
        let replica = groups
            .get_mut(&req.group_id)
            .ok_or_else(|| RaftBlockError::Store(format!("group {} not started", req.group_id)))?;
        replica.append_command(
            req.term,
            req.leader_id.unwrap_or(replica.node_id()?),
            req.command,
        )
    }

    pub async fn append_command(
        &self,
        group_id: Uuid,
        term: u64,
        leader_id: Option<u64>,
        command: BlockCommand,
    ) -> Result<BlockResponse, RaftBlockError> {
        self.append(AppendReq {
            group_id,
            term,
            leader_id,
            command,
        })
        .await
    }

    async fn append_entries(
        &self,
        req: AppendEntriesReq,
    ) -> Result<Vec<BlockResponse>, RaftBlockError> {
        let groups = self.groups.lock().await;
        let replica = groups
            .get(&req.group_id)
            .ok_or_else(|| RaftBlockError::Store(format!("group {} not started", req.group_id)))?;
        let entries = req
            .entries
            .into_iter()
            .map(|entry| openraft_entry(req.term, req.leader_id, entry.index, entry.command));
        replica.append_openraft_entries(entries)
    }

    async fn openraft_append_entries(
        &self,
        group_id: Uuid,
        req: openraft::raft::AppendEntriesRequest<BlockRaftTypeConfig>,
    ) -> Result<openraft::raft::AppendEntriesResponse<u64>, RaftBlockError> {
        // Real Raft mode: a runtime is registered for this group, dispatch
        // through Openraft's incoming-RPC handler so leader election, term
        // tracking, and log replication go through the production state
        // machine. Falls back to direct-storage append when no runtime is
        // registered (legacy prototype tests, populate_streaming path).
        if let Some(runtime) = self.runtime_for(group_id).await {
            return runtime
                .raft
                .append_entries(req)
                .await
                .map_err(|e| RaftBlockError::Store(format!("Raft::append_entries: {e}")));
        }
        let groups = self.groups.lock().await;
        let replica = groups
            .get(&group_id)
            .ok_or_else(|| RaftBlockError::Store(format!("group {group_id} not started")))?;
        if !req.entries.is_empty() {
            replica.append_openraft_entries(req.entries)?;
        }
        Ok(openraft::raft::AppendEntriesResponse::Success)
    }

    async fn snapshot(&self, group_id: Uuid) -> Result<BlockSnapshot, RaftBlockError> {
        let groups = self.groups.lock().await;
        let replica = groups
            .get(&group_id)
            .ok_or_else(|| RaftBlockError::Store(format!("group {group_id} not started")))?;
        replica.block_snapshot()
    }

    pub async fn snapshot_bytes(&self, group_id: Uuid) -> Result<Vec<u8>, RaftBlockError> {
        self.snapshot(group_id).await.map(|snapshot| snapshot.bytes)
    }

    async fn read(&self, req: ReadReq) -> Result<ReadResp, RaftBlockError> {
        let groups = self.groups.lock().await;
        let replica = groups
            .get(&req.group_id)
            .ok_or_else(|| RaftBlockError::Store(format!("group {} not started", req.group_id)))?;
        let bytes = replica.read_range(req.offset, req.len)?;
        Ok(ReadResp { bytes })
    }

    async fn install_snapshot(&self, req: InstallSnapshotReq) -> Result<(), RaftBlockError> {
        let mut groups = self.groups.lock().await;
        let replica = groups
            .get_mut(&req.group_id)
            .ok_or_else(|| RaftBlockError::Store(format!("group {} not started", req.group_id)))?;
        replica.install_block_snapshot(&req.snapshot)
    }

    async fn openraft_install_snapshot(
        &self,
        group_id: Uuid,
        req: openraft::raft::InstallSnapshotRequest<BlockRaftTypeConfig>,
    ) -> Result<openraft::raft::InstallSnapshotResponse<u64>, RaftBlockError> {
        if let Some(runtime) = self.runtime_for(group_id).await {
            #[allow(deprecated)]
            return runtime
                .raft
                .install_snapshot(req)
                .await
                .map_err(|e| RaftBlockError::Store(format!("Raft::install_snapshot: {e}")));
        }
        let groups = self.groups.lock().await;
        let replica = groups
            .get(&group_id)
            .ok_or_else(|| RaftBlockError::Store(format!("group {group_id} not started")))?;
        if !req.done || req.offset != 0 {
            return Err(RaftBlockError::Store(
                "raft block prototype accepts only single-chunk Openraft snapshots".into(),
            ));
        }
        let snapshot: BlockSnapshot =
            serde_json::from_slice(&req.data).map_err(|e| RaftBlockError::Store(e.to_string()))?;
        replica.install_openraft_snapshot(&req.meta, &snapshot)?;
        Ok(openraft::raft::InstallSnapshotResponse { vote: req.vote })
    }

    async fn vote(&self, req: VoteReq) -> Result<VoteOutcome, RaftBlockError> {
        let groups = self.groups.lock().await;
        let replica = groups
            .get(&req.group_id)
            .ok_or_else(|| RaftBlockError::Store(format!("group {} not started", req.group_id)))?;
        replica.request_vote(req.term, req.candidate_id)
    }

    async fn openraft_vote(
        &self,
        group_id: Uuid,
        req: openraft::raft::VoteRequest<u64>,
    ) -> Result<openraft::raft::VoteResponse<u64>, RaftBlockError> {
        if let Some(runtime) = self.runtime_for(group_id).await {
            return runtime
                .raft
                .vote(req)
                .await
                .map_err(|e| RaftBlockError::Store(format!("Raft::vote: {e}")));
        }
        let groups = self.groups.lock().await;
        let replica = groups
            .get(&group_id)
            .ok_or_else(|| RaftBlockError::Store(format!("group {group_id} not started")))?;
        let candidate_id = req
            .vote
            .leader_id
            .voted_for()
            .ok_or_else(|| RaftBlockError::Store("Openraft vote has no candidate".into()))?;
        let outcome = replica.request_vote(req.vote.leader_id.term, candidate_id)?;
        let vote = openraft::Vote {
            leader_id: outcome
                .voted_for
                .map(|node_id| openraft::LeaderId::new(outcome.term, node_id))
                .unwrap_or_default(),
            committed: outcome.committed,
        };
        Ok(openraft::raft::VoteResponse {
            vote,
            vote_granted: outcome.granted,
            last_log_id: None,
        })
    }

    pub async fn status(&self, group_id: Uuid) -> RaftBlockStatus {
        let groups = self.groups.lock().await;
        if let Some(replica) = groups.get(&group_id) {
            let node_id = replica.node_id().ok();
            let (store_kind, store_path) = node_id
                .map(|node_id| self.store_descriptor(group_id, node_id))
                .unwrap_or_else(|| (self.current_store_kind(), None));
            let capacity_bytes = replica.capacity_bytes().ok();
            let block_size = replica.block_size().ok();
            let last_applied_index = replica.last_applied_index().ok();
            let compacted_through = replica.compacted_through().ok();
            let retained_log_entries = replica.retained_log_entries().unwrap_or(0);
            drop(groups);
            let metrics = self
                .runtime_for(group_id)
                .await
                .map(|runtime| runtime.metrics().borrow().clone());
            RaftBlockStatus {
                group_id,
                state: "started".into(),
                data_path: "persistent_local_replica".into(),
                transport: "openraft_entry_local".into(),
                raft_state: metrics
                    .as_ref()
                    .map(|metrics| format!("{:?}", metrics.state)),
                current_term: metrics.as_ref().map(|metrics| metrics.current_term),
                current_leader: metrics.as_ref().and_then(|metrics| metrics.current_leader),
                last_log_index: metrics.as_ref().and_then(|metrics| metrics.last_log_index),
                millis_since_quorum_ack: metrics
                    .as_ref()
                    .and_then(|metrics| metrics.millis_since_quorum_ack),
                store_kind,
                store_path,
                node_id,
                capacity_bytes,
                block_size,
                last_applied_index,
                compacted_through,
                retained_log_entries,
            }
        } else {
            RaftBlockStatus {
                group_id,
                state: "not_started".into(),
                data_path: "raftblk_pending".into(),
                transport: "not_started".into(),
                raft_state: None,
                current_term: None,
                current_leader: None,
                last_log_index: None,
                millis_since_quorum_ack: None,
                store_kind: self.current_store_kind(),
                store_path: None,
                node_id: None,
                capacity_bytes: None,
                block_size: None,
                last_applied_index: None,
                compacted_through: None,
                retained_log_entries: 0,
            }
        }
    }
}

fn validate_existing_group(
    existing: &InMemoryOpenraftBlockStore,
    req: &CreateGroupReq,
) -> Result<(), RaftBlockError> {
    if existing.node_id()? != req.node_id
        || existing.capacity_bytes()? != req.capacity_bytes
        || existing.block_size()? != req.block_size
    {
        return Err(RaftBlockError::Store(format!(
            "group {} already exists with node_id={}, capacity_bytes={}, block_size={}; requested node_id={}, capacity_bytes={}, block_size={}",
            req.group_id,
            existing.node_id()?,
            existing.capacity_bytes()?,
            existing.block_size()?,
            req.node_id,
            req.capacity_bytes,
            req.block_size
        )));
    }
    Ok(())
}

fn destroy_spdk_store_path(store_path: &str) -> Result<(), RaftBlockError> {
    let path = std::path::Path::new(store_path);
    if path.starts_with("/dev") {
        return Err(RaftBlockError::Store(format!(
            "refusing to unlink SPDK NBD device {store_path}; real lvol destroy must release it through SPDK"
        )));
    }
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(RaftBlockError::Store(format!(
            "remove SPDK store {store_path}: {err}"
        ))),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateGroupReq {
    pub group_id: Uuid,
    pub node_id: u64,
    pub capacity_bytes: u64,
    pub block_size: u64,
    #[serde(default)]
    pub desired_store_kind: Option<RaftBlockStoreKind>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendReq {
    pub group_id: Uuid,
    pub term: u64,
    #[serde(default)]
    pub leader_id: Option<u64>,
    pub command: BlockCommand,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendEntriesReq {
    pub group_id: Uuid,
    pub term: u64,
    pub leader_id: u64,
    pub entries: Vec<AppendEntryReq>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendEntryReq {
    pub index: u64,
    pub command: BlockCommand,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallSnapshotReq {
    pub group_id: Uuid,
    pub snapshot: BlockSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopGroupReq {
    pub group_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DestroyGroupReq {
    pub group_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatReq {
    pub group_id: Uuid,
    pub term: u64,
    pub leader_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteReq {
    pub group_id: Uuid,
    pub term: u64,
    pub candidate_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadReq {
    pub group_id: Uuid,
    pub offset: u64,
    pub len: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadResp {
    pub bytes: Vec<u8>,
}

pub async fn create(
    State(state): State<Arc<RaftBlockState>>,
    Json(req): Json<CreateGroupReq>,
) -> impl IntoResponse {
    match state.create_group(req).await {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({}))).into_response(),
        Err(err) => error_response(StatusCode::BAD_REQUEST, err),
    }
}

pub async fn status(
    State(state): State<Arc<RaftBlockState>>,
    Path(group_id): Path<Uuid>,
) -> impl IntoResponse {
    (StatusCode::OK, Json(state.status(group_id).await))
}

pub async fn append(
    State(state): State<Arc<RaftBlockState>>,
    Json(req): Json<AppendReq>,
) -> impl IntoResponse {
    match state.append(req).await {
        Ok(response) => (StatusCode::OK, Json(response)).into_response(),
        Err(err) => error_response(StatusCode::BAD_REQUEST, err),
    }
}

pub async fn append_entries(
    State(state): State<Arc<RaftBlockState>>,
    Json(req): Json<AppendEntriesReq>,
) -> impl IntoResponse {
    match state.append_entries(req).await {
        Ok(response) => (StatusCode::OK, Json(response)).into_response(),
        Err(err) => error_response(StatusCode::BAD_REQUEST, err),
    }
}

pub async fn openraft_append_entries(
    State(state): State<Arc<RaftBlockState>>,
    Path(group_id): Path<Uuid>,
    Json(req): Json<openraft::raft::AppendEntriesRequest<BlockRaftTypeConfig>>,
) -> impl IntoResponse {
    match state.openraft_append_entries(group_id, req).await {
        Ok(response) => (StatusCode::OK, Json(response)).into_response(),
        Err(err) => error_response(StatusCode::BAD_REQUEST, err),
    }
}

pub async fn stop(
    State(state): State<Arc<RaftBlockState>>,
    Json(req): Json<StopGroupReq>,
) -> impl IntoResponse {
    match state.stop_group(req.group_id).await {
        Ok(stopped) => (
            StatusCode::OK,
            Json(serde_json::json!({ "stopped": stopped })),
        )
            .into_response(),
        Err(err) => error_response(StatusCode::BAD_REQUEST, err),
    }
}

pub async fn destroy(
    State(state): State<Arc<RaftBlockState>>,
    Json(req): Json<DestroyGroupReq>,
) -> impl IntoResponse {
    match state.destroy_group(req.group_id).await {
        Ok(destroyed) => (
            StatusCode::OK,
            Json(serde_json::json!({ "destroyed": destroyed })),
        )
            .into_response(),
        Err(err) => error_response(StatusCode::BAD_REQUEST, err),
    }
}

pub async fn snapshot(
    State(state): State<Arc<RaftBlockState>>,
    Path(group_id): Path<Uuid>,
) -> impl IntoResponse {
    match state.snapshot(group_id).await {
        Ok(snapshot) => (StatusCode::OK, Json(snapshot)).into_response(),
        Err(err) => error_response(StatusCode::BAD_REQUEST, err),
    }
}

pub async fn read(
    State(state): State<Arc<RaftBlockState>>,
    Json(req): Json<ReadReq>,
) -> impl IntoResponse {
    match state.read(req).await {
        Ok(response) => (StatusCode::OK, Json(response)).into_response(),
        Err(err) => error_response(StatusCode::BAD_REQUEST, err),
    }
}

pub async fn vote(
    State(state): State<Arc<RaftBlockState>>,
    Json(req): Json<VoteReq>,
) -> impl IntoResponse {
    match state.vote(req).await {
        Ok(response) => (StatusCode::OK, Json(response)).into_response(),
        Err(err) => error_response(StatusCode::BAD_REQUEST, err),
    }
}

pub async fn openraft_vote(
    State(state): State<Arc<RaftBlockState>>,
    Path(group_id): Path<Uuid>,
    Json(req): Json<openraft::raft::VoteRequest<u64>>,
) -> impl IntoResponse {
    match state.openraft_vote(group_id, req).await {
        Ok(response) => (StatusCode::OK, Json(response)).into_response(),
        Err(err) => error_response(StatusCode::BAD_REQUEST, err),
    }
}

pub async fn install_snapshot(
    State(state): State<Arc<RaftBlockState>>,
    Json(req): Json<InstallSnapshotReq>,
) -> impl IntoResponse {
    match state.install_snapshot(req).await {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({}))).into_response(),
        Err(err) => error_response(StatusCode::BAD_REQUEST, err),
    }
}

pub async fn openraft_install_snapshot(
    State(state): State<Arc<RaftBlockState>>,
    Path(group_id): Path<Uuid>,
    Json(req): Json<openraft::raft::InstallSnapshotRequest<BlockRaftTypeConfig>>,
) -> impl IntoResponse {
    match state.openraft_install_snapshot(group_id, req).await {
        Ok(response) => (StatusCode::OK, Json(response)).into_response(),
        Err(err) => error_response(StatusCode::BAD_REQUEST, err),
    }
}

pub async fn heartbeat(
    State(state): State<Arc<RaftBlockState>>,
    Json(req): Json<HeartbeatReq>,
) -> impl IntoResponse {
    let status = state.status(req.group_id).await;
    if status.state != "started" {
        return error_response(
            StatusCode::BAD_REQUEST,
            RaftBlockError::Store(format!("group {} not started", req.group_id)),
        );
    }
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "group_id": req.group_id,
            "term": req.term,
            "leader_id": req.leader_id,
            "status": status
        })),
    )
        .into_response()
}

fn error_response(status: StatusCode, err: RaftBlockError) -> axum::response::Response {
    (
        status,
        Json(serde_json::json!({
            "error": err.to_string()
        })),
    )
        .into_response()
}

pub fn router(state: Arc<RaftBlockState>) -> Router {
    // Raft block writes carry a JSON-encoded byte vec; populate uses 1 MiB
    // chunks which expand 3-4x in JSON ("0,0,0,..." form). The default 2 MiB
    // body limit rejects them as 413 once the leader-forward path is taken.
    // Add-replica stresses this further: the leader sends a backlog of
    // AppendEntries to the new learner that can batch many populate
    // chunks into a single request. 512 MiB is comfortably above what
    // a 64 MiB rootfs (the smoke-test fixture) can produce at 1 MiB
    // chunks with the current 3-4x JSON inflation, and well under the
    // physical RAM available on a typical agent host.
    const MAX_BODY_BYTES: usize = 512 * 1024 * 1024;
    Router::new()
        .route("/:group_id/status", get(status))
        .route("/:group_id/snapshot", get(snapshot))
        .route(
            "/:group_id/openraft/append_entries",
            post(openraft_append_entries),
        )
        .route("/:group_id/openraft/vote", post(openraft_vote))
        .route(
            "/:group_id/openraft/install_snapshot",
            post(openraft_install_snapshot),
        )
        .route(
            "/:group_id/openraft/change_membership",
            post(openraft_change_membership),
        )
        .route(
            "/:group_id/openraft/add_learner",
            post(openraft_add_learner),
        )
        .route(
            "/:group_id/runtime_update_peers",
            post(runtime_update_peers),
        )
        .route("/create", post(create))
        .route("/append", post(append))
        .route("/append_entries", post(append_entries))
        .route("/read", post(read))
        .route("/stop", post(stop))
        .route("/destroy", post(destroy))
        .route("/vote", post(vote))
        .route("/install_snapshot", post(install_snapshot))
        .route("/heartbeat", post(heartbeat))
        .route("/runtime_start", post(runtime_start))
        .route("/runtime_write", post(runtime_write))
        .route("/runtime_initialize", post(runtime_initialize))
        .layer(axum::extract::DefaultBodyLimit::max(MAX_BODY_BYTES))
        .with_state(state)
}

/// Request shape for `POST /v1/raft_block/runtime_start`. The agent uses
/// this to bind an Openraft runtime to an existing storage group; the
/// peer URL map is the static three-node membership.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeStartReq {
    pub group_id: Uuid,
    pub peers: HashMap<u64, String>,
}

/// Request shape for `POST /v1/raft_block/runtime_initialize`. Bootstrap
/// the cluster (only the leader calls this; followers learn membership
/// through subsequent append_entries).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeInitializeReq {
    pub group_id: Uuid,
    pub members: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeMembershipReq {
    pub voters: Vec<u64>,
    #[serde(default)]
    pub retain: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeMembershipResp {
    pub summary: String,
}

/// Request shape for `POST /v1/raft_block/runtime_write`. This is the
/// production write path used by `raftblk-vhost`'s `RaftBlockBackend`:
/// every guest write becomes one of these and the response only returns
/// after the entry is committed and applied across a quorum of replicas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeWriteReq {
    pub group_id: Uuid,
    pub command: BlockCommand,
}

pub async fn runtime_start(
    State(state): State<Arc<RaftBlockState>>,
    Json(req): Json<RuntimeStartReq>,
) -> impl IntoResponse {
    match state.start_runtime(req.group_id, req.peers).await {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({}))).into_response(),
        Err(err) => error_response(StatusCode::BAD_REQUEST, err),
    }
}

pub async fn runtime_initialize(
    State(state): State<Arc<RaftBlockState>>,
    Json(req): Json<RuntimeInitializeReq>,
) -> impl IntoResponse {
    let mut members = std::collections::BTreeMap::new();
    for node_id in req.members {
        members.insert(node_id, openraft::BasicNode::default());
    }
    match state.initialize_runtime(req.group_id, members).await {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({}))).into_response(),
        Err(err) => error_response(StatusCode::BAD_REQUEST, err),
    }
}

pub async fn openraft_change_membership(
    State(state): State<Arc<RaftBlockState>>,
    Path(group_id): Path<Uuid>,
    Json(req): Json<ChangeMembershipReq>,
) -> impl IntoResponse {
    let voters = req.voters.into_iter().collect();
    match state.change_membership(group_id, voters, req.retain).await {
        Ok(summary) => (StatusCode::OK, Json(ChangeMembershipResp { summary })).into_response(),
        Err(err) => error_response(StatusCode::BAD_REQUEST, err),
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct AddLearnerReq {
    pub node_id: u64,
}

pub async fn openraft_add_learner(
    State(state): State<Arc<RaftBlockState>>,
    Path(group_id): Path<Uuid>,
    Json(req): Json<AddLearnerReq>,
) -> impl IntoResponse {
    match state.add_learner(group_id, req.node_id).await {
        Ok(summary) => (StatusCode::OK, Json(serde_json::json!({"summary": summary}))).into_response(),
        Err(err) => error_response(StatusCode::BAD_REQUEST, err),
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct UpdatePeersReq {
    pub peers: HashMap<u64, String>,
}

pub async fn runtime_update_peers(
    State(state): State<Arc<RaftBlockState>>,
    Path(group_id): Path<Uuid>,
    Json(req): Json<UpdatePeersReq>,
) -> impl IntoResponse {
    match state.update_runtime_peers(group_id, req.peers).await {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({}))).into_response(),
        Err(err) => error_response(StatusCode::BAD_REQUEST, err),
    }
}

pub async fn runtime_write(
    State(state): State<Arc<RaftBlockState>>,
    Json(req): Json<RuntimeWriteReq>,
) -> impl IntoResponse {
    match state.runtime_client_write(req.group_id, req.command).await {
        Ok(response) => (StatusCode::OK, Json(response)).into_response(),
        Err(err) => error_response(StatusCode::BAD_REQUEST, err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::storage::spdk_replica_store::METADATA_REGION_BYTES;
    use axum::body::to_bytes;
    use nexus_raft_block::openraft_log_id;

    #[tokio::test]
    async fn status_reports_pending_data_path() {
        let group_id = Uuid::new_v4();
        let state = Arc::new(RaftBlockState::new(tempfile::tempdir().unwrap().path()));
        let response = status(State(state), Path(group_id)).await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn append_is_rejected_before_group_start() {
        let state = Arc::new(RaftBlockState::new(tempfile::tempdir().unwrap().path()));
        let response = append(
            State(state),
            Json(AppendReq {
                group_id: Uuid::new_v4(),
                term: 1,
                leader_id: None,
                command: BlockCommand::Flush,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn create_append_and_reopen_group_state() {
        let dir = tempfile::tempdir().unwrap();
        let group_id = Uuid::new_v4();
        let state = Arc::new(RaftBlockState::new(dir.path()));
        let response = create(
            State(state.clone()),
            Json(CreateGroupReq {
                group_id,
                node_id: 1,
                capacity_bytes: 4096,
                block_size: 512,
                desired_store_kind: None,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let response = append(
            State(state),
            Json(AppendReq {
                group_id,
                term: 1,
                leader_id: None,
                command: BlockCommand::Write {
                    offset: 0,
                    bytes: vec![5; 512],
                },
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let restarted = Arc::new(RaftBlockState::new(dir.path()));
        let response = create(
            State(restarted.clone()),
            Json(CreateGroupReq {
                group_id,
                node_id: 1,
                capacity_bytes: 4096,
                block_size: 512,
                desired_store_kind: None,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let response = status(State(restarted), Path(group_id))
            .await
            .into_response();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let status: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(status["state"], "started");
        assert_eq!(status["retained_log_entries"], 1);
        assert_eq!(status["last_applied_index"], 1);
        assert_eq!(status["node_id"], 1);
        assert_eq!(status["store_kind"], "sidecar");
        assert!(status["store_path"]
            .as_str()
            .unwrap()
            .contains("node-1.json"));
    }

    #[tokio::test]
    async fn startup_loads_existing_group_state() {
        let dir = tempfile::tempdir().unwrap();
        let group_id = Uuid::new_v4();
        let state = Arc::new(RaftBlockState::new(dir.path()));
        let response = create(
            State(state.clone()),
            Json(CreateGroupReq {
                group_id,
                node_id: 1,
                capacity_bytes: 4096,
                block_size: 512,
                desired_store_kind: None,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let response = append(
            State(state),
            Json(AppendReq {
                group_id,
                term: 1,
                leader_id: None,
                command: BlockCommand::Write {
                    offset: 0,
                    bytes: vec![5; 512],
                },
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let restarted = Arc::new(RaftBlockState::new(dir.path()));
        assert_eq!(restarted.load_existing_groups().await.unwrap(), 1);
        let status = restarted.status(group_id).await;
        assert_eq!(status.state, "started");
        assert_eq!(status.retained_log_entries, 1);
        assert_eq!(status.last_applied_index, Some(1));
        let response = read(
            State(restarted),
            Json(ReadReq {
                group_id,
                offset: 0,
                len: 512,
            }),
        )
        .await
        .into_response();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let response: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(response["bytes"].as_array().unwrap()[0], 5);
    }

    #[tokio::test]
    async fn create_rejects_requested_store_kind_mismatch() {
        let dir = tempfile::tempdir().unwrap();
        let group_id = Uuid::new_v4();
        let state = Arc::new(RaftBlockState::new(dir.path()));
        let response = create(
            State(state),
            Json(CreateGroupReq {
                group_id,
                node_id: 1,
                capacity_bytes: 4096,
                block_size: 512,
                desired_store_kind: Some(RaftBlockStoreKind::SpdkLvol),
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let response: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(response["error"]
            .as_str()
            .unwrap()
            .contains("store kind mismatch"));
    }

    #[tokio::test]
    async fn destroy_stops_group_and_removes_sidecar_state() {
        let dir = tempfile::tempdir().unwrap();
        let group_id = Uuid::new_v4();
        let state = Arc::new(RaftBlockState::new(dir.path()));
        let response = create(
            State(state.clone()),
            Json(CreateGroupReq {
                group_id,
                node_id: 1,
                capacity_bytes: 4096,
                block_size: 512,
                desired_store_kind: None,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let sidecar_dir = dir.path().join("raft-block").join(group_id.to_string());
        assert!(sidecar_dir.exists());

        let response = destroy(State(state.clone()), Json(DestroyGroupReq { group_id }))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(!sidecar_dir.exists());
        assert_eq!(state.status(group_id).await.state, "not_started");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn spdk_lvol_groups_reload_from_manifest_after_restart() {
        let run_dir = tempfile::tempdir().unwrap();
        let device_dir = tempfile::tempdir().unwrap();
        let group_id = Uuid::new_v4();
        let template = device_dir
            .path()
            .join("{group_id}-node-{node_id}.dev")
            .to_string_lossy()
            .into_owned();
        let device = device_dir.path().join(format!("{group_id}-node-1.dev"));
        std::fs::File::create(&device)
            .unwrap()
            .set_len(METADATA_REGION_BYTES + 4096)
            .unwrap();

        let state = Arc::new(RaftBlockState::new_with_store_config(
            run_dir.path(),
            RaftBlockStoreConfig::SpdkLvol {
                template: template.clone(),
            },
        ));
        let response = create(
            State(state.clone()),
            Json(CreateGroupReq {
                group_id,
                node_id: 1,
                capacity_bytes: 4096,
                block_size: 512,
                desired_store_kind: Some(RaftBlockStoreKind::SpdkLvol),
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let response = append(
            State(state),
            Json(AppendReq {
                group_id,
                term: 1,
                leader_id: None,
                command: BlockCommand::Write {
                    offset: 0,
                    bytes: vec![8; 512],
                },
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let restarted = Arc::new(RaftBlockState::new_with_store_config(
            run_dir.path(),
            RaftBlockStoreConfig::SpdkLvol { template },
        ));
        assert_eq!(restarted.load_existing_groups().await.unwrap(), 1);
        let status = restarted.status(group_id).await;
        assert_eq!(status.state, "started");
        assert_eq!(status.store_kind, RaftBlockStoreKind::SpdkLvol);
        assert_eq!(status.store_path.as_deref(), Some(device.to_str().unwrap()));
        let bytes = restarted
            .read(ReadReq {
                group_id,
                offset: 0,
                len: 512,
            })
            .await
            .unwrap()
            .bytes;
        assert_eq!(bytes, vec![8; 512]);
    }

    #[test]
    fn destroy_spdk_store_path_unlinks_file_backed_stub() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("node-1.dev");
        std::fs::write(&path, [1, 2, 3]).unwrap();
        destroy_spdk_store_path(path.to_str().unwrap()).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn destroy_spdk_store_path_refuses_device_nodes() {
        let err = destroy_spdk_store_path("/dev/nbd0").unwrap_err();
        assert!(err.to_string().contains("refusing to unlink"));
    }

    #[tokio::test]
    async fn create_rejects_mismatched_existing_group_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let group_id = Uuid::new_v4();
        let state = Arc::new(RaftBlockState::new(dir.path()));
        let response = create(
            State(state.clone()),
            Json(CreateGroupReq {
                group_id,
                node_id: 1,
                capacity_bytes: 4096,
                block_size: 512,
                desired_store_kind: None,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let response = create(
            State(state.clone()),
            Json(CreateGroupReq {
                group_id,
                node_id: 1,
                capacity_bytes: 8192,
                block_size: 512,
                desired_store_kind: None,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let restarted = Arc::new(RaftBlockState::new(dir.path()));
        let response = create(
            State(restarted),
            Json(CreateGroupReq {
                group_id,
                node_id: 1,
                capacity_bytes: 8192,
                block_size: 512,
                desired_store_kind: None,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn snapshot_and_install_snapshot_are_durable() {
        let dir = tempfile::tempdir().unwrap();
        let source_group = Uuid::new_v4();
        let target_group = Uuid::new_v4();
        let state = Arc::new(RaftBlockState::new(dir.path()));

        let response = create(
            State(state.clone()),
            Json(CreateGroupReq {
                group_id: source_group,
                node_id: 1,
                capacity_bytes: 4096,
                block_size: 512,
                desired_store_kind: None,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let response = append(
            State(state.clone()),
            Json(AppendReq {
                group_id: source_group,
                term: 1,
                leader_id: None,
                command: BlockCommand::Write {
                    offset: 0,
                    bytes: vec![7; 512],
                },
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let response = snapshot(State(state.clone()), Path(source_group))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let source_snapshot: BlockSnapshot = serde_json::from_slice(&body).unwrap();

        let response = create(
            State(state.clone()),
            Json(CreateGroupReq {
                group_id: target_group,
                node_id: 2,
                capacity_bytes: 4096,
                block_size: 512,
                desired_store_kind: None,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let response = install_snapshot(
            State(state.clone()),
            Json(InstallSnapshotReq {
                group_id: target_group,
                snapshot: source_snapshot,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        drop(state);

        let restarted = Arc::new(RaftBlockState::new(dir.path()));
        let response = create(
            State(restarted.clone()),
            Json(CreateGroupReq {
                group_id: target_group,
                node_id: 2,
                capacity_bytes: 4096,
                block_size: 512,
                desired_store_kind: None,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let response = snapshot(State(restarted), Path(target_group))
            .await
            .into_response();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let snapshot: BlockSnapshot = serde_json::from_slice(&body).unwrap();
        assert_eq!(&snapshot.bytes[0..512], &[7; 512]);
    }

    #[tokio::test]
    async fn read_returns_persisted_range_and_rejects_bounds() {
        let dir = tempfile::tempdir().unwrap();
        let group_id = Uuid::new_v4();
        let state = Arc::new(RaftBlockState::new(dir.path()));
        let response = create(
            State(state.clone()),
            Json(CreateGroupReq {
                group_id,
                node_id: 1,
                capacity_bytes: 4096,
                block_size: 512,
                desired_store_kind: None,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let response = append(
            State(state.clone()),
            Json(AppendReq {
                group_id,
                term: 1,
                leader_id: None,
                command: BlockCommand::Write {
                    offset: 0,
                    bytes: vec![3; 512],
                },
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let response = read(
            State(state.clone()),
            Json(ReadReq {
                group_id,
                offset: 0,
                len: 512,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let response: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(response["bytes"].as_array().unwrap().len(), 512);

        let response = read(
            State(state),
            Json(ReadReq {
                group_id,
                offset: 4096,
                len: 1,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn append_entries_applies_openraft_shaped_batch() {
        let dir = tempfile::tempdir().unwrap();
        let group_id = Uuid::new_v4();
        let state = Arc::new(RaftBlockState::new(dir.path()));
        let response = create(
            State(state.clone()),
            Json(CreateGroupReq {
                group_id,
                node_id: 1,
                capacity_bytes: 4096,
                block_size: 512,
                desired_store_kind: None,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let response = append_entries(
            State(state.clone()),
            Json(AppendEntriesReq {
                group_id,
                term: 2,
                leader_id: 1,
                entries: vec![
                    AppendEntryReq {
                        index: 1,
                        command: BlockCommand::Write {
                            offset: 0,
                            bytes: vec![2; 512],
                        },
                    },
                    AppendEntryReq {
                        index: 2,
                        command: BlockCommand::Flush,
                    },
                ],
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let response = read(
            State(state),
            Json(ReadReq {
                group_id,
                offset: 0,
                len: 512,
            }),
        )
        .await
        .into_response();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let response: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(response["bytes"].as_array().unwrap()[0], 2);
    }

    #[tokio::test]
    async fn openraft_native_routes_accept_rpc_shapes() {
        let dir = tempfile::tempdir().unwrap();
        let group_id = Uuid::new_v4();
        let state = Arc::new(RaftBlockState::new(dir.path()));
        let response = create(
            State(state.clone()),
            Json(CreateGroupReq {
                group_id,
                node_id: 1,
                capacity_bytes: 4096,
                block_size: 512,
                desired_store_kind: None,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let vote = openraft::Vote {
            leader_id: openraft::LeaderId::new(2, 2),
            committed: false,
        };
        let response = openraft_vote(
            State(state.clone()),
            Path(group_id),
            Json(openraft::raft::VoteRequest {
                vote,
                last_log_id: None,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let response: openraft::raft::VoteResponse<u64> = serde_json::from_slice(&body).unwrap();
        assert!(response.vote_granted);

        let response = openraft_append_entries(
            State(state.clone()),
            Path(group_id),
            Json(openraft::raft::AppendEntriesRequest {
                vote,
                prev_log_id: None,
                entries: vec![openraft_entry(
                    2,
                    1,
                    1,
                    BlockCommand::Write {
                        offset: 0,
                        bytes: vec![11; 512],
                    },
                )],
                leader_commit: Some(openraft_log_id(2, 1, 1)),
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let response = read(
            State(state.clone()),
            Json(ReadReq {
                group_id,
                offset: 0,
                len: 512,
            }),
        )
        .await
        .into_response();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let read: ReadResp = serde_json::from_slice(&body).unwrap();
        assert_eq!(read.bytes[0], 11);

        let snapshot = BlockSnapshot {
            replica_id: 9,
            last_included_index: 3,
            highest_term_seen: 3,
            bytes: vec![4; 4096],
        };
        let response = openraft_install_snapshot(
            State(state.clone()),
            Path(group_id),
            Json(openraft::raft::InstallSnapshotRequest {
                vote,
                meta: openraft::SnapshotMeta {
                    last_log_id: Some(openraft_log_id(3, 1, 3)),
                    last_membership: openraft::StoredMembership::default(),
                    snapshot_id: "native-test".into(),
                },
                offset: 0,
                data: serde_json::to_vec(&snapshot).unwrap(),
                done: true,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let status = state.status(group_id).await;
        assert_eq!(status.last_applied_index, Some(3));
        let read = state
            .read(ReadReq {
                group_id,
                offset: 0,
                len: 512,
            })
            .await
            .unwrap();
        assert_eq!(read.bytes[0], 4);
    }

    #[tokio::test]
    async fn stop_unloads_group_but_preserves_durable_state() {
        let dir = tempfile::tempdir().unwrap();
        let group_id = Uuid::new_v4();
        let state = Arc::new(RaftBlockState::new(dir.path()));
        let response = create(
            State(state.clone()),
            Json(CreateGroupReq {
                group_id,
                node_id: 1,
                capacity_bytes: 4096,
                block_size: 512,
                desired_store_kind: None,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let response = append(
            State(state.clone()),
            Json(AppendReq {
                group_id,
                term: 1,
                leader_id: None,
                command: BlockCommand::Write {
                    offset: 0,
                    bytes: vec![4; 512],
                },
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let response = stop(State(state.clone()), Json(StopGroupReq { group_id }))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(state.status(group_id).await.state, "not_started");

        let response = create(
            State(state.clone()),
            Json(CreateGroupReq {
                group_id,
                node_id: 1,
                capacity_bytes: 4096,
                block_size: 512,
                desired_store_kind: None,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let response = read(
            State(state),
            Json(ReadReq {
                group_id,
                offset: 0,
                len: 512,
            }),
        )
        .await
        .into_response();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let response: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(response["bytes"].as_array().unwrap().len(), 512);
    }

    #[tokio::test]
    async fn heartbeat_reports_started_group_status() {
        let dir = tempfile::tempdir().unwrap();
        let group_id = Uuid::new_v4();
        let state = Arc::new(RaftBlockState::new(dir.path()));
        let response = create(
            State(state.clone()),
            Json(CreateGroupReq {
                group_id,
                node_id: 1,
                capacity_bytes: 4096,
                block_size: 512,
                desired_store_kind: None,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let response = heartbeat(
            State(state.clone()),
            Json(HeartbeatReq {
                group_id,
                term: 3,
                leader_id: 1,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let response: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(response["term"], 3);
        assert_eq!(response["leader_id"], 1);
        assert_eq!(response["status"]["state"], "started");
        assert_eq!(response["status"]["transport"], "openraft_entry_local");
    }

    #[tokio::test]
    async fn heartbeat_rejects_unstarted_group() {
        let state = Arc::new(RaftBlockState::new(tempfile::tempdir().unwrap().path()));
        let response = heartbeat(
            State(state),
            Json(HeartbeatReq {
                group_id: Uuid::new_v4(),
                term: 1,
                leader_id: 1,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn change_membership_rejects_unstarted_runtime() {
        let state = Arc::new(RaftBlockState::new(tempfile::tempdir().unwrap().path()));
        let response = openraft_change_membership(
            State(state),
            Path(Uuid::new_v4()),
            Json(ChangeMembershipReq {
                voters: vec![1, 2, 3],
                retain: false,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn vote_grants_once_and_rejects_conflicting_same_term_candidate() {
        let dir = tempfile::tempdir().unwrap();
        let group_id = Uuid::new_v4();
        let state = Arc::new(RaftBlockState::new(dir.path()));
        let response = create(
            State(state.clone()),
            Json(CreateGroupReq {
                group_id,
                node_id: 1,
                capacity_bytes: 4096,
                block_size: 512,
                desired_store_kind: None,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let response = vote(
            State(state.clone()),
            Json(VoteReq {
                group_id,
                term: 2,
                candidate_id: 2,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let response: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(response["granted"], true);
        assert_eq!(response["term"], 2);
        assert_eq!(response["voted_for"], 2);

        let response = vote(
            State(state),
            Json(VoteReq {
                group_id,
                term: 2,
                candidate_id: 3,
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let response: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(response["granted"], false);
        assert_eq!(response["voted_for"], 2);
    }

    #[tokio::test]
    async fn http_client_drives_remote_group_routes() {
        let dir = tempfile::tempdir().unwrap();
        let group_id = Uuid::new_v4();
        let state = Arc::new(RaftBlockState::new(dir.path()));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, router(state)).await.unwrap();
        });
        let client =
            RaftBlockHttpClient::with_client(reqwest::Client::new(), format!("http://{addr}"));

        client
            .create_group(&CreateGroupReq {
                group_id,
                node_id: 1,
                capacity_bytes: 4096,
                block_size: 512,
                desired_store_kind: None,
            })
            .await
            .unwrap();
        let vote_outcome = client
            .vote(&VoteReq {
                group_id,
                term: 2,
                candidate_id: 2,
            })
            .await
            .unwrap();
        assert!(vote_outcome.granted);
        let native_request_vote = openraft::Vote {
            leader_id: openraft::LeaderId::new(2, 2),
            committed: false,
        };

        let response = client
            .append_entries(&AppendEntriesReq {
                group_id,
                term: 2,
                leader_id: 1,
                entries: vec![AppendEntryReq {
                    index: 1,
                    command: BlockCommand::Write {
                        offset: 0,
                        bytes: vec![9; 512],
                    },
                }],
            })
            .await
            .unwrap();
        assert_eq!(response[0].applied_index, 1);
        let native_append = client
            .openraft_append_entries(
                group_id,
                &openraft::raft::AppendEntriesRequest {
                    vote: native_request_vote,
                    prev_log_id: Some(openraft_log_id(2, 1, 1)),
                    entries: vec![openraft_entry(
                        2,
                        1,
                        2,
                        BlockCommand::Write {
                            offset: 512,
                            bytes: vec![8; 512],
                        },
                    )],
                    leader_commit: Some(openraft_log_id(2, 1, 2)),
                },
            )
            .await
            .unwrap();
        assert_eq!(
            native_append,
            openraft::raft::AppendEntriesResponse::Success
        );
        let native_vote = client
            .openraft_vote(
                group_id,
                &openraft::raft::VoteRequest {
                    vote: native_request_vote,
                    last_log_id: Some(openraft_log_id(2, 1, 2)),
                },
            )
            .await
            .unwrap();
        assert!(native_vote.vote_granted);
        let read = client
            .read(&ReadReq {
                group_id,
                offset: 0,
                len: 512,
            })
            .await
            .unwrap();
        assert_eq!(read.bytes[0], 9);

        let status = client.status(group_id).await.unwrap();
        assert_eq!(status.state, "started");
        assert_eq!(status.transport, "openraft_entry_local");

        let heartbeat = client
            .heartbeat(&HeartbeatReq {
                group_id,
                term: 2,
                leader_id: 1,
            })
            .await
            .unwrap();
        assert_eq!(heartbeat["status"]["state"], "started");

        let snapshot = client.snapshot(group_id).await.unwrap();
        let target_group = Uuid::new_v4();
        client
            .create_group(&CreateGroupReq {
                group_id: target_group,
                node_id: 2,
                capacity_bytes: 4096,
                block_size: 512,
                desired_store_kind: None,
            })
            .await
            .unwrap();
        client
            .install_snapshot(&InstallSnapshotReq {
                group_id: target_group,
                snapshot,
            })
            .await
            .unwrap();
        let native_snapshot = BlockSnapshot {
            replica_id: 2,
            last_included_index: 4,
            highest_term_seen: 4,
            bytes: vec![6; 4096],
        };
        client
            .openraft_install_snapshot(
                target_group,
                &openraft::raft::InstallSnapshotRequest {
                    vote: native_request_vote,
                    meta: openraft::SnapshotMeta {
                        last_log_id: Some(openraft_log_id(4, 1, 4)),
                        last_membership: openraft::StoredMembership::default(),
                        snapshot_id: "http-native-test".into(),
                    },
                    offset: 0,
                    data: serde_json::to_vec(&native_snapshot).unwrap(),
                    done: true,
                },
            )
            .await
            .unwrap();
        let restored = client
            .read(&ReadReq {
                group_id: target_group,
                offset: 0,
                len: 512,
            })
            .await
            .unwrap();
        assert_eq!(restored.bytes[0], 6);

        server.abort();
    }

    #[tokio::test]
    async fn http_client_surfaces_remote_errors() {
        let state = Arc::new(RaftBlockState::new(tempfile::tempdir().unwrap().path()));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, router(state)).await.unwrap();
        });
        let client = RaftBlockHttpClient::new(format!("http://{addr}/"));

        let err = client
            .append_entries(&AppendEntriesReq {
                group_id: Uuid::new_v4(),
                term: 1,
                leader_id: 1,
                entries: vec![],
            })
            .await
            .unwrap_err();
        match err {
            RaftBlockTransportError::Remote { status, body } => {
                assert_eq!(status, reqwest::StatusCode::BAD_REQUEST);
                assert!(body.contains("not started"));
            }
            other => panic!("unexpected error: {other}"),
        }

        server.abort();
    }

    /// Spin up an agent router on a random port and return (handle, base_url).
    /// Used by the network-adapter tests below.
    async fn spawn_agent_for_network_tests(
        state: Arc<RaftBlockState>,
    ) -> (tokio::task::JoinHandle<()>, String) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            axum::serve(listener, router(state)).await.unwrap();
        });
        (handle, format!("http://{addr}"))
    }

    /// Driving append_entries through `RaftNetworkFactory::new_client`
    /// must reach the remote agent's `/:group_id/openraft/append_entries`
    /// route and apply the entry to its replica.
    #[tokio::test]
    async fn network_factory_routes_append_entries_to_remote_agent() {
        use openraft::network::{RaftNetwork, RaftNetworkFactory};

        let dir = tempfile::tempdir().unwrap();
        let group_id = Uuid::new_v4();
        let remote_state = Arc::new(RaftBlockState::new(dir.path()));
        remote_state
            .ensure_group(group_id, 2, 4096, 512)
            .await
            .unwrap();
        let (server, base_url) = spawn_agent_for_network_tests(remote_state.clone()).await;

        let mut peers = HashMap::new();
        peers.insert(2u64, base_url);
        let mut factory = RaftBlockNetworkFactory::new(group_id, peers);
        let mut conn = factory.new_client(2, &openraft::BasicNode::default()).await;

        let leader_vote = openraft::Vote {
            leader_id: openraft::LeaderId::new(2, 1),
            committed: false,
        };
        let req = openraft::raft::AppendEntriesRequest {
            vote: leader_vote,
            prev_log_id: None,
            entries: vec![openraft_entry(
                2,
                1,
                1,
                BlockCommand::Write {
                    offset: 0,
                    bytes: vec![7; 512],
                },
            )],
            leader_commit: Some(openraft_log_id(2, 1, 1)),
        };
        let resp = conn
            .append_entries(
                req,
                openraft::network::RPCOption::new(std::time::Duration::from_secs(1)),
            )
            .await
            .unwrap();
        assert_eq!(resp, openraft::raft::AppendEntriesResponse::Success);

        // Confirm the remote applied the bytes by reading them back.
        let read = remote_state
            .read(ReadReq {
                group_id,
                offset: 0,
                len: 512,
            })
            .await
            .unwrap();
        assert_eq!(read.bytes[0], 7);

        server.abort();
    }

    /// Vote routes through the same factory pathway and a granted vote
    /// returns `vote_granted = true`.
    #[tokio::test]
    async fn network_factory_routes_vote_to_remote_agent() {
        use openraft::network::{RaftNetwork, RaftNetworkFactory};

        let dir = tempfile::tempdir().unwrap();
        let group_id = Uuid::new_v4();
        let remote_state = Arc::new(RaftBlockState::new(dir.path()));
        remote_state
            .ensure_group(group_id, 3, 4096, 512)
            .await
            .unwrap();
        let (server, base_url) = spawn_agent_for_network_tests(remote_state).await;

        let mut peers = HashMap::new();
        peers.insert(3u64, base_url);
        let mut factory = RaftBlockNetworkFactory::new(group_id, peers);
        let mut conn = factory.new_client(3, &openraft::BasicNode::default()).await;

        let candidate_vote = openraft::Vote {
            leader_id: openraft::LeaderId::new(7, 1),
            committed: false,
        };
        let req = openraft::raft::VoteRequest {
            vote: candidate_vote,
            last_log_id: None,
        };
        let resp = conn
            .vote(
                req,
                openraft::network::RPCOption::new(std::time::Duration::from_secs(1)),
            )
            .await
            .unwrap();
        assert!(resp.vote_granted);

        server.abort();
    }

    /// A node that isn't in the peer table must yield `Unreachable` rather
    /// than panicking. Openraft retries on Unreachable; panicking would tear
    /// down the runtime.
    #[tokio::test]
    async fn network_factory_unreachable_when_peer_url_missing() {
        use openraft::network::{RaftNetwork, RaftNetworkFactory};

        let group_id = Uuid::new_v4();
        let mut factory = RaftBlockNetworkFactory::new(group_id, HashMap::new());
        let mut conn = factory
            .new_client(99, &openraft::BasicNode::default())
            .await;

        let leader_vote = openraft::Vote {
            leader_id: openraft::LeaderId::new(1, 1),
            committed: false,
        };
        let err = conn
            .append_entries(
                openraft::raft::AppendEntriesRequest {
                    vote: leader_vote,
                    prev_log_id: None,
                    entries: vec![],
                    leader_commit: None,
                },
                openraft::network::RPCOption::new(std::time::Duration::from_secs(1)),
            )
            .await
            .unwrap_err();
        match err {
            openraft::error::RPCError::Unreachable(_) => {}
            other => panic!("expected Unreachable for missing peer URL, got {other:?}"),
        }
    }

    /// A single-node Raft runtime can be constructed, initialized,
    /// transition to leader, accept a `client_write`, and apply the command
    /// to its state machine. This is the minimal end-to-end proof that the
    /// Openraft runtime is wired correctly: storage v1->v2 adaptor,
    /// network factory, type config, and async runtime all agree.
    #[tokio::test]
    async fn runtime_single_node_accepts_client_write() {
        let dir = tempfile::tempdir().unwrap();
        let group_id = Uuid::new_v4();
        let store_path = dir.path().join("node-1.json");
        let mut peers = HashMap::new();
        // Local URL is unused by Openraft (never sends RPCs to itself) but
        // keeps the peer table shape consistent with multi-node groups.
        peers.insert(1u64, "http://127.0.0.1:0".to_string());

        let runtime = RaftBlockRuntime::start(group_id, 1, 4096, 512, store_path, peers)
            .await
            .expect("start runtime");
        runtime
            .initialize_single_node()
            .await
            .expect("initialize as sole member");
        runtime
            .await_leader(std::time::Duration::from_secs(5))
            .await
            .expect("become leader within 5s");

        let resp = runtime
            .client_write(BlockCommand::Write {
                offset: 0,
                bytes: vec![0xab; 512],
            })
            .await
            .expect("client_write commits via Raft");
        assert_eq!(
            resp.applied_index, 2,
            "first user write commits at index 2 (initialize is index 1)"
        );

        // The state machine applied the write: read it back through the
        // storage harness.
        let bytes = runtime
            .store
            .read_range(0, 512)
            .expect("read applied bytes");
        assert_eq!(bytes[0], 0xab);

        runtime.shutdown().await.expect("clean shutdown");
    }

    /// A 5xx response from the remote agent must surface as `RPCError::Network`
    /// rather than `Unreachable`. Openraft treats Network errors differently
    /// from Unreachable (less aggressive retry).
    #[tokio::test]
    async fn network_factory_translates_remote_4xx_to_network_error() {
        use openraft::network::{RaftNetwork, RaftNetworkFactory};

        let dir = tempfile::tempdir().unwrap();
        let group_id = Uuid::new_v4(); // intentionally NOT created on the remote
        let remote_state = Arc::new(RaftBlockState::new(dir.path()));
        let (server, base_url) = spawn_agent_for_network_tests(remote_state).await;

        let mut peers = HashMap::new();
        peers.insert(4u64, base_url);
        let mut factory = RaftBlockNetworkFactory::new(group_id, peers);
        let mut conn = factory.new_client(4, &openraft::BasicNode::default()).await;

        let leader_vote = openraft::Vote {
            leader_id: openraft::LeaderId::new(1, 1),
            committed: false,
        };
        let err = conn
            .append_entries(
                openraft::raft::AppendEntriesRequest {
                    vote: leader_vote,
                    prev_log_id: None,
                    entries: vec![],
                    leader_commit: None,
                },
                openraft::network::RPCOption::new(std::time::Duration::from_secs(1)),
            )
            .await
            .unwrap_err();
        match err {
            openraft::error::RPCError::Network(_) => {}
            other => panic!("expected Network error for 4xx remote, got {other:?}"),
        }

        server.abort();
    }

    // -------------------------------------------------------------------
    // Three-node integration tests.
    //
    // These start three in-process Openraft groups (one per simulated agent),
    // wired via the production HTTP transport (RaftBlockNetworkFactory ->
    // /openraft/* routes). They prove:
    //  - leader election in a static three-member group;
    //  - committed writes replicate to all replicas;
    //  - leader kill triggers failover and a new leader accepts writes
    //    that propagate to remaining replicas;
    //  - quorum loss (two of three down) prevents new commits but the
    //    survivor's earlier committed state is intact.
    //
    // These tests are real Raft, not the storage harness. They exercise the
    // RaftBlockRuntime + RaftNetworkFactory adapter end-to-end.
    // -------------------------------------------------------------------

    /// One node in the in-process three-node test cluster: its server task,
    /// its `RaftBlockState`, its base URL, and the dir backing its storage.
    struct TestNode {
        node_id: u64,
        state: Arc<RaftBlockState>,
        #[allow(dead_code)]
        url: String,
        server: tokio::task::JoinHandle<()>,
        _dir: tempfile::TempDir,
    }

    impl TestNode {
        async fn shutdown_runtime(&self, group_id: Uuid) {
            let _ = self.state.stop_runtime(group_id).await;
        }
    }

    /// Spin up `count` agents, each with its own RaftBlockState, axum router,
    /// and tempdir. Returns the nodes + a node_id -> url map suitable for
    /// passing to `start_runtime`.
    async fn spawn_cluster(count: u64) -> (Vec<TestNode>, HashMap<u64, String>) {
        let mut nodes = Vec::with_capacity(count as usize);
        let mut peer_map = HashMap::new();
        for node_id in 1..=count {
            let dir = tempfile::tempdir().unwrap();
            let state = Arc::new(RaftBlockState::new(dir.path()));
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            let url = format!("http://{addr}");
            let state_for_server = state.clone();
            let server = tokio::spawn(async move {
                let _ = axum::serve(listener, router(state_for_server)).await;
            });
            peer_map.insert(node_id, url.clone());
            nodes.push(TestNode {
                node_id,
                state,
                url,
                server,
                _dir: dir,
            });
        }
        (nodes, peer_map)
    }

    /// Bring up a real three-node Raft group across three in-process agents:
    /// create the group on each, start a runtime on each with the full peer
    /// URL map, then initialize membership on node 1 as the bootstrap leader.
    /// Returns the cluster + the elected leader id.
    async fn bootstrap_three_node_cluster(
        group_id: Uuid,
        capacity_bytes: u64,
        block_size: u64,
    ) -> (Vec<TestNode>, HashMap<u64, String>, u64) {
        let (nodes, peer_map) = spawn_cluster(3).await;

        for node in &nodes {
            node.state
                .ensure_group(group_id, node.node_id, capacity_bytes, block_size)
                .await
                .unwrap();
            node.state
                .start_runtime(group_id, peer_map.clone())
                .await
                .unwrap();
        }

        // Bootstrap membership on node 1 with all three members. Followers
        // learn membership through subsequent append_entries.
        let mut members = std::collections::BTreeMap::new();
        for node in &nodes {
            members.insert(node.node_id, openraft::BasicNode::default());
        }
        nodes[0]
            .state
            .initialize_runtime(group_id, members)
            .await
            .unwrap();
        nodes[0]
            .state
            .await_leader(group_id, std::time::Duration::from_secs(5))
            .await
            .unwrap();

        (nodes, peer_map, 1)
    }

    /// Wait for `from_node` to observe a leader that is NOT in `excluded`
    /// (used after a kill to find the new leader, ignoring the dead one
    /// while it's still cached in the watch channel). Returns the new
    /// leader's node_id, or None on timeout.
    async fn find_new_leader_from(
        from_node: &TestNode,
        group_id: Uuid,
        excluded: &[u64],
        timeout: std::time::Duration,
    ) -> Option<u64> {
        let runtime = from_node.state.runtime_for(group_id).await?;
        let deadline = tokio::time::Instant::now() + timeout;
        let mut metrics = runtime.metrics();
        loop {
            let snapshot = metrics.borrow().clone();
            if let Some(leader) = snapshot.current_leader {
                if !excluded.contains(&leader) {
                    return Some(leader);
                }
            }
            if tokio::time::Instant::now() >= deadline {
                return None;
            }
            tokio::select! {
                _ = tokio::time::sleep_until(deadline) => return None,
                changed = metrics.changed() => {
                    if changed.is_err() {
                        return None;
                    }
                }
            }
        }
    }

    /// All three replicas commit a write through the leader and converge to
    /// the same applied bytes.
    #[tokio::test]
    async fn three_node_cluster_replicates_committed_write() {
        let group_id = Uuid::new_v4();
        let (nodes, _peers, leader_id) = bootstrap_three_node_cluster(group_id, 4096, 512).await;
        let leader = &nodes[(leader_id - 1) as usize];

        let resp = leader
            .state
            .runtime_client_write(
                group_id,
                BlockCommand::Write {
                    offset: 0,
                    bytes: vec![0xaa; 512],
                },
            )
            .await
            .expect("leader accepts write");
        assert_eq!(resp.applied_index, 2, "write commits at index 2");

        // Give followers a moment to apply the entry. Openraft's
        // commit-replicate-apply pipeline is async; the leader's response
        // returns as soon as quorum acks, but follower application may lag.
        for _ in 0..50 {
            let mut all_have_bytes = true;
            for node in &nodes {
                let groups = node.state.groups.lock().await;
                if let Some(replica) = groups.get(&group_id) {
                    match replica.read_range(0, 512) {
                        Ok(bytes) if bytes[0] == 0xaa => {}
                        _ => {
                            all_have_bytes = false;
                            break;
                        }
                    }
                }
            }
            if all_have_bytes {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        for node in &nodes {
            let groups = node.state.groups.lock().await;
            let replica = groups.get(&group_id).expect("replica exists");
            let bytes = replica.read_range(0, 512).expect("read bytes");
            assert_eq!(
                bytes[0], 0xaa,
                "node {} did not converge to committed value",
                node.node_id
            );
        }

        for node in &nodes {
            node.shutdown_runtime(group_id).await;
            node.server.abort();
        }
    }

    /// After the leader is removed, the remaining two nodes elect a new
    /// leader within the election timeout window and accept further writes
    /// that propagate to the surviving follower.
    #[tokio::test]
    async fn three_node_cluster_fails_over_when_leader_is_killed() {
        let group_id = Uuid::new_v4();
        let (mut nodes, _peers, leader_id) =
            bootstrap_three_node_cluster(group_id, 4096, 512).await;

        // Leader writes the first byte before the kill.
        let leader = &nodes[(leader_id - 1) as usize];
        leader
            .state
            .runtime_client_write(
                group_id,
                BlockCommand::Write {
                    offset: 0,
                    bytes: vec![0x11; 512],
                },
            )
            .await
            .expect("first write commits");

        // Kill node 1 (the bootstrap leader). Stopping the runtime drops the
        // Raft instance; aborting the server breaks any remote calls aimed at
        // it. The remaining two members must form a quorum, time out an
        // election, and elect a new leader.
        nodes[0].shutdown_runtime(group_id).await;
        nodes[0].server.abort();

        // Find the new leader from one of the survivors. With two members
        // remaining, election must succeed within ~3x election_timeout_max.
        // The watch channel may transiently still report the killed leader
        // until election timeout fires; `find_new_leader_from` ignores any
        // leader id in `excluded`.
        let new_leader = find_new_leader_from(
            &nodes[1],
            group_id,
            &[1],
            std::time::Duration::from_secs(10),
        )
        .await
        .expect("survivors elect a new leader");
        assert!(
            new_leader == 2 || new_leader == 3,
            "new leader is a survivor (got {new_leader})"
        );

        // The new leader accepts a follow-up write. It may take a moment for
        // the elected node to complete its leadership transition (apply
        // blank-payload entry); retry a few times before failing.
        let new_leader_node = &nodes[(new_leader - 1) as usize];
        let mut attempts = 0;
        let resp = loop {
            attempts += 1;
            match new_leader_node
                .state
                .runtime_client_write(
                    group_id,
                    BlockCommand::Write {
                        offset: 512,
                        bytes: vec![0x22; 512],
                    },
                )
                .await
            {
                Ok(r) => break r,
                Err(e) if attempts < 30 => {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    let _ = e;
                }
                Err(e) => panic!("post-failover write failed after retries: {e}"),
            }
        };
        assert!(resp.applied_index >= 3, "post-failover write commits");

        // The other survivor replicates the post-failover bytes.
        let other_survivor_id = if new_leader == 2 { 3 } else { 2 };
        let other_survivor = &nodes[(other_survivor_id - 1) as usize];
        for _ in 0..50 {
            let groups = other_survivor.state.groups.lock().await;
            if let Some(replica) = groups.get(&group_id) {
                if let Ok(bytes) = replica.read_range(512, 512) {
                    if bytes[0] == 0x22 {
                        drop(groups);
                        for node in &mut nodes[1..] {
                            node.shutdown_runtime(group_id).await;
                            node.server.abort();
                        }
                        return;
                    }
                }
            }
            drop(groups);
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        panic!("survivor did not replicate post-failover bytes");
    }

    /// Quorum loss: two of three down means no new writes commit. The lone
    /// survivor must reject `client_write` (cannot reach majority), but its
    /// previously committed bytes remain readable from local storage.
    #[tokio::test]
    async fn three_node_cluster_blocks_writes_under_quorum_loss() {
        let group_id = Uuid::new_v4();
        let (mut nodes, _peers, leader_id) =
            bootstrap_three_node_cluster(group_id, 4096, 512).await;

        // Commit a write while quorum is healthy.
        let leader = &nodes[(leader_id - 1) as usize];
        leader
            .state
            .runtime_client_write(
                group_id,
                BlockCommand::Write {
                    offset: 0,
                    bytes: vec![0x33; 512],
                },
            )
            .await
            .expect("pre-failure write commits");
        // Allow follower to apply.
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // Kill two nodes, leaving only one alive. The surviving node, which
        // may or may not be the previous leader, cannot form a quorum with
        // itself alone, so future client_write attempts must fail or time out.
        let survivor_id = 3u64;
        for n in &mut nodes {
            if n.node_id != survivor_id {
                n.shutdown_runtime(group_id).await;
                n.server.abort();
            }
        }

        // Give time for the survivor to notice peers are gone (election
        // timeouts may flap; we just want to assert "no progress on writes").
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        let survivor = &nodes[(survivor_id - 1) as usize];

        // A write attempt with a bounded timeout must not commit. We expect
        // either an explicit error (NoQuorum-shaped) or a timeout.
        let attempt = tokio::time::timeout(
            std::time::Duration::from_millis(800),
            survivor.state.runtime_client_write(
                group_id,
                BlockCommand::Write {
                    offset: 1024,
                    bytes: vec![0x44; 512],
                },
            ),
        )
        .await;
        match attempt {
            Err(_elapsed) => {
                // Timeout - expected when there's no quorum.
            }
            Ok(Err(_)) => {
                // Explicit error - also acceptable; Openraft may surface a
                // ChangeMembership / forward-to-leader / no-leader shape.
            }
            Ok(Ok(_)) => panic!("write committed without quorum"),
        }

        // The pre-failure committed bytes must still be readable on the
        // survivor's storage even though it's lost quorum.
        let groups = survivor.state.groups.lock().await;
        let replica = groups.get(&group_id).expect("replica exists");
        let bytes = replica.read_range(0, 512).expect("read pre-failure bytes");
        assert_eq!(bytes[0], 0x33, "pre-failure committed bytes survived");
        drop(groups);

        survivor.shutdown_runtime(group_id).await;
        survivor.server.abort();
    }
}
