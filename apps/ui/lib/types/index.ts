// Types matching the new Rust backend (nexus-types)

export interface OkResponse {
  ok: boolean;
}

export interface CreateVmResponse {
  id: string;
}

export interface ImageResponse {
  "id": string,
  "kind": string,
  "name": string,
  "host_path": string,
  "sha256": string,
  "size": number,
  "project": string,
  "created_at": string,
  "updated_at": string
}

export interface Vm {
  id: string;
  name: string;
  state: string;
  host_id: string;
  template_id?: string;
  host_addr: string;
  api_sock: string;
  tap: string;
  log_path: string;
  http_port: number;
  fc_unit: string;
  vcpu: number;
  mem_mib: number;
  kernel_path: string;
  rootfs_path: string;
  source_snapshot_id?: string;
  guest_ip: string;
  tags: string[];
  created_by_user_id?: string;
  created_at: string;
  updated_at: string;
  // Runtime metrics (populated separately, not from REST list)
  cpu_usage_percent?: number;
  memory_usage_percent?: number;
}

export interface UpdateVmRequest {
  name?: string;
  tags?: string[];
}

export interface ListVmsResponse {
  items: Vm[];
}

export interface GetVmResponse {
  item: Vm;
}

export interface CreateVmReq {
  name: string;
  vcpu: number;
  mem_mib: number;
  kernel_image_id?: string;
  rootfs_image_id?: string;
  kernel_path?: string;
  rootfs_path?: string;
  source_snapshot_id?: string;
  username?: string;
  password?: string;
  rootfs_size_mb?: number;
  network_id?: string;
}

export interface TemplateSpec {
  vcpu: number;
  mem_mib: number;
  kernel_image_id?: string;
  rootfs_image_id?: string;
  kernel_path?: string;
  rootfs_path?: string;
}

export interface CreateTemplateReq {
  name: string;
  spec: TemplateSpec;
}

export interface UpdateTemplateReq {
  name?: string;
  description?: string;
  spec?: {
    vcpu?: number;
    mem_mib?: number;
  };
}

export interface Template {
  kernel_path: string;
  mem_mib: number;
  vcpu: number;
  description: string;
  id: string;
  name: string;
  spec: TemplateSpec;
  created_at: string;
  updated_at: string;
}

export interface CreateTemplateResp {
  id: string;
}

export interface ListTemplatesResp {
  items: Template[];
}

export interface GetTemplateResp {
  item: Template;
}

export interface InstantiateTemplateReq {
  name: string;
}

export interface InstantiateTemplateResp {
  id: string;
}

export interface VmSummary {
  id: string;
  name: string;
  state: string;
}

export interface Snapshot {
  id: string;
  vm_id: string;
  snapshot_path: string;
  mem_path: string;
  size_bytes: number;
  state: string;
  name?: string;
  created_at: string;
  updated_at: string;
}

export interface CreateSnapshotRequest {
  // Empty for now, backend doesn't require any fields
}

export interface CreateSnapshotResponse {
  id: string;
}

export interface ListSnapshotsResponse {
  items: Snapshot[];
}

export interface GetSnapshotResponse {
  item: Snapshot;
}

export interface InstantiateSnapshotReq {
  name?: string
  snapshot_path?: any,
  mem_file_path?: string
}

export interface InstantiateSnapshotResp {
  id: string;
  name: string;
}

export interface Image {
  id: string;
  kind: string;
  name: string;
  host_path: string;
  sha256: string;
  size: number;
  project?: string;
  created_at: string;
  updated_at: string;
}

export interface CreateImageReq {
  kind: string;
  name: string;
  host_path: string;
  sha256: string;
  size: number;
  project?: string;
}

export interface CreateImageResp {
  id: string;
}

export interface ImageFilter {
  kind?: string;
  project?: string;
  name?: string;
}

export interface ListImagesResp {
  items: Image[];
}

export interface GetImageResp {
  item: Image;
}

export interface RegisterHostRequest {
  name: string;
  addr: string;
  capabilities?: any;
}

