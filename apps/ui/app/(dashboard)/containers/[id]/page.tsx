"use client"

import { ReusableTabs, TabItem, TabContentItem } from "@/components/dashboard/tabs-new"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { ContainerOverview } from "@/components/container/container-overview"
import { ContainerConfig } from "@/components/container/container-config"
import { ContainerLogs } from "@/components/container/container-logs"
import { ContainerStats } from "@/components/container/container-stats"
import { ContainerEvents } from "@/components/container/container-events"
import { EditContainerDialog } from "@/components/container/edit-container-dialog"
import { Play, Square, RotateCw, Trash2, ArrowLeft, Loader2, Pause, PlayCircle, Edit, ExternalLink, Eye, ScrollText, BarChart3, Settings, Activity } from "lucide-react"
import Link from "next/link"
import { use, useState, useMemo } from "react"
import { useContainer, useStartContainer, useStopContainer, useRestartContainer, useDeleteContainer, usePauseContainer, useResumeContainer } from "@/lib/queries"
import { useRouter, useSearchParams } from "next/navigation"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { useAuthStore, canModifyResource, canDeleteResource } from "@/lib/auth/store"

const getStatusColor = (status: string) => {
  switch (status) {
    case "running":
      return "bg-green-500/10 text-green-700 border-green-200"
    case "stopped":
      return "bg-gray-500/10 text-gray-700 border-gray-200"
    case "error":
      return "bg-red-500/10 text-red-700 border-red-200"
    case "paused":
      return "bg-yellow-500/10 text-yellow-700 border-yellow-200"
    case "creating":
    case "booting":
    case "initializing":
      return "bg-blue-500/10 text-blue-700 border-blue-200"
    default:
      return "bg-gray-500/10 text-gray-700 border-gray-200"
  }
}

export default function ContainerDetailPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = use(params)
  const router = useRouter()
  const searchParams = useSearchParams()
  const tabParam = searchParams.get('tab')
  const { user } = useAuthStore()

  // Valid tab values
  const validTabs = ['overview', 'logs', 'stats', 'config', 'events']
  const defaultTab = tabParam && validTabs.includes(tabParam) ? tabParam : 'overview'

  const { data: container, isLoading, error, refetch } = useContainer(id)
  const startContainer = useStartContainer()
  const stopContainer = useStopContainer()
  const restartContainer = useRestartContainer()
  const pauseContainer = usePauseContainer()
  const resumeContainer = useResumeContainer()
  const deleteContainer = useDeleteContainer()

  const [editDialogOpen, setEditDialogOpen] = useState(false)

  const handleDelete = () => {
    if (confirm(`Are you sure you want to delete container "${container?.name}"?`)) {
      deleteContainer.mutate(id, {
        onSuccess: () => {
          router.push("/containers")
        },
      })
    }
  }

  // Extract VM UUID from container_runtime_id (format: "vm-{uuid}")
  const getVmId = () => {
    if (!container?.container_runtime_id) return null
    return container.container_runtime_id.replace('vm-', '')
  }

  const handleViewVm = () => {
    const vmId = getVmId()
    if (vmId) {
      router.push(`/vms/${vmId}`)
    }
  }

  // Define tabs dengan icon
  const tabs: TabItem[] = useMemo(() => [
    { value: "overview", label: "Overview", icon: <Eye size={16} /> },
    { value: "logs", label: "Logs", icon: <ScrollText size={16} /> },
    { value: "stats", label: "Stats", icon: <BarChart3 size={16} /> },
    { value: "config", label: "Config", icon: <Settings size={16} /> },
    { value: "events", label: "Events", icon: <Activity size={16} /> },
  ], [])

  // Define contents untuk setiap tab
  const tabContents: TabContentItem[] = useMemo(() => {
    if (!container) return []

    return [
      {
        value: "overview",
        content: <ContainerOverview container={container} vmId={getVmId()} />,
      },
      {
        value: "logs",
        content: <ContainerLogs containerId={container.id} />,
      },
      {
        value: "stats",
        content: <ContainerStats containerId={container.id} vmId={getVmId()} containerState={container.state} />,
      },
      {
        value: "config",
        content: <ContainerConfig container={container} />,
      },
      {
        value: "events",
        content: <ContainerEvents containerId={container.id} />,
      },
    ]
  }, [container])

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-12">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    )
  }

  if (error || !container) {
    return (
      <Alert variant="destructive">
        <AlertDescription>
          Failed to load container. Please try again.
        </AlertDescription>
      </Alert>
    )
  }
  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <Link href="/containers">
            <Button variant="ghost" size="icon">
              <ArrowLeft className="h-4 w-4" />
            </Button>
          </Link>
          <div>
            <div className="flex items-center gap-3">
              <h1 className="text-3xl font-bold text-foreground">{container.name}</h1>
              <Badge className={getStatusColor(container.state)}>{container.state}</Badge>
            </div>
            <p className="text-sm text-muted-foreground mt-1">
              {container.image} • ID: {container.id}
            </p>
            {container.error_message && (
              <p className="text-sm text-destructive mt-1">{container.error_message}</p>
            )}
          </div>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="outline" size="sm" onClick={() => refetch()}>
            Refresh
          </Button>

          {canModifyResource(user, (container as any).created_by_user_id) && (
            <>
              {/* Edit button - only enabled when stopped or error */}
              <Button
                variant="outline"
                size="sm"
                onClick={() => setEditDialogOpen(true)}
                disabled={container.state !== "stopped" && container.state !== "error"}
              >
                <Edit className="mr-2 h-4 w-4" />
                Edit
              </Button>

              {container.state === "stopped" && (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => startContainer.mutate(container.id)}
                  disabled={startContainer.isPending}
                >
                  <Play className="mr-2 h-4 w-4" />
                  Start
                </Button>
              )}
              {container.state === "running" && (
                <>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => pauseContainer.mutate(container.id)}
                    disabled={pauseContainer.isPending}
                  >
                    <Pause className="mr-2 h-4 w-4" />
                    Pause
                  </Button>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => stopContainer.mutate(container.id)}
                    disabled={stopContainer.isPending}
                  >
                    <Square className="mr-2 h-4 w-4" />
                    Stop
                  </Button>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => restartContainer.mutate(container.id)}
                    disabled={restartContainer.isPending}
                  >
                    <RotateCw className="mr-2 h-4 w-4" />
                    Restart
                  </Button>
                </>
              )}
              {container.state === "paused" && (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => resumeContainer.mutate(container.id)}
                  disabled={resumeContainer.isPending}
                >
                  <PlayCircle className="mr-2 h-4 w-4" />
                  Resume
                </Button>
              )}
            </>
          )}

          {/* View Container VM button */}
          <Button
            variant="outline"
            size="sm"
            onClick={handleViewVm}
            disabled={!getVmId()}
          >
            <ExternalLink className="mr-2 h-4 w-4" />
            View Container VM
          </Button>

          {canDeleteResource(user, (container as any).created_by_user_id) && (
            <Button
              variant="destructive"
              size="sm"
              onClick={handleDelete}
              disabled={deleteContainer.isPending}
            >
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

      {/* Edit Container Dialog */}
      <EditContainerDialog
        container={container}
        open={editDialogOpen}
        onOpenChange={setEditDialogOpen}
      />
    </div>
  )
}
