import { apiClient } from "./http";
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
  VmDrive,
  CreateDriveReq,
  UpdateDriveReq,
  ListDrivesResponse,
  VmNic,
  CreateNicReq,
  UpdateNicReq,
  ListNicsResponse,
  Function as Fn,
} from "@/lib/types";

/**
 * Composite façade endpoints that orchestrate multiple Firecracker API calls
 */
export class FacadeApi {
  /**
   * Create VM - Using new backend API structure
   * POST /v1/vms
   */
  async createVM(params: CreateVmReq): Promise<CreateVmResponse> {
    return apiClient.post<CreateVmResponse>("/vms", params);
  }

  /**
   * Initialize VM - Set up logger and metrics
   * POST /api/vms/:id/initialize → logger + metrics init
   */
  async initializeVM(id: string): Promise<void> {
    // TODO(backend): Implement VM initialization endpoint
    return apiClient.post<void>(`/vms/${id}/initialize`);
  }

  /**
   * Create Snapshot - Using new backend API
   * POST /v1/vms/:id/snapshots
   */
  async createSnapshot(
    vmId: string,
    params: CreateSnapshotRequest = {}
  ): Promise<CreateSnapshotResponse> {
    return apiClient.post<CreateSnapshotResponse>(
      `/vms/${vmId}/snapshots`,
      params
    );
  }

  /**
   * Get all VMs
   */
  async getVMs(): Promise<Vm[]> {
    const res = await apiClient.get<ListVmsResponse>("/vms");
    return res.items;
  }

  /**
   * Get single VM
   */
  async getVM(id: string): Promise<Vm> {
    const res = await apiClient.get<GetVmResponse>(`/vms/${id}`);
    return res.item;
  }

  /**
   * Delete VM
   */
  async deleteVM(id: string): Promise<void> {
    await apiClient.delete<OkResponse>(`/vms/${id}`);
  }

  /**
   * Stop VM
   */
  async stopVM(id: string): Promise<void> {
    await apiClient.post<OkResponse>(`/vms/${id}/stop`, {});
  }

  /**
   * Update VM configuration (not yet implemented in new backend)
   */
  async updateVM(id: string, config: any): Promise<any> {
    throw new Error("VM updates not yet supported in current backend");
  }

  /**
   * Get VM metrics (not yet implemented in new backend)
   */
  async getVMMetrics(id: string): Promise<any> {
    throw new Error("VM metrics not yet supported in current backend");
  }

  /**
   * Get VM snapshots
   */
  async getVMSnapshots(vmId: string): Promise<Snapshot[]> {
    const res = await apiClient.get<ListSnapshotsResponse>(
      `/vms/${vmId}/snapshots`
    );
    return res.items;
  }

  /**
   * Restore VM from snapshot
   */
  async restoreSnapshot(
    snapshotId: string,
    params: InstantiateSnapshotReq = {}
  ): Promise<InstantiateSnapshotResp> {
    return apiClient.post<InstantiateSnapshotResp>(
      `/snapshots/${snapshotId}/instantiate`,
      params
    );
  }

  async deleteVMSnapshot(vmId: string, snapshotId: string): Promise<void> {
    await apiClient.delete<OkResponse>(`/snapshots/${snapshotId}`);
  }

  /**
   * Get registry images for VM creation
   */
  async getRegistryImages(): Promise<Image[]> {
    const res = await apiClient.get<ListImagesResp>("/images");
    return res.items;
  }

  /**
   * Get registry volumes for VM creation (using images endpoint)
   */
  async getRegistryVolumes(): Promise<Image[]> {
    const res = await apiClient.get<ListImagesResp>("/images?kind=rootfs");
    return res.items;
  }

  /**
   * Get images by kind (kernel, rootfs, etc.)
   */
  async getImagesByKind(kind: string): Promise<Image[]> {
    const res = await apiClient.get<ListImagesResp>(`/images?kind=${kind}`);
    return res.items;
  }

  async importRegistryImage(params: {
    type: "kernel" | "rootfs" | "data";
    name?: string;
    path?: string;
    url?: string;
  }): Promise<CreateImageResp> {
    // Convert to new backend format
    const imageReq: CreateImageReq = {
      kind: params.type,
      name: params.name || "imported-image",
      host_path: params.path || params.url || "",
      sha256: "pending", // Backend should calculate this
      size: 0, // Backend should determine this
      project: "imported",
    };
    return apiClient.post<CreateImageResp>("/images", imageReq);
  }

  async createRegistryVolume(params: {
    name: string;
    size_bytes: number;
    type?: "rootfs" | "data";
  }): Promise<CreateImageResp> {
    // Convert to image creation format
    const imageReq: CreateImageReq = {
      kind: params.type || "rootfs",
      name: params.name,
      host_path: `/srv/images/${params.name}`,
      sha256: "pending",
      size: params.size_bytes,
      project: "created",
    };
    return apiClient.post<CreateImageResp>("/images", imageReq);
  }

  async deleteRegistryItem(id: string): Promise<void> {
    await apiClient.delete<OkResponse>(`/images/${id}`);
  }

  async renameRegistryItem(path: string, new_name: string) {
    // Not directly supported in new backend
    throw new Error("Rename not supported in current backend");
  }

  /**
   * Template Management - New backend feature
   */
  async getTemplates(): Promise<Template[]> {
    const res = await apiClient.get<ListTemplatesResp>("/templates");
    return res.items;
  }

