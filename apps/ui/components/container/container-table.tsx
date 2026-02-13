"use client"

import { useState } from "react"
import Link from "next/link"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { StatusBadge } from "@/components/shared/status-badge"
import { ConfirmDialog } from "@/components/shared/confirm-dialog"
import { Play, Square, RotateCw, FileText, Terminal, Trash2, Search } from "lucide-react"
import { formatDuration } from "@/lib/utils/format"
import type { Container } from "@/lib/types"
import { useStartContainer, useStopContainer, useRestartContainer, useDeleteContainer, useVolumes, useDeleteVolume } from "@/lib/queries"
import { useAuthStore, canModifyResource, canDeleteResource } from "@/lib/auth/store"
import { Checkbox } from "@/components/ui/checkbox"
import { Label } from "@/components/ui/label"
import { toast } from "sonner"

interface ContainerTableProps {
  containers: Container[]
}

export function ContainerTable({ containers }: ContainerTableProps) {
  const [searchQuery, setSearchQuery] = useState("")
  const [statusFilter, setStatusFilter] = useState<string>("all")
  const [deleteDialog, setDeleteDialog] = useState<{ open: boolean; containerId: string; containerName: string }>({
    open: false,
    containerId: "",
    containerName: "",
  })
  const { user } = useAuthStore()

  const startContainer = useStartContainer()
  const stopContainer = useStopContainer()
  const restartContainer = useRestartContainer()
  const deleteContainer = useDeleteContainer()
  const { data: allVolumes = [] } = useVolumes()
  const deleteVolumeMutation = useDeleteVolume()
  const [deleteVolumesChecked, setDeleteVolumesChecked] = useState(true)

  const deletingContainer = containers.find(c => c.id === deleteDialog.containerId)
  const containerVmId = deletingContainer?.container_runtime_id?.replace('vm-', '')
  const attachedVolumes = allVolumes.filter(v => containerVmId && v.attached_to_vm_id === containerVmId)

  const handleDelete = () => {
    if (deleteDialog.containerId && deleteDialog.containerName) {
      const volumeIdsToDelete = deleteVolumesChecked ? attachedVolumes.map(v => v.id) : []
      deleteContainer.mutate(deleteDialog.containerId, {
        onSuccess: async () => {
          if (volumeIdsToDelete.length > 0) {
            await Promise.allSettled(
              volumeIdsToDelete.map(id => deleteVolumeMutation.mutateAsync(id))
            )
          }
          toast.success("Container Deleted", {
            description: `${deleteDialog.containerName} has been deleted successfully`,
          })
          setDeleteDialog({ open: false, containerId: "", containerName: "" })
        },
        onError: (error) => {
          toast.error("Delete Failed", {
            description: `Failed to delete ${deleteDialog.containerName}: ${error.message}`,
          })
        }
      })
    }
  }

  const filteredContainers = containers.filter((container) => {
    const matchesSearch =
      container.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      container.image.toLowerCase().includes(searchQuery.toLowerCase())
    const matchesStatus = statusFilter === "all" || container.state === statusFilter

    // Filter by ownership for non-admin/non-viewer users
    const canView = user?.role === "admin" || user?.role === "viewer" ||
                    !(container as any).created_by_user_id ||
                    (container as any).created_by_user_id === user?.id

    return matchesSearch && matchesStatus && canView
  })

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-4">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            placeholder="Search containers..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-9"
          />
        </div>
        <Select value={statusFilter} onValueChange={setStatusFilter}>
          <SelectTrigger className="w-40">
            <SelectValue placeholder="Status" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All Status</SelectItem>
            <SelectItem value="running">Running</SelectItem>
            <SelectItem value="stopped">Stopped</SelectItem>
            <SelectItem value="creating">Creating</SelectItem>
            <SelectItem value="booting">Booting</SelectItem>
            <SelectItem value="initializing">Initializing</SelectItem>
            <SelectItem value="paused">Paused</SelectItem>
            <SelectItem value="error">Error</SelectItem>
          </SelectContent>
        </Select>
      </div>

      <div className="rounded-lg border border-border">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Name</TableHead>
              <TableHead>Image</TableHead>
              <TableHead>Status</TableHead>
              <TableHead>Uptime</TableHead>
              <TableHead>CPU</TableHead>
              <TableHead>Memory</TableHead>
              <TableHead>Ports</TableHead>
              <TableHead>Owner</TableHead>
              <TableHead className="text-right">Actions</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {filteredContainers.length === 0 ? (
              <TableRow>
                <TableCell colSpan={9} className="text-center py-8 text-muted-foreground">
                  No containers found
                </TableCell>
              </TableRow>
            ) : (
              filteredContainers.map((container) => (
                <TableRow key={container.id}>
                  <TableCell>
                    <Link href={`/containers/${container.id}`} className="font-medium hover:underline">
                      {container.name}
                    </Link>
                  </TableCell>
                  <TableCell>
                    <code className="text-xs bg-muted px-1.5 py-0.5 rounded">{container.image}</code>
                  </TableCell>
                  <TableCell>
                    <StatusBadge status={container.state} />
                  </TableCell>
                  <TableCell className="text-sm text-muted-foreground">
                    {container.uptime_seconds ? formatDuration(container.uptime_seconds) : "N/A"}
                  </TableCell>
                  <TableCell className="text-sm">
                    {container.cpu_limit !== undefined ? `${container.cpu_limit} vCPU` : "N/A"}
                  </TableCell>
                  <TableCell className="text-sm">
                    {container.memory_limit_mb !== undefined ? `${container.memory_limit_mb} MB` : "N/A"}
                  </TableCell>
                  <TableCell className="text-xs">
                    {container.port_mappings && container.port_mappings.length > 0 ? (
                      container.port_mappings.map((p, i) => (
                        <div key={i} className="font-mono">
                          {p.host}:{p.container} ({p.protocol})
                        </div>
                      ))
                    ) : (
                      <span className="text-muted-foreground">No ports</span>
                    )}
                  </TableCell>
                  <TableCell className="text-sm text-muted-foreground">
                    {(container as any).created_by_user_id ? (
                      (container as any).created_by_user_id === user?.id ? (
                        <span className="text-primary font-medium">You</span>
                      ) : (
                        <span className="text-muted-foreground">Other User</span>
                      )
                    ) : (
                      <span className="text-muted-foreground italic">System</span>
                    )}
                  </TableCell>
                  <TableCell className="text-right">
                    {!canModifyResource(user, (container as any).created_by_user_id) &&
                     !canDeleteResource(user, (container as any).created_by_user_id) ? (
                      <span className="text-muted-foreground text-sm">Not permitted</span>
                    ) : (
                      <div className="flex justify-end gap-1">
                        <Button variant="ghost" size="icon" title="Logs" asChild>
                          <Link href={`/containers/${container.id}?tab=logs`}>
                            <FileText className="h-4 w-4" />
                          </Link>
                        </Button>
                        <Button variant="ghost" size="icon" title="Shell" asChild>
                          <Link href={`/containers/${container.id}?tab=shell`}>
                            <Terminal className="h-4 w-4" />
                          </Link>
                        </Button>
                        {canModifyResource(user, (container as any).created_by_user_id) && (
                          <>
                            {container.state === "stopped" && (
                              <Button
                                variant="ghost"
                                size="icon"
                                title="Start"
                                onClick={() => startContainer.mutate(container.id)}
                                disabled={startContainer.isPending}
                              >
                                <Play className="h-4 w-4" />
                              </Button>
                            )}
                            {container.state === "running" && (
                              <>
                                <Button
                                  variant="ghost"
                                  size="icon"
                                  title="Restart"
                                  onClick={() => restartContainer.mutate(container.id)}
                                  disabled={restartContainer.isPending}
                                >
                                  <RotateCw className="h-4 w-4" />
                                </Button>
                                <Button
                                  variant="ghost"
                                  size="icon"
                                  title="Stop"
                                  onClick={() => stopContainer.mutate(container.id)}
                                  disabled={stopContainer.isPending}
                                >
                                  <Square className="h-4 w-4" />
                                </Button>
                              </>
                            )}
                          </>
                        )}
                        {canDeleteResource(user, (container as any).created_by_user_id) && (
                          <Button
                            variant="ghost"
                            size="icon"
                            title="Delete"
                            onClick={() => setDeleteDialog({ open: true, containerId: container.id, containerName: container.name })}
                          >
                            <Trash2 className="h-4 w-4" />
                          </Button>
                        )}
                      </div>
                    )}
                  </TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </div>

      <ConfirmDialog
        open={deleteDialog.open}
        onOpenChange={(open) => setDeleteDialog({ ...deleteDialog, open })}
        title="Delete Container"
        description={`Are you sure you want to delete "${deleteDialog.containerName}"? This action cannot be undone and will permanently remove the container and its data.`}
        confirmText="Delete"
        onConfirm={() => handleDelete()}
        variant="destructive"
      >
        {attachedVolumes.length > 0 && (
          <div className="flex items-center space-x-2 py-2">
            <Checkbox
              id="delete-container-volumes"
              checked={deleteVolumesChecked}
              onCheckedChange={(checked) => setDeleteVolumesChecked(checked as boolean)}
            />
            <Label htmlFor="delete-container-volumes" className="text-sm cursor-pointer">
              Also delete {attachedVolumes.length} attached volume{attachedVolumes.length !== 1 ? "s" : ""}
            </Label>
          </div>
        )}
      </ConfirmDialog>
    </div>
  )
}