export interface RegisterHostResponse {
  id: string;
}

export interface HostHeartbeatRequest {
  capabilities?: any;
}

export interface TailLogResponse {
  text: string;
}

// Audit Logs
export interface AuditLog {
  id: string;
  user_id: string | null;
  username: string;
  action: string;
  resource_type: string | null;
  resource_id: string | null;
  details: Record<string, unknown> | null;
  ip_address: string | null;
  success: boolean;
  error_message: string | null;
  created_at: string;
}

export interface ListAuditLogsResponse {
  items: AuditLog[];
  total: number;
}

export interface AuditLogQueryParams {
  action?: string;
  resource_type?: string;
  limit?: number;
  offset?: number;
}

export interface DbConnectionInfo {
  host: string;
  port: string;
  database: string;
  username: string;
  connection_string_masked: string;
}

export interface SystemStats {
  total_hosts: number;
  total_vms: number;
  running_vms: number;
  total_functions: number;
  total_containers: number;
  running_containers: number;
}

// Path parameters for API routes
export interface VmPathParams {
  id: string;
}

export interface TemplatePathParams {
  id: string;
}

export interface SnapshotPathParams {
  id: string;
}

export interface ImagePathParams {
  id: string;
}

export interface HostPathParams {
  id: string;
}

// VM Device types
export interface VmDrive {
  id: string;
  vm_id: string;
  drive_id: string;
  path_on_host: string;
  size_bytes?: number;
  is_root_device: boolean;
  is_read_only: boolean;
  cache_type?: string;
  io_engine?: string;
  rate_limiter?: any;
  created_at: string;
  updated_at: string;
}

export interface CreateDriveReq {
  drive_id: string;
  path_on_host?: string | null; // Optional - manager will auto-provision if omitted
  size_bytes?: number | null; // Size hint for auto-provisioned disks
  is_root_device?: boolean;
  is_read_only?: boolean;
  cache_type?: string;
  io_engine?: string;
  rate_limiter?: any;
}

export interface UpdateDriveReq {
  path_on_host?: string;
  rate_limiter?: any;
}

export interface ListDrivesResponse {
  items: VmDrive[];
}

export interface VmNic {
  id: string;
  vm_id: string;
  iface_id: string;
  host_dev_name: string;
  guest_mac?: string;
  rx_rate_limiter?: any;
  tx_rate_limiter?: any;
  network_id?: string;
  assigned_ip?: string;
  created_at: string;
  updated_at: string;
}

export interface CreateNicReq {
  iface_id: string;
  host_dev_name: string;
  guest_mac?: string;
  rx_rate_limiter?: {
    size?: number;
    one_time_burst?: number;
    refill_time?: number;
  };
  tx_rate_limiter?: {
    size?: number;
    one_time_burst?: number;
    refill_time?: number;
  };
}

export interface UpdateNicReq {
  rx_rate_limiter?: {
    size?: number;
    one_time_burst?: number;
    refill_time?: number;
  };
  tx_rate_limiter?: {
    size?: number;
    one_time_burst?: number;
    refill_time?: number;
  };
}

export interface ListNicsResponse {
  items: VmNic[];
}

// Port Forwarding types
export interface PortForward {
  id: string;
  vm_id: string;
  host_port: number;
  guest_port: number;
  protocol: string;
  description?: string;
  created_at: string;
  updated_at: string;
}

export interface CreatePortForwardReq {
  host_port: number;
  guest_port: number;
  protocol?: string;
  description?: string;
}

export interface ListPortForwardsResponse {
  items: PortForward[];
}

// Balloon device types
export interface BalloonConfig {
  amount_mib: number;
  deflate_on_oom: boolean;
  stats_polling_interval_s?: number;
}

export interface BalloonStatsConfig {
  stats_polling_interval_s?: number;
}

// Entropy device types
export interface EntropyConfigReq {
  // Add fields as needed from OpenAPI spec
}

// Serial device types
export interface SerialConfigReq {
  // Add fields as needed from OpenAPI spec
}

