"use client"

import { useState } from "react"
import { useVMs } from "@/lib/queries"
import { useUIStore } from "@/lib/store"
import { VMCard } from "@/components/vm-card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Badge } from "@/components/ui/badge"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Search, Filter, Plus, Server, Play, StopCircle, PauseCircle, RefreshCw } from "lucide-react"
import type { VM } from "@/types/firecracker"
import type { Vm } from "@/types/nexus"
import Link from "next/link"

export default function DashboardPage() {
  const { data: vms, isLoading, error, refetch } = useVMs()
  const { dashboardFilters, setDashboardFilters, selectedVMs, toggleVMSelection, clearSelection } = useUIStore()

  const [showFilters, setShowFilters] = useState(false)

  // Filter VMs based on current filters - adapt to new backend structure
  const filteredVMs =
    vms?.filter((vm: Vm | VM) => {
      // Search filter
      if (dashboardFilters.search) {
        const searchTerm = dashboardFilters.search.toLowerCase()
        const matchesSearch =
          vm.name.toLowerCase().includes(searchTerm) ||
          (vm as VM).description?.toLowerCase().includes(searchTerm) ||
          (vm as VM).owner?.toLowerCase().includes(searchTerm) ||
          (vm as Vm).host_addr?.toLowerCase().includes(searchTerm) ||
          ((vm as VM).tags && Object.values((vm as VM).tags).some((tag) => tag.toLowerCase().includes(searchTerm)))
        if (!matchesSearch) return false
      }

      // State filter - normalize state names
      if (dashboardFilters.state.length > 0) {
        const normalizedState = vm.state?.toLowerCase() === 'running' ? 'running' : 
                               vm.state?.toLowerCase() === 'paused' ? 'paused' : 'stopped'
        if (!dashboardFilters.state.includes(normalizedState)) {
          return false
        }
      }

      // Owner filter - use host_addr for new backend
      if (dashboardFilters.owner.length > 0) {
        const owner = (vm as VM).owner || (vm as Vm).host_addr || 'unknown'
        if (!dashboardFilters.owner.includes(owner)) {
          return false
        }
      }

      // Environment filter - not available in new backend, skip
      if (dashboardFilters.environment.length > 0 && (vm as VM).environment && !dashboardFilters.environment.includes((vm as VM).environment)) {
        return false
      }

      return true
    }) || []

  // Get unique values for filter options - adapt to new backend
  const uniqueOwners = [...new Set(vms?.map((vm: Vm | VM) => (vm as VM).owner || (vm as Vm).host_addr || 'unknown') || [])]
  const uniqueEnvironments = [...new Set(vms?.map((vm: VM) => vm.environment).filter(Boolean) || [])]

  // Calculate stats - normalize state names
  const stats = {
    total: vms?.length || 0,
    running: vms?.filter((vm: Vm | VM) => vm.state?.toLowerCase() === "running").length || 0,
    stopped: vms?.filter((vm: Vm | VM) => vm.state?.toLowerCase() !== "running" && vm.state?.toLowerCase() !== "paused").length || 0,
    paused: vms?.filter((vm: Vm | VM) => vm.state?.toLowerCase() === "paused").length || 0,
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-96">
        <div className="text-center">
          <p className="text-lg font-medium text-destructive">Failed to load VMs</p>
          <p className="text-sm text-muted-foreground mt-1">Please check your connection and try again</p>
          <Button onClick={() => refetch()} className="mt-4">
            <RefreshCw className="h-4 w-4 mr-2" />
            Retry
          </Button>
        </div>
      </div>
    )
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Dashboard</h1>
          <p className="text-muted-foreground">Manage and monitor your Firecracker microVMs</p>
        </div>
  <Button asChild className="bg-primary text-primary-foreground hover:outline hover:outline-2 hover:outline-offset-2 hover:[outline-color:hsl(var(--success))]">
          <Link href="/vms/create">
            <Plus className="h-4 w-4 mr-2" />
            Create VM
          </Link>
        </Button>
      </div>

      {/* Stats Cards */}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Total VMs</CardTitle>
            <Server className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{stats.total}</div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Running</CardTitle>
            <Play className="h-4 w-4 text-success" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-success">{stats.running}</div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Stopped</CardTitle>
            <StopCircle className="h-4 w-4 text-destructive" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-destructive">{stats.stopped}</div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Paused</CardTitle>
            <PauseCircle className="h-4 w-4 text-warning" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-warning">{stats.paused}</div>
          </CardContent>
        </Card>
      </div>

      {/* Filters */}
      <div className="space-y-4">
        <div className="flex items-center gap-4">
          <div className="relative flex-1 max-w-sm">
            <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-muted-foreground" />
            <Input
              placeholder="Search VMs..."
              value={dashboardFilters.search}
              onChange={(e) => setDashboardFilters({ search: e.target.value })}
              className="pl-9"
            />
          </div>

          <Button variant="outline" onClick={() => setShowFilters(!showFilters)} className="shrink-0">
            <Filter className="h-4 w-4 mr-2" />
            Filters
          </Button>

          {selectedVMs.length > 0 && (
            <div className="flex items-center gap-2">
              <Badge variant="secondary">{selectedVMs.length} selected</Badge>
              <Button variant="outline" size="sm" onClick={clearSelection}>
                Clear
              </Button>
            </div>
          )}
        </div>

        {showFilters && (
          <div className="grid gap-4 md:grid-cols-3 p-4 border rounded-lg bg-muted/50">
            <Select
              value={dashboardFilters.state.join(",")}
              onValueChange={(value) => setDashboardFilters({ state: value ? value.split(",") : [] })}
            >
              <SelectTrigger>
                <SelectValue placeholder="Filter by state" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="running">Running</SelectItem>
                <SelectItem value="stopped">Stopped</SelectItem>
                <SelectItem value="paused">Paused</SelectItem>
              </SelectContent>
            </Select>

            <Select
              value={dashboardFilters.owner.join(",")}
              onValueChange={(value) => setDashboardFilters({ owner: value ? value.split(",") : [] })}
            >
              <SelectTrigger>
                <SelectValue placeholder="Filter by owner" />
              </SelectTrigger>
              <SelectContent>
                {uniqueOwners.map((owner) => (
                  <SelectItem key={owner} value={owner}>
                    {owner}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>

            <Select
              value={dashboardFilters.environment.join(",")}
              onValueChange={(value) => setDashboardFilters({ environment: value ? value.split(",") : [] })}
            >
              <SelectTrigger>
                <SelectValue placeholder="Filter by environment" />
              </SelectTrigger>
              <SelectContent>
                {uniqueEnvironments.map((env) => (
                  <SelectItem key={env} value={env}>
                    {env}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        )}
      </div>

      {/* VM Grid */}
      {isLoading ? (
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          {Array.from({ length: 6 }).map((_, i) => (
            <Card key={i} className="animate-pulse">
              <CardHeader>
                <div className="h-4 bg-muted rounded w-3/4"></div>
                <div className="h-3 bg-muted rounded w-1/2"></div>
              </CardHeader>
              <CardContent>
                <div className="space-y-2">
                  <div className="h-3 bg-muted rounded"></div>
                  <div className="h-3 bg-muted rounded w-2/3"></div>
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      ) : filteredVMs.length === 0 ? (
        <div className="text-center py-12 flow-blob rounded-2xl">
          <Server className="h-12 w-12 text-primary mx-auto mb-4" />
          <h3 className="text-lg font-medium">No VMs found</h3>
          <p className="text-muted-foreground mb-4">
            {dashboardFilters.search || dashboardFilters.state.length > 0
              ? "Try adjusting your filters or search terms"
              : "Get started by creating your first virtual machine"}
          </p>
          <Button asChild className="bg-primary text-primary-foreground hover:outline hover:outline-2 hover:outline-offset-2 hover:[outline-color:hsl(var(--success))]">
            <Link href="/vms/create">
              <Plus className="h-4 w-4 mr-2" />
              Create VM
            </Link>
          </Button>
        </div>
      ) : (
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          {filteredVMs.map((vm: Vm | VM) => (
            <VMCard key={vm.id} vm={vm} isSelected={selectedVMs.includes(vm.id)} onSelect={toggleVMSelection} />
          ))}
        </div>
      )}
    </div>
  )
}
