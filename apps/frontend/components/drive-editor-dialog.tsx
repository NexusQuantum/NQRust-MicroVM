"use client"

import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import type { DriveConfig, VMState } from "@/types/firecracker"
import { driveConfigSchema } from "@/lib/validators"
import { useCreateDrive, usePatchDriveRateLimit } from "@/lib/queries"
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Switch } from "@/components/ui/switch"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Separator } from "@/components/ui/separator"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { AlertBanner } from "@/components/alert-banner"
import { Save, X } from "lucide-react"
import type { z } from "zod"

interface DriveEditorDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  drive?: DriveConfig | null
  mode: "create" | "edit" | "rate-limit"
  vmState: VMState
}

type DriveForm = z.infer<typeof driveConfigSchema>

export function DriveEditorDialog({ open, onOpenChange, drive, mode, vmState }: DriveEditorDialogProps) {
  const createDrive = useCreateDrive()
  const patchDriveRateLimit = usePatchDriveRateLimit()

  const isRateLimitMode = mode === "rate-limit"
  const canEdit = vmState === "stopped" || isRateLimitMode

  const form = useForm<DriveForm>({
    resolver: zodResolver(driveConfigSchema),
    defaultValues: drive || {
      drive_id: "",
      path_on_host: "",
      is_root_device: false,
      is_read_only: false,
      cache_type: "Unsafe",
      io_engine: "Async",
      rate_limiter: {
        bandwidth: {
          size: 0,
          one_time_burst: 0,
          refill_time: 100,
        },
        ops: {
          size: 0,
          one_time_burst: 0,
          refill_time: 100,
        },
      },
    },
  })

  const onSubmit = (data: DriveForm) => {
    if (isRateLimitMode && drive) {
      // Only submit rate limiter data
      patchDriveRateLimit.mutate(
        {
          id: drive.drive_id,
          config: { rate_limiter: data.rate_limiter },
        },
        {
          onSuccess: () => onOpenChange(false),
        },
      )
    } else {
      // Create or update full drive config
      createDrive.mutate(
        { id: data.drive_id, config: data },
        {
          onSuccess: () => onOpenChange(false),
        },
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

        <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-6">
          {/* Guardrail Alert */}
          {!canEdit && (
            <AlertBanner
              type="warning"
              title="Configuration Locked"
              message="VM must be stopped to modify drive configuration."
            />
          )}

          {isRateLimitMode && (
            <AlertBanner
              type="info"
              title="Runtime Rate Limit Editing"
              message="You can modify rate limiters while the VM is running."
            />
          )}

          {/* Basic Drive Configuration */}
          {!isRateLimitMode && (
            <div className="space-y-4">
              <div className="grid gap-4 md:grid-cols-2">
                <div className="space-y-2">
                  <Label htmlFor="drive_id">Drive ID</Label>
                  <Input id="drive_id" disabled={!canEdit || mode === "edit"} {...form.register("drive_id")} />
                  {form.formState.errors.drive_id && (
                    <p className="text-sm text-destructive">{form.formState.errors.drive_id.message}</p>
                  )}
                </div>

                <div className="space-y-2">
                  <Label htmlFor="path_on_host">Host Path</Label>
                  <Input id="path_on_host" disabled={!canEdit} {...form.register("path_on_host")} />
                  {form.formState.errors.path_on_host && (
                    <p className="text-sm text-destructive">{form.formState.errors.path_on_host.message}</p>
                  )}
                </div>
              </div>

              <div className="grid gap-4 md:grid-cols-2">
                <div className="space-y-2">
                  <Label htmlFor="cache_type">Cache Type</Label>
                  <Select
                    disabled={!canEdit}
                    value={form.watch("cache_type")}
                    onValueChange={(value: "Unsafe" | "Writeback") => form.setValue("cache_type", value)}
                  >
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="Unsafe">Unsafe</SelectItem>
                      <SelectItem value="Writeback">Writeback</SelectItem>
                    </SelectContent>
                  </Select>
                </div>

                <div className="space-y-2">
                  <Label htmlFor="io_engine">I/O Engine</Label>
                  <Select
                    disabled={!canEdit}
                    value={form.watch("io_engine")}
                    onValueChange={(value: "Sync" | "Async") => form.setValue("io_engine", value)}
                  >
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="Sync">Sync</SelectItem>
                      <SelectItem value="Async">Async</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </div>

              <div className="flex items-center justify-between">
                <div className="space-y-1">
                  <Label htmlFor="is_root_device">Root Device</Label>
                  <p className="text-xs text-muted-foreground">Mark this drive as the root filesystem</p>
                </div>
                <Switch
                  id="is_root_device"
                  disabled={!canEdit}
                  checked={form.watch("is_root_device")}
                  onCheckedChange={(checked) => form.setValue("is_root_device", checked)}
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
                  checked={form.watch("is_read_only")}
                  onCheckedChange={(checked) => form.setValue("is_read_only", checked)}
                />
              </div>

              <Separator />
            </div>
          )}

          {/* Rate Limiter Configuration */}
          <Card>
            <CardHeader>
              <CardTitle className="text-lg">Rate Limiters</CardTitle>
            </CardHeader>
            <CardContent className="space-y-6">
              {/* Bandwidth Rate Limiter */}
              <div className="space-y-4">
                <h4 className="font-medium">Bandwidth Limiter</h4>
                <div className="grid gap-4 md:grid-cols-3">
                  <div className="space-y-2">
                    <Label htmlFor="bandwidth_size">Size (bytes/sec)</Label>
                    <Input
                      id="bandwidth_size"
                      type="number"
                      min="0"
                      disabled={!canEdit && !isRateLimitMode}
                      {...form.register("rate_limiter.bandwidth.size", { valueAsNumber: true })}
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="bandwidth_burst">One-time Burst</Label>
                    <Input
                      id="bandwidth_burst"
                      type="number"
                      min="0"
                      disabled={!canEdit && !isRateLimitMode}
                      {...form.register("rate_limiter.bandwidth.one_time_burst", { valueAsNumber: true })}
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="bandwidth_refill">Refill Time (ms)</Label>
                    <Input
                      id="bandwidth_refill"
                      type="number"
                      min="0"
                      disabled={!canEdit && !isRateLimitMode}
                      {...form.register("rate_limiter.bandwidth.refill_time", { valueAsNumber: true })}
                    />
                  </div>
                </div>
              </div>

              <Separator />

              {/* Operations Rate Limiter */}
              <div className="space-y-4">
                <h4 className="font-medium">Operations Limiter</h4>
                <div className="grid gap-4 md:grid-cols-3">
                  <div className="space-y-2">
                    <Label htmlFor="ops_size">Size (ops/sec)</Label>
                    <Input
                      id="ops_size"
                      type="number"
                      min="0"
                      disabled={!canEdit && !isRateLimitMode}
                      {...form.register("rate_limiter.ops.size", { valueAsNumber: true })}
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="ops_burst">One-time Burst</Label>
                    <Input
                      id="ops_burst"
                      type="number"
                      min="0"
                      disabled={!canEdit && !isRateLimitMode}
                      {...form.register("rate_limiter.ops.one_time_burst", { valueAsNumber: true })}
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="ops_refill">Refill Time (ms)</Label>
                    <Input
                      id="ops_refill"
                      type="number"
                      min="0"
                      disabled={!canEdit && !isRateLimitMode}
                      {...form.register("rate_limiter.ops.refill_time", { valueAsNumber: true })}
                    />
                  </div>
                </div>
              </div>
            </CardContent>
          </Card>

          {/* Form Actions */}
          <div className="flex justify-end gap-2">
            <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
              <X className="h-4 w-4 mr-2" />
              Cancel
            </Button>
            <Button
              type="submit"
              disabled={(!canEdit && !isRateLimitMode) || createDrive.isPending || patchDriveRateLimit.isPending}
            >
              <Save className="h-4 w-4 mr-2" />
              {createDrive.isPending || patchDriveRateLimit.isPending ? "Saving..." : "Save"}
            </Button>
          </div>
        </form>
      </DialogContent>
    </Dialog>
  )
}
