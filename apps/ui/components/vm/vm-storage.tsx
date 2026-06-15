"use client"

import { useState, useMemo } from "react"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Badge } from "@/components/ui/badge"
import { Plus, Trash2, HardDrive, Maximize2 } from "lucide-react"
import { useVMDrives, useCreateVMDrive, useDeleteVMDrive, useResizeVMDrive, useVM, useVolumes, useAttachVolume } from "@/lib/queries"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
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
  const resizeDrive = useResizeVMDrive()
  const { data: allVolumes = [] } = useVolumes()
  const attachVolume = useAttachVolume()
  const availableVolumes = useMemo(
    () => (allVolumes as any[]).filter((v) => !v.attached_to_vm_id),
    [allVolumes],
  )

  // QEMU hot-plugs disks live (no restart). Firecracker needs a restart, so its
  // add/remove stays disabled while running.
  const isQemu = vm?.vmm_kind === "qemu"
  const mutationsBlocked = vm?.state === "running" && !isQemu

  const [showAddDialog, setShowAddDialog] = useState(false)
  const [showDeleteDialog, setShowDeleteDialog] = useState(false)
  const [showResizeDialog, setShowResizeDialog] = useState(false)
  const [resizeGb, setResizeGb] = useState("")
  const [showAttachDialog, setShowAttachDialog] = useState(false)
  const [attachVolumeId, setAttachVolumeId] = useState("")
  const [attachDriveId, setAttachDriveId] = useState("")
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

  const handleConfirmAttach = () => {
    if (!attachVolumeId || !attachDriveId) return
    attachVolume.mutate(
      { id: attachVolumeId, params: { vm_id: vmId, drive_id: attachDriveId } },
      {
        onSuccess: () => {
          setShowAttachDialog(false)
          setAttachVolumeId("")
          setAttachDriveId("")
        },
      },
    )
  }

  const handleResize = (drive: VmDrive) => {
    setSelectedDrive(drive)
    const curGb = drive.size_bytes ? Math.ceil(drive.size_bytes / (1024 * 1024 * 1024)) : 10
    setResizeGb(String(curGb))
    setShowResizeDialog(true)
  }

  const handleConfirmResize = () => {
    if (!selectedDrive) return
    const gb = parseInt(resizeGb, 10)
    if (!gb || gb < 1) return
    resizeDrive.mutate(
      { vmId, driveId: selectedDrive.id, sizeBytes: gb * 1024 * 1024 * 1024 },
      {
        onSuccess: () => {
          setShowResizeDialog(false)
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
          <div className="flex gap-2">
            <Button variant="outline" onClick={() => { setAttachVolumeId(""); setAttachDriveId(""); setShowAttachDialog(true) }} disabled={mutationsBlocked}>
              <HardDrive className="mr-2 h-4 w-4" />
              Attach Volume
            </Button>
            <Button onClick={handleAdd} disabled={mutationsBlocked}>
              <Plus className="mr-2 h-4 w-4" />
              Add Drive
            </Button>
          </div>
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
                            title="Resize disk (grow)"
                            onClick={() => handleResize(drive)}
                            disabled={mutationsBlocked}
                          >
                            <Maximize2 className="h-4 w-4" />
                          </Button>
                          <Button
                            variant="ghost"
                            size="icon"
                            title="Detach disk"
                            onClick={() => handleDelete(drive)}
                            disabled={drive.is_root_device || mutationsBlocked}
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
              {isQemu ? (
                <span><strong>Note:</strong> the disk is hot-plugged into the running VM.</span>
              ) : (
                <span><strong>Note:</strong> the VM must be restarted for this change to take effect.</span>
              )}
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
        title="Detach Drive"
        description={`Detach drive "${selectedDrive?.drive_id}"?${isQemu ? " It will be removed from the running VM." : " The VM must be restarted for this to take effect."}`}
        confirmText="Detach"
        isLoading={deleteDrive.isPending}
      />

      {/* Attach Existing Volume Dialog */}
      <Dialog open={showAttachDialog} onOpenChange={setShowAttachDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Attach Existing Volume</DialogTitle>
            <DialogDescription>
              Attach an existing unattached volume to this VM as a data disk.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label>Volume</Label>
              <Select value={attachVolumeId} onValueChange={setAttachVolumeId}>
                <SelectTrigger>
                  <SelectValue placeholder={availableVolumes.length ? "Select a volume" : "No unattached volumes"} />
                </SelectTrigger>
                <SelectContent>
                  {availableVolumes.map((v: any) => (
                    <SelectItem key={v.id} value={v.id}>
                      {v.name} ({formatDriveSize(v.size_bytes)})
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-2">
              <Label htmlFor="attach_drive_id">Drive ID</Label>
              <Input
                id="attach_drive_id"
                placeholder="e.g., vdb"
                value={attachDriveId}
                onChange={(e) => setAttachDriveId(e.target.value)}
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowAttachDialog(false)}>
              Cancel
            </Button>
            <Button onClick={handleConfirmAttach} disabled={!attachVolumeId || !attachDriveId || attachVolume.isPending}>
              {attachVolume.isPending ? "Attaching..." : "Attach"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Resize Dialog */}
      <Dialog open={showResizeDialog} onOpenChange={setShowResizeDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Resize Drive</DialogTitle>
            <DialogDescription>
              Grow drive &quot;{selectedDrive?.drive_id}&quot;. Disks can only be grown, not shrunk.
              {isQemu ? " The new size applies live." : " A restart may be required."}
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-2 py-4">
            <Label htmlFor="resize_gb">New size (GB)</Label>
            <Input
              id="resize_gb"
              type="number"
              min={1}
              value={resizeGb}
              onChange={(e) => setResizeGb(e.target.value)}
            />
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowResizeDialog(false)}>
              Cancel
            </Button>
            <Button onClick={handleConfirmResize} disabled={!resizeGb || parseInt(resizeGb, 10) < 1 || resizeDrive.isPending}>
              {resizeDrive.isPending ? "Resizing..." : "Resize"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  )
}
