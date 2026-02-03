import { apiClient, ApiClient } from "./http";

// Determine API base URL at runtime
// Priority: env var > same-host with manager port > localhost fallback
function getApiBaseUrl(): string {
  if (process.env.NEXT_PUBLIC_API_BASE_URL) {
    return process.env.NEXT_PUBLIC_API_BASE_URL;
  }
  if (typeof window !== "undefined") {
    const hostname = window.location.hostname;
    const protocol = window.location.protocol;
    return `${protocol}//${hostname}:18080/v1`;
  }
  return "http://localhost:18080/v1";
}

const API_BASE_URL = getApiBaseUrl();
import type {
  ImageResponse,
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
  GetImageResp,
  ListTemplatesResp,
  Template,
  CreateTemplateReq,
  CreateTemplateResp,
  UpdateTemplateReq,
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
  CreateFunction,
  UpdateFunction,
  InvokeFunction,
  ListInvocationsResp,
  Container,
  CreateContainerReq,
  CreateContainerResp,
  ListContainersResp,
  GetContainerResp,
  ContainerStatsResp,
  ContainerLogsResp,
  ContainerExecReq,
  UpdateContainerReq,
  DockerHubSearchResp,
  DockerHubImage,
  DockerImageTag,
  DockerImageTagsResp,
  DownloadDockerImageReq,
  DownloadDockerImageResp,
  TestFunction,
  Host,
  ListHostsResponse,
  GetHostResponse,
  Network,
  CreateNetworkRequest,
  UpdateNetworkRequest,
  CreateNetworkResponse,
  ListNetworksResponse,
  GetNetworkResponse,
  NetworkVmsResponse,
  Volume,
  CreateVolumeRequest,
  AttachVolumeRequest,
  DetachVolumeRequest,
  CreateVolumeResponse,
  ListVolumesResponse,
  GetVolumeResponse,
  ListAuditLogsResponse,
  AuditLogQueryParams,
  DbConnectionInfo,
  SystemStats,
  HostMetric,
  VmMetric,
  ContainerMetric,
  MetricsQueryParams,
} from "@/lib/types"

