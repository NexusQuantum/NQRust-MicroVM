"use client"

import { useMemo, useState } from "react"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Textarea } from "@/components/ui/textarea"
import { AlertTriangle, Camera, Download, Trash2, Plus } from "lucide-react"
import { useSnapshots, useCreateSnapshot, useRestoreSnapshot, useDeleteSnapshot } from "@/lib/queries"
import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import { z } from "zod"
import { toast } from "sonner"
import { AlertBanner } from "./alert-banner"
import type { VM } from "@/types/firecracker"
import type { Vm } from "@/types/nexus"

const createSnapshotSchema = z.object({
  name: z.string().min(1, "Snapshot name is required"),
  snapshot_type: z.enum(["Full", "Diff"]).default("Full"),
  track_dirty_pages: z.enum(["true", "false"]).transform((val) => val === "true"),
  version: z.string().optional(),
})

type CreateSnapshotForm = z.infer<typeof createSnapshotSchema>

interface SnapshotsTabProps {
  vm: VM | Vm
}

export function SnapshotsTab({ vm }: SnapshotsTabProps) {
  const [createDialogOpen, setCreateDialogOpen] = useState(false)
  const { data: snapshots, isLoading } = useSnapshots(vm.id)
  const createSnapshot = useCreateSnapshot()
  const restoreSnapshot = useRestoreSnapshot()
  const deleteSnapshot = useDeleteSnapshot()

  const defaultName = useMemo(() => {
    const prefix = `${vm.name.replace(/[^a-zA-Z0-9-]/g, "-") || "snapshot"}`
    const timestamp = new Date().toISOString().replace(/[:.]/g, "-")
    return `${prefix}-${timestamp}`.toLowerCase()
  }, [vm.name])

  const {
    register,
    handleSubmit,
    reset,
    watch,
    formState: { errors, isSubmitting },
  } = useForm<CreateSnapshotForm>({
    resolver: zodResolver(createSnapshotSchema),
    defaultValues: {
      name: defaultName,
      snapshot_type: "Full",
      track_dirty_pages: "false",
    },
  })

  const canCreateSnapshot = vm.state === "running" || vm.state === "paused"
  const canRestore = vm.state === "stopped"

  const onCreateSnapshot = async (data: CreateSnapshotForm) => {
    try {
      await createSnapshot.mutateAsync({
        vmId: vm.id,
        name: data.name,
        snapshot_type: data.snapshot_type,
        track_dirty_pages: data.snapshot_type === "Diff" ? data.track_dirty_pages : undefined,
        version: data.version,
      })
      toast.success("Snapshot created successfully")
      setCreateDialogOpen(false)
      reset()
    } catch (error) {
      toast.error("Failed to create snapshot")
    }
  }

  const handleRestore = async (snapshotId: string, snapshotName: string) => {
    if (
      !confirm(`Are you sure you want to restore snapshot "${snapshotName}"? This will replace the current VM state.`)
    ) {
      return
    }

    try {
      await restoreSnapshot.mutateAsync({ vmId: vm.id, snapshotId })
      toast.success("Snapshot restored successfully")
    } catch (error) {
      toast.error("Failed to restore snapshot")
    }
  }

  const handleDelete = async (snapshotId: string, snapshotName: string) => {
    if (!confirm(`Are you sure you want to delete snapshot "${snapshotName}"? This action cannot be undone.`)) {
      return
    }

    try {
      await deleteSnapshot.mutateAsync({ vmId: vm.id, snapshotId })
      toast.success("Snapshot deleted successfully")
    } catch (error) {
      toast.error("Failed to delete snapshot")
    }
  }

  if (isLoading) {
    return <div className="flex items-center justify-center h-32">Loading snapshots...</div>
  }

  return (
    <div className="space-y-6">
      {!canCreateSnapshot && !canRestore && (
        <AlertBanner
          variant="warning"
          icon={AlertTriangle}
          title="Limited snapshot operations"
          description="VM must be running or paused to create snapshots, and stopped to restore them."
        />
      )}

      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-lg font-semibold">VM Snapshots</h3>
          <p className="text-sm text-muted-foreground">Create and manage point-in-time snapshots of your VM</p>
        </div>

        <Dialog open={createDialogOpen} onOpenChange={setCreateDialogOpen}>
          <DialogTrigger asChild>
            <Button disabled={!canCreateSnapshot} className="gap-2">
              <Plus className="h-4 w-4" />
              Create Snapshot
            </Button>
          </DialogTrigger>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Create VM Snapshot</DialogTitle>
            </DialogHeader>
            <form onSubmit={handleSubmit(onCreateSnapshot)} className="space-y-4">
              <div>
                <Label htmlFor="name">Snapshot Name</Label>
                <Input id="name" {...register("name")} placeholder={defaultName} />
                {errors.name && <p className="text-sm text-destructive mt-1">{errors.name.message}</p>}
              </div>

              <div className="grid md:grid-cols-2 gap-3">
                <div>
                  <Label htmlFor="snapshot_type">Snapshot Type</Label>
                  <select id="snapshot_type" {...register("snapshot_type")} className="w-full border rounded-md h-9 px-2">
                    <option value="Full">Full</option>
                    <option value="Diff">Diff</option>
                  </select>
                </div>
                <div>
                  <Label htmlFor="track_dirty_pages">Track Dirty Pages (Diff only)</Label>
                  <select
                    id="track_dirty_pages"
                    {...register("track_dirty_pages")}
                    className="w-full border rounded-md h-9 px-2"
                    disabled={watch("snapshot_type") !== "Diff"}
                  >
                    <option value="false">Disabled</option>
                    <option value="true">Enabled</option>
                  </select>
                </div>
              <div>
                <Label htmlFor="version">Version (Optional)</Label>
                <Input id="version" {...register("version")} placeholder="1.0.0" />
              </div>
              </div>

              <div className="flex justify-end gap-2">
                <Button type="button" variant="outline" onClick={() => setCreateDialogOpen(false)}>
                  Cancel
                </Button>
                <Button type="submit" disabled={isSubmitting}>
                  {isSubmitting ? "Creating..." : "Create Snapshot"}
                </Button>
              </div>
            </form>
          </DialogContent>
        </Dialog>
      </div>

      {!snapshots?.length ? (
        <Card>
          <CardContent className="flex flex-col items-center justify-center py-12">
            <Camera className="h-12 w-12 text-muted-foreground mb-4" />
            <h3 className="text-lg font-semibold mb-2">No snapshots yet</h3>
            <p className="text-muted-foreground text-center mb-4">
              Create your first snapshot to save the current state of your VM
            </p>
            <Button disabled={!canCreateSnapshot} onClick={() => setCreateDialogOpen(true)} className="gap-2">
              <Plus className="h-4 w-4" />
              Create First Snapshot
            </Button>
          </CardContent>
        </Card>
      ) : (
        <div className="grid gap-4">
          {snapshots.map((snapshot) => (
            <Card key={snapshot.id}>
              <CardHeader className="pb-3">
                <div className="flex items-center justify-between">
                  <div>
                    <CardTitle className="text-base">{snapshot.name || snapshot.id}</CardTitle>
                    <div className="flex items-center gap-2 mt-1">
                      <Badge variant="secondary" className="text-xs">
                        {snapshot.created_at ? new Date(snapshot.created_at).toLocaleString() : "Unknown"}
                      </Badge>
                      <Badge variant="outline" className="text-xs">
                        {typeof snapshot.size_bytes === "number" && Number.isFinite(snapshot.size_bytes)
                          ? (snapshot.size_bytes / 1024 / 1024).toFixed(1)
                          : "0.0"} MB
                      </Badge>
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    <Button
                      size="sm"
                      variant="outline"
                      disabled={!canRestore || restoreSnapshot.isPending}
                      onClick={() => handleRestore(snapshot.id, snapshot.name || snapshot.id)}
                      className="gap-2"
                    >
                      <Download className="h-4 w-4" />
                      Restore
                    </Button>
                    <Button
                      size="sm"
                      variant="outline"
                      disabled={deleteSnapshot.isPending}
                      onClick={() => handleDelete(snapshot.id, snapshot.name)}
                      className="gap-2 text-destructive hover:text-destructive"
                    >
                      <Trash2 className="h-4 w-4" />
                      Delete
                    </Button>
                  </div>
                </div>
              </CardHeader>
              {snapshot.description && (
                <CardContent className="pt-0">
                  <p className="text-sm text-muted-foreground">{snapshot.description}</p>
                </CardContent>
              )}
            </Card>
          ))}
        </div>
      )}
    </div>
  )
}
