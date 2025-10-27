// Types matching the new Rust backend (nexus-types)

export interface OkResponse {
  ok: boolean;
}

export interface CreateVmResponse {
  id: string;
}

export interface Vm {
  memory_usage_percent: undefined;
  cpu_usage_percent: undefined;
  guest_ip: string;
  vm_name: string;
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
  created_at: string;
  updated_at: string;
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

export interface Template {
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
  name?: string;
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
export interface CreateFunction {
  name: string;
  runtime: "node" | "python" | "go" | "rust";
  handler: string;
  code: string;
  vcpu: number;
  memory_mb: number;
}

export interface Function {
  id: string;
  name: string;
  runtime: "node" | "python" | "go" | "rust";
  handler: string;
  timeout_seconds: number;
  code: string;
  vcpu: number;
  memory_mb: number;
  env_vars?: Record<string, string>;
  created_at: string;
  updated_at: string;
  last_invoked_at?: string;
  invocation_count_24h?: number;
  avg_duration_ms?: number;
}

export interface FunctionInvocation {
  id: string;
  function_id: string;
  status: "success" | "error" | "timeout";
  duration_ms: number;
  memory_used_mb: number;
  request_id: string;
  event: any;
  response?: any;
  logs: string[];
  error?: string;
  invoked_at: string;
}

// Container Types
export interface Container {
  id: string;
  name: string;
  image: string;
  status: "running" | "stopped" | "restarting" | "error";
  uptime_seconds?: number;
  cpu_percent?: number;
  memory_used_mb?: number;
  memory_limit_mb?: number;
  port_mappings: PortMapping[];
  env_vars?: Record<string, string>;
  volumes?: VolumeMount[];
  command?: string;
  args?: string[];
  restart_policy?: string;
  created_at: string;
  started_at?: string;
}

export interface PortMapping {
  host: number;
  container: number;
  protocol: "tcp" | "udp";
}

export interface VolumeMount {
  host: string;
  container: string;
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
