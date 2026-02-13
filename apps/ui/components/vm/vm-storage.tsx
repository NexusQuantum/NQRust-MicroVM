"use client"

import { useState, useMemo } from "react"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Badge } from "@/components/ui/badge"
import { Plus, Trash2, HardDrive } from "lucide-react"
import { useVMDrives, useCreateVMDrive, useDeleteVMDrive, useVM } from "@/lib/queries"
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
import { Checkbox } from "@/components/ui/checkbox"
import { ConfirmDialog } from "@/components/shared/confirm-dialog"
import type { VmDrive } from "@/lib/types"

interface VMStorageProps {
  vmId: string
}

function formatDriveSize(sizeBytes?: number): string {
  if (sizeBytes == null) return "—"
  const sizeGB = sizeBytes / (1024 * 1024 * 1024)
  if (sizeGB >= 1) {
    return `${sizeGB.toFixed(1)} GB`
  }
  const sizeMB = sizeBytes / (1024 * 1024)
  return `${sizeMB.toFixed(0)} MB`
}

export function VMStorage({ vmId }: VMStorageProps) {
  const { data: vm } = useVM(vmId)
  const { data: drives = [], isLoading, error } = useVMDrives(vmId)

  // Create a virtual rootfs drive from VM data
  const allDrives = useMemo(() => {
    if (!vm) return drives

    const rootfsDrive: VmDrive = {
      id: "default-rootfs",
      vm_id: vmId,
      drive_id: "rootfs",
      path_on_host: vm.rootfs_path,
      is_root_device: true,
      is_read_only: false,
      created_at: vm.created_at,
      updated_at: vm.updated_at,
    }

    return [rootfsDrive, ...drives]
  }, [vm, drives, vmId])
  const createDrive = useCreateVMDrive()
  const deleteDrive = useDeleteVMDrive()

  const [showAddDialog, setShowAddDialog] = useState(false)
  const [showDeleteDialog, setShowDeleteDialog] = useState(false)
  const [selectedDrive, setSelectedDrive] = useState<VmDrive | null>(null)
  const [duplicateError, setDuplicateError] = useState(false)

  const [formData, setFormData] = useState({
    drive_id: "",
    path_on_host: "",
    size_bytes: "",
    is_root_device: false,
    is_read_only: false,
  })

  const resetForm = () => {
    setFormData({
      drive_id: "",
      path_on_host: "",
      size_bytes: "",
      is_root_device: false,
      is_read_only: false,
    })
    setDuplicateError(false)
  }

  const handleAdd = () => {
    resetForm()
    setShowAddDialog(true)
  }

  const handleDelete = (drive: VmDrive) => {
    setSelectedDrive(drive)
    setShowDeleteDialog(true)
  }

  const handleSubmitAdd = () => {
    // Check for duplicate drive_id
    if (allDrives.some(d => d.drive_id === formData.drive_id)) {
      setDuplicateError(true)
      return
    }

    const payload: any = {
      drive_id: formData.drive_id,
      is_root_device: formData.is_root_device,
      is_read_only: formData.is_read_only,
      path_on_host: null, // Always auto-provision
    }

    // Always use size for auto-provisioning
    if (formData.size_bytes) {
      // Convert MB to bytes (backend expects bytes)
      payload.size_bytes = parseInt(formData.size_bytes, 10) * 1024 * 1024
    }

    createDrive.mutate(
      { vmId, drive: payload },
      {
        onSuccess: () => {
          setShowAddDialog(false)
          resetForm()
        },
      }
    )
  }

  const handleConfirmDelete = () => {
    if (!selectedDrive) return

    deleteDrive.mutate(
      { vmId, driveId: selectedDrive.id },
      {
        onSuccess: () => {
          setShowDeleteDialog(false)
          setSelectedDrive(null)
        },
      }
    )
  }

  return (
    <>
      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <div className="flex items-center gap-2">
            <HardDrive className="h-5 w-5" />
            <CardTitle>Attached Drives</CardTitle>
          </div>
          <Button onClick={handleAdd} disabled={vm?.state === 'running'}>
            <Plus className="mr-2 h-4 w-4" />
            Add Drive
          </Button>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="space-y-4">
              {[...Array(2)].map((_, i) => (
                <div key={i} className="flex items-center space-x-4 p-4 border rounded">
                  <Skeleton className="h-4 w-20" />
                  <Skeleton className="h-4 w-40" />
                  <Skeleton className="h-4 w-16" />
                  <Skeleton className="h-4 w-16" />
                  <Skeleton className="h-8 w-20 ml-auto" />
                </div>
              ))}
            </div>
          ) : error ? (
            <Alert variant="destructive">
              <AlertCircle className="h-4 w-4" />
              <AlertTitle>Error</AlertTitle>
              <AlertDescription>Failed to load VM drives. Please try again later.</AlertDescription>
            </Alert>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Drive ID</TableHead>
                  <TableHead>Path</TableHead>
                  <TableHead>Size</TableHead>
                  <TableHead>Root Device</TableHead>
                  <TableHead>Read Only</TableHead>
                  <TableHead className="text-right">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {allDrives.map((drive) => (
                  <TableRow key={drive.drive_id}>
                    <TableCell className="font-mono text-sm">
                      {drive.drive_id}
                      {drive.drive_id === "rootfs" && (
                        <Badge variant="outline" className="ml-2 bg-purple-100 text-purple-700 border-purple-200">
                          Default
                        </Badge>
                      )}
                    </TableCell>
                    <TableCell className="font-mono text-sm break-all">{drive.path_on_host}</TableCell>
                    <TableCell className="text-sm">{formatDriveSize(drive.size_bytes)}</TableCell>
                    <TableCell>
                      {drive.is_root_device ? (
                        <Badge variant="outline" className="bg-blue-100 text-blue-700 border-blue-200">
                          Root
                        </Badge>
                      ) : (
                        <span className="text-muted-foreground">No</span>
                      )}
                    </TableCell>
                    <TableCell>{drive.is_read_only ? "Yes" : "No"}</TableCell>
                    <TableCell className="text-right">
                      {drive.drive_id === "rootfs" ? (
                        <span className="text-xs text-muted-foreground">-</span>
                      ) : (
                        <div className="flex justify-end gap-2">
                          <Button
                            variant="ghost"
                            size="icon"
                            onClick={() => handleDelete(drive)}
                            disabled={drive.is_root_device || vm?.state === 'running'}
                          >
                            <Trash2 className="h-4 w-4" />
                          </Button>
                        </div>
                      )}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      {/* Add Drive Dialog */}
      <Dialog open={showAddDialog} onOpenChange={setShowAddDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Add New Drive</DialogTitle>
            <DialogDescription>
              Create a new drive for this VM. A volume will be automatically provisioned.
              <br />
              <strong>Note:</strong> The VM must be restarted for this change to take effect (hot-plug is not supported).
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="drive_id">Drive ID *</Label>
              <Input
                id="drive_id"
                placeholder="e.g., vdb, vdc, scratch"
                value={formData.drive_id}
                onChange={(e) => {
                  setFormData({ ...formData, drive_id: e.target.value })
                  setDuplicateError(false)
                }}
                className={duplicateError ? "border-red-500" : ""}
              />
              {duplicateError && (
                <p className="text-xs text-red-500">A drive with this ID already exists</p>
              )}
              <p className="text-xs text-muted-foreground">Unique identifier for this drive</p>
            </div>

            <div className="space-y-2">
              <Label htmlFor="size_bytes">Size (MB) *</Label>
              <Input
                id="size_bytes"
                type="number"
                placeholder="1024"
                value={formData.size_bytes}
                onChange={(e) => setFormData({ ...formData, size_bytes: e.target.value })}
              />
              <p className="text-xs text-muted-foreground">
                Size for the volume in megabytes
              </p>
            </div>

            <div className="flex items-center space-x-2">
              <Checkbox
                id="is_root_device"
                checked={formData.is_root_device}
                onCheckedChange={(checked) =>
                  setFormData({ ...formData, is_root_device: checked as boolean })
                }
              />
              <Label htmlFor="is_root_device" className="cursor-pointer">
                Root device
              </Label>
            </div>

            <div className="flex items-center space-x-2">
              <Checkbox
                id="is_read_only"
                checked={formData.is_read_only}
                onCheckedChange={(checked) =>
                  setFormData({ ...formData, is_read_only: checked as boolean })
                }
              />
              <Label htmlFor="is_read_only" className="cursor-pointer">
                Read-only
              </Label>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowAddDialog(false)}>
              Cancel
            </Button>
            <Button onClick={handleSubmitAdd} disabled={!formData.drive_id || !formData.size_bytes || parseInt(formData.size_bytes, 10) < 1 || createDrive.isPending}>
              {createDrive.isPending ? "Adding..." : "Add Drive"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation */}
      <ConfirmDialog
        open={showDeleteDialog}
        onOpenChange={setShowDeleteDialog}
        onConfirm={handleConfirmDelete}
        title="Delete Drive"
        description={`Are you sure you want to delete drive "${selectedDrive?.drive_id}"? This action cannot be undone. The VM must be restarted for this change to take effect.`}
        confirmText="Delete"
        isLoading={deleteDrive.isPending}
      />
    </>
  )
}