  async createTemplate(params: CreateTemplateReq): Promise<CreateTemplateResp> {
    return apiClient.post<CreateTemplateResp>("/templates", params);
  }

  async getTemplate(id: string): Promise<Template> {
    const res = await apiClient.get<any>(`/templates/${id}`);
    return res.item;
  }

  async instantiateTemplate(
    id: string,
    params: InstantiateTemplateReq
  ): Promise<InstantiateTemplateResp> {
    return apiClient.post<InstantiateTemplateResp>(
      `/templates/${id}/instantiate`,
      params
    );
  }

  async uploadRegistryFile(params: {
    type: "kernel" | "rootfs" | "data";
    file: File;
    name?: string;
  }) {
    // File upload not yet supported in new backend
    throw new Error("File upload not yet supported in current backend");
  }

  /**
   * VM state transitions - using new backend endpoints
   */
  async updateVMState(
    id: string,
    action:
      | "start"
      | "stop"
      | "pause"
      | "resume"
      | "flush_metrics"
      | "ctrl_alt_del"
  ): Promise<void> {
    switch (action) {
      case "start":
        await apiClient.post<OkResponse>(`/vms/${id}/start`, {});
        break;
      case "stop":
        await apiClient.post<OkResponse>(`/vms/${id}/stop`, {});
        break;
      case "pause":
        await apiClient.post<OkResponse>(`/vms/${id}/pause`, {});
        break;
      case "resume":
        await apiClient.post<OkResponse>(`/vms/${id}/resume`, {});
        break;
      case "flush_metrics":
        await apiClient.post<OkResponse>(`/vms/${id}/flush-metrics`, {});
        break;
      case "ctrl_alt_del":
        await apiClient.post<OkResponse>(`/vms/${id}/ctrl-alt-del`, {});
        break;
      default:
        throw new Error(`Unknown action: ${action}`);
    }
  }

  /**
   * Drive Management - Database-backed persistent drives
   */
  async getVMDrives(vmId: string): Promise<VmDrive[]> {
    const res = await apiClient.get<ListDrivesResponse>(`/vms/${vmId}/drives`);
    return res.items;
  }

  async getVMDrive(vmId: string, driveId: string): Promise<VmDrive> {
    return apiClient.get<VmDrive>(`/vms/${vmId}/drives/${driveId}`);
  }

  async createVMDrive(vmId: string, drive: CreateDriveReq): Promise<VmDrive> {
    return apiClient.post<VmDrive>(`/vms/${vmId}/drives`, drive);
  }

  async updateVMDrive(
    vmId: string,
    driveId: string,
    drive: UpdateDriveReq
  ): Promise<VmDrive> {
    return apiClient.patch<VmDrive>(`/vms/${vmId}/drives/${driveId}`, drive);
  }

  async deleteVMDrive(vmId: string, driveId: string): Promise<void> {
    await apiClient.delete<OkResponse>(`/vms/${vmId}/drives/${driveId}`);
  }

  /**
   * Network Interface Management - Database-backed persistent NICs
   */
  async getVMNics(vmId: string): Promise<VmNic[]> {
    const res = await apiClient.get<ListNicsResponse>(`/vms/${vmId}/nics`);
    return res.items;
  }

  async getVMNic(vmId: string, nicId: string): Promise<VmNic> {
    return apiClient.get<VmNic>(`/vms/${vmId}/nics/${nicId}`);
  }

  async createVMNic(vmId: string, nic: CreateNicReq): Promise<VmNic> {
    return apiClient.post<VmNic>(`/vms/${vmId}/nics`, nic);
  }

  async updateVMNic(
    vmId: string,
    nicId: string,
    nic: UpdateNicReq
  ): Promise<VmNic> {
    return apiClient.patch<VmNic>(`/vms/${vmId}/nics/${nicId}`, nic);
  }

  async deleteVMNic(vmId: string, nicId: string): Promise<void> {
    await apiClient.delete<OkResponse>(`/vms/${vmId}/nics/${nicId}`);
  }

  /**
   * Shell Access - WebSocket terminal connection
   */
  getShellWebSocketUrl(vmId: string): string {
    // Get WebSocket base URL from environment or derive from API base URL
    const apiBaseUrl =
      process.env.NEXT_PUBLIC_API_BASE_URL || "http://localhost:18080/v1";
    const wsBaseUrl =
      process.env.NEXT_PUBLIC_WS_BASE_URL ||
      apiBaseUrl.replace(/^http/, "ws").replace(/\/v1$/, "");
    return `${wsBaseUrl}/v1/vms/${vmId}/shell/ws`;
  }

  async getShellCredentials(
    vmId: string
  ): Promise<{ username: string; password: string }> {
    return apiClient.get<{ username: string; password: string }>(
      `/vms/${vmId}/shell`
    );
  }

  async getFunctions(): Promise<Fn[]> {
    const res = await apiClient.get("/functions");
    const json =
      res && typeof res === "object" && "data" in res ? (res as any).data : res;

    const list = Array.isArray(json)
      ? json
      : Array.isArray((json as any)?.items)
      ? (json as any).items
      : Array.isArray((json as any)?.data)
      ? (json as any).data
      : Array.isArray((json as any)?.functions)
      ? (json as any).functions
      : [];

    return list as Fn[];
  }

  async getFunction(id: string): Promise<Fn> {
    const res = await apiClient.get(`/functions/${id}`);
    const json =
      res && typeof res === "object" && "data" in res ? (res as any).data : res;

    return json.item as Fn;
  }
}

// Export singleton instance
export const facadeApi = new FacadeApi();
