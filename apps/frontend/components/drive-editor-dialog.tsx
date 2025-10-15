"use client"

import { useState } from "react"
import { useForm } from "react-hook-form"
import type { VmDrive, CreateDriveReq } from "@/types/nexus"
import { useCreateVMDrive, useUpdateVMDrive } from "@/lib/queries"
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Switch } from "@/components/ui/switch"
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group"
import { Separator } from "@/components/ui/separator"
import { AlertBanner } from "@/components/alert-banner"
import { Save, X, Info } from "lucide-react"

interface DriveEditorDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  drive?: VmDrive | null
  mode: "create" | "edit" | "rate-limit"
  vmState: string
  vmId?: string  // Required for creating drives
}

interface DriveFormData {
  drive_id: string
  path_on_host?: string
  size_bytes?: number
  is_root_device: boolean
  is_read_only: boolean
}

export function DriveEditorDialog({ open, onOpenChange, drive, mode, vmState, vmId }: DriveEditorDialogProps) {
  const createDrive = useCreateVMDrive()
  const updateDrive = useUpdateVMDrive()

  const [pathMode, setPathMode] = useState<"auto" | "manual">("auto")

  const isRateLimitMode = mode === "rate-limit"
  const canEdit = true // Drives are database-backed, can be modified at any time

  const { register, handleSubmit, watch, formState: { errors } } = useForm<DriveFormData>({
    defaultValues: drive ? {
      drive_id: drive.drive_id,
      path_on_host: drive.path_on_host,
      is_root_device: drive.is_root_device,
      is_read_only: drive.is_read_only,
    } : {
      drive_id: "",
      path_on_host: "",
      size_bytes: 10 * 1024 * 1024 * 1024, // 10GB default
      is_root_device: false,
      is_read_only: false,
    },
  })

  const onSubmit = (data: DriveFormData) => {
    if (isRateLimitMode && drive) {
      // Rate limit mode not fully implemented yet
      return
    }

    // Build the drive request - use explicit object building to control what's sent
    const driveReq: Record<string, any> = {}

    // Always include these
    driveReq.drive_id = data.drive_id
    driveReq.is_root_device = data.is_root_device ?? false
    driveReq.is_read_only = data.is_read_only ?? false

    // Add path or size based on mode
    if (pathMode === "manual") {
      // Manual mode: must provide a path
      if (data.path_on_host && data.path_on_host.trim() !== "") {
        driveReq.path_on_host = data.path_on_host.trim()
      } else {
        // If in manual mode but no path, that's an error
        alert("Please provide a path for manual mode")
        return
      }
    } else {
      // Auto-provision mode: provide size_bytes, DO NOT send path_on_host at all
      driveReq.size_bytes = data.size_bytes || 10737418240 // Default 10GB
    }

    console.log("Submitting drive request:", JSON.stringify(driveReq, null, 2))

    if (mode === "create" && vmId) {
      createDrive.mutate(
        { vmId, drive: driveReq },
        { onSuccess: () => onOpenChange(false) }
      )
    } else if (mode === "edit" && drive) {
      updateDrive.mutate(
        { vmId: drive.vm_id, driveId: drive.id, drive: { path_on_host: data.path_on_host } },
        { onSuccess: () => onOpenChange(false) }
      )
    }
  }

  const getTitle = () => {
    switch (mode) {
      case "create":
        return "Add Storage Drive"
      case "edit":
        return "Edit Storage Drive"
      case "rate-limit":
        return "Edit Rate Limits"
      default:
        return "Drive Configuration"
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>{getTitle()}</DialogTitle>
        </DialogHeader>

        <form onSubmit={handleSubmit(onSubmit)} className="space-y-6">
          {/* Info banner for create mode */}
          {mode === "create" && (
            <AlertBanner
              type="info"
              title="Database-Backed Drive Management"
              message="This drive will be saved to the database immediately and attached to Firecracker on the next VM start or restart. Changes take effect after restart."
              icon={Info}
            />
          )}

          {/* Info banner for edit mode */}
          {mode === "edit" && vmState !== "stopped" && (
            <AlertBanner
              type="info"
              title="Changes Apply on Restart"
              message="Drive changes are saved to the database immediately but will take effect when the VM is restarted."
              icon={Info}
            />
          )}

          {/* Basic Drive Configuration */}
          {!isRateLimitMode && (
            <div className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="drive_id">Drive ID</Label>
                <Input
                  id="drive_id"
                  disabled={!canEdit || mode === "edit"}
                  {...register("drive_id", { required: "Drive ID is required" })}
                />
                {errors.drive_id && (
                  <p className="text-sm text-destructive">{errors.drive_id.message}</p>
                )}
                <p className="text-xs text-muted-foreground">
                  Unique identifier for this drive (e.g., "data", "scratch")
                </p>
              </div>

              {mode === "create" && (
                <>
                  <Separator />

                  <div className="space-y-3">
                    <Label>Storage Mode</Label>
                    <RadioGroup value={pathMode} onValueChange={(v) => setPathMode(v as "auto" | "manual")}>
                      <div className="flex items-center space-x-2">
                        <RadioGroupItem value="auto" id="auto" />
                        <Label htmlFor="auto" className="font-normal cursor-pointer">
                          Auto-provision (Recommended) - Manager creates and manages storage automatically
                        </Label>
                      </div>
                      <div className="flex items-center space-x-2">
                        <RadioGroupItem value="manual" id="manual" />
                        <Label htmlFor="manual" className="font-normal cursor-pointer">
                          Manual path - Specify an existing file path
                        </Label>
                      </div>
                    </RadioGroup>
                  </div>

                  {pathMode === "auto" ? (
                    <div className="space-y-2">
                      <Label htmlFor="size_bytes">Disk Size (bytes)</Label>
                      <Input
                        id="size_bytes"
                        type="number"
                        disabled={!canEdit}
                        {...register("size_bytes", { valueAsNumber: true })}
                      />
                      <p className="text-xs text-muted-foreground">
                        Hint: 10737418240 = 10GB, 107374182400 = 100GB
                      </p>
                    </div>
                  ) : (
                    <div className="space-y-2">
                      <Label htmlFor="path_on_host">Host Path</Label>
                      <Input
                        id="path_on_host"
                        disabled={!canEdit}
                        {...register("path_on_host")}
                        placeholder="/path/to/disk.img"
                      />
                      <p className="text-xs text-muted-foreground">
                        Full path to an existing disk image file
                      </p>
                    </div>
                  )}
                </>
              )}

              <Separator />

              <div className="flex items-center justify-between">
                <div className="space-y-1">
                  <Label htmlFor="is_root_device">Root Device</Label>
                  <p className="text-xs text-muted-foreground">Mark this drive as the root filesystem</p>
                </div>
                <Switch
                  id="is_root_device"
                  disabled={!canEdit}
                  {...register("is_root_device")}
                />
              </div>

              <div className="flex items-center justify-between">
                <div className="space-y-1">
                  <Label htmlFor="is_read_only">Read Only</Label>
                  <p className="text-xs text-muted-foreground">Mount the drive in read-only mode</p>
                </div>
                <Switch
                  id="is_read_only"
                  disabled={!canEdit}
                  {...register("is_read_only")}
                />
              </div>
            </div>
          )}

          {/* Form Actions */}
          <div className="flex justify-end gap-2">
            <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
              <X className="h-4 w-4 mr-2" />
              Cancel
            </Button>
            <Button
              type="submit"
              disabled={(!canEdit && !isRateLimitMode) || createDrive.isPending || updateDrive.isPending}
            >
              <Save className="h-4 w-4 mr-2" />
              {createDrive.isPending || updateDrive.isPending ? "Saving..." : "Save"}
            </Button>
          </div>
        </form>
      </DialogContent>
    </Dialog>
  )
}
