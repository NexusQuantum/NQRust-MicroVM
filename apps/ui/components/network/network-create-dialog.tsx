"use client"

import { useState, useEffect } from "react"
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Textarea } from "@/components/ui/textarea"
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { AlertCircle, Info } from "lucide-react"
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip"
import { useCreateNetwork, useHosts, useHost } from "@/lib/queries"

interface NetworkCreateDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function NetworkCreateDialog({ open, onOpenChange }: NetworkCreateDialogProps) {
  const { data: hosts = [] } = useHosts()
  const createNetwork = useCreateNetwork()

  const [formData, setFormData] = useState({
    name: "",
    description: "",
    type: "bridge",
    bridge_name: "",
    vlan_id: "",
    host_id: "",
    cidr: "",
    gateway: "",
  })

  const [errors, setErrors] = useState<Record<string, string>>({})
  const [bridgeAutoFilled, setBridgeAutoFilled] = useState(false)

  // Fetch selected host details for smart defaults
  const { data: selectedHost } = useHost(formData.host_id)

  // Auto-select defaults when dialog opens: host (port 19090) and bridge (fcbr0)
  useEffect(() => {
    if (open && !formData.host_id) {
      if (hosts.length > 0) {
        // Find host with port 19090 in address (manager host)
        const defaultHost = hosts.find(h => h.addr.includes(':19090')) || hosts[0]
        setFormData(prev => ({
          ...prev,
          host_id: defaultHost.id,
          bridge_name: 'fcbr0'  // Always default to fcbr0
        }))
        setBridgeAutoFilled(true)
      }
    }
  }, [open, hosts, formData.host_id])

  const resetForm = () => {
    setFormData({
      name: "",
      description: "",
      type: "bridge",
      bridge_name: "",
      vlan_id: "",
      host_id: "",
      cidr: "",
      gateway: "",
    })
    setErrors({})
    setBridgeAutoFilled(false)
  }

  const validateForm = () => {
    const newErrors: Record<string, string> = {}

    if (!formData.name.trim()) {
      newErrors.name = "Name is required"
    }

    if (!formData.bridge_name.trim()) {
      newErrors.bridge_name = "Bridge name is required"
    }

    if (!formData.host_id) {
      newErrors.host_id = "Host is required"
    }

    if (formData.type === "vlan") {
      const vlanId = parseInt(formData.vlan_id, 10)
      if (!formData.vlan_id || isNaN(vlanId)) {
        newErrors.vlan_id = "VLAN ID is required for VLAN type"
      } else if (vlanId < 1 || vlanId > 4094) {
        newErrors.vlan_id = "VLAN ID must be between 1 and 4094"
      }
    }

    // Validate CIDR format if provided
    if (formData.cidr && !/^(\d{1,3}\.){3}\d{1,3}\/\d{1,2}$/.test(formData.cidr)) {
      newErrors.cidr = "Invalid CIDR format (e.g., 10.100.0.0/24)"
    }

    // Validate gateway IP if provided
    if (formData.gateway && !/^(\d{1,3}\.){3}\d{1,3}$/.test(formData.gateway)) {
      newErrors.gateway = "Invalid IP address format"
    }

    setErrors(newErrors)
    return Object.keys(newErrors).length === 0
  }

  const handleSubmit = () => {
    if (!validateForm()) {
      return
    }

    const payload: any = {
      name: formData.name,
      description: formData.description || null,
      type: formData.type,
      bridge_name: formData.bridge_name,
      host_id: formData.host_id,
      cidr: formData.cidr || null,
      gateway: formData.gateway || null,
    }

    if (formData.type === "vlan") {
      payload.vlan_id = parseInt(formData.vlan_id, 10)
    } else {
      payload.vlan_id = null
    }

    createNetwork.mutate(payload, {
      onSuccess: () => {
        resetForm()
        onOpenChange(false)
      },
    })
  }

  // Common bridge names for dropdown
  const commonBridges = ["fcbr0", "fcbr1", "fcbr2", "virbr0", "br0"]