// Logger types
export interface LoggerUpdateReq {
  level?: string;
  log_path?: string;
  module?: string;
  show_level?: boolean;
  show_log_origin?: boolean;
}

// Error types
export interface NexusError {
  error: string;
  fault_message?: string;
  status: number;
  suggestion?: string;
  request_id: string;
}

// Additional types for UI compatibility
export interface VmMetrics {
  cpu_usage_percent: number;
  memory_usage_percent: number;
  memory_used_kb: number;
  memory_total_kb: number;
  network_in_bytes: number;
  network_out_bytes: number;
  disk_read_bytes: number;
  disk_write_bytes: number;
}

export interface RateLimiter {
  bandwidth?: {
    size: number;
    refill_time: number;
  };
  ops?: {
    size: number;
    refill_time: number;
  };
}

// Function Types (for serverless functions)
export interface TestFunction {
  code: string;
  runtime: 'node' | 'python';
  handler: string;
  event: any;
}

export interface Function {
  state: "creating" | "booting" | "deploying" | "error" |"ready";
  id: string;
  name: string;
  runtime: "python" | "javascript" | "typescript";
  handler: string;
  timeout_seconds: number;
  code: string;
  vcpu: number;
  memory_mb: number;
  env_vars?: Record<string, string>;
  created_by_user_id?: string;
  created_at: string;
  updated_at: string;
  last_invoked_at?: string;
  invocation_count_24h?: number;
  avg_duration_ms?: number;
  vm_id?: string
  port?: number;
  guest_ip?: string;
}

export interface FunctionInvocation {
  id: string
  function_id: string
  status: "success" | "error" | "timeout"
  duration_ms: number
  memory_used_mb: number
  request_id: string
  event: any
  response?: any
  logs: string[]
  error?: string
  invoked_at: string
}

export interface ListInvocationsResp {
  items: FunctionInvocation[]
}

export type JSONValue =
  | string
  | number
  | boolean
  | null
  | { [key: string]: JSONValue }
  | JSONValue[];

export interface InvokeFunction {
  event: {
    [key: string]: JSONValue;
  }
}

export interface CreateFunction {
  "name": string,
  "runtime": "python" | "javascript" | "typescript";
  "handler": string,
  "code": string,
  "vcpu": number,
  "memory_mb": number
}

export interface UpdateFunction {
  "name": string,
  // "runtime": string,  // "node" or "python"
  "handler": string,
  "code": string,
  // "vcpu": number,
  "memory_mb": number,
  "timeout_seconds" : number
}

// Container Types (matching backend API)
export interface Container {
  id: string;
  name: string;
  image: string;
  command?: string;
  args: string[];
  env_vars: Record<string, string>;
  volumes: VolumeMount[];
  port_mappings: PortMapping[];
  cpu_limit?: number;
  memory_limit_mb?: number;
  restart_policy: string;
  state: "creating" | "booting" | "initializing" | "running" | "stopped" | "paused" | "error";
  container_runtime_id?: string;
  error_message?: string;
  created_by_user_id?: string;
  created_at: string;
  updated_at: string;
  started_at?: string;
  stopped_at?: string;
  uptime_seconds?: number;
  cpu_percent?: number;
  memory_used_mb?: number;
  guest_ip?: string;
}

export interface PortMapping {
  host: number;
  container: number;
  protocol: "tcp" | "udp";
}

export interface VolumeMount {
  host: string;
  container: string;
  read_only?: boolean;
}

export interface RegistryAuth {
  username: string;
  password: string;
  server_address?: string; // Optional, defaults to Docker Hub
}

export interface CreateContainerReq {
  name: string;
  image: string;
  command?: string;
  args?: string[];
  env_vars?: Record<string, string>;
  volumes?: VolumeMount[];
  port_mappings?: PortMapping[];
  cpu_limit?: number;
  memory_limit_mb?: number;
  restart_policy?: string;
  registry_auth?: RegistryAuth;
}

export interface CreateContainerResp {
  id: string;
}

export interface ListContainersResp {
  items: Container[];
}

export interface GetContainerResp {
  item: Container;
}

