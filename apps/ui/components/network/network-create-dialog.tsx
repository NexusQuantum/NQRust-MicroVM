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
import { Switch } from "@/components/ui/switch"
import { Globe, Lock, Network, Layers, Loader2, AlertTriangle } from "lucide-react"
import { useCreateNetwork, useHosts, useNetworkSuggestion, useNetworkInterfaces } from "@/lib/queries"
import { toast } from "sonner"

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
    type: "nat" as "nat" | "isolated" | "bridged" | "vxlan",
    host_id: "",
    cidr: "",
    vlan_id: "",
    dhcp_enabled: true,
    dhcp_range_start: "",
    dhcp_range_end: "",
    uplink_interface: "",
    gateway_host_id: "",
  })

  const [errors, setErrors] = useState<Record<string, string>>({})

  // Fetch suggestion for the selected host (for NAT/isolated/vxlan — not bridged)
  const suggestionHostId = formData.type === "vxlan" ? formData.gateway_host_id : formData.host_id
  const { data: suggestion, isLoading: suggestionLoading } = useNetworkSuggestion(
    formData.type !== "bridged" ? suggestionHostId : ""
  )

  // Fetch available interfaces for bridged mode
  const { data: interfacesData, isLoading: interfacesLoading } = useNetworkInterfaces(
    formData.type === "bridged" ? formData.host_id : ""
  )

  // Auto-select host when dialog opens
  useEffect(() => {
    if (open && !formData.host_id && hosts.length > 0) {
      const defaultHost = hosts.find(h => h.addr.includes(':19090')) || hosts[0]
      setFormData(prev => ({ ...prev, host_id: defaultHost.id }))
    }
  }, [open, hosts, formData.host_id])

  // Auto-fill CIDR and DHCP range from suggestion when host changes
  useEffect(() => {
    if (suggestion) {
      setFormData(prev => ({
        ...prev,
        cidr: prev.cidr || suggestion.cidr,
        dhcp_range_start: prev.dhcp_range_start || suggestion.dhcp_range_start,
        dhcp_range_end: prev.dhcp_range_end || suggestion.dhcp_range_end,
      }))
    }
  }, [suggestion])

  // Close dialog and reset form after successful creation
  useEffect(() => {
    if (createNetwork.isSuccess) {
      resetForm()
      onOpenChange(false)
      createNetwork.reset()
    }
  }, [createNetwork.isSuccess])

  const resetForm = () => {
    setFormData({
      name: "",
      description: "",
      type: "nat",
      host_id: "",
      cidr: "",
      vlan_id: "",
      dhcp_enabled: true,
      dhcp_range_start: "",
      dhcp_range_end: "",
      uplink_interface: "",
      gateway_host_id: "",
    })
    setErrors({})
  }

  const validateForm = () => {
    const newErrors: Record<string, string> = {}

    if (!formData.name.trim()) {
      newErrors.name = "Name is required"
    }

    if (!formData.host_id) {
      newErrors.host_id = "Host is required"
    }

    // Bridged: require uplink interface
    if (formData.type === "bridged" && !formData.uplink_interface) {
      newErrors.uplink_interface = "Select a network interface to bridge"
    }

    // VXLAN: require gateway host
    if (formData.type === "vxlan" && !formData.gateway_host_id) {
      newErrors.gateway_host_id = "Gateway host is required for VXLAN networks"
    }

    // Validate CIDR format if provided (not required for bridged)
    if (formData.cidr && !/^(\d{1,3}\.){3}\d{1,3}\/\d{1,2}$/.test(formData.cidr)) {
      newErrors.cidr = "Invalid CIDR format (e.g., 10.0.2.0/24)"
    }

    // Validate DHCP range if DHCP enabled and custom range provided
    if (formData.dhcp_enabled) {
      const ipPattern = /^(\d{1,3}\.){3}\d{1,3}$/
      if (formData.dhcp_range_start && !ipPattern.test(formData.dhcp_range_start)) {
        newErrors.dhcp_range_start = "Invalid IP address"
      }
      if (formData.dhcp_range_end && !ipPattern.test(formData.dhcp_range_end)) {
        newErrors.dhcp_range_end = "Invalid IP address"
      }
    }

    // Validate VLAN ID if provided
    if (formData.vlan_id) {
      const vlanNum = parseInt(formData.vlan_id, 10)
      if (isNaN(vlanNum) || vlanNum < 1 || vlanNum > 4094) {
        newErrors.vlan_id = "VLAN ID must be between 1 and 4094"
      }
    }

    setErrors(newErrors)
    return Object.keys(newErrors).length === 0
  }

  const selectedInterface = interfacesData?.interfaces.find(i => i.name === formData.uplink_interface)

  const handleSubmit = () => {
    if (!validateForm()) return

    const isBridged = formData.type === "bridged"
    const isVxlan = formData.type === "vxlan"
    const hasDhcp = !isBridged && formData.dhcp_enabled

    createNetwork.mutate({
      name: formData.name,
      description: formData.description || undefined,
      type: formData.type,
      host_id: isVxlan ? formData.gateway_host_id : formData.host_id,
      cidr: !isBridged && formData.cidr ? formData.cidr : undefined,
      vlan_id: formData.vlan_id ? parseInt(formData.vlan_id, 10) : undefined,
      dhcp_enabled: isBridged ? false : formData.dhcp_enabled,
      dhcp_range_start: hasDhcp && formData.dhcp_range_start ? formData.dhcp_range_start : undefined,
      dhcp_range_end: hasDhcp && formData.dhcp_range_end ? formData.dhcp_range_end : undefined,
      uplink_interface: isBridged ? formData.uplink_interface : undefined,
      gateway_host_id: isVxlan ? formData.gateway_host_id : undefined,
    })
  }

  return (
    <Dialog open={open} onOpenChange={(open) => {
      onOpenChange(open)
      if (!open) resetForm()
    }}>
      <DialogContent className="min-w-2xl max-w-lg max-h-[90vh] flex flex-col">
        <DialogHeader className="flex-shrink-0">
          <DialogTitle>Create Network</DialogTitle>
          <DialogDescription>
            Create a new virtual network. The system will provision the bridge, DHCP, and firewall rules on the host.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4 overflow-y-auto flex-1 min-h-0">
          <div className="space-y-2">
            <Label htmlFor="name">Network Name *</Label>
            <Input
              id="name"
              placeholder="e.g., Dev Network"
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
              onValueChange={(value: "nat" | "isolated" | "bridged" | "vxlan") => setFormData({ ...formData, type: value, uplink_interface: "", gateway_host_id: "" })}
            >
              <div className="flex items-start space-x-3 p-3 rounded-md border cursor-pointer hover:bg-muted/50"
                onClick={() => setFormData({ ...formData, type: "nat", uplink_interface: "" })}>
                <RadioGroupItem value="nat" id="type-nat" className="mt-0.5" />
                <div className="space-y-1">
                  <Label htmlFor="type-nat" className="font-medium cursor-pointer flex items-center gap-2">
                    <Globe className="h-4 w-4" />
                    NAT
                  </Label>
                  <p className="text-xs text-muted-foreground">
                    Private subnet with internet access via host NAT. VMs get DHCP addresses and can reach the internet through the host.
                  </p>
                </div>
              </div>
              <div className="flex items-start space-x-3 p-3 rounded-md border cursor-pointer hover:bg-muted/50"
                onClick={() => setFormData({ ...formData, type: "isolated", uplink_interface: "" })}>
                <RadioGroupItem value="isolated" id="type-isolated" className="mt-0.5" />
                <div className="space-y-1">
                  <Label htmlFor="type-isolated" className="font-medium cursor-pointer flex items-center gap-2">
                    <Lock className="h-4 w-4" />
                    Isolated
                  </Label>
                  <p className="text-xs text-muted-foreground">
                    Private subnet with no internet access. VMs can only communicate with each other. Ideal for air-gapped workloads.
                  </p>
                </div>
              </div>
              <div className="flex items-start space-x-3 p-3 rounded-md border cursor-pointer hover:bg-muted/50"
                onClick={() => setFormData({ ...formData, type: "bridged", uplink_interface: "", gateway_host_id: "" })}>
                <RadioGroupItem value="bridged" id="type-bridged" className="mt-0.5" />
                <div className="space-y-1">
                  <Label htmlFor="type-bridged" className="font-medium cursor-pointer flex items-center gap-2">
                    <Network className="h-4 w-4" />
                    Bridged
                  </Label>
                  <p className="text-xs text-muted-foreground">
                    Direct LAN access. A physical NIC is attached to a bridge, giving VMs addresses on your external network.
                  </p>
                </div>
              </div>
              <div className="flex items-start space-x-3 p-3 rounded-md border cursor-pointer hover:bg-muted/50"
                onClick={() => setFormData({ ...formData, type: "vxlan", uplink_interface: "", gateway_host_id: "" })}>
                <RadioGroupItem value="vxlan" id="type-vxlan" className="mt-0.5" />
                <div className="space-y-1">
                  <Label htmlFor="type-vxlan" className="font-medium cursor-pointer flex items-center gap-2">
                    <Layers className="h-4 w-4" />
                    VXLAN (Overlay)
                  </Label>
                  <p className="text-xs text-muted-foreground">
                    Multi-host overlay network. VMs on different hosts communicate via VXLAN tunnels. Select a gateway host for DHCP and internet access.
                  </p>
                </div>
              </div>
            </RadioGroup>
          </div>

          {formData.type !== "vxlan" && (
            <div className="space-y-2">
              <Label htmlFor="host_id">Host *</Label>
              <Select
                value={formData.host_id}
                onValueChange={(value) => {
                  setFormData({ ...formData, host_id: value, cidr: "", dhcp_range_start: "", dhcp_range_end: "", uplink_interface: "" })
                }}
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
            </div>
          )}

          {formData.type === "vxlan" && (
            <div className="space-y-2">
              <Label htmlFor="gateway_host_id">Gateway Host *</Label>
              <Select
                value={formData.gateway_host_id}
                onValueChange={(value) => {
                  setFormData({ ...formData, gateway_host_id: value, cidr: "", dhcp_range_start: "", dhcp_range_end: "" })
                }}
              >
                <SelectTrigger id="gateway_host_id">
                  <SelectValue placeholder="Select gateway host" />
                </SelectTrigger>
                <SelectContent>
                  {hosts.map((host) => (
                    <SelectItem key={host.id} value={host.id}>
                      {host.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              {errors.gateway_host_id && <p className="text-xs text-destructive">{errors.gateway_host_id}</p>}
              <p className="text-xs text-muted-foreground">
                The gateway host runs DHCP and NAT for the overlay. VNI will be auto-assigned. The overlay auto-expands to other hosts when VMs are created.
              </p>
            </div>
          )}

          {formData.host_id && formData.type === "bridged" && (
            <div className="space-y-3">
              <div className="space-y-2">
                <Label htmlFor="uplink_interface">Network Interface *</Label>
                <Select
                  value={formData.uplink_interface}
                  onValueChange={(value) => setFormData({ ...formData, uplink_interface: value })}
                >
                  <SelectTrigger id="uplink_interface">
                    <SelectValue placeholder={interfacesLoading ? "Loading interfaces..." : "Select a physical NIC"} />
                  </SelectTrigger>
                  <SelectContent>
                    {interfacesData?.interfaces.map((iface) => (
                      <SelectItem key={iface.name} value={iface.name}>
                        <span className="flex items-center gap-2">
                          {iface.name}
                          <span className="text-muted-foreground text-xs">
                            {iface.addresses.length > 0 ? iface.addresses[0] : iface.mac}
                          </span>
                          {iface.is_management && (
                            <span className="text-xs text-amber-500 font-medium">mgmt</span>
                          )}
                        </span>
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                {errors.uplink_interface && <p className="text-xs text-destructive">{errors.uplink_interface}</p>}
              </div>

              {selectedInterface?.is_management && (
                <Alert variant="destructive">
                  <AlertTriangle className="h-4 w-4" />
                  <AlertDescription className="text-xs">
                    <strong>{selectedInterface.name}</strong> is the management interface (default route). Bridging it will disrupt host connectivity and you may lose access to this server. Choose a different NIC unless you know what you are doing.
                  </AlertDescription>
                </Alert>
              )}

              <div className="space-y-2">
                <Label htmlFor="vlan_id">VLAN ID</Label>
                <Input
                  id="vlan_id"
                  type="number"
                  min={1}
                  max={4094}
                  placeholder="e.g., 100"
                  value={formData.vlan_id}
                  onChange={(e) => setFormData({ ...formData, vlan_id: e.target.value })}
                />
                {errors.vlan_id && <p className="text-xs text-destructive">{errors.vlan_id}</p>}
                <p className="text-xs text-muted-foreground">
                  Optional. Tag traffic with an 802.1Q VLAN ID. Requires the host uplink to be on a trunk port.
                </p>
              </div>

              <p className="text-xs text-muted-foreground">
                The external network handles IP assignment. No CIDR, gateway, or DHCP is configured by the platform for bridged networks.
              </p>
            </div>
          )}

          {((formData.type === "vxlan" ? formData.gateway_host_id : formData.host_id) && formData.type !== "bridged") && (
            <div className="space-y-3">
              <div className="space-y-2">
                <Label htmlFor="cidr">Subnet CIDR</Label>
                <Input
                  id="cidr"
                  placeholder={suggestionLoading ? "Loading..." : "e.g., 10.0.2.0/24"}
                  value={formData.cidr}
                  onChange={(e) => setFormData({ ...formData, cidr: e.target.value })}
                />
                {errors.cidr && <p className="text-xs text-destructive">{errors.cidr}</p>}
                <p className="text-xs text-muted-foreground">
                  Leave empty to use the suggested subnet. The system auto-assigns an available range.
                </p>
              </div>

              {formData.type !== "vxlan" && (
                <div className="space-y-2">
                  <Label htmlFor="vlan_id">VLAN ID</Label>
                  <Input
                    id="vlan_id"
                    type="number"
                    min={1}
                    max={4094}
                    placeholder="e.g., 100"
                    value={formData.vlan_id}
                    onChange={(e) => setFormData({ ...formData, vlan_id: e.target.value })}
                  />
                  {errors.vlan_id && <p className="text-xs text-destructive">{errors.vlan_id}</p>}
                  <p className="text-xs text-muted-foreground">
                    Optional. Tag traffic with an 802.1Q VLAN ID. Requires the host uplink to be on a trunk port.
                  </p>
                </div>
              )}

              <div className="space-y-3">
                <div className="flex items-center justify-between">
                  <div className="space-y-0.5">
                    <Label htmlFor="dhcp_enabled">DHCP Server</Label>
                    <p className="text-xs text-muted-foreground">
                      Automatically assign IP addresses to VMs on this network.
                    </p>
                  </div>
                  <Switch
                    id="dhcp_enabled"
                    checked={formData.dhcp_enabled}
                    onCheckedChange={(checked) => setFormData({ ...formData, dhcp_enabled: checked })}
                  />
                </div>
                {formData.dhcp_enabled && (
                  <div className="grid grid-cols-2 gap-3">
                    <div className="space-y-1">
                      <Label htmlFor="dhcp_start" className="text-xs">Range Start</Label>
                      <Input
                        id="dhcp_start"
                        placeholder={suggestionLoading ? "..." : "e.g., 10.0.2.10"}
                        value={formData.dhcp_range_start}
                        onChange={(e) => setFormData({ ...formData, dhcp_range_start: e.target.value })}
                      />
                      {errors.dhcp_range_start && <p className="text-xs text-destructive">{errors.dhcp_range_start}</p>}
                    </div>
                    <div className="space-y-1">
                      <Label htmlFor="dhcp_end" className="text-xs">Range End</Label>
                      <Input
                        id="dhcp_end"
                        placeholder={suggestionLoading ? "..." : "e.g., 10.0.2.250"}
                        value={formData.dhcp_range_end}
                        onChange={(e) => setFormData({ ...formData, dhcp_range_end: e.target.value })}
                      />
                      {errors.dhcp_range_end && <p className="text-xs text-destructive">{errors.dhcp_range_end}</p>}
                    </div>
                  </div>
                )}
              </div>

              {suggestion && (
                <Alert>
                  {suggestionLoading ? (
                    <Loader2 className="h-4 w-4 animate-spin" />
                  ) : null}
                  <AlertDescription className="text-xs space-y-1">
                    <div className="font-medium mb-1">Auto-assigned configuration:</div>
                    <div className="grid grid-cols-2 gap-x-4 gap-y-0.5">
                      <span className="text-muted-foreground">Bridge:</span>
                      <code className="text-xs">{suggestion.bridge_name}</code>
                      <span className="text-muted-foreground">Subnet:</span>
                      <code className="text-xs">{suggestion.cidr}</code>
                      <span className="text-muted-foreground">Gateway:</span>
                      <code className="text-xs">{suggestion.gateway}</code>
                      <span className="text-muted-foreground">DHCP Range:</span>
                      <code className="text-xs">{suggestion.dhcp_range_start} - {suggestion.dhcp_range_end}</code>
                    </div>
                  </AlertDescription>
                </Alert>
              )}
            </div>
          )}
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