  return (
    <Dialog open={open} onOpenChange={(open) => {
      onOpenChange(open)
      if (!open) resetForm()
    }}>
      <DialogContent className="min-w-2xl max-w-lg max-h-[90vh] flex flex-col">
        <DialogHeader className="flex-shrink-0">
          <DialogTitle>Create Network</DialogTitle>
          <DialogDescription>
            Create a new network for VM connectivity. Networks can be simple bridges or VLAN-isolated segments.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4 overflow-y-auto flex-1 min-h-0">
          <Alert>
            <AlertCircle className="h-4 w-4" />
            <AlertDescription>
              Networks are host-specific. VMs can only use networks on the same host.
            </AlertDescription>
          </Alert>

          <div className="space-y-2">
            <Label htmlFor="name">Network Name *</Label>
            <Input
              id="name"
              placeholder="e.g., Production Network"
              value={formData.name}
              onChange={(e) => setFormData({ ...formData, name: e.target.value })}
            />
            {errors.name && <p className="text-xs text-destructive">{errors.name}</p>}
          </div>

          <div className="space-y-2">
            <Label htmlFor="description">Description</Label>
            <Textarea
              id="description"
              placeholder="Optional description for this network"
              value={formData.description}
              onChange={(e) => setFormData({ ...formData, description: e.target.value })}
              rows={2}
            />
          </div>

          <div className="space-y-2">
            <Label>Network Type *</Label>
            <RadioGroup
              value={formData.type}
              onValueChange={(value) => setFormData({ ...formData, type: value })}
            >
              <div className="flex items-center space-x-2">
                <RadioGroupItem value="bridge" id="type-bridge" />
                <Label htmlFor="type-bridge" className="font-normal cursor-pointer">
                  Bridge - Standard bridge network
                </Label>
              </div>
              <div className="flex items-center space-x-2">
                <RadioGroupItem value="vlan" id="type-vlan" />
                <Label htmlFor="type-vlan" className="font-normal cursor-pointer">
                  VLAN - Isolated network with VLAN tagging (802.1Q)
                </Label>
              </div>
            </RadioGroup>
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <div className="flex items-center gap-2">
                <Label htmlFor="bridge_name">
                  {formData.type === "vlan" ? "Parent Bridge *" : "Bridge Name *"}
                </Label>
                {formData.type === "vlan" && (
                  <TooltipProvider>
                    <Tooltip>
                      <TooltipTrigger asChild>
                        <Info className="h-4 w-4 text-muted-foreground cursor-help" />
                      </TooltipTrigger>
                      <TooltipContent className="max-w-xs">
                        <p>VLANs require a parent bridge to create isolated network segments. The system will create a VLAN sub-interface (e.g., fcbr0.100) on this bridge.</p>
                      </TooltipContent>
                    </Tooltip>
                  </TooltipProvider>
                )}
              </div>
              <Select
                value={formData.bridge_name}
                onValueChange={(value) => {
                  setFormData({ ...formData, bridge_name: value })
                  setBridgeAutoFilled(false)
                }}
              >
                <SelectTrigger id="bridge_name">
                  <SelectValue placeholder="Select bridge" />
                </SelectTrigger>
                <SelectContent>
                  {commonBridges.map((bridge) => (
                    <SelectItem key={bridge} value={bridge}>
                      {bridge}
                      {bridge === formData.bridge_name && bridgeAutoFilled && (
                        <span className="ml-2 text-xs text-muted-foreground">(default)</span>
                      )}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              {errors.bridge_name && <p className="text-xs text-destructive">{errors.bridge_name}</p>}
              <p className="text-xs text-muted-foreground">
                {bridgeAutoFilled ? (
                  <span className="text-primary">Auto-filled from host default</span>
                ) : (
                  "Linux bridge device (e.g., fcbr0)"
                )}
              </p>
            </div>

            <div className="space-y-2">
              <Label htmlFor="host_id">Host *</Label>
              <Select
                value={formData.host_id}
                onValueChange={(value) => setFormData({ ...formData, host_id: value })}
              >
                <SelectTrigger id="host_id">
                  <SelectValue placeholder="Select host" />
                </SelectTrigger>
                <SelectContent>
                  {hosts.map((host) => (
                    <SelectItem key={host.id} value={host.id}>
                      {host.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              {errors.host_id && <p className="text-xs text-destructive">{errors.host_id}</p>}
              <p className="text-xs text-muted-foreground">
                Which host manages this network
              </p>
            </div>
          </div>

          {formData.type === "vlan" && (
            <div className="space-y-2">
              <Label htmlFor="vlan_id">VLAN ID *</Label>
              <Input
                id="vlan_id"
                type="number"
                min="1"
                max="4094"
                placeholder="e.g., 100"
                value={formData.vlan_id}
                onChange={(e) => setFormData({ ...formData, vlan_id: e.target.value })}
              />
              {errors.vlan_id && <p className="text-xs text-destructive">{errors.vlan_id}</p>}
              <p className="text-xs text-muted-foreground">
                VLAN tag (1-4094) for network isolation
              </p>
            </div>
          )}

          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label htmlFor="cidr">CIDR (optional)</Label>
              <Input
                id="cidr"
                placeholder="e.g., 10.100.0.0/24"
                value={formData.cidr}
                onChange={(e) => setFormData({ ...formData, cidr: e.target.value })}
              />
              {errors.cidr && <p className="text-xs text-destructive">{errors.cidr}</p>}
              <p className="text-xs text-muted-foreground">
                Network address range
              </p>
            </div>

            <div className="space-y-2">
              <Label htmlFor="gateway">Gateway (optional)</Label>
              <Input
                id="gateway"
                placeholder="e.g., 10.100.0.1"
                value={formData.gateway}
                onChange={(e) => setFormData({ ...formData, gateway: e.target.value })}
              />
              {errors.gateway && <p className="text-xs text-destructive">{errors.gateway}</p>}
              <p className="text-xs text-muted-foreground">
                Default gateway IP
              </p>
            </div>
          </div>
        </div>

        <DialogFooter className="flex-shrink-0">
          <Button
            variant="outline"
            onClick={() => {
              resetForm()
              onOpenChange(false)
            }}
          >
            Cancel
          </Button>
          <Button onClick={handleSubmit} disabled={createNetwork.isPending}>
            {createNetwork.isPending ? "Creating..." : "Create Network"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
