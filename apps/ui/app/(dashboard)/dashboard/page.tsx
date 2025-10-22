"use client"

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Server, Zap, Container, HardDrive, Plus, TrendingUp, Activity } from "lucide-react"
import { ResourceTable } from "@/components/dashboard/resource-table"
import Link from "next/link"
import { useVMs } from "@/lib/queries"
import { Skeleton } from "@/components/ui/skeleton"

// Mock data for services that don't have APIs yet
const mockStats = {
  total_functions: 24,
  invocations_24h: 15420,
  total_containers: 18,
  running_containers: 14,
  total_hosts: 4,
}

// const mockNonVMResources = [
//   {
//     id: "fn-1",
//     name: "image-processor",
//     type: "function" as const,
//     state: "idle",
//     metrics: { lastInvoked: new Date(Date.now() - 3600000).toISOString() },
//   },
//   {
//     id: "fn-2",
//     name: "email-sender",
//     type: "function" as const,
//     state: "idle",
//     metrics: { lastInvoked: new Date(Date.now() - 300000).toISOString() },
//   },
//   {
//     id: "ct-1",
//     name: "postgres-main",
//     type: "container" as const,
//     state: "running",
//     metrics: { cpu: 32.1, memory: 512 },
//   },
//   {
//     id: "ct-2",
//     name: "redis-cache",
//     type: "container" as const,
//     state: "running",
//     metrics: { cpu: 12.4, memory: 128 },
//   },
// ]

export default function DashboardPage() {
  const { data: vms = [], isLoading: vmsLoading } = useVMs()

  // Calculate VM stats from real data
  const totalVMs = vms.length
  const runningVMs = vms.filter(vm => vm.state === 'running').length

  // Transform VMs for resource table
  const vmResources = vms.map(vm => ({
    id: vm.id,
    name: vm.vm_name || `VM-${vm.id}`,
    type: "vm" as const,
    state: vm.state.toLowerCase(),
    metrics: { cpu: 0, memory: 0 }, // TODO: Get real metrics when available
  }))

  // Combine VMs with mock resources
  const allResources = [...vmResources]
  // console.log('all resource: ', allResources)


  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-foreground">Dashboard</h1>
          <p className="text-muted-foreground">Overview of all your workloads and resources</p>
        </div>
        <div className="flex gap-2">
          <Button asChild>
            <Link href="/vms/create">
              <Plus className="mr-2 h-4 w-4" />
              New VM
            </Link>
          </Button>
          <Button asChild variant="outline">
            <Link href="/functions/new">
              <Plus className="mr-2 h-4 w-4" />
              New Function
            </Link>
          </Button>
          <Button asChild variant="outline">
            <Link href="/containers/new">
              <Plus className="mr-2 h-4 w-4" />
              New Container
            </Link>
          </Button>
        </div>
      </div>

      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        <Card className="overflow-hidden">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Virtual Machines</CardTitle>
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
            <div className="text-xs text-muted-foreground flex items-center gap-1 mt-1">
              <TrendingUp className="h-3 w-3 text-green-600" />
              {vmsLoading ? <Skeleton className="h-3 w-12" /> : `${runningVMs} running`}
            </div>
          </CardContent>
        </Card>

        <Card className="overflow-hidden">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Functions</CardTitle>
            <div className="rounded-lg bg-yellow-500/10 p-2">
              <Zap className="h-4 w-4 text-yellow-600" />
            </div>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{mockStats.total_functions}</div>
            <div className="text-xs text-muted-foreground flex items-center gap-1 mt-1">
              <Activity className="h-3 w-3 text-yellow-600" />
              {mockStats.invocations_24h.toLocaleString('en-US')} invocations (24h)
            </div>
          </CardContent>
        </Card>

        <Card className="overflow-hidden">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Containers</CardTitle>
            <div className="rounded-lg bg-purple-500/10 p-2">
              <Container className="h-4 w-4 text-purple-600" />
            </div>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {mockStats.running_containers}/{mockStats.total_containers}
            </div>
            <p className="text-xs text-muted-foreground flex items-center gap-1 mt-1">
              <TrendingUp className="h-3 w-3 text-green-600" />
              {mockStats.running_containers} running
            </p>
          </CardContent>
        </Card>

        <Card className="overflow-hidden">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Hosts</CardTitle>
            <div className="rounded-lg bg-green-500/10 p-2">
              <HardDrive className="h-4 w-4 text-green-600" />
            </div>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{mockStats.total_hosts}</div>
            <p className="text-xs text-muted-foreground mt-1">Available hosts</p>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>All Resources</CardTitle>
        </CardHeader>
        <CardContent>
          {vmsLoading ? (
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
