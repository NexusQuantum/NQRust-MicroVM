import { apiClient } from "./http"
import type { 
  DriveConfig, 
  NetworkConfig,
  RateLimiterTokenBucket 
} from "@/types/firecracker"

/**
 * Direct Firecracker API passthrough wrappers
 * Allow-listed endpoints only - no deprecated endpoints
 */
export class FirecrackerApi {
  private static readonly FC_PREFIX = "/firecracker"

  // ========================================
  // VM Status & Info
  // ========================================

  /**
   * GET /api/firecracker/status - Check Firecracker instance status
   */
  async getStatus(): Promise<{
    id?: string
    state?: string
    vmm_version?: string
    app_name?: string
  }> {
    return apiClient.get(`${FirecrackerApi.FC_PREFIX}/status`)
  }

  // ========================================
  // Machine Configuration 
  // ========================================

  /**
   * PUT /machine-config - Configure machine settings (pre-boot only)
   */
  async putMachineConfig(config: {
    vcpu_count: number
    mem_size_mib: number
    smt?: boolean
    cpu_template?: string
    track_dirty_pages?: boolean
  }): Promise<void> {
    return apiClient.put(`${FirecrackerApi.FC_PREFIX}/machine-config`, config)
  }

  /**
   * PUT /boot-source - Configure boot source (pre-boot only)
   */
  async putBootSource(config: {
    kernel_image_path: string
    initrd_path?: string
    boot_args?: string
  }): Promise<void> {
    return apiClient.put(`${FirecrackerApi.FC_PREFIX}/boot-source`, config)
  }

  // ========================================
  // Storage Management
  // ========================================

  /**
   * PUT /drives/{id} - Create/configure drive (pre-boot only)
   */
  async putDrive(driveId: string, config: DriveConfig): Promise<void> {
    return apiClient.put(`${FirecrackerApi.FC_PREFIX}/drives/${driveId}`, config)
  }

  /**
   * PATCH /drives/{id} - Update drive rate limiters (runtime only)
   */
  async patchDriveRateLimiters(
    driveId: string, 
    rateLimiters: {
      bandwidth?: RateLimiterTokenBucket
      ops?: RateLimiterTokenBucket
    }
  ): Promise<void> {
    return apiClient.patch(`${FirecrackerApi.FC_PREFIX}/drives/${driveId}`, {
      rate_limiter: rateLimiters
    })
  }

  // ========================================
  // Network Management
  // ========================================

  /**
   * PUT /network-interfaces/{id} - Create/configure network interface (pre-boot only)
   */
  async putNetworkInterface(ifaceId: string, config: NetworkConfig): Promise<void> {
    return apiClient.put(`${FirecrackerApi.FC_PREFIX}/network-interfaces/${ifaceId}`, config)
  }

  /**
   * PATCH /network-interfaces/{id} - Update network interface rate limiters (runtime only)
   */
  async patchNetworkInterfaceRateLimiters(
    ifaceId: string, 
    rateLimiters: {
      rx_rate_limiter?: RateLimiterTokenBucket
      tx_rate_limiter?: RateLimiterTokenBucket
    }
  ): Promise<void> {
    return apiClient.patch(`${FirecrackerApi.FC_PREFIX}/network-interfaces/${ifaceId}`, rateLimiters)
  }

  // ========================================
  // Memory Management
  // ========================================

  /**
   * PUT /balloon - Configure balloon device
   */
  async putBalloon(config: {
    amount_mib: number
    deflate_on_oom?: boolean
    stats_polling_interval_s?: number
  }): Promise<void> {
    return apiClient.put(`${FirecrackerApi.FC_PREFIX}/balloon`, config)
  }

  /**
   * PATCH /balloon - Update balloon configuration (runtime)
   */
  async patchBalloon(config: {
    amount_mib?: number
    stats_polling_interval_s?: number
  }): Promise<void> {
    return apiClient.patch(`${FirecrackerApi.FC_PREFIX}/balloon`, config)
  }