export interface ContainerStats {
  cpu_percent?: number;
  memory_used_mb?: number;
  memory_limit_mb?: number;
  network_rx_bytes?: number;
  network_tx_bytes?: number;
  block_read_bytes?: number;
  block_write_bytes?: number;
  pids?: number;
  recorded_at: string;
}

export interface ContainerStatsResp {
  items: ContainerStats[];
}

export interface ContainerLog {
  container_id: string;
  timestamp: string;
  stream: string;
  message: string;
}

export interface ContainerLogsResp {
  items: ContainerLog[];
}

export interface ContainerExecReq {
  command: string[];
  attach_stdout?: boolean;
  attach_stderr?: boolean;
}

export interface UpdateContainerReq {
  name?: string;
  env_vars?: Record<string, string>;
  cpu_limit?: number;
  memory_limit_mb?: number;
  restart_policy?: string;
}

// Dashboard Stats
export interface DashboardStats {
  total_vms: number;
  running_vms: number;
  total_functions: number;
  invocations_24h: number;
  total_containers: number;
  running_containers: number;
  total_hosts: number;
}

// Docker Hub API Types
export interface DockerHubSearchReq {
  query: string;
  limit?: number;
}

export interface DockerHubImage {
  name: string;
  description?: string;
  star_count: number;
  is_official: boolean;
  is_automated: boolean;
  pull_count: number;
}

export interface DockerHubSearchResp {
  items: DockerHubImage[];
}

export interface DockerImageTag {
  name: string;
  last_updated?: string;
  digest?: string;
  size?: number;
}

export interface DockerImageTagsResp {
  items: DockerImageTag[];
}

export interface DownloadDockerImageReq {
  image: string; // e.g., "nginx:latest"
  registry_auth?: RegistryAuth;
}

export interface DownloadDockerImageResp {
  id: string;
  path: string;
}

export interface DownloadProgress {
  image: string;
  status: string;
  current_bytes: number;
  total_bytes: number;
  completed: boolean;
  error?: string;
}

// Host Management Types
export interface Host {
  id: string;
  name: string;
  addr: string;
  status: "healthy" | "degraded" | "offline";
  capabilities_json?: {
    bridge?: string;
    run_dir?: string;
    cpus?: number;
    total_memory_mb?: number;
    total_disk_gb?: number;
    used_disk_gb?: number;
  };
  total_cpus?: number;
  total_memory_mb?: number;
  total_disk_gb?: number;
  used_disk_gb?: number;
  vm_count: number;
  last_seen_at: string;
  last_metrics_at?: string;
}

export interface ListHostsResponse {
  items: Host[];
}

export interface GetHostResponse {
  item: Host;
}

// Network Management Types
export interface Network {
  id: string;
  name: string;
  description?: string;
  type: "nat" | "bridged" | "isolated" | "vxlan";
  vlan_id?: number;
  vni?: number;
  bridge_name: string;
  host_id?: string;
  host_name?: string;
  cidr?: string;
  gateway?: string;
  status: "pending" | "provisioning" | "active" | "error" | "deleting";
  error_message?: string;
  managed: boolean;
  dhcp_enabled: boolean;
  dhcp_range_start?: string;
  dhcp_range_end?: string;
  vm_count: number;
  participating_hosts?: number;
  created_at: string;
  updated_at: string;
}

export interface CreateNetworkRequest {
  name: string;
  description?: string;
  type: "nat" | "isolated" | "bridged" | "vxlan";
  host_id: string;
  cidr?: string;
  vlan_id?: number;
  dhcp_enabled?: boolean;
  dhcp_range_start?: string;
  dhcp_range_end?: string;
  /** Required for bridged networks: the physical NIC to attach */
  uplink_interface?: string;
  /** Required for VXLAN networks: the gateway host that runs DHCP + NAT */
  gateway_host_id?: string;
}

export interface HostInterface {
  name: string;
  mac: string;
  state: string;
  addresses: string[];
  is_management: boolean;
  master?: string;
}

export interface ListInterfacesResponse {
  interfaces: HostInterface[];
}

