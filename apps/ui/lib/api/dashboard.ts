import { apiGet } from "./client"
import type { DashboardStats, VM, Function, Container } from "@/lib/types"

export interface UnifiedResource {
  id: string
  name: string
  type: "vm" | "function" | "container"
  state: string
  metrics?: {
    cpu?: number
    memory?: number
    lastInvoked?: string
  }
}

export async function getDashboardStats(): Promise<DashboardStats> {
  return apiGet<DashboardStats>("/v1/dashboard/stats")
}

export async function getAllResources(): Promise<UnifiedResource[]> {
  // Fetch all resource types in parallel
  const [vms, functions, containers] = await Promise.all([
    apiGet<VM[]>("/v1/vms").catch(() => []),
    apiGet<Function[]>("/v1/functions").catch(() => []),
    apiGet<Container[]>("/v1/containers").catch(() => []),
  ])

  // Transform to unified format
  const vmResources: UnifiedResource[] = vms.map((vm) => ({
    id: vm.id,
    name: vm.name,
    type: "vm",
    state: vm.state,
    metrics: {
      cpu: vm.cpu_usage_percent,
      memory: vm.memory_usage_percent,
    },
  }))

  const functionResources: UnifiedResource[] = functions.map((fn) => ({
    id: fn.id,
    name: fn.name,
    type: "function",
    state: fn.last_invoked_at ? "idle" : "idle",
    metrics: {
      lastInvoked: fn.last_invoked_at,
    },
  }))

  const containerResources: UnifiedResource[] = containers.map((container) => ({
    id: container.id,
    name: container.name,
    type: "container",
    state: container.status,
    metrics: {
      cpu: container.cpu_percent,
      memory: container.memory_used_mb,
    },
  }))

  return [...vmResources, ...functionResources, ...containerResources]
}
