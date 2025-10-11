// Types matching the new Rust backend (nexus-types)

export interface OkResponse {
  ok: boolean
}

export interface CreateVmResponse {
  id: string
}

export interface Vm {
  id: string
  name: string
  state: string
  host_id: string
  template_id?: string
  host_addr: string
  api_sock: string
  tap: string
  log_path: string
  http_port: number
  fc_unit: string
  vcpu: number
  mem_mib: number
  kernel_path: string
  rootfs_path: string
  source_snapshot_id?: string
  created_at: string
  updated_at: string
}

export interface ListVmsResponse {
  items: Vm[]
}

export interface GetVmResponse {
  item: Vm
}

export interface CreateVmReq {
  name: string
  vcpu: number
  mem_mib: number
  kernel_image_id?: string
  rootfs_image_id?: string
  kernel_path?: string
  rootfs_path?: string
  source_snapshot_id?: string
}

export interface TemplateSpec {
  vcpu: number
  mem_mib: number
  kernel_image_id?: string
  rootfs_image_id?: string
  kernel_path?: string
  rootfs_path?: string
}

export interface CreateTemplateReq {
  name: string
  spec: TemplateSpec
}

export interface Template {
  id: string
  name: string
  spec: TemplateSpec
  created_at: string
  updated_at: string
}

export interface CreateTemplateResp {
  id: string
}

export interface ListTemplatesResp {
  items: Template[]
}

export interface GetTemplateResp {
  item: Template
}

export interface InstantiateTemplateReq {
  name: string
}

export interface InstantiateTemplateResp {
  id: string
}

export interface VmSummary {
  id: string
  name: string
  state: string
}

export interface Snapshot {
  id: string
  vm_id: string
  snapshot_path: string
  mem_path: string
  size_bytes: number
  state: string
  created_at: string
  updated_at: string
}

export interface CreateSnapshotRequest {
  // Empty for now, backend doesn't require any fields
}

export interface CreateSnapshotResponse {
  id: string
}

export interface ListSnapshotsResponse {
  items: Snapshot[]
}

export interface GetSnapshotResponse {
  item: Snapshot
}

export interface InstantiateSnapshotReq {
  name?: string
}

export interface InstantiateSnapshotResp {
  id: string
  name: string
}

export interface Image {
  id: string
  kind: string
  name: string
  host_path: string
  sha256: string
  size: number
  project?: string
  created_at: string
  updated_at: string
}

export interface CreateImageReq {
  kind: string
  name: string
  host_path: string
  sha256: string
  size: number
  project?: string
}

export interface CreateImageResp {
  id: string
}

export interface ImageFilter {
  kind?: string
  project?: string
  name?: string
}

export interface ListImagesResp {
  items: Image[]
}

export interface GetImageResp {
  item: Image
}

export interface RegisterHostRequest {
  name: string
  addr: string
  capabilities?: any
}

export interface RegisterHostResponse {
  id: string
}

export interface HostHeartbeatRequest {
  capabilities?: any
}

export interface TailLogResponse {
  text: string
}

// Path parameters for API routes
export interface VmPathParams {
  id: string
}

export interface TemplatePathParams {
  id: string
}

export interface SnapshotPathParams {
  id: string
}

export interface ImagePathParams {
  id: string
}

export interface HostPathParams {
  id: string
}

// Error types
export interface NexusError {
  error: string
  fault_message?: string
  status: number
  suggestion?: string
  request_id: string
}
