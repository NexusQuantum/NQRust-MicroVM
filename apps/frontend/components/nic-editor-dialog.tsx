"use client"

import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import type { NetworkConfig, VMState } from "@/types/firecracker"
import { networkConfigSchema } from "@/lib/validators"
import { useCreateNic, usePatchNicRateLimit } from "@/lib/queries"
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Switch } from "@/components/ui/switch"
import { Separator } from "@/components/ui/separator"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { AlertBanner } from "@/components/alert-banner"
import { Save, X, Shuffle } from "lucide-react"
import type { z } from "zod"

interface NicEditorDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  networkInterface?: NetworkConfig | null
  mode: "create" | "edit" | "rate-limit"
  vmState: VMState
  onGenerateMac: () => string
}

type NetworkForm = z.infer<typeof networkConfigSchema>

export function NicEditorDialog({
  open,
  onOpenChange,
  networkInterface,
  mode,
  vmState,
  onGenerateMac,
}: NicEditorDialogProps) {
  const createNic = useCreateNic()
  const patchNicRateLimit = usePatchNicRateLimit()

  const isRateLimitMode = mode === "rate-limit"
  const canEdit = vmState === "stopped" || isRateLimitMode

  const form = useForm<NetworkForm>({
    resolver: zodResolver(networkConfigSchema),
    defaultValues: networkInterface || {
      iface_id: "",
      host_dev_name: "",
      guest_mac: "",
      allow_mmds_requests: true,
      rx_rate_limiter: {
        size: 0,
        one_time_burst: 0,
        refill_time: 100,
      },
      tx_rate_limiter: {
        size: 0,
        one_time_burst: 0,
        refill_time: 100,
      },
    },
  })

  const onSubmit = (data: NetworkForm) => {
    if (isRateLimitMode && networkInterface) {
      // Only submit rate limiter data
      patchNicRateLimit.mutate(
        {
          id: networkInterface.iface_id,
          config: {
            rx_rate_limiter: data.rx_rate_limiter,
            tx_rate_limiter: data.tx_rate_limiter,
          },
        },
        {
          onSuccess: () => onOpenChange(false),
        },
      )
    } else {
      // Create or update full interface config
      const cleanData = {
        ...data,
        guest_mac: data.guest_mac || undefined,
      }
      createNic.mutate(
        { id: data.iface_id, config: cleanData },
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
          {/* Guardrail Alert */}
          {!canEdit && (
            <AlertBanner
              type="warning"
              title="Configuration Locked"
              message="VM must be stopped to modify network interface configuration."
            />
          )}

          {isRateLimitMode && (
            <AlertBanner
              type="info"
              title="Runtime Rate Limit Editing"
              message="You can modify rate limiters while the VM is running."
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
                    disabled={!canEdit || mode === "edit"}
                    placeholder="eth0"
                    {...form.register("iface_id")}
                  />
                  {form.formState.errors.iface_id && (
                    <p className="text-sm text-destructive">{form.formState.errors.iface_id.message}</p>
                  )}
                </div>

                <div className="space-y-2">
                  <Label htmlFor="host_dev_name">Host Device Name</Label>
                  <Input
                    id="host_dev_name"
                    disabled={!canEdit}
                    placeholder="tap0"
                    {...form.register("host_dev_name")}
                  />
                  {form.formState.errors.host_dev_name && (
                    <p className="text-sm text-destructive">{form.formState.errors.host_dev_name.message}</p>
                  )}
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
              disabled={(!canEdit && !isRateLimitMode) || createNic.isPending || patchNicRateLimit.isPending}
            >
              <Save className="h-4 w-4 mr-2" />
              {createNic.isPending || patchNicRateLimit.isPending ? "Saving..." : "Save"}
            </Button>
          </div>
        </form>
      </DialogContent>
    </Dialog>
  )
}
