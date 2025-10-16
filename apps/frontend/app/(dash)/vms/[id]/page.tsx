"use client"

import { use } from "react"
import { useVM } from "@/lib/queries"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Skeleton } from "@/components/ui/skeleton"
import { StatusIndicator } from "@/components/status-indicator"
import { ActionMenu } from "@/components/action-menu"
import { VMOverviewTab } from "@/components/vm-overview-tab"
import { VMConfigTab } from "@/components/vm-config-tab"
import { DriveList } from "@/components/drive-list"
import { NetworkTable } from "@/components/network-table"
import { SnapshotsTab } from "@/components/snapshots-tab"
import { MetricsTab } from "@/components/metrics-tab"
import { VMTerminal } from "@/components/vm-terminal"
import { ArrowLeft, ExternalLink } from "lucide-react"
import Link from "next/link"
import { notFound } from "next/navigation"

interface VMDetailPageProps {
  params: Promise<{
    id: string
  }>
}

export default function VMDetailPage({ params }: VMDetailPageProps) {
  const { id } = use(params)
  const { data: vm, isLoading, error } = useVM(id)

  if (error) {
    notFound()
  }

  if (isLoading) {
    return (
      <div className="space-y-6">
        {/* Header Skeleton */}
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-4">
            <Skeleton className="h-8 w-8" />
            <div className="space-y-2">
              <Skeleton className="h-8 w-48" />
              <Skeleton className="h-4 w-32" />
            </div>
          </div>
          <Skeleton className="h-9 w-24" />
        </div>

        {/* Tabs Skeleton */}
        <div className="space-y-4">
          <Skeleton className="h-10 w-full" />
          <Card>
            <CardHeader>
              <Skeleton className="h-6 w-32" />
            </CardHeader>
            <CardContent className="space-y-4">
              <Skeleton className="h-4 w-full" />
              <Skeleton className="h-4 w-3/4" />
              <Skeleton className="h-4 w-1/2" />
            </CardContent>
          </Card>
        </div>
      </div>
    )
  }

  if (!vm) {
    notFound()
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <Button variant="ghost" size="icon" asChild>
            <Link href="/dashboard">
              <ArrowLeft className="h-4 w-4" />
              <span className="sr-only">Back to dashboard</span>
            </Link>
          </Button>

          <div className="space-y-1">
            <div className="flex items-center gap-3">
              <h1 className="text-2xl font-bold tracking-tight">{vm.name}</h1>
              <StatusIndicator state={vm.state} />
            </div>
            <p className="text-muted-foreground text-sm">ID: {vm.id}</p>
          </div>
        </div>

        <div className="flex items-center gap-2">
          <Button variant="outline" size="sm" asChild>
            <Link href={`/vms/${vm.id}/logs`}>
              <ExternalLink className="h-4 w-4" />
              View Logs
            </Link>
          </Button>
          <ActionMenu vm={vm} variant="button" />
        </div>
      </div>

      {/* VM Info Cards */}
      <div className="grid gap-4 md:grid-cols-4">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Host</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="font-semibold">{vm.host_addr}</p>
            <p className="text-xs text-muted-foreground">ID: {vm.host_id.slice(0, 8)}</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Resources</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-1">
              <p className="text-sm">{vm.vcpu} vCPU</p>
              <p className="text-sm">{vm.mem_mib} MiB RAM</p>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Created</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-sm">{new Date(vm.created_at).toLocaleDateString()}</p>
            <p className="text-xs text-muted-foreground">{new Date(vm.created_at).toLocaleTimeString()}</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Last Updated</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-sm">{new Date(vm.updated_at).toLocaleDateString()}</p>
            <p className="text-xs text-muted-foreground">{new Date(vm.updated_at).toLocaleTimeString()}</p>
          </CardContent>
        </Card>
      </div>

      {/* Tabs */}
      <Tabs defaultValue="overview" className="space-y-4">
        <TabsList className="grid w-full grid-cols-7">
          <TabsTrigger value="overview">Overview</TabsTrigger>
          <TabsTrigger value="config">Config</TabsTrigger>
          <TabsTrigger value="storage">Storage</TabsTrigger>
          <TabsTrigger value="network">Network</TabsTrigger>
          <TabsTrigger value="terminal">Terminal</TabsTrigger>
          <TabsTrigger value="snapshots">Snapshots</TabsTrigger>
          <TabsTrigger value="metrics">Metrics</TabsTrigger>
        </TabsList>

        <TabsContent value="overview">
          <VMOverviewTab vm={vm} />
        </TabsContent>

        <TabsContent value="config">
          <VMConfigTab vm={vm} />
        </TabsContent>

        <TabsContent value="storage">
          <DriveList vm={vm} />
        </TabsContent>

        <TabsContent value="network">
          <NetworkTable vm={vm} />
        </TabsContent>

        <TabsContent value="terminal">
          <VMTerminal vm={vm} />
        </TabsContent>

        <TabsContent value="snapshots">
          <SnapshotsTab vm={vm} />
        </TabsContent>

        <TabsContent value="metrics">
          <MetricsTab vm={vm} />
        </TabsContent>
      </Tabs>
    </div>
  )
}
