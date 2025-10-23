"use client"

import { useState } from "react"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Badge } from "@/components/ui/badge"
import { Plus, Edit, Trash2, HardDrive } from "lucide-react"
import { useVMDrives, useCreateVMDrive, useUpdateVMDrive, useDeleteVMDrive } from "@/lib/queries"
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

export function VMStorage({ vmId }: VMStorageProps) {
  const { data: drives = [], isLoading, error } = useVMDrives(vmId)
  const createDrive = useCreateVMDrive()
  const updateDrive = useUpdateVMDrive()
  const deleteDrive = useDeleteVMDrive()

  const [showAddDialog, setShowAddDialog] = useState(false)
  const [showEditDialog, setShowEditDialog] = useState(false)
  const [showDeleteDialog, setShowDeleteDialog] = useState(false)
  const [selectedDrive, setSelectedDrive] = useState<VmDrive | null>(null)

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
  }

  const handleAdd = () => {
    resetForm()
    setShowAddDialog(true)
  }

  const handleEdit = (drive: VmDrive) => {
    setSelectedDrive(drive)
    setFormData({
      drive_id: drive.drive_id,
      path_on_host: drive.path_on_host || "",
      size_bytes: "",
      is_root_device: drive.is_root_device,
      is_read_only: drive.is_read_only,
    })
    setShowEditDialog(true)
  }

  const handleDelete = (drive: VmDrive) => {
    setSelectedDrive(drive)
    setShowDeleteDialog(true)
  }

  const handleSubmitAdd = () => {
    const payload: any = {
      drive_id: formData.drive_id,
      is_root_device: formData.is_root_device,
      is_read_only: formData.is_read_only,
    }

    // If path is provided, use it; otherwise let backend auto-provision
    if (formData.path_on_host) {
      payload.path_on_host = formData.path_on_host
    } else {
      payload.path_on_host = null
    }

    // If size is provided and no path, use it for auto-provisioning
    if (formData.size_bytes && !formData.path_on_host) {
      payload.size_bytes = parseInt(formData.size_bytes, 10)
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

  const handleSubmitEdit = () => {
    if (!selectedDrive) return

    const payload: any = {}

    if (formData.path_on_host && formData.path_on_host !== selectedDrive.path_on_host) {
      payload.path_on_host = formData.path_on_host
    }

    // Only send if there are changes
    if (Object.keys(payload).length > 0) {
      updateDrive.mutate(
        { vmId, driveId: selectedDrive.drive_id, drive: payload },
        {
          onSuccess: () => {
            setShowEditDialog(false)
            setSelectedDrive(null)
            resetForm()
          },
        }
      )
    } else {
      setShowEditDialog(false)
      setSelectedDrive(null)
      resetForm()
    }
  }

  const handleConfirmDelete = () => {
    if (!selectedDrive) return

    deleteDrive.mutate(
      { vmId, driveId: selectedDrive.drive_id },
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
          <Button onClick={handleAdd}>
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
          ) : drives.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              <HardDrive className="h-12 w-12 mx-auto mb-3 opacity-50" />
              <p>No drives attached to this VM.</p>
              <p className="text-sm mt-1">Click "Add Drive" to attach a new drive.</p>
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Drive ID</TableHead>
                  <TableHead>Path</TableHead>
                  <TableHead>Root Device</TableHead>
                  <TableHead>Read Only</TableHead>
                  <TableHead className="text-right">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {drives.map((drive) => (
                  <TableRow key={drive.drive_id}>
                    <TableCell className="font-mono text-sm">{drive.drive_id}</TableCell>
                    <TableCell className="font-mono text-sm">{drive.path_on_host}</TableCell>
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
                      <div className="flex justify-end gap-2">
                        <Button variant="ghost" size="icon" onClick={() => handleEdit(drive)}>
                          <Edit className="h-4 w-4" />
                        </Button>
                        <Button
                          variant="ghost"
                          size="icon"
                          onClick={() => handleDelete(drive)}
                          disabled={drive.is_root_device}
                        >
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

      {/* Add Drive Dialog */}
      <Dialog open={showAddDialog} onOpenChange={setShowAddDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Add New Drive</DialogTitle>
            <DialogDescription>
              Attach a new drive to this VM. Leave path empty to auto-provision a new volume.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="drive_id">Drive ID *</Label>
              <Input
                id="drive_id"
                placeholder="e.g., vdb, vdc, scratch"
                value={formData.drive_id}
                onChange={(e) => setFormData({ ...formData, drive_id: e.target.value })}
              />
              <p className="text-xs text-muted-foreground">Unique identifier for this drive</p>
            </div>

            <div className="space-y-2">
              <Label htmlFor="path_on_host">Host Path (optional)</Label>
              <Input
                id="path_on_host"
                placeholder="/srv/images/my-disk.img"
                value={formData.path_on_host}
                onChange={(e) => setFormData({ ...formData, path_on_host: e.target.value })}
              />
              <p className="text-xs text-muted-foreground">
                Path to existing image file. Leave empty to auto-provision.
              </p>
            </div>

            {!formData.path_on_host && (
              <div className="space-y-2">
                <Label htmlFor="size_bytes">Size (MB)</Label>
                <Input
                  id="size_bytes"
                  type="number"
                  placeholder="1024"
                  value={formData.size_bytes}
                  onChange={(e) => setFormData({ ...formData, size_bytes: e.target.value })}
                />
                <p className="text-xs text-muted-foreground">
                  Size for auto-provisioned volume in megabytes
                </p>
              </div>
            )}

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
            <Button onClick={handleSubmitAdd} disabled={!formData.drive_id || createDrive.isPending}>
              {createDrive.isPending ? "Adding..." : "Add Drive"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Edit Drive Dialog */}
      <Dialog open={showEditDialog} onOpenChange={setShowEditDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Edit Drive</DialogTitle>
            <DialogDescription>Update drive configuration (limited changes allowed)</DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label>Drive ID</Label>
              <Input value={formData.drive_id} disabled />
              <p className="text-xs text-muted-foreground">Drive ID cannot be changed</p>
            </div>

            <div className="space-y-2">
              <Label htmlFor="edit_path_on_host">Host Path</Label>
              <Input
                id="edit_path_on_host"
                placeholder="/srv/images/my-disk.img"
                value={formData.path_on_host}
                onChange={(e) => setFormData({ ...formData, path_on_host: e.target.value })}
              />
              <p className="text-xs text-muted-foreground">Update the path to the image file</p>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowEditDialog(false)}>
              Cancel
            </Button>
            <Button onClick={handleSubmitEdit} disabled={updateDrive.isPending}>
              {updateDrive.isPending ? "Updating..." : "Update Drive"}
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
        description={`Are you sure you want to delete drive "${selectedDrive?.drive_id}"? This action cannot be undone.`}
        confirmText="Delete"
        isLoading={deleteDrive.isPending}
      />
    </>
  )
}