export interface UpdateNetworkRequest {
  name?: string;
  description?: string;
  cidr?: string;
  gateway?: string;
}

export interface NetworkDetailResponse {
  item: Network;
}

export interface ListNetworksResponse {
  items: Network[];
}

export interface GetNetworkResponse {
  item: Network;
}

export interface NetworkVmsResponse {
  vm_ids: string[];
}

export interface NetworkSuggestion {
  bridge_name: string;
  cidr: string;
  gateway: string;
  dhcp_range_start: string;
  dhcp_range_end: string;
}

// Volume Management Types
export interface Volume {
  id: string;
  name: string;
  description?: string;
  path: string;
  size_bytes: number;
  size_gb: number;
  type: "raw" | "qcow2" | "ext4";
  status: "available" | "attached" | "creating" | "error";
  host_id: string;
  host_name?: string;
  attached_to_vm_id?: string;
  attached_to_vm_name?: string;
  created_at: string;
}

export interface CreateVolumeRequest {
  name: string;
  description?: string;
  size_gb: number;
  type: "raw" | "qcow2" | "ext4";
  host_id: string;
}

export interface AttachVolumeRequest {
  vm_id: string;
  drive_id: string;
}

export interface DetachVolumeRequest {
  vm_id: string;
}

export interface CreateVolumeResponse {
  id: string;
}

export interface ListVolumesResponse {
  items: Volume[];
}

export interface GetVolumeResponse {
  item: Volume;
}

// User Management Types
export interface User {
  id: string;
  username: string;
  role: "admin" | "user" | "viewer";
  created_at: string;
  last_login_at?: string;
  avatar_path?: string;
  timezone?: string;
  theme?: string;
}

export interface CreateUserRequest {
  username: string;
  password: string;
  role: "admin" | "user" | "viewer";
}

export interface UpdateUserRequest {
  username?: string;
  password?: string;
  role?: "admin" | "user" | "viewer";
}

export interface ListUsersResponse {
  items: User[];
}

export interface GetUserResponse {
  item: User;
}

export interface CreateUserResponse {
  id: string;
}

// User Preferences Types
export interface NotificationPreferences {
  email: boolean;
  browser: boolean;
  desktop: boolean;
}

export interface VmDefaults {
  vcpu: number;
  mem_mib: number;
  disk_gb: number;
}

export interface UserPreferences {
  timezone?: string;
  theme?: string;
  date_format?: string;
  notifications: NotificationPreferences;
  vm_defaults: VmDefaults;
  auto_refresh?: number;
  metrics_retention?: number;
}

export interface GetPreferencesResponse {
  preferences: UserPreferences;
}

export interface UpdatePreferencesRequest {
  timezone?: string;
  theme?: string;
  date_format?: string;
  notifications?: NotificationPreferences;
  vm_defaults?: VmDefaults;
  auto_refresh?: number;
  metrics_retention?: number;
}

// Profile Management Types
export interface UpdateProfileRequest {
  username?: string;
}

export interface ChangePasswordRequest {
  current_password: string;
  new_password: string;
}

// Time-Series Metrics Types
export interface HostMetric {
  host_id: string;
  recorded_at: string;
  cpu_usage_percent: number | null;
  memory_used_mb: number | null;
  memory_total_mb: number | null;
  disk_used_gb: number | null;
  disk_total_gb: number | null;
}

export interface VmMetric {
  vm_id: string;
  recorded_at: string;
  cpu_usage_percent: number | null;
  memory_usage_percent: number | null;
  memory_used_kb: number | null;
  memory_total_kb: number | null;
  load_average: number | null;
}

export interface ContainerMetric {
  container_id: string;
  recorded_at: string;
  cpu_percent: number | null;
  memory_used_mb: number | null;
  memory_limit_mb: number | null;
  network_rx_bytes: number | null;
  network_tx_bytes: number | null;
  block_read_bytes: number | null;
  block_write_bytes: number | null;
  pids: number | null;
}

export interface MetricsQueryParams {
  from?: string;
  to?: string;
  limit?: number;
}