  /**
   * GET /balloon/statistics - Get balloon statistics
   */
  async getBalloonStatistics(): Promise<{
    target_pages: number
    actual_pages: number
    target_mib: number
    actual_mib: number
    swap_in?: number
    swap_out?: number
    major_faults?: number
    minor_faults?: number
    free_memory?: number
    total_memory?: number
    available_memory?: number
    disk_caches?: number
    hugetlb_allocations?: number
    hugetlb_failures?: number
  }> {
    return apiClient.get(`${FirecrackerApi.FC_PREFIX}/balloon/statistics`)
  }

  // ========================================
  // Snapshots
  // ========================================

  /**
   * PUT /snapshot/create - Create VM snapshot
   */
  async putSnapshotCreate(params: {
    snapshot_path: string
    mem_file_path: string
    snapshot_type?: "Full" | "Diff"
    version?: string
  }): Promise<void> {
    return apiClient.put(`${FirecrackerApi.FC_PREFIX}/snapshot/create`, params)
  }

  /**
   * PUT /snapshot/load - Load VM from snapshot
   */
  async putSnapshotLoad(params: {
    snapshot_path: string
    mem_file_path: string
    enable_diff_snapshots?: boolean
    resume_vm?: boolean
  }): Promise<void> {
    return apiClient.put(`${FirecrackerApi.FC_PREFIX}/snapshot/load`, params)
  }

  // ========================================
  // Logging & Metrics
  // ========================================

  /**
   * PUT /logger - Configure logging
   */
  async putLogger(config: {
    log_path: string
    level: "Error" | "Warning" | "Info" | "Debug" | "Trace" | "Off"
    show_level?: boolean
    show_log_origin?: boolean
  }): Promise<void> {
    return apiClient.put(`${FirecrackerApi.FC_PREFIX}/logger`, config)
  }

  /**
   * PUT /metrics - Configure metrics
   */
  async putMetrics(config: {
    metrics_path: string
  }): Promise<void> {
    return apiClient.put(`${FirecrackerApi.FC_PREFIX}/metrics`, config)
  }

  // ========================================
  // MMDS (Metadata Service)
  // ========================================

  /**
   * PUT /mmds - Configure MMDS data
   */
  async putMMDS(metadata: Record<string, any>): Promise<void> {
    return apiClient.put(`${FirecrackerApi.FC_PREFIX}/mmds`, metadata)
  }

  /**
   * PUT /mmds/config - Configure MMDS settings
   */
  async putMMDSConfig(config: {
    version: "V1" | "V2"
    ipv4_address?: string
    network_interfaces: string[]
  }): Promise<void> {
    return apiClient.put(`${FirecrackerApi.FC_PREFIX}/mmds/config`, config)
  }

  // ========================================
  // VM Actions
  // ========================================

  /**
   * PUT /actions - Execute VM actions
   */
  async putActions(action: {
    action_type: "InstanceStart" | "InstanceStop" | "InstanceReboot" | "FlushMetrics" | "SendCtrlAltDel"
  }): Promise<void> {
    return apiClient.put(`${FirecrackerApi.FC_PREFIX}/actions`, action)
  }

  // Convenience methods for common actions
  async startInstance(): Promise<void> {
    return this.putActions({ action_type: "InstanceStart" })
  }

  async stopInstance(): Promise<void> {
    return this.putActions({ action_type: "InstanceStop" })
  }

  async rebootInstance(): Promise<void> {
    return this.putActions({ action_type: "InstanceReboot" })
  }

  async flushMetrics(): Promise<void> {
    return this.putActions({ action_type: "FlushMetrics" })
  }

  async sendCtrlAltDel(): Promise<void> {
    return this.putActions({ action_type: "SendCtrlAltDel" })
  }
}

// Export singleton instance
export const firecrackerApi = new FirecrackerApi()