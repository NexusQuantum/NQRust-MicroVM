import { apiGet, apiPost, apiDelete } from "./client"
import type { VM, VmDrive, VmNic, Snapshot } from "@/lib/types"

export async function getVMs(): Promise<VM[]> {
  return apiGet<VM[]>("/v1/vms")
}

export async function getVM(id: string): Promise<VM> {
  return apiGet<VM>(`/v1/vms/${id}`)
}

export async function createVM(data: any): Promise<VM> {
  return apiPost<VM>("/v1/vms", data)
}

export async function deleteVM(id: string): Promise<void> {
  return apiDelete(`/v1/vms/${id}`)
}

export async function startVM(id: string): Promise<void> {
  return apiPost(`/v1/vms/${id}/start`)
}

export async function stopVM(id: string): Promise<void> {
  return apiPost(`/v1/vms/${id}/stop`)
}

export async function pauseVM(id: string): Promise<void> {
  return apiPost(`/v1/vms/${id}/pause`)
}

export async function resumeVM(id: string): Promise<void> {
  return apiPost(`/v1/vms/${id}/resume`)
}

export async function sendCtrlAltDel(id: string): Promise<void> {
  return apiPost(`/v1/vms/${id}/ctrl-alt-del`)
}

export async function getVMDrives(id: string): Promise<VmDrive[]> {
  return apiGet<VmDrive[]>(`/v1/vms/${id}/drives`)
}

export async function getVMNics(id: string): Promise<VmNic[]> {
  return apiGet<VmNic[]>(`/v1/vms/${id}/nics`)
}

export async function getVMSnapshots(id: string): Promise<Snapshot[]> {
  return apiGet<Snapshot[]>(`/v1/vms/${id}/snapshots`)
}

export async function createSnapshot(id: string, data: any): Promise<Snapshot> {
  return apiPost<Snapshot>(`/v1/vms/${id}/snapshots`, data)
}

export async function getShellCredentials(id: string): Promise<{ username: string; password: string }> {
  return apiGet(`/v1/vms/${id}/shell`)
}
