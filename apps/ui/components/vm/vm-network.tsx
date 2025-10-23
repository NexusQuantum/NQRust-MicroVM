"use client"

import { useState } from "react"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Plus, Edit, Trash2, Network } from "lucide-react"
import { useVMNics, useCreateVMNic, useUpdateVMNic, useDeleteVMNic } from "@/lib/queries"
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
import { ConfirmDialog } from "@/components/shared/confirm-dialog"
import type { VmNic } from "@/lib/types"

interface VMNetworkProps {
  vmId: string
}

export function VMNetwork({ vmId }: VMNetworkProps) {
  const { data: nics = [], isLoading, error } = useVMNics(vmId)
  const createNic = useCreateVMNic()
  const updateNic = useUpdateVMNic()
  const deleteNic = useDeleteVMNic()

  const [showAddDialog, setShowAddDialog] = useState(false)
  const [showEditDialog, setShowEditDialog] = useState(false)
  const [showDeleteDialog, setShowDeleteDialog] = useState(false)
  const [selectedNic, setSelectedNic] = useState<VmNic | null>(null)

  const [formData, setFormData] = useState({
    iface_id: "",
    host_dev_name: "",
    guest_mac: "",
  })

  const resetForm = () => {
    setFormData({
      iface_id: "",
      host_dev_name: "",
      guest_mac: "",
    })
  }

  const handleAdd = () => {
    resetForm()
    setShowAddDialog(true)
  }

  const handleEdit = (nic: VmNic) => {
    setSelectedNic(nic)
    setFormData({
      iface_id: nic.iface_id,
      host_dev_name: nic.host_dev_name,
      guest_mac: nic.guest_mac || "",
    })
    setShowEditDialog(true)
  }

  const handleDelete = (nic: VmNic) => {
    setSelectedNic(nic)
    setShowDeleteDialog(true)
  }

  const handleSubmitAdd = () => {
    const payload: any = {
      iface_id: formData.iface_id,
      host_dev_name: formData.host_dev_name,
    }

    if (formData.guest_mac) {
      payload.guest_mac = formData.guest_mac
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

  const handleSubmitEdit = () => {
    if (!selectedNic) return

    // For NICs, we can only update rate limiters (not implemented in this UI yet)
    // Just close the dialog for now
    setShowEditDialog(false)
    setSelectedNic(null)
    resetForm()
  }

  const handleConfirmDelete = () => {
    if (!selectedNic) return

    deleteNic.mutate(
      { vmId, nicId: selectedNic.iface_id },
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
          <Button onClick={handleAdd}>
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
          ) : nics.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              <Network className="h-12 w-12 mx-auto mb-3 opacity-50" />
              <p>No network interfaces configured for this VM.</p>
              <p className="text-sm mt-1">Click "Add NIC" to add a network interface.</p>
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Interface ID</TableHead>
                  <TableHead>Guest MAC</TableHead>
                  <TableHead>Host Device</TableHead>
                  <TableHead className="text-right">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {nics.map((nic) => (
                  <TableRow key={nic.iface_id}>
                    <TableCell className="font-mono text-sm">{nic.iface_id}</TableCell>
                    <TableCell className="font-mono text-sm">{nic.guest_mac || "Auto"}</TableCell>
                    <TableCell className="font-mono text-sm">{nic.host_dev_name}</TableCell>
                    <TableCell className="text-right">
                      <div className="flex justify-end gap-2">
                        <Button variant="ghost" size="icon" onClick={() => handleEdit(nic)}>
                          <Edit className="h-4 w-4" />
                        </Button>
                        <Button variant="ghost" size="icon" onClick={() => handleDelete(nic)}>
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

      {/* Add NIC Dialog */}
      <Dialog open={showAddDialog} onOpenChange={setShowAddDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Add Network Interface</DialogTitle>
            <DialogDescription>
              Add a new network interface to this VM. The VM must be stopped to attach a NIC.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="iface_id">Interface ID *</Label>
              <Input
                id="iface_id"
                placeholder="e.g., eth0, eth1"
                value={formData.iface_id}
                onChange={(e) => setFormData({ ...formData, iface_id: e.target.value })}
              />
              <p className="text-xs text-muted-foreground">Unique identifier for this interface</p>
            </div>

            <div className="space-y-2">
              <Label htmlFor="host_dev_name">Host Device *</Label>
              <Input
                id="host_dev_name"
                placeholder="e.g., tap0, vmtap0"
                value={formData.host_dev_name}
                onChange={(e) => setFormData({ ...formData, host_dev_name: e.target.value })}
              />
              <p className="text-xs text-muted-foreground">TAP device name on the host</p>
            </div>

            <div className="space-y-2">
              <Label htmlFor="guest_mac">Guest MAC Address (optional)</Label>
              <Input
                id="guest_mac"
                placeholder="AA:BB:CC:DD:EE:FF"
                value={formData.guest_mac}
                onChange={(e) => setFormData({ ...formData, guest_mac: e.target.value })}
              />
              <p className="text-xs text-muted-foreground">
                MAC address for the guest. Leave empty for auto-generation.
              </p>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowAddDialog(false)}>
              Cancel
            </Button>
            <Button
              onClick={handleSubmitAdd}
              disabled={!formData.iface_id || !formData.host_dev_name || createNic.isPending}
            >
              {createNic.isPending ? "Adding..." : "Add NIC"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Edit NIC Dialog */}
      <Dialog open={showEditDialog} onOpenChange={setShowEditDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Edit Network Interface</DialogTitle>
            <DialogDescription>
              View network interface details. Most properties cannot be modified after creation.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label>Interface ID</Label>
              <Input value={formData.iface_id} disabled />
              <p className="text-xs text-muted-foreground">Cannot be changed</p>
            </div>

            <div className="space-y-2">
              <Label>Host Device</Label>
              <Input value={formData.host_dev_name} disabled />
              <p className="text-xs text-muted-foreground">Cannot be changed</p>
            </div>

            <div className="space-y-2">
              <Label>Guest MAC Address</Label>
              <Input value={formData.guest_mac || "Auto-generated"} disabled />
              <p className="text-xs text-muted-foreground">Cannot be changed</p>
            </div>

            <Alert>
              <AlertCircle className="h-4 w-4" />
              <AlertDescription>
                Network interface properties cannot be modified after creation. To change
                configuration, delete this NIC and create a new one.
              </AlertDescription>
            </Alert>
          </div>
          <DialogFooter>
            <Button onClick={() => setShowEditDialog(false)}>Close</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation */}
      <ConfirmDialog
        open={showDeleteDialog}
        onOpenChange={setShowDeleteDialog}
        onConfirm={handleConfirmDelete}
        title="Delete Network Interface"
        description={`Are you sure you want to delete network interface "${selectedNic?.iface_id}"? This action cannot be undone.`}
        confirmText="Delete"
        variant="destructive"
        isLoading={deleteNic.isPending}
      />
    </>
  )
}
