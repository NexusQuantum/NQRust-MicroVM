"use client"

import { useState, useMemo } from "react"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Badge } from "@/components/ui/badge"
import { Plus, Trash2, Network, Tag } from "lucide-react"
import { useVMNics, useCreateVMNic, useUpdateVMNic, useDeleteVMNic, useVM, useNetworks } from "@/lib/queries"
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
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { ConfirmDialog } from "@/components/shared/confirm-dialog"
import type { VmNic } from "@/lib/types"

interface VMNetworkProps {
  vmId: string
}

export function VMNetwork({ vmId }: VMNetworkProps) {
  const { data: vm } = useVM(vmId)
  const { data: nics = [], isLoading, error } = useVMNics(vmId)
  const { data: allNetworks = [] } = useNetworks()

  // Filter networks to only those on this VM's host
  const hostNetworks = useMemo(() => {
    if (!vm) return []
    return allNetworks.filter(network => network.host_id === vm.host_id)
  }, [vm, allNetworks])

  // Create a virtual default NIC from VM data only if eth0 doesn't exist in database
  const allNics = useMemo(() => {
    if (!vm) return nics

    // Check if eth0 already exists in database
    const hasEth0 = nics.some(nic => nic.iface_id === "eth0")

    if (hasEth0) {
      // eth0 exists in database, just return database NICs
      return nics
    }

    // No eth0 in database, create virtual default NIC from VM data (legacy VMs)
    const defaultNic: VmNic = {
      id: "default-eth0",
      vm_id: vmId,
      iface_id: "eth0",
      host_dev_name: vm.tap || "N/A",
      guest_mac: undefined,
      created_at: vm.created_at,
      updated_at: vm.updated_at,
    }

    return [defaultNic, ...nics]
  }, [vm, nics, vmId])
  const createNic = useCreateVMNic()
  const updateNic = useUpdateVMNic()
  const deleteNic = useDeleteVMNic()

  const [showAddDialog, setShowAddDialog] = useState(false)
  const [showDeleteDialog, setShowDeleteDialog] = useState(false)
  const [selectedNic, setSelectedNic] = useState<VmNic | null>(null)

  const [formData, setFormData] = useState({
    network_id: "",
  })

  const resetForm = () => {
    setFormData({
      network_id: "",
    })
  }

  // Calculate next sequential interface ID
  const nextInterfaceId = useMemo(() => {
    const maxIndex = allNics
      .map(nic => {
        const match = nic.iface_id.match(/^eth(\d+)$/)
        return match ? parseInt(match[1], 10) : 0
      })
      .reduce((max, num) => Math.max(max, num), 0)
    return `eth${maxIndex + 1}`
  }, [allNics])

  const handleAdd = () => {
    resetForm()
    setShowAddDialog(true)
  }

  // Edit is disabled - NICs are immutable after creation
  // const handleEdit = (nic: VmNic) => {
  //   // Not implemented - use delete + recreate workflow
  // }

  const handleDelete = (nic: VmNic) => {
    setSelectedNic(nic)
    setShowDeleteDialog(true)
  }

  const handleSubmitAdd = () => {
    const payload: any = {
      network_id: formData.network_id,
      // iface_id is not provided - backend will auto-assign next sequential interface
    }

    createNic.mutate(
      { vmId, nic: payload },
      {
        onSuccess: () => {
          setShowAddDialog(false)
          resetForm()
        },
      }
    )
  }

  // Edit functionality removed - NICs are immutable after creation

  const handleConfirmDelete = () => {
    if (!selectedNic) return

    deleteNic.mutate(
      { vmId, nicId: selectedNic.id },
      {
        onSuccess: () => {
          setShowDeleteDialog(false)
          setSelectedNic(null)
        },
      }
    )
  }

  return (
    <>
      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <div className="flex items-center gap-2">
            <Network className="h-5 w-5" />
            <CardTitle>Network Interfaces</CardTitle>
          </div>
          <Button onClick={handleAdd} disabled={vm?.state === 'running'}>
            <Plus className="mr-2 h-4 w-4" />
            Add NIC
          </Button>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="space-y-4">
              {[...Array(1)].map((_, i) => (
                <div key={i} className="flex items-center space-x-4 p-4 border rounded">
                  <Skeleton className="h-4 w-20" />
                  <Skeleton className="h-4 w-32" />
                  <Skeleton className="h-4 w-24" />
                  <Skeleton className="h-8 w-20 ml-auto" />
                </div>
              ))}
            </div>
          ) : error ? (
            <Alert variant="destructive">
              <AlertCircle className="h-4 w-4" />
              <AlertTitle>Error</AlertTitle>
              <AlertDescription>
                Failed to load VM network interfaces. Please try again later.
              </AlertDescription>
            </Alert>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Interface ID</TableHead>
                  <TableHead>Assigned IP</TableHead>
                  <TableHead>Guest MAC</TableHead>
                  <TableHead>Host Device</TableHead>
                  <TableHead className="text-right">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {allNics.map((nic) => (
                  <TableRow key={nic.iface_id}>
                    <TableCell className="font-mono text-sm">
                      {nic.iface_id}
                      {nic.iface_id === "eth0" && (
                        <Badge variant="outline" className="ml-2 bg-purple-100 text-purple-700 border-purple-200">
                          Default
                        </Badge>
                      )}
                    </TableCell>
                    <TableCell className="font-mono text-sm">
                      {nic.assigned_ip ? (
                        <span className="text-teal-600">{nic.assigned_ip}</span>
                      ) : (
                        <span className="text-muted-foreground">DHCP</span>
                      )}
                    </TableCell>
                    <TableCell className="font-mono text-sm">{nic.guest_mac || "Auto"}</TableCell>
                    <TableCell className="font-mono text-sm">{nic.host_dev_name}</TableCell>
                    <TableCell className="text-right">
                      {nic.iface_id === "eth0" ? (
                        <span className="text-xs text-muted-foreground">-</span>
                      ) : (
                        <div className="flex justify-end gap-2">
                          <Button variant="ghost" size="icon" onClick={() => handleDelete(nic)} disabled={vm?.state === 'running'}>
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

      {/* Add NIC Dialog */}
      <Dialog open={showAddDialog} onOpenChange={setShowAddDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Add Network Interface</DialogTitle>
            <DialogDescription>
              Add a new network interface to this VM by selecting an existing network. The interface will be automatically assigned as <strong>{nextInterfaceId}</strong>.
              <br />
              <strong>Note:</strong> The VM must be restarted for this change to take effect (hot-plug is not supported).
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            {hostNetworks.length === 0 ? (
              <Alert>
                <AlertCircle className="h-4 w-4" />
                <AlertDescription>
                  No networks available on this host. Please create a network first on the Networks page.
                </AlertDescription>
              </Alert>
            ) : (
              <>
                <div className="space-y-2">
                  <Label htmlFor="network_id">Network *</Label>
                  <Select
                    value={formData.network_id}
                    onValueChange={(value) => setFormData({ ...formData, network_id: value })}
                  >
                    <SelectTrigger id="network_id">
                      <SelectValue placeholder="Select network" />
                    </SelectTrigger>
                    <SelectContent>
                      {hostNetworks.map((network) => (
                        <SelectItem key={network.id} value={network.id}>
                          <div className="flex items-center gap-2">
                            <span>{network.name}</span>
                            <Badge variant={network.type === "vlan" ? "default" : "secondary"} className="text-xs">
                              {network.type === "vlan" && <Tag className="h-3 w-3 mr-1" />}
                              {network.type.toUpperCase()}
                            </Badge>
                            <code className="text-xs bg-muted px-1 rounded">{network.bridge_name}</code>
                            {network.vlan_id && (
                              <Badge variant="outline" className="text-xs">VLAN {network.vlan_id}</Badge>
                            )}
                            {network.cidr && (
                              <Badge variant="outline" className="text-xs bg-teal-50 text-teal-700 border-teal-200">
                                {network.cidr}
                              </Badge>
                            )}
                          </div>
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                  <p className="text-xs text-muted-foreground">
                    Choose which network to attach this interface to. If the network has a CIDR configured, a static IP will be automatically assigned.
                  </p>
                </div>

                <div className="rounded-lg border border-border bg-muted/50 p-4 space-y-2">
                  <div className="flex items-center justify-between">
                    <Label className="text-sm font-medium">Auto-assigned Interface ID</Label>
                    <code className="text-sm bg-background px-2 py-1 rounded border font-mono">{nextInterfaceId}</code>
                  </div>
                  <p className="text-xs text-muted-foreground">
                    The interface will be automatically assigned as {nextInterfaceId} (sequential numbering)
                  </p>
                </div>
              </>
            )}
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowAddDialog(false)}>
              Cancel
            </Button>
            <Button
              onClick={handleSubmitAdd}
              disabled={!formData.network_id || hostNetworks.length === 0 || createNic.isPending}
            >
              {createNic.isPending ? "Adding..." : `Add ${nextInterfaceId}`}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation */}
      <ConfirmDialog
        open={showDeleteDialog}
        onOpenChange={setShowDeleteDialog}
        onConfirm={handleConfirmDelete}
        title="Delete Network Interface"
        description={`Are you sure you want to delete network interface "${selectedNic?.iface_id}"? This action cannot be undone. The VM must be restarted for this change to take effect.`}
        confirmText="Delete"
        variant="destructive"
        isLoading={deleteNic.isPending}
      />
    </>
  )
}
