"use client"

import { useEffect } from "react"
import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import type { NetworkConfig, VMState } from "@/types/firecracker"
import { networkConfigSchema } from "@/lib/validators"
import { useCreateVMNic, useUpdateVMNic } from "@/lib/queries"
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Switch } from "@/components/ui/switch"
import { Separator } from "@/components/ui/separator"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { AlertBanner } from "@/components/alert-banner"
import { Save, Shuffle, X } from "lucide-react"
import type { z } from "zod"

interface NicEditorDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  networkInterface?: NetworkConfig | null
  mode: "create" | "edit" | "rate-limit"
  vmState: VMState
  vmId: string
  onGenerateMac: () => string
}

type NetworkForm = z.infer<typeof networkConfigSchema>

export function NicEditorDialog({
  open,
  onOpenChange,
  networkInterface,
  mode,
  vmState,
  vmId,
  onGenerateMac,
}: NicEditorDialogProps) {
  const createNic = useCreateVMNic()
  const updateNic = useUpdateVMNic()

  const isRateLimitMode = mode === "rate-limit"
  const canEdit = true // NICs are database-backed, can be modified at any time

  const defaultLimiter = () => ({
    size: 125000000,
    one_time_burst: 125000000,
    refill_time: 1000,
  })

  const baseDefaults: NetworkForm = {
    iface_id: "",
    host_dev_name: "",
    guest_mac: "",
    allow_mmds_requests: true,
    rx_rate_limiter: defaultLimiter(),
    tx_rate_limiter: defaultLimiter(),
  }

  const form = useForm<NetworkForm>({
    resolver: zodResolver(networkConfigSchema),
    defaultValues: baseDefaults,
  })

  useEffect(() => {
    const source = networkInterface ?? {}
    const values: NetworkForm = {
      ...baseDefaults,
      ...source,
      guest_mac: source.guest_mac ?? "",
      rx_rate_limiter: source.rx_rate_limiter ?? defaultLimiter(),
      tx_rate_limiter: source.tx_rate_limiter ?? defaultLimiter(),
    }
    form.reset(values)
  }, [networkInterface, form])

  const onSubmit = (data: NetworkForm) => {
    if (isRateLimitMode && networkInterface) {
      // Update rate limiters
      updateNic.mutate(
        {
          vmId,
          nicId: networkInterface.iface_id,
          nic: {
            rx_rate_limiter: data.rx_rate_limiter,
            tx_rate_limiter: data.tx_rate_limiter,
          },
        },
        {
          onSuccess: () => onOpenChange(false),
        },
      )
    } else {
      // Create new interface
      const cleanData = {
        iface_id: data.iface_id.trim(),
        host_dev_name: data.host_dev_name.trim(),
        guest_mac: data.guest_mac?.trim() ? data.guest_mac.trim() : undefined,
        rx_rate_limiter:
          data.rx_rate_limiter && data.rx_rate_limiter.size > 0 ? data.rx_rate_limiter : undefined,
        tx_rate_limiter:
          data.tx_rate_limiter && data.tx_rate_limiter.size > 0 ? data.tx_rate_limiter : undefined,
      }
      createNic.mutate(
        { vmId, nic: cleanData },
        {
          onSuccess: () => onOpenChange(false),
        },
      )
    }
  }

  const handleGenerateMac = () => {
    const newMac = onGenerateMac()
    form.setValue("guest_mac", newMac)
  }

  const getTitle = () => {
    switch (mode) {
      case "create":
        return "Add Network Interface"
      case "edit":
        return "Edit Network Interface"
      case "rate-limit":
        return "Edit Rate Limits"
      default:
        return "Network Interface Configuration"
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>{getTitle()}</DialogTitle>
        </DialogHeader>

        <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-6">
          {/* Info banner for create mode */}
          {mode === "create" && (
            <AlertBanner
              type="info"
              title="Database-Backed Network Management"
              message="This network interface will be saved to the database immediately and attached to Firecracker on the next VM start or restart."
            />
          )}

          {/* Info banner for edit mode */}
          {mode === "edit" && vmState !== "stopped" && (
            <AlertBanner
              type="info"
              title="Changes Apply on Restart"
              message="Network interface changes are saved to the database immediately but will take effect when the VM is restarted."
            />
          )}

          {/* Info banner for rate-limit mode */}
          {isRateLimitMode && vmState !== "stopped" && (
            <AlertBanner
              type="info"
              title="Rate Limit Changes on Restart"
              message="Rate limiter changes are saved to the database and will take effect when the VM is restarted."
            />
          )}

          {/* Basic Interface Configuration */}
          {!isRateLimitMode && (
            <div className="space-y-4">
              <div className="grid gap-4 md:grid-cols-2">
                <div className="space-y-2">
                  <Label htmlFor="iface_id">Interface ID</Label>
                  <Input
                    id="iface_id"
                    placeholder="eth1"
                    disabled={!canEdit || mode === "edit"}
                    {...form.register("iface_id")}
                  />
                  {form.formState.errors.iface_id && (
                    <p className="text-sm text-destructive">{form.formState.errors.iface_id.message}</p>
                  )}
                  <p className="text-xs text-muted-foreground">Use eth1, eth2, etc. (eth0 is reserved)</p>
                </div>

                <div className="space-y-2">
                  <Label htmlFor="host_dev_name">Host Device Name</Label>
                  <Input
                    id="host_dev_name"
                    placeholder="tap-<vm-id>-eth1"
                    disabled={!canEdit || mode === "edit"}
                    {...form.register("host_dev_name")}
                  />
                  {form.formState.errors.host_dev_name && (
                    <p className="text-sm text-destructive">{form.formState.errors.host_dev_name.message}</p>
                  )}
                  <p className="text-xs text-muted-foreground">
                    Each interface needs its own TAP (15 char max). Defaults to vm tap + interface suffix.
                  </p>
                </div>
              </div>

              <div className="space-y-2">
                <Label htmlFor="guest_mac">Guest MAC Address (Optional)</Label>
                <div className="flex gap-2">
                  <Input
                    id="guest_mac"
                    disabled={!canEdit}
                    placeholder="Auto-generated if empty"
                    {...form.register("guest_mac")}
                  />
                  <Button type="button" variant="outline" size="icon" disabled={!canEdit} onClick={handleGenerateMac}>
                    <Shuffle className="h-4 w-4" />
                    <span className="sr-only">Generate MAC address</span>
                  </Button>
                </div>
                <p className="text-xs text-muted-foreground">
                  Leave empty for auto-generation or specify a custom MAC address
                </p>
              </div>

              <div className="flex items-center justify-between">
                <div className="space-y-1">
                  <Label htmlFor="allow_mmds_requests">Allow MMDS Requests</Label>
                  <p className="text-xs text-muted-foreground">Enable Metadata Service requests from this interface</p>
                </div>
                <Switch
                  id="allow_mmds_requests"
                  disabled={!canEdit}
                  checked={form.watch("allow_mmds_requests")}
                  onCheckedChange={(checked) => form.setValue("allow_mmds_requests", checked)}
                />
              </div>

              <Separator />
            </div>
          )}

          {/* Rate Limiter Configuration */}
          <div className="space-y-6">
            {/* RX Rate Limiter */}
            <Card>
              <CardHeader>
                <CardTitle className="text-lg">RX (Receive) Rate Limiter</CardTitle>
              </CardHeader>
              <CardContent>
                <div className="grid gap-4 md:grid-cols-3">
                  <div className="space-y-2">
                    <Label htmlFor="rx_size">Size (bytes/sec)</Label>
                    <Input
                      id="rx_size"
                      type="number"
                      min="0"
                      disabled={!canEdit && !isRateLimitMode}
                      {...form.register("rx_rate_limiter.size", { valueAsNumber: true })}
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="rx_burst">One-time Burst</Label>
                    <Input
                      id="rx_burst"
                      type="number"
                      min="0"
                      disabled={!canEdit && !isRateLimitMode}
                      {...form.register("rx_rate_limiter.one_time_burst", { valueAsNumber: true })}
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="rx_refill">Refill Time (ms)</Label>
                    <Input
                      id="rx_refill"
                      type="number"
                      min="0"
                      disabled={!canEdit && !isRateLimitMode}
                      {...form.register("rx_rate_limiter.refill_time", { valueAsNumber: true })}
                    />
                  </div>
                </div>
              </CardContent>
            </Card>

            {/* TX Rate Limiter */}
            <Card>
              <CardHeader>
                <CardTitle className="text-lg">TX (Transmit) Rate Limiter</CardTitle>
              </CardHeader>
              <CardContent>
                <div className="grid gap-4 md:grid-cols-3">
                  <div className="space-y-2">
                    <Label htmlFor="tx_size">Size (bytes/sec)</Label>
                    <Input
                      id="tx_size"
                      type="number"
                      min="0"
                      disabled={!canEdit && !isRateLimitMode}
                      {...form.register("tx_rate_limiter.size", { valueAsNumber: true })}
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="tx_burst">One-time Burst</Label>
                    <Input
                      id="tx_burst"
                      type="number"
                      min="0"
                      disabled={!canEdit && !isRateLimitMode}
                      {...form.register("tx_rate_limiter.one_time_burst", { valueAsNumber: true })}
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="tx_refill">Refill Time (ms)</Label>
                    <Input
                      id="tx_refill"
                      type="number"
                      min="0"
                      disabled={!canEdit && !isRateLimitMode}
                      {...form.register("tx_rate_limiter.refill_time", { valueAsNumber: true })}
                    />
                  </div>
                </div>
              </CardContent>
            </Card>
          </div>

          {/* Form Actions */}
          <div className="flex justify-end gap-2">
            <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
              <X className="h-4 w-4 mr-2" />
              Cancel
            </Button>
            <Button
              type="submit"
              disabled={(!canEdit && !isRateLimitMode) || createNic.isPending || updateNic.isPending}
            >
              <Save className="h-4 w-4 mr-2" />
              {createNic.isPending || updateNic.isPending ? "Saving..." : "Save"}
            </Button>
          </div>
        </form>
      </DialogContent>
    </Dialog>
  )
}
