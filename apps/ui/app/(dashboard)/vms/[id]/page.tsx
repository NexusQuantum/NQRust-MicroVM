"use client"

import { useVM, useVmStatePatch, useDeleteVM } from "@/lib/queries"
import { ReusableTabs, TabItem, TabContentItem } from "@/components/dashboard/tabs-new"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { VMOverview } from "@/components/vm/vm-overview"
import { VMConfig } from "@/components/vm/vm-config"
import { VMStorage } from "@/components/vm/vm-storage"
import { VMNetwork } from "@/components/vm/vm-network"
import { VMSnapshots } from "@/components/vm/vm-snapshots"
import { XTermWrapper } from "@/components/shared/xterm-wrapper"
import { MetricsChart } from "@/components/shared/metrics-chart"
import { Play, Square, Trash2, ArrowLeft, Zap, Pause, Settings, HardDrive, Network, Terminal, Camera, BarChart3, Eye } from "lucide-react"
import Link from "next/link"
import { ConfirmDialog } from "@/components/shared/confirm-dialog"
import { useState, useMemo } from "react"
import { use } from "react"
import { useSearchParams } from "next/navigation"
import { useAuthStore, canModifyResource, canDeleteResource } from "@/lib/auth/store"

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

export default function VMDetailPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = use(params)
  const searchParams = useSearchParams()
  const tabParam = searchParams.get('tab')
  const { data: vm, isLoading, error } = useVM(id)
  const vmStatePatch = useVmStatePatch()
  const deleteVM = useDeleteVM()
  const [deleteDialog, setDeleteDialog] = useState(false)
  const { user } = useAuthStore()

  // Valid tab values
  const validTabs = ['overview', 'config', 'storage', 'network', 'terminal', 'snapshots', 'metrics']
  const defaultTab = tabParam && validTabs.includes(tabParam) ? tabParam : 'overview'

  const handleAction = (action: 'start' | 'stop' | 'pause' | 'resume' | 'ctrl_alt_del') => {
    vmStatePatch.mutate({ id, action })
  }

  const handleDelete = () => {
    deleteVM.mutate(id, {
      onSuccess: () => {
        window.location.href = '/vms'
      }
    })
  }

  // Define tabs dengan icon
  const tabs: TabItem[] = useMemo(() => [
    { value: "overview", label: "Overview", icon: <Eye size={16} /> },
    { value: "config", label: "Config", icon: <Settings size={16} /> },
    { value: "storage", label: "Storage", icon: <HardDrive size={16} /> },
    { value: "network", label: "Network", icon: <Network size={16} /> },
    { value: "terminal", label: "Terminal", icon: <Terminal size={16} /> },
    { value: "snapshots", label: "Snapshots", icon: <Camera size={16} /> },
    { value: "metrics", label: "Metrics", icon: <BarChart3 size={16} /> },
  ], [])

  // Define contents untuk setiap tab
  const tabContents: TabContentItem[] = useMemo(() => {
    if (!vm) return []

    return [
      {
        value: "overview",
        content: <VMOverview vm={vm} />,
      },
      {
        value: "config",
        content: <VMConfig vm={vm} />,
      },
      {
        value: "storage",
        content: <VMStorage vmId={vm.id} />,
      },
      {
        value: "network",
        content: <VMNetwork vmId={vm.id} />,
      },
      {
        value: "terminal",
        content: <XTermWrapper vmId={vm.id} />,
      },
      {
        value: "snapshots",
        content: <VMSnapshots vmId={vm.id} />,
      },
      {
        value: "metrics",
        content: <MetricsChart resourceId={vm.id} />,
      },
    ]
  }, [vm])

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
          {canModifyResource(user, (vm as any).created_by_user_id) && (
            <>
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
            </>
          )}
          {canDeleteResource(user, (vm as any).created_by_user_id) && (
            <Button variant="destructive" size="sm" onClick={() => setDeleteDialog(true)}>
              <Trash2 className="mr-2 h-4 w-4" />
              Delete
            </Button>
          )}
        </div>
      </div>

      <ReusableTabs
        tabs={tabs}
        contents={tabContents}
        defaultValue={defaultTab}
        className="space-y-4"
        tabsContentClassName="space-y-4"
      />

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
