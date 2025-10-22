import { apiGet, apiPost, apiDelete } from "./client"
import type { Container } from "@/lib/types"

export async function getContainers(): Promise<Container[]> {
  return apiGet<Container[]>("/v1/containers")
}

export async function getContainer(id: string): Promise<Container> {
  return apiGet<Container>(`/v1/containers/${id}`)
}

export async function createContainer(data: any): Promise<Container> {
  return apiPost<Container>("/v1/containers", data)
}

export async function deleteContainer(id: string): Promise<void> {
  return apiDelete(`/v1/containers/${id}`)
}

export async function startContainer(id: string): Promise<void> {
  return apiPost(`/v1/containers/${id}/start`)
}

export async function stopContainer(id: string): Promise<void> {
  return apiPost(`/v1/containers/${id}/stop`)
}

export async function restartContainer(id: string): Promise<void> {
  return apiPost(`/v1/containers/${id}/restart`)
}

export async function getContainerLogs(id: string): Promise<string[]> {
  return apiGet<string[]>(`/v1/containers/${id}/logs`)
}
