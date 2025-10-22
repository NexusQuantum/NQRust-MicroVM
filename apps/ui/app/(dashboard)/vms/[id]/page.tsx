"use client"

import { useVM, useVmStatePatch, useDeleteVM } from "@/lib/queries"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { VMOverview } from "@/components/vm/vm-overview"
import { VMConfig } from "@/components/vm/vm-config"
import { VMStorage } from "@/components/vm/vm-storage"
import { VMNetwork } from "@/components/vm/vm-network"
import { VMSnapshots } from "@/components/vm/vm-snapshots"
import { XTermWrapper } from "@/components/shared/xterm-wrapper"
import { MetricsChart } from "@/components/shared/metrics-chart"
import { Play, Square, RotateCw, Trash2, ArrowLeft, Zap, Pause } from "lucide-react"
import Link from "next/link"
import { ConfirmDialog } from "@/components/shared/confirm-dialog"
import { useState } from "react"

const getStatusColor = (state: string) => {
  switch (state) {
    case "running":
      return "bg-green-500/10 text-green-700 border-green-200"
    case "stopped":
      return "bg-gray-500/10 text-gray-700 border-gray-200"
    case "paused":
      return "bg-yellow-500/10 text-yellow-700 border-yellow-200"
    default:
      return "bg-blue-500/10 text-blue-700 border-blue-200"
  }
}

export default function VMDetailPage({ params }: { params: { id: string } }) {
  const { data: vm, isLoading, error } = useVM(params.id)
  const vmStatePatch = useVmStatePatch()
  const deleteVM = useDeleteVM()
  const [deleteDialog, setDeleteDialog] = useState(false)

  const handleAction = (action: 'start'|'stop'|'pause'|'resume'|'ctrl_alt_del') => {
    vmStatePatch.mutate({ id: params.id, action })
  }

  const handleDelete = () => {
    deleteVM.mutate(params.id, {
      onSuccess: () => {
        window.location.href = '/vms'
      }
    })
  }

  if (isLoading) {
    return (
      <div className="space-y-6">
        <div className="animate-pulse space-y-4">
          <div className="h-8 bg-muted rounded w-1/3"></div>
          <div className="h-64 bg-muted rounded-lg"></div>
        </div>
      </div>
    )
  }

  if (error) {
    return (
      <div className="space-y-6">
        <div className="text-center space-y-4">
          <h1 className="text-2xl font-bold text-destructive">Failed to load VM</h1>
          <p className="text-muted-foreground">
            Unable to fetch VM details. Please check your connection and try again.
          </p>
        </div>
      </div>
    )
  }

  if (!vm) {
    return (
      <div className="space-y-6">
        <div className="text-center space-y-4">
          <h1 className="text-2xl font-bold">VM Not Found</h1>
          <p className="text-muted-foreground">
            The requested VM could not be found.
          </p>
        </div>
      </div>
    )
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <Link href="/vms">
            <Button variant="ghost" size="icon">
              <ArrowLeft className="h-4 w-4" />
            </Button>
          </Link>
          <div>
            <div className="flex items-center gap-3">
              <h1 className="text-3xl font-bold text-foreground">{vm.name}</h1>
              <Badge className={getStatusColor(vm.state)}>{vm.state}</Badge>
            </div>
            <p className="text-sm text-muted-foreground mt-1">
              {vm.vcpu} vCPU • {vm.mem_mib} MB RAM • {vm.guest_ip || 'No IP'}
            </p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          {vm.state === 'stopped' && (
            <Button variant="outline" size="sm" onClick={() => handleAction('start')}>
              <Play className="mr-2 h-4 w-4" />
              Start
            </Button>
          )}
          {vm.state === 'running' && (
            <>
              <Button variant="outline" size="sm" onClick={() => handleAction('pause')}>
                <Pause className="mr-2 h-4 w-4" />
                Pause
              </Button>
              <Button variant="outline" size="sm" onClick={() => handleAction('stop')}>
                <Square className="mr-2 h-4 w-4" />
                Stop
              </Button>
              <Button variant="outline" size="sm" onClick={() => handleAction('ctrl_alt_del')}>
                <Zap className="mr-2 h-4 w-4" />
                Ctrl+Alt+Del
              </Button>
            </>
          )}
          {vm.state === 'paused' && (
            <Button variant="outline" size="sm" onClick={() => handleAction('resume')}>
              <Play className="mr-2 h-4 w-4" />
              Resume
            </Button>
          )}
          <Button variant="destructive" size="sm" onClick={() => setDeleteDialog(true)}>
            <Trash2 className="mr-2 h-4 w-4" />
            Delete
          </Button>
        </div>
      </div>

      <Tabs defaultValue="overview" className="space-y-4">
        <TabsList className="bg-muted/50">
          <TabsTrigger value="overview">Overview</TabsTrigger>
          <TabsTrigger value="config">Config</TabsTrigger>
          <TabsTrigger value="storage">Storage</TabsTrigger>
          <TabsTrigger value="network">Network</TabsTrigger>
          <TabsTrigger value="terminal">Terminal</TabsTrigger>
          <TabsTrigger value="snapshots">Snapshots</TabsTrigger>
          <TabsTrigger value="metrics">Metrics</TabsTrigger>
        </TabsList>

        <TabsContent value="overview" className="space-y-4">
          <VMOverview vm={vm} />
        </TabsContent>

        <TabsContent value="config" className="space-y-4">
          <VMConfig vm={vm} />
        </TabsContent>

        <TabsContent value="storage" className="space-y-4">
          <VMStorage vmId={vm.id} />
        </TabsContent>

        <TabsContent value="network" className="space-y-4">
          <VMNetwork vmId={vm.id} />
        </TabsContent>

        <TabsContent value="terminal" className="space-y-4">
          <XTermWrapper vmId={vm.id} />
        </TabsContent>

        <TabsContent value="snapshots" className="space-y-4">
          <VMSnapshots vmId={vm.id} />
        </TabsContent>

        <TabsContent value="metrics" className="space-y-4">
          <MetricsChart resourceId={vm.id} resourceType="vm" />
        </TabsContent>
      </Tabs>

      <ConfirmDialog
        open={deleteDialog}
        onOpenChange={setDeleteDialog}
        title="Delete VM"
        description={`Are you sure you want to delete "${vm.name}"? This action cannot be undone.`}
        onConfirm={handleDelete}
        isPending={deleteVM.isPending}
      />
    </div>
  )
}
