export type VMState = "stopped" | "running" | "paused"

export interface VMConfig {
  machine: {
    vcpu_count: number // 1-32
    mem_size_mib: number // >= 128
    smt: boolean
    cpu_template: string
  }
  boot: {
    kernel_image_path: string
    initrd_path?: string
    boot_args?: string
  }
  metadata: {
    name: string
    description?: string
    tags: Record<string, string>
  }
}

export interface VM {
  id: string
  name: string
  description?: string
  created_at: string
  updated_at: string
  config: VMConfig
  state: VMState
  firecracker_pid?: number
  socket_path: string
  bridge_port?: number
  tags: Record<string, string>
  owner: string
  environment: string
}

export interface RateLimiterTokenBucket {
  size: number
  one_time_burst: number
  refill_time: number
}

export interface DriveConfig {
  drive_id: string
  path_on_host: string
  is_root_device: boolean
  is_read_only: boolean
  cache_type: "Unsafe" | "Writeback"
  io_engine: "Sync" | "Async"
  rate_limiter?: {
    bandwidth?: RateLimiterTokenBucket
    ops?: RateLimiterTokenBucket
  }
}

export interface NetworkConfig {
  iface_id: string
  host_dev_name: string
  guest_mac?: string
  allow_mmds_requests: boolean
  rx_rate_limiter?: RateLimiterTokenBucket
  tx_rate_limiter?: RateLimiterTokenBucket
}

export interface VMMetrics {
  vm_id: string
  timestamp: string
  cpu_usage_percent: number
  memory_usage_bytes: number
  memory_available_bytes: number
  network_rx_bytes: number
  network_tx_bytes: number
  disk_read_bytes: number
  disk_write_bytes: number
  cpu_trend?: "up" | "down" | "stable"
  memory_pressure?: "low" | "medium" | "high"
}

export interface ApiError {
  error: string
  fault_message?: string
  status: number
  suggestion?: string
  request_id: string
}

export interface RegistryImage {
  id: string
  name: string
  path: string
  type: "kernel" | "initrd" | "rootfs" | "data"
  size_bytes: number
  created_at: string
  tags: string[]
}

export interface SnapshotConfig {
  snapshot_type: "Diff" | "Full"
  snapshot_path: string
  mem_file_path: string
  version?: string
}
