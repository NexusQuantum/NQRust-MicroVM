"use client"

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Server, Zap, Container, HardDrive, Plus, TrendingUp, Activity } from "lucide-react"
import { ResourceTable } from "@/components/dashboard/resource-table"
import Link from "next/link"
import { useVMs, useFunctions, useContainers, usePreferences, useHosts } from "@/lib/queries"
import { Skeleton } from "@/components/ui/skeleton"
import { useAuthStore, canCreateResource } from "@/lib/auth/store"

export default function DashboardPage() {
  // Load user preferences for auto-refresh settings
  const { data: preferences } = usePreferences()
  const autoRefreshInterval = preferences?.auto_refresh
    ? preferences.auto_refresh * 1000 // Convert seconds to milliseconds
    : undefined // No auto-refresh if not set

  const { data: vms = [], isLoading: vmsLoading } = useVMs(true, autoRefreshInterval)
  const { data: functions = [], isLoading: functionsLoading } = useFunctions(autoRefreshInterval)
  const { data: containers = [], isLoading: containersLoading } = useContainers(autoRefreshInterval)
  const { data: hosts = [], isLoading: hostsLoading } = useHosts()
  const { user } = useAuthStore()

  // Filter resources by ownership
  const filterByOwnership = (resource: any) => {
    // Admin and viewer can see all resources
    if (user?.role === "admin" || user?.role === "viewer") {
      return true
    }
    // User can see resources without owner (system) or their own resources
    return !resource.created_by_user_id || resource.created_by_user_id === user?.id
  }

  const filteredVMs = vms.filter(filterByOwnership)
  const filteredFunctions = functions.filter(filterByOwnership)
  const filteredContainers = containers.filter(filterByOwnership)

  // Get VM IDs that are used by functions and containers (to exclude from "all" view)
  const functionVmIds = new Set(
    filteredFunctions
      .map(func => func.vm_id)
      .filter((id): id is string => !!id)
  )

  const containerVmIds = new Set(
    filteredContainers
      .map(container => container.container_runtime_id?.replace('vm-', ''))
      .filter((id): id is string => !!id)
  )

  const usedVmIds = new Set([...functionVmIds, ...containerVmIds])

  // Filter out VMs used by functions/containers
  const pureVMs = filteredVMs.filter(vm => {
    // Exclude VMs used by functions/containers (by ID)
    if (usedVmIds.has(vm.id)) return false

    // Exclude VMs tagged as function or container VMs
    if (vm.tags?.includes("type:function") || vm.tags?.includes("type:container")) {
      return false
    }

    // Exclude VMs with names starting with "fn-" or "container-"
    if (vm.name?.startsWith("fn-") || vm.name?.startsWith("container-")) {
      return false
    }

    return true
  })

  // Calculate stats from pure VMs (excluding function/container VMs)
  const totalVMs = pureVMs.length
  const totalFunctions = filteredFunctions.length
  const totalContainers = filteredContainers.length
  const totalHosts = hosts.length
  console.log("totalHosts:", totalHosts)
  const runningVMs = pureVMs.filter(vm => vm.state === 'running').length
  const runningContainers = filteredContainers.filter(c => c.state === 'running').length

  // Transform VMs for resource table
  const vmResources = pureVMs.map(vm => ({
    id: vm.id,
    name: vm.name,
    type: "vm" as const,
    state: vm.state.toLowerCase(),
    metrics: {
      cpu: vm.vcpu || 0,
      memory: vm.mem_mib || 0, // Memory in MiB
    },
    created_by_user_id: (vm as any).created_by_user_id,
  }))

  // Transform functions for resource table
  const functionResources = filteredFunctions.map(func => ({
    id: func.id,
    name: func.name,
    type: "function" as const,
    state: func.state?.toLowerCase() || 'unknown',
    metrics: {
      cpu: func.vcpu || 0,
      memory: func.memory_mb || 0, // Memory in MB
    },
    created_by_user_id: (func as any).created_by_user_id,
  }))

  // Transform containers for resource table
  const containerResources = filteredContainers.map(container => ({
    id: container.id,
    name: container.name,
    type: "container" as const,
    state: container.state?.toLowerCase() || 'unknown',
    metrics: {
      cpu: container.cpu_limit || 0,
      memory: container.memory_limit_mb || 0, // Memory in MB
    },
    created_by_user_id: (container as any).created_by_user_id,
  }))

  // Combine all resources (VMs used by functions/containers are already excluded)
  const allResources = [...vmResources, ...functionResources, ...containerResources]

  const isLoading = vmsLoading || functionsLoading || containersLoading


  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-foreground">Dashboard</h1>
          <p className="text-muted-foreground">Overview of all your workloads and resources</p>
        </div>
        {canCreateResource(user) && (
          <div className="flex gap-2">
            <Button asChild>
              <Link href="/vms/create">
                <Plus className="mr-2 h-4 w-4" />
                New VM
              </Link>
            </Button>
            <Button asChild variant="outline" className="text-primary">
              <Link href="/functions/new">
                <Plus className="mr-2 h-4 w-4" />
                New Function
              </Link>
            </Button>
            <Button asChild variant="outline" className="text-primary">
              <Link href="/containers/new">
                <Plus className="mr-2 h-4 w-4" />
                New Container
              </Link>
            </Button>
          </div>
        )}
      </div>

      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        <Card className="overflow-hidden">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">
              <Link href="/vms">Virtual Machines</Link>
            </CardTitle>
            <div className="rounded-lg bg-blue-500/10 p-2">
              <Server className="h-4 w-4 text-blue-600" />
            </div>
          </CardHeader>
          <CardContent>
            {vmsLoading ? (
              <Skeleton className="h-8 w-16 mb-2" />
            ) : (
              <div className="text-2xl font-bold">
                {runningVMs}/{totalVMs}
              </div>
            )}
            <div className="text-xs text-foreground flex items-center gap-1 mt-1">
              <TrendingUp className="h-3 w-3 text-green-600" />
              {vmsLoading ? <Skeleton className="h-3 w-12" /> : `${runningVMs} running`}
            </div>
          </CardContent>
        </Card>

        <Card className="overflow-hidden">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">
              <Link href="/functions">Functions</Link>
            </CardTitle>
            <div className="rounded-lg bg-yellow-500/10 p-2">
              <Zap className="h-4 w-4 text-yellow-600" />
            </div>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{totalFunctions}</div>
            <div className="text-xs text-foreground flex items-center gap-1 mt-1">
              <Activity className="h-3 w-3 text-yellow-600" />
              {totalFunctions} running
            </div>
          </CardContent>
        </Card>

        <Card className="overflow-hidden">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">
              <Link href="/containers">Container</Link>
            </CardTitle>
            <div className="rounded-lg bg-purple-500/10 p-2">
              <Container className="h-4 w-4 text-purple-600" />
            </div>
          </CardHeader>
          <CardContent>
            {containersLoading ? (
              <Skeleton className="h-8 w-16 mb-2" />
            ) : (
              <div className="text-2xl font-bold">
                {runningContainers}/{totalContainers}
              </div>
            )}
            <div className="text-xs text-foreground flex items-center gap-1 mt-1">
              <TrendingUp className="h-3 w-3 text-green-600" />
              {containersLoading ? <Skeleton className="h-3 w-12" /> : `${runningContainers} running`}
            </div>
          </CardContent>
        </Card>

        <Card className="overflow-hidden">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">
              <Link href="/hosts">Hosts</Link>
            </CardTitle>
            <div className="rounded-lg bg-green-500/10 p-2">
              <HardDrive className="h-4 w-4 text-green-600" />
            </div>
          </CardHeader>
          <CardContent>
            {hostsLoading ? (
              <Skeleton className="h-8 w-16 mb-2" />
            ) : (
              <div className="text-2xl font-bold">{totalHosts}</div>
            )}
            <p className="text-xs text-foreground mt-1">
              {hostsLoading ? <Skeleton className="h-3 w-24" /> : "Available hosts"}
            </p>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>All Resources</CardTitle>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="space-y-4">
              {[...Array(4)].map((_, i) => (
                <div key={i} className="flex items-center space-x-4 p-4 border rounded">
                  <Skeleton className="h-4 w-24" />
                  <Skeleton className="h-4 w-32" />
                  <Skeleton className="h-4 w-20" />
                  <Skeleton className="h-4 w-16" />
                  <Skeleton className="h-8 w-20 ml-auto" />
                </div>
              ))}
            </div>
          ) : (
            <ResourceTable resources={allResources} />
          )}
        </CardContent>
      </Card>
    </div>
  )
}
