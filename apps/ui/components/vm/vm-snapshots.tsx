"use client"

import { useState } from "react"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Badge } from "@/components/ui/badge"
import { Plus, RotateCcw, Trash2, Camera } from "lucide-react"
import { formatRelativeTime } from "@/lib/utils/format"
import { useSnapshots, useCreateSnapshot, useRestoreSnapshot, useDeleteSnapshot } from "@/lib/queries"
import { Skeleton } from "@/components/ui/skeleton"
import { AlertCircle } from "lucide-react"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Label } from "@/components/ui/label"
import { Input } from "@/components/ui/input"
import { ConfirmDialog } from "@/components/shared/confirm-dialog"
import type { Snapshot } from "@/lib/types"

interface VMSnapshotsProps {
  vmId: string
}

export function VMSnapshots({ vmId }: VMSnapshotsProps) {
  const { data: snapshots = [], isLoading, error } = useSnapshots(vmId)
  const createSnapshot = useCreateSnapshot()
  const restoreSnapshot = useRestoreSnapshot()
  const deleteSnapshot = useDeleteSnapshot()

  const [showCreateDialog, setShowCreateDialog] = useState(false)
  const [showRestoreDialog, setShowRestoreDialog] = useState(false)
  const [showDeleteDialog, setShowDeleteDialog] = useState(false)
  const [selectedSnapshot, setSelectedSnapshot] = useState<Snapshot | null>(null)

  const [formData, setFormData] = useState({
    snapshot_path: "",
    mem_file_path: "",
  })

  const resetForm = () => {
    setFormData({
      snapshot_path: "",
      mem_file_path: "",
    })
  }

  const handleCreate = () => {
    resetForm()
    setShowCreateDialog(true)
  }

  const handleRestore = (snapshot: Snapshot) => {
    setSelectedSnapshot(snapshot)
    setShowRestoreDialog(true)
  }

  const handleDelete = (snapshot: Snapshot) => {
    setSelectedSnapshot(snapshot)
    setShowDeleteDialog(true)
  }

  const handleSubmitCreate = () => {
    const payload: any = {
      snapshot_path: formData.snapshot_path || undefined,
      mem_file_path: formData.mem_file_path || undefined,
    }

    createSnapshot.mutate(
      { vmId, ...payload },
      {
        onSuccess: () => {
          setShowCreateDialog(false)
          resetForm()
        },
      }
    )
  }

  const handleConfirmRestore = () => {
    if (!selectedSnapshot) return

    restoreSnapshot.mutate(
      { vmId, snapshotId: selectedSnapshot.id },
      {
        onSuccess: () => {
          setShowRestoreDialog(false)
          setSelectedSnapshot(null)
        },
      }
    )
  }

  const handleConfirmDelete = () => {
    if (!selectedSnapshot) return

    deleteSnapshot.mutate(
      { vmId, snapshotId: selectedSnapshot.id },
      {
        onSuccess: () => {
          setShowDeleteDialog(false)
          setSelectedSnapshot(null)
        },
      }
    )
  }

  const formatBytes = (bytes: number) => {
    const sizes = ["Bytes", "KB", "MB", "GB", "TB"]
    if (bytes === 0) return "0 Bytes"
    const i = Math.floor(Math.log(bytes) / Math.log(1024))
    return Math.round((bytes / Math.pow(1024, i)) * 100) / 100 + " " + sizes[i]
  }

  return (
    <>
      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <div className="flex items-center gap-2">
            <Camera className="h-5 w-5" />
            <CardTitle>Snapshots</CardTitle>
          </div>
          <Button onClick={handleCreate}>
            <Plus className="mr-2 h-4 w-4" />
            Create Snapshot
          </Button>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="space-y-4">
              {[...Array(2)].map((_, i) => (
                <div key={i} className="flex items-center space-x-4 p-4 border rounded">
                  <Skeleton className="h-4 w-24" />
                  <Skeleton className="h-6 w-16" />
                  <Skeleton className="h-4 w-20" />
                  <Skeleton className="h-8 w-24 ml-auto" />
                </div>
              ))}
            </div>
          ) : error ? (
            <Alert variant="destructive">
              <AlertCircle className="h-4 w-4" />
              <AlertTitle>Error</AlertTitle>
              <AlertDescription>Failed to load VM snapshots. Please try again later.</AlertDescription>
            </Alert>
          ) : snapshots.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              <Camera className="h-12 w-12 mx-auto mb-3 opacity-50" />
              <p>No snapshots available for this VM.</p>
              <p className="text-sm mt-1">Click "Create Snapshot" to save the VM state.</p>
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>ID</TableHead>
                  <TableHead>State</TableHead>
                  <TableHead>Size</TableHead>
                  <TableHead>Created</TableHead>
                  <TableHead className="text-right">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {snapshots.map((snapshot) => (
                  <TableRow key={snapshot.id}>
                    <TableCell className="font-mono text-sm">{snapshot.id}</TableCell>
                    <TableCell>
                      <Badge
                        variant="outline"
                        className={
                          snapshot.state === "complete"
                            ? "bg-green-100 text-green-700 border-green-200"
                            : "bg-yellow-100 text-yellow-700 border-yellow-200"
                        }
                      >
                        {snapshot.state}
                      </Badge>
                    </TableCell>
                    <TableCell className="text-sm">{formatBytes(snapshot.size_bytes)}</TableCell>
                    <TableCell className="text-sm text-muted-foreground">
                      {formatRelativeTime(snapshot.created_at)}
                    </TableCell>
                    <TableCell className="text-right">
                      <div className="flex justify-end gap-2">
                        <Button variant="outline" size="sm" onClick={() => handleRestore(snapshot)}>
                          <RotateCcw className="mr-2 h-4 w-4" />
                          Restore
                        </Button>
                        <Button variant="ghost" size="icon" onClick={() => handleDelete(snapshot)}>
                          <Trash2 className="h-4 w-4" />
                        </Button>
                      </div>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      {/* Create Snapshot Dialog */}
      <Dialog open={showCreateDialog} onOpenChange={setShowCreateDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Create Snapshot</DialogTitle>
            <DialogDescription>
              Create a snapshot of the current VM state. The VM should be paused or stopped for best
              results.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <Alert>
              <AlertCircle className="h-4 w-4" />
              <AlertDescription>
                Leave the paths empty to let the backend auto-generate snapshot paths. Manual paths are
                for advanced use only.
              </AlertDescription>
            </Alert>

            <div className="space-y-2">
              <Label htmlFor="snapshot_path">Snapshot Path (optional)</Label>
              <Input
                id="snapshot_path"
                placeholder="Auto-generated if empty"
                value={formData.snapshot_path}
                onChange={(e) => setFormData({ ...formData, snapshot_path: e.target.value })}
              />
              <p className="text-xs text-muted-foreground">
                Path where the snapshot state will be saved
              </p>
            </div>

            <div className="space-y-2">
              <Label htmlFor="mem_file_path">Memory File Path (optional)</Label>
              <Input
                id="mem_file_path"
                placeholder="Auto-generated if empty"
                value={formData.mem_file_path}
                onChange={(e) => setFormData({ ...formData, mem_file_path: e.target.value })}
              />
              <p className="text-xs text-muted-foreground">
                Path where the memory snapshot will be saved
              </p>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowCreateDialog(false)}>
              Cancel
            </Button>
            <Button onClick={handleSubmitCreate} disabled={createSnapshot.isPending}>
              {createSnapshot.isPending ? "Creating..." : "Create Snapshot"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Restore Confirmation Dialog */}
      <Dialog open={showRestoreDialog} onOpenChange={setShowRestoreDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Restore Snapshot</DialogTitle>
            <DialogDescription>
              Are you sure you want to restore this snapshot? This will replace the current VM state.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <Alert>
              <AlertCircle className="h-4 w-4" />
              <AlertDescription>
                The VM must be stopped before restoring a snapshot. Any unsaved changes will be lost.
              </AlertDescription>
            </Alert>

            {selectedSnapshot && (
              <div className="space-y-2 rounded-lg border p-4 bg-muted/50">
                <div className="grid grid-cols-2 gap-2 text-sm">
                  <div className="text-muted-foreground">Snapshot ID:</div>
                  <div className="font-mono">{selectedSnapshot.id}</div>

                  <div className="text-muted-foreground">State:</div>
                  <div>
                    <Badge variant="outline">{selectedSnapshot.state}</Badge>
                  </div>

                  <div className="text-muted-foreground">Size:</div>
                  <div>{formatBytes(selectedSnapshot.size_bytes)}</div>

                  <div className="text-muted-foreground">Created:</div>
                  <div>{formatRelativeTime(selectedSnapshot.created_at)}</div>
                </div>
              </div>
            )}
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowRestoreDialog(false)}>
              Cancel
            </Button>
            <Button onClick={handleConfirmRestore} disabled={restoreSnapshot.isPending}>
              {restoreSnapshot.isPending ? "Restoring..." : "Restore Snapshot"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation */}
      <ConfirmDialog
        open={showDeleteDialog}
        onOpenChange={setShowDeleteDialog}
        onConfirm={handleConfirmDelete}
        title="Delete Snapshot"
        description={`Are you sure you want to delete snapshot "${selectedSnapshot?.id}"? This action cannot be undone and the snapshot data will be permanently removed.`}
        confirmText="Delete"
        variant="destructive"
        isLoading={deleteSnapshot.isPending}
      />
    </>
  )
}
