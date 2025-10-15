import { apiClient } from "./http"
import type {
  CreateVmReq,
  CreateVmResponse,
  ListVmsResponse,
  GetVmResponse,
  Vm,
  CreateSnapshotRequest,
  CreateSnapshotResponse,
  ListSnapshotsResponse,
  Snapshot,
  InstantiateSnapshotReq,
  InstantiateSnapshotResp,
  ListImagesResp,
  Image,
  CreateImageReq,
  CreateImageResp,
  ListTemplatesResp,
  Template,
  CreateTemplateReq,
  CreateTemplateResp,
  InstantiateTemplateReq,
  InstantiateTemplateResp,
  OkResponse,
} from "@/types/nexus"

/**
 * Composite façade endpoints that orchestrate multiple Firecracker API calls
 */
export class FacadeApi {
  /**
   * Create VM - Using new backend API structure
   * POST /v1/vms
   */
  async createVM(params: CreateVmReq): Promise<CreateVmResponse> {
    return apiClient.post<CreateVmResponse>("/vms", params)
  }

  /**
   * Initialize VM - Set up logger and metrics
   * POST /api/vms/:id/initialize → logger + metrics init
   */
  async initializeVM(id: string): Promise<void> {
    // TODO(backend): Implement VM initialization endpoint
    return apiClient.post<void>(`/vms/${id}/initialize`)
  }

  /**
   * Create Snapshot - Using new backend API
   * POST /v1/vms/:id/snapshots
   */
  async createSnapshot(
    vmId: string,
    params: CreateSnapshotRequest = {}
  ): Promise<CreateSnapshotResponse> {
    return apiClient.post<CreateSnapshotResponse>(`/vms/${vmId}/snapshots`, params)
  }

  /**
   * Get all VMs
   */
  async getVMs(): Promise<Vm[]> {
    const res = await apiClient.get<ListVmsResponse>("/vms")
    return res.items
  }

  /**
   * Get single VM
   */
  async getVM(id: string): Promise<Vm> {
    const res = await apiClient.get<GetVmResponse>(`/vms/${id}`)
    return res.item
  }

  /**
   * Delete VM
   */
  async deleteVM(id: string): Promise<void> {
    await apiClient.delete<OkResponse>(`/vms/${id}`)
  }

  /**
   * Stop VM
   */
  async stopVM(id: string): Promise<void> {
    await apiClient.post<OkResponse>(`/vms/${id}/stop`, {})
  }

  /**
   * Update VM configuration (not yet implemented in new backend)
   */
  async updateVM(id: string, config: any): Promise<any> {
    throw new Error('VM updates not yet supported in current backend')
  }

  /**
   * Get VM metrics (not yet implemented in new backend)
   */
  async getVMMetrics(id: string): Promise<any> {
    throw new Error('VM metrics not yet supported in current backend')
  }

  /**
   * Get VM snapshots
   */
  async getVMSnapshots(vmId: string): Promise<Snapshot[]> {
    const res = await apiClient.get<ListSnapshotsResponse>(`/vms/${vmId}/snapshots`)
    return res.items
  }

  /**
   * Restore VM from snapshot
   */
  async restoreSnapshot(
    snapshotId: string,
    params: InstantiateSnapshotReq = {}
  ): Promise<InstantiateSnapshotResp> {
    return apiClient.post<InstantiateSnapshotResp>(`/snapshots/${snapshotId}/instantiate`, params)
  }

  async deleteVMSnapshot(vmId: string, snapshotId: string): Promise<void> {
    await apiClient.delete<OkResponse>(`/snapshots/${snapshotId}`)
  }

  /**
   * Get registry images for VM creation
   */
  async getRegistryImages(): Promise<Image[]> {
    const res = await apiClient.get<ListImagesResp>("/images")
    return res.items
  }

  /**
   * Get registry volumes for VM creation (using images endpoint)
   */
  async getRegistryVolumes(): Promise<Image[]> {
    const res = await apiClient.get<ListImagesResp>("/images?kind=rootfs")
    return res.items
  }

  /**
   * Get images by kind (kernel, rootfs, etc.)
   */
  async getImagesByKind(kind: string): Promise<Image[]> {
    const res = await apiClient.get<ListImagesResp>(`/images?kind=${kind}`)
    return res.items
  }

  async importRegistryImage(params: { type: 'kernel'|'rootfs'|'data'; name?: string; path?: string; url?: string }): Promise<CreateImageResp> {
    // Convert to new backend format
    const imageReq: CreateImageReq = {
      kind: params.type,
      name: params.name || 'imported-image',
      host_path: params.path || params.url || '',
      sha256: 'pending', // Backend should calculate this
      size: 0, // Backend should determine this
      project: 'imported'
    }
    return apiClient.post<CreateImageResp>('/images', imageReq)
  }

  async createRegistryVolume(params: { name: string; size_bytes: number; type?: 'rootfs'|'data' }): Promise<CreateImageResp> {
    // Convert to image creation format
    const imageReq: CreateImageReq = {
      kind: params.type || 'rootfs',
      name: params.name,
      host_path: `/srv/images/${params.name}`,
      sha256: 'pending',
      size: params.size_bytes,
      project: 'created'
    }
    return apiClient.post<CreateImageResp>('/images', imageReq)
  }

  async deleteRegistryItem(id: string): Promise<void> {
    await apiClient.delete<OkResponse>(`/images/${id}`)
  }

  async renameRegistryItem(path: string, new_name: string) {
    // Not directly supported in new backend
    throw new Error('Rename not supported in current backend')
  }

  /**
   * Template Management - New backend feature
   */
  async getTemplates(): Promise<Template[]> {
    const res = await apiClient.get<ListTemplatesResp>("/templates")
    return res.items
  }

  async createTemplate(params: CreateTemplateReq): Promise<CreateTemplateResp> {
    return apiClient.post<CreateTemplateResp>("/templates", params)
  }

  async getTemplate(id: string): Promise<Template> {
    const res = await apiClient.get<GetTemplateResp>(`/templates/${id}`)
    return res.item
  }

  async instantiateTemplate(id: string, params: InstantiateTemplateReq): Promise<InstantiateTemplateResp> {
    return apiClient.post<InstantiateTemplateResp>(`/templates/${id}/instantiate`, params)
  }

  async uploadRegistryFile(params: { type: 'kernel'|'rootfs'|'data'; file: File; name?: string }) {
    // File upload not yet supported in new backend
    throw new Error('File upload not yet supported in current backend')
  }

  /**
   * VM state transitions - using new backend endpoints
   */
  async updateVMState(id: string, action: 'start'|'stop'|'pause'|'resume'|'flush_metrics'|'ctrl_alt_del'): Promise<void> {
    switch (action) {
      case 'start':
        await apiClient.post<OkResponse>(`/vms/${id}/start`, {})
        break
      case 'stop':
        await apiClient.post<OkResponse>(`/vms/${id}/stop`, {})
        break
      case 'pause':
        await apiClient.post<OkResponse>(`/vms/${id}/pause`, {})
        break
      case 'resume':
        await apiClient.post<OkResponse>(`/vms/${id}/resume`, {})
        break
      case 'flush_metrics':
        await apiClient.post<OkResponse>(`/vms/${id}/flush-metrics`, {})
        break
      case 'ctrl_alt_del':
        await apiClient.post<OkResponse>(`/vms/${id}/ctrl-alt-del`, {})
        break
      default:
        throw new Error(`Unknown action: ${action}`)
    }
  }
}

// Export singleton instance
export const facadeApi = new FacadeApi()
