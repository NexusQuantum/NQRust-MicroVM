use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct OkResponse {
    pub ok: bool,
}

impl Default for OkResponse {
    fn default() -> Self {
        Self { ok: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct CreateVmResponse {
    pub id: uuid::Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Vm {
    pub id: uuid::Uuid,
    pub name: String,
    pub state: String,
    pub host_id: uuid::Uuid,
    pub template_id: Option<uuid::Uuid>,
    pub host_addr: String,
    pub api_sock: String,
    pub tap: String,
    pub log_path: String,
    pub http_port: i32,
    pub fc_unit: String,
    pub vcpu: i32,
    pub mem_mib: i32,
    pub kernel_path: String,
    pub rootfs_path: String,
    pub source_snapshot_id: Option<uuid::Uuid>,
    pub guest_ip: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_by_user_id: Option<uuid::Uuid>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListVmsResponse {
    pub items: Vec<Vm>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GetVmResponse {
    pub item: Vm,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateVmReq {
    pub name: String,
    pub vcpu: u8,
    pub mem_mib: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kernel_image_id: Option<uuid::Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rootfs_image_id: Option<uuid::Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kernel_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rootfs_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_snapshot_id: Option<uuid::Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TemplateSpec {
    pub vcpu: u8,
    pub mem_mib: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kernel_image_id: Option<uuid::Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rootfs_image_id: Option<uuid::Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kernel_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rootfs_path: Option<String>,
}

impl TemplateSpec {
    pub fn into_vm_req(self, name: String) -> CreateVmReq {
        CreateVmReq {
            name,
            vcpu: self.vcpu,
            mem_mib: self.mem_mib,
            kernel_image_id: self.kernel_image_id,
            rootfs_image_id: self.rootfs_image_id,
            kernel_path: self.kernel_path,
            rootfs_path: self.rootfs_path,
            source_snapshot_id: None,
            username: None,
            password: None,
            tags: vec![], // Templates don't have tags, user VMs get no tags by default
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VmDrive {
    pub id: uuid::Uuid,
    pub vm_id: uuid::Uuid,
    pub drive_id: String,
    pub path_on_host: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<i64>,
    pub is_root_device: bool,
    pub is_read_only: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub io_engine: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate_limiter: Option<serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VmNic {
    pub id: uuid::Uuid,
    pub vm_id: uuid::Uuid,
    pub iface_id: String,
    pub host_dev_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guest_mac: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rx_rate_limiter: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tx_rate_limiter: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network_id: Option<uuid::Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assigned_ip: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateDriveReq {
    pub drive_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_on_host: Option<String>,
    #[serde(default)]
    pub is_root_device: bool,
    #[serde(default)]
    pub is_read_only: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub io_engine: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate_limiter: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateDriveReq {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_on_host: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate_limiter: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateNicReq {
    /// Optional interface ID (e.g., "eth1"). If not provided, will auto-assign next sequential interface (eth1, eth2, eth3, etc.)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iface_id: Option<String>,
    pub network_id: uuid::Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guest_mac: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rx_rate_limiter: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tx_rate_limiter: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateNicReq {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rx_rate_limiter: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tx_rate_limiter: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListDrivesResponse {
    pub items: Vec<VmDrive>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListNicsResponse {
    pub items: Vec<VmNic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MachineConfigPatchReq {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vcpu_count: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mem_size_mib: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub smt: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub track_dirty_pages: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu_template: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub huge_pages: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CpuConfigReq {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpuid_modifiers: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msr_modifiers: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reg_modifiers: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vcpu_features: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kvm_capabilities: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MmdsDataReq {
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MmdsConfigReq {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network_interfaces: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ipv4_address: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub imds_compat: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VsockConfigReq {
    pub guest_cid: u32,
    pub uds_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vsock_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EntropyConfigReq {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate_limiter: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SerialConfigReq {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LoggerUpdateReq {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_level: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_log_origin: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub module: Option<String>,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct VmDrivePathParams {
    pub id: uuid::Uuid,
    pub drive_id: uuid::Uuid,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct VmNicPathParams {
    pub id: uuid::Uuid,
    pub nic_id: uuid::Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateTemplateReq {
    pub name: String,
    pub spec: TemplateSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Template {
    pub id: uuid::Uuid,
    pub name: String,
    pub spec: TemplateSpec,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct CreateTemplateResp {
    pub id: uuid::Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateTemplateReq {
    pub name: String,
    pub spec: TemplateSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateTemplateResp {
    pub item: Template,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListTemplatesResp {
    pub items: Vec<Template>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GetTemplateResp {
    pub item: Template,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InstantiateTemplateReq {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct InstantiateTemplateResp {
    pub id: uuid::Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VmSummary {
    pub id: uuid::Uuid,
    pub name: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Snapshot {
    pub id: uuid::Uuid,
    pub vm_id: uuid::Uuid,
    pub snapshot_path: String,
    pub mem_path: String,
    pub size_bytes: i64,
    pub state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<uuid::Uuid>,
    #[serde(default)]
    pub track_dirty_pages: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct CreateSnapshotRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<uuid::Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub track_dirty_pages: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct CreateSnapshotResponse {
    pub id: uuid::Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListSnapshotsResponse {
    pub items: Vec<Snapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GetSnapshotResponse {
    pub item: Snapshot,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct InstantiateSnapshotReq {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct InstantiateSnapshotResp {
    pub id: uuid::Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Image {
    pub id: uuid::Uuid,
    pub kind: String,
    pub name: String,
    pub host_path: String,
    pub sha256: String,
    pub size: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateImageReq {
    pub kind: String,
    pub name: String,
    pub host_path: String,
    pub sha256: String,
    pub size: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct CreateImageResp {
    pub id: uuid::Uuid,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, IntoParams)]
pub struct ImageFilter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListImagesResp {
    pub items: Vec<Image>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GetImageResp {
    pub item: Image,
}

// Docker Hub API types
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DockerHubSearchReq {
    pub query: String,
    #[serde(default)]
    pub limit: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DockerHubImage {
    pub name: String,
    pub description: Option<String>,
    pub star_count: i32,
    pub is_official: bool,
    pub is_automated: bool,
    pub pull_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DockerHubSearchResp {
    pub items: Vec<DockerHubImage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DockerImageTag {
    pub name: String,
    pub last_updated: Option<String>,
    pub digest: Option<String>,
    pub size: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DockerImageTagsResp {
    pub items: Vec<DockerImageTag>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DownloadDockerImageReq {
    pub image: String, // e.g., "nginx:latest" or "library/nginx:1.25"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry_auth: Option<RegistryAuth>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DownloadDockerImageResp {
    pub id: uuid::Uuid,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RegisterHostRequest {
    pub name: String,
    pub addr: String,
    #[serde(default)]
    pub capabilities: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct RegisterHostResponse {
    pub id: uuid::Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HostHeartbeatRequest {
    #[serde(default)]
    pub capabilities: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct TailLogResponse {
    pub text: String,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct VmPathParams {
    pub id: uuid::Uuid,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct TemplatePathParams {
    pub id: uuid::Uuid,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct SnapshotPathParams {
    pub id: uuid::Uuid,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct ImagePathParams {
    pub id: uuid::Uuid,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct HostPathParams {
    pub id: uuid::Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BalloonConfig {
    pub amount_mib: u64,
    pub deflate_on_oom: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stats_polling_interval_s: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BalloonStatsConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stats_polling_interval_s: Option<u64>,
}

// ========================================
// Functions (Serverless Lambda)
// ========================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Function {
    pub id: uuid::Uuid,
    pub name: String,
    pub runtime: String, // node, python, go, rust
    pub code: String,
    pub handler: String,
    pub timeout_seconds: i32,
    pub memory_mb: i32,
    pub vcpu: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env_vars: Option<serde_json::Value>,
    // MicroVM information
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vm_id: Option<uuid::Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guest_ip: Option<String>,
    pub port: i32,
    pub state: String, // creating, ready, error, stopped
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_by_user_id: Option<uuid::Uuid>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_invoked_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FunctionInvocation {
    pub id: uuid::Uuid,
    pub function_id: uuid::Uuid,
    pub status: String, // success, error, timeout
    pub duration_ms: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_used_mb: Option<i32>,
    pub request_id: String,
    pub event: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response: Option<serde_json::Value>,
    #[serde(default)]
    pub logs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub invoked_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateFunctionReq {
    pub name: String,
    pub runtime: String,
    pub code: String,
    pub handler: String,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: i32,
    #[serde(default = "default_memory")]
    pub memory_mb: i32,
    #[serde(default = "default_vcpu")]
    pub vcpu: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env_vars: Option<serde_json::Value>,
}

fn default_timeout() -> i32 {
    30
}

fn default_memory() -> i32 {
    128
}

fn default_vcpu() -> i32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateFunctionReq {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handler: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_mb: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env_vars: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InvokeFunctionReq {
    pub event: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct CreateFunctionResp {
    pub id: uuid::Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListFunctionsResp {
    pub items: Vec<Function>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GetFunctionResp {
    pub item: Function,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InvokeFunctionResp {
    pub request_id: String,
    pub status: String,
    pub duration_ms: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response: Option<serde_json::Value>,
    #[serde(default)]
    pub logs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListInvocationsResp {
    pub items: Vec<FunctionInvocation>,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct FunctionPathParams {
    pub id: uuid::Uuid,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct ListInvocationsParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
}

// ========================================
// Containers (Docker/OCI)
// ========================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Container {
    pub id: uuid::Uuid,
    pub name: String,
    pub image: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env_vars: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub volumes: Vec<VolumeMount>,
    #[serde(default)]
    pub port_mappings: Vec<PortMapping>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu_limit: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_limit_mb: Option<i32>,
    pub restart_policy: String,
    pub state: String, // creating, running, stopped, restarting, error, paused
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_id: Option<uuid::Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container_runtime_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_by_user_id: Option<uuid::Uuid>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stopped_at: Option<chrono::DateTime<chrono::Utc>>,
    // Computed fields (not in DB)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uptime_seconds: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu_percent: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_used_mb: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guest_ip: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PortMapping {
    pub host: i32,
    pub container: i32,
    pub protocol: String, // tcp, udp
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VolumeMount {
    pub host: String,
    pub container: String,
    #[serde(default)]
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RegistryAuth {
    pub username: String,
    pub password: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_address: Option<String>, // e.g., "registry.example.com" or leave None for Docker Hub
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateContainerReq {
    pub name: String,
    pub image: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env_vars: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub volumes: Vec<VolumeMount>,
    #[serde(default)]
    pub port_mappings: Vec<PortMapping>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu_limit: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_limit_mb: Option<i32>,
    #[serde(default = "default_restart_policy")]
    pub restart_policy: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry_auth: Option<RegistryAuth>,
}

fn default_restart_policy() -> String {
    "no".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateContainerReq {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env_vars: Option<std::collections::HashMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu_limit: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_limit_mb: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restart_policy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct CreateContainerResp {
    pub id: uuid::Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListContainersResp {
    pub items: Vec<Container>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GetContainerResp {
    pub item: Container,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ContainerStats {
    pub id: uuid::Uuid,
    pub container_id: uuid::Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu_percent: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_used_mb: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_limit_mb: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network_rx_bytes: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network_tx_bytes: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_read_bytes: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_write_bytes: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pids: Option<i32>,
    pub recorded_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ContainerStatsResp {
    pub items: Vec<ContainerStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ContainerLog {
    pub id: uuid::Uuid,
    pub container_id: uuid::Uuid,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub stream: String, // stdout, stderr
    pub message: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ContainerLogsResp {
    pub items: Vec<ContainerLog>,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct ContainerPathParams {
    pub id: uuid::Uuid,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct ListContainersParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_id: Option<uuid::Uuid>,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct ContainerLogsParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub since: Option<String>, // RFC3339 timestamp
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<String>, // RFC3339 timestamp
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tail: Option<i64>, // Last N lines
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub follow: Option<bool>, // Stream logs
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExecCommandReq {
    pub command: Vec<String>,
    #[serde(default)]
    pub attach_stdin: bool,
    #[serde(default)]
    pub attach_stdout: bool,
    #[serde(default)]
    pub attach_stderr: bool,
    #[serde(default)]
    pub tty: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExecCommandResp {
    pub exec_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
}

// User Management Types

/// User role for role-based access control (RBAC)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Administrator with full access to all resources
    Admin,
    /// Regular user who can create and manage their own resources
    User,
    /// Viewer with read-only access to all resources
    Viewer,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Admin => "admin",
            Role::User => "user",
            Role::Viewer => "viewer",
        }
    }
}

impl std::str::FromStr for Role {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "admin" => Ok(Role::Admin),
            "user" => Ok(Role::User),
            "viewer" => Ok(Role::Viewer),
            _ => Err(format!("Invalid role: {}", s)),
        }
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Actions that can be audited in the system
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    // Authentication actions
    Login,
    Logout,
    LoginFailed,

    // User management actions
    CreateUser,
    UpdateUser,
    DeleteUser,

    // VM actions
    CreateVm,
    StartVm,
    StopVm,
    PauseVm,
    ResumeVm,
    DeleteVm,
    UpdateVm,
    CreateVmSnapshot,
    RestoreVmSnapshot,

    // Function actions
    CreateFunction,
    InvokeFunction,
    UpdateFunction,
    DeleteFunction,

    // Container actions
    CreateContainer,
    StartContainer,
    StopContainer,
    DeleteContainer,

    // Network actions
    CreateNetwork,
    UpdateNetwork,
    DeleteNetwork,
    CreateNic,
    DeleteNic,

    // Volume actions
    CreateVolume,
    AttachVolume,
    DetachVolume,
    DeleteVolume,
}

impl AuditAction {
    pub fn as_str(&self) -> &str {
        match self {
            AuditAction::Login => "login",
            AuditAction::Logout => "logout",
            AuditAction::LoginFailed => "login_failed",
            AuditAction::CreateUser => "create_user",
            AuditAction::UpdateUser => "update_user",
            AuditAction::DeleteUser => "delete_user",
            AuditAction::CreateVm => "create_vm",
            AuditAction::StartVm => "start_vm",
            AuditAction::StopVm => "stop_vm",
            AuditAction::PauseVm => "pause_vm",
            AuditAction::ResumeVm => "resume_vm",
            AuditAction::DeleteVm => "delete_vm",
            AuditAction::UpdateVm => "update_vm",
            AuditAction::CreateVmSnapshot => "create_vm_snapshot",
            AuditAction::RestoreVmSnapshot => "restore_vm_snapshot",
            AuditAction::CreateFunction => "create_function",
            AuditAction::InvokeFunction => "invoke_function",
            AuditAction::UpdateFunction => "update_function",
            AuditAction::DeleteFunction => "delete_function",
            AuditAction::CreateContainer => "create_container",
            AuditAction::StartContainer => "start_container",
            AuditAction::StopContainer => "stop_container",
            AuditAction::DeleteContainer => "delete_container",
            AuditAction::CreateNetwork => "create_network",
            AuditAction::UpdateNetwork => "update_network",
            AuditAction::DeleteNetwork => "delete_network",
            AuditAction::CreateNic => "create_nic",
            AuditAction::DeleteNic => "delete_nic",
            AuditAction::CreateVolume => "create_volume",
            AuditAction::AttachVolume => "attach_volume",
            AuditAction::DetachVolume => "detach_volume",
            AuditAction::DeleteVolume => "delete_volume",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct User {
    pub id: uuid::Uuid,
    pub username: String,
    pub role: Role,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_login_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LoginResponse {
    pub token: String,
    pub user: User,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub role: Role,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateUserRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<Role>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListUsersResponse {
    pub items: Vec<User>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GetUserResponse {
    pub item: User,
}

// User Preferences
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct NotificationPreferences {
    #[serde(default)]
    pub email: bool,
    #[serde(default)]
    pub browser: bool,
    #[serde(default)]
    pub desktop: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct VmDefaults {
    #[serde(default)]
    pub vcpu: u8,
    #[serde(default)]
    pub mem_mib: u32,
    #[serde(default)]
    pub disk_gb: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct UserPreferences {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_format: Option<String>,
    #[serde(default)]
    pub notifications: NotificationPreferences,
    #[serde(default)]
    pub vm_defaults: VmDefaults,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_refresh: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metrics_retention: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GetPreferencesResponse {
    pub preferences: UserPreferences,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdatePreferencesRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_format: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notifications: Option<NotificationPreferences>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vm_defaults: Option<VmDefaults>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_refresh: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metrics_retention: Option<u32>,
}

// Profile Management
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateProfileRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, IntoParams)]
pub struct UserPathParams {
    pub id: uuid::Uuid,
}

// Audit Log Types

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuditLog {
    pub id: uuid::Uuid,
    pub user_id: Option<uuid::Uuid>,
    pub username: String,
    pub action: String,
    pub resource_type: Option<String>,
    pub resource_id: Option<uuid::Uuid>,
    pub details: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub success: bool,
    pub error_message: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListAuditLogsResponse {
    pub items: Vec<AuditLog>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, IntoParams)]
pub struct AuditLogQueryParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<uuid::Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<i64>,
}