/**
 * Composite façade endpoints for VM orchestration
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
   * Docker Hub API Methods
   */
  async searchDockerHub(query: string, limit?: number): Promise<DockerHubImage[]> {
    const res = await apiClient.post<DockerHubSearchResp>("/images/dockerhub/search", {
      query,
      limit,
    });
    return res.items;
  }

  async getDockerImageTags(imageName: string): Promise<DockerImageTag[]> {
    const res = await apiClient.post<DockerImageTagsResp>("/images/dockerhub/tags", imageName);
    return res.items;
  }

  async downloadDockerImage(params: DownloadDockerImageReq): Promise<DownloadDockerImageResp> {
    // Create a new client with extended timeout for downloads (10 minutes)
    // Docker image downloads can take a long time depending on image size
    const downloadClient = new ApiClient(API_BASE_URL, 10 * 60 * 1000);
    return downloadClient.post<DownloadDockerImageResp>("/images/dockerhub/download", params);
  }

  async getDockerDownloadProgress(imageName: string): Promise<import("@/lib/types").DownloadProgress> {
    const encodedImageName = encodeURIComponent(imageName);
    return apiClient.get(`/images/dockerhub/download/progress/${encodedImageName}`);
  }

  async uploadImage(file: File, kind: "docker" | "kernel" | "rootfs", name?: string, project?: string): Promise<CreateImageResp> {
    const formData = new FormData();
    formData.append("file", file);
    formData.append("kind", kind);
    if (name) formData.append("name", name);
    if (project) formData.append("project", project);

    const response = await fetch(`${apiClient.baseURL}/images/upload`, {
      method: "POST",
      body: formData,
    });

    if (!response.ok) {
      const error = await response.text();
      throw new Error(error);
    }

    return response.json();
  }

  /**
   * Image Management
   */
  async getImage(id: string): Promise<Image> {
    const res = await apiClient.get<GetImageResp>(`/images/${id}`);
    return res.item;
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

  async updateTemplate(id: string, params: UpdateTemplateReq): Promise<Template> {
    const res = await apiClient.put<any>(`/templates/${id}`, params);
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

  async deleteTemplate(id: string): Promise<OkResponse> {
    return apiClient.delete<OkResponse>(`/templates/${id}`);
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
    // Dynamically determine WebSocket URL based on current browser location
    // This ensures WebSocket connections work when accessing the UI remotely
    if (typeof window !== "undefined") {
      const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
      const hostname = window.location.hostname;
      return `${protocol}//${hostname}:18080/v1/vms/${vmId}/shell/ws`;
    }
    // Fallback for SSR (shouldn't be used for actual WebSocket connections)
    const apiBaseUrl =
      process.env.NEXT_PUBLIC_API_BASE_URL || "http://localhost:18080/v1";
    const wsBaseUrl =
      process.env.NEXT_PUBLIC_WS_BASE_URL ||
      apiBaseUrl.replace(/^http/, "ws").replace(/\/v1$/, "");
    return `${wsBaseUrl}/v1/vms/${vmId}/shell/ws`;
  }

  async getShellCredentials(vmId: string): Promise<{ username: string; password: string }> {
    return apiClient.get<{ username: string; password: string }>(`/vms/${vmId}/shell`)
  }


  // Functions
  async getFunctions(): Promise<Fn[]>{
    const res = await apiClient.get(`/functions`)
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

  async deleteFunction(id: string): Promise<void> {
    await apiClient.delete<OkResponse>(`/functions/${id}`)
  }

  async createFunction(params: CreateFunction) {
    return apiClient.post<CreateFunction>("/functions", params)
  }
  
  async updateFunction(id: string, data: UpdateFunction) {
    return apiClient.put(`/functions/${id}`,data);
  }

  async invokeFunction(id: string, data: InvokeFunction) {
    return apiClient.post(`/functions/${id}/invoke`, data )
  }

  async testFunction(params: TestFunction) {
    return apiClient.post(`/functions/test`, params)
  }

  async getFunctionLogs(id: string, filters?: { status?: string; limit?: number }): Promise<ListInvocationsResp> {
    let url = `/functions/${id}/logs`
    if (filters) {
      const params = new URLSearchParams()
      if (filters.status) params.append("status", filters.status)
      if (filters.limit) params.append("limit", filters.limit.toString())
      if (params.toString()) url += `?${params.toString()}`
    }
    return apiClient.get(url)
  }

  /**
   * Container Management
   */
  async getContainers(filters?: { state?: string; host_id?: string }): Promise<Container[]> {
    let url = "/containers";
    if (filters) {
      const params = new URLSearchParams();
      if (filters.state) params.append("state", filters.state);
      if (filters.host_id) params.append("host_id", filters.host_id);
      if (params.toString()) url += `?${params.toString()}`;
    }
    const res = await apiClient.get<ListContainersResp>(url);
    return res.items;
  }

  async getContainer(id: string): Promise<Container> {
    const res = await apiClient.get<GetContainerResp>(`/containers/${id}`);
    return res.item;
  }

  async createContainer(params: CreateContainerReq): Promise<CreateContainerResp> {
    return apiClient.post<CreateContainerResp>("/containers", params);
  }

  async updateContainer(id: string, params: UpdateContainerReq): Promise<Container> {
    const res = await apiClient.put<GetContainerResp>(`/containers/${id}`, params);
    return res.item;
  }

  async deleteContainer(id: string): Promise<void> {
    await apiClient.delete<OkResponse>(`/containers/${id}`);
  }

  async startContainer(id: string): Promise<void> {
    await apiClient.post<OkResponse>(`/containers/${id}/start`, {});
  }

  async stopContainer(id: string): Promise<void> {
    await apiClient.post<OkResponse>(`/containers/${id}/stop`, {});
  }

  async restartContainer(id: string): Promise<void> {
    await apiClient.post<OkResponse>(`/containers/${id}/restart`, {});
  }

  async pauseContainer(id: string): Promise<void> {
    await apiClient.post<OkResponse>(`/containers/${id}/pause`, {});
  }

  async resumeContainer(id: string): Promise<void> {
    await apiClient.post<OkResponse>(`/containers/${id}/resume`, {});
  }

  async getContainerLogs(id: string, tail?: number): Promise<ContainerLogsResp> {
    const url = tail ? `/containers/${id}/logs?tail=${tail}` : `/containers/${id}/logs`;
    return apiClient.get<ContainerLogsResp>(url);
  }

  async getContainerStats(id: string): Promise<ContainerStatsResp> {
    return apiClient.get<ContainerStatsResp>(`/containers/${id}/stats`);
  }

  async execContainerCommand(id: string, params: ContainerExecReq): Promise<any> {
    return apiClient.post(`/containers/${id}/exec`, params);
  }

  // ==============
  // Host Management
  // ==============

  async getHosts(): Promise<Host[]> {
    const res = await apiClient.get<ListHostsResponse>("/hosts");
    return res.items;
  }

  async getHost(id: string): Promise<Host> {
    const res = await apiClient.get<GetHostResponse>(`/hosts/${id}`);
    return res.item;
  }

  async deleteHost(id: string): Promise<void> {
    await apiClient.delete<OkResponse>(`/hosts/${id}`);
  }

  // ==============
  // Network Management
  // ==============

  async getNetworks(): Promise<Network[]> {
    const res = await apiClient.get<ListNetworksResponse>("/networks");
    return res.items;
  }

  async createNetwork(params: any): Promise<Network> {
    const res = await apiClient.post<CreateNetworkResponse>("/networks", params);
    // Backend returns { id: "..." } not { item: ... }
    return res as any;
  }

  async getNetwork(id: string): Promise<Network> {
    const res = await apiClient.get<GetNetworkResponse>(`/networks/${id}`);
    return res.item;
  }

  async updateNetwork(id: string, params: UpdateNetworkRequest): Promise<Network> {
    const res = await apiClient.patch<GetNetworkResponse>(`/networks/${id}`, params);
    return res.item;
  }

  async deleteNetwork(id: string): Promise<void> {
    await apiClient.delete<OkResponse>(`/networks/${id}`);
  }

  async getNetworkVms(id: string): Promise<NetworkVmsResponse> {
    return apiClient.get<NetworkVmsResponse>(`/networks/${id}/vms`);
  }

  // ==============
  // Volume Management
  // ==============

  async getVolumes(): Promise<Volume[]> {
    const res = await apiClient.get<ListVolumesResponse>("/volumes");
    return res.items;
  }

  async getVolume(id: string): Promise<Volume> {
    const res = await apiClient.get<GetVolumeResponse>(`/volumes/${id}`);
    return res.item;
  }

  async createVolume(params: CreateVolumeRequest): Promise<CreateVolumeResponse> {
    return apiClient.post<CreateVolumeResponse>("/volumes", params);
  }

  async attachVolume(id: string, params: AttachVolumeRequest): Promise<void> {
    await apiClient.post<OkResponse>(`/volumes/${id}/attach`, params);
  }

  async detachVolume(id: string, params: DetachVolumeRequest): Promise<void> {
    await apiClient.post<OkResponse>(`/volumes/${id}/detach`, params);
  }

  async deleteVolume(id: string): Promise<void> {
    await apiClient.delete<OkResponse>(`/volumes/${id}`);
  }

  // ==============
  // User Management
  // ==============

  async getUsers(): Promise<import("@/lib/types").User[]> {
    const res = await apiClient.get<import("@/lib/types").ListUsersResponse>("/users");
    return res.items;
  }

  async getUser(id: string): Promise<import("@/lib/types").User> {
    const res = await apiClient.get<import("@/lib/types").GetUserResponse>(`/users/${id}`);
    return res.item;
  }

  async createUser(params: import("@/lib/types").CreateUserRequest): Promise<import("@/lib/types").CreateUserResponse> {
    return apiClient.post<import("@/lib/types").CreateUserResponse>("/users", params);
  }

  async updateUser(id: string, params: import("@/lib/types").UpdateUserRequest): Promise<import("@/lib/types").User> {
    const res = await apiClient.patch<import("@/lib/types").GetUserResponse>(`/users/${id}`, params);
    return res.item;
  }

  async deleteUser(id: string): Promise<void> {
    await apiClient.delete<OkResponse>(`/users/${id}`);
  }

  // User Preferences
  async getPreferences(): Promise<import("@/lib/types").UserPreferences> {
    const res = await apiClient.get<import("@/lib/types").GetPreferencesResponse>("/auth/me/preferences");
    return res.preferences;
  }

  async updatePreferences(params: import("@/lib/types").UpdatePreferencesRequest): Promise<import("@/lib/types").UserPreferences> {
    const res = await apiClient.patch<import("@/lib/types").GetPreferencesResponse>("/auth/me/preferences", params);
    return res.preferences;
  }

  // Profile Management
  async getProfile(): Promise<import("@/lib/types").User> {
    return apiClient.get<import("@/lib/types").User>("/auth/me/profile");
  }

  async updateProfile(params: import("@/lib/types").UpdateProfileRequest): Promise<import("@/lib/types").User> {
    return apiClient.patch<import("@/lib/types").User>("/auth/me/profile", params);
  }

  async changePassword(params: import("@/lib/types").ChangePasswordRequest): Promise<void> {
    await apiClient.post<OkResponse>("/auth/me/password", params);
  }

  // Avatar Management
  async uploadAvatar(file: File): Promise<void> {
    const formData = new FormData();
    formData.append("avatar", file);

    // Get the auth token
    const { getAuthToken } = await import("@/lib/auth/store");
    const token = getAuthToken();

    const headers: HeadersInit = {};
    if (token) {
      headers["Authorization"] = `Bearer ${token}`;
    }

    // Use fetch directly for multipart/form-data to avoid setting Content-Type manually
    const response = await fetch(`${apiClient.baseURL}/auth/me/avatar`, {
      method: "POST",
      body: formData,
      headers,
    });

    if (!response.ok) {
      const error = await response.text();
      throw new Error(error);
    }
  }

  getAvatarUrl(userId: string): string {
    return `${apiClient.baseURL}/users/${userId}/avatar`;
  }

  getMyAvatarUrl(): string {
    return `${apiClient.baseURL}/auth/me/avatar`;
  }

  async deleteAvatar(): Promise<void> {
    await apiClient.delete<OkResponse>("/auth/me/avatar");
  }

  // ==============
  // Audit Logs
  // ==============

  async getAuditLogs(params?: AuditLogQueryParams): Promise<ListAuditLogsResponse> {
    let url = "/logs/audit";
    if (params) {
      const qp = new URLSearchParams();
      if (params.action) qp.append("action", params.action);
      if (params.resource_type) qp.append("resource_type", params.resource_type);
      if (params.limit != null) qp.append("limit", String(params.limit));
      if (params.offset != null) qp.append("offset", String(params.offset));
      const qs = qp.toString();
      if (qs) url += `?${qs}`;
    }
    return apiClient.get<ListAuditLogsResponse>(url);
  }

  async getDbInfo(): Promise<DbConnectionInfo> {
    return apiClient.get<DbConnectionInfo>("/logs/db-info");
  }

  async getSystemStats(): Promise<SystemStats> {
    return apiClient.get<SystemStats>("/logs/stats");
  }

  // ── Time-Series Metrics ─────────────────────────────────────────

  async getHostMetrics(hostId: string, params?: MetricsQueryParams): Promise<HostMetric[]> {
    let url = `/metrics/hosts/${hostId}`;
    const qp = new URLSearchParams();
    if (params?.from) qp.append("from", params.from);
    if (params?.to) qp.append("to", params.to);
    if (params?.limit != null) qp.append("limit", String(params.limit));
    const qs = qp.toString();
    if (qs) url += `?${qs}`;
    return apiClient.get<HostMetric[]>(url);
  }

  async getVmMetrics(vmId: string, params?: MetricsQueryParams): Promise<VmMetric[]> {
    let url = `/metrics/vms/${vmId}`;
    const qp = new URLSearchParams();
    if (params?.from) qp.append("from", params.from);
    if (params?.to) qp.append("to", params.to);
    if (params?.limit != null) qp.append("limit", String(params.limit));
    const qs = qp.toString();
    if (qs) url += `?${qs}`;
    return apiClient.get<VmMetric[]>(url);
  }

  async getContainerMetrics(containerId: string, params?: MetricsQueryParams): Promise<ContainerMetric[]> {
    let url = `/metrics/containers/${containerId}`;
    const qp = new URLSearchParams();
    if (params?.from) qp.append("from", params.from);
    if (params?.to) qp.append("to", params.to);
    if (params?.limit != null) qp.append("limit", String(params.limit));
    const qs = qp.toString();
    if (qs) url += `?${qs}`;
    return apiClient.get<ContainerMetric[]>(url);
  }
}

// Export singleton instance
export const facadeApi = new FacadeApi();
