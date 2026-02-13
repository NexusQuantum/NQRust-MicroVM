"use client"

import { useState, useEffect } from "react"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Textarea } from "@/components/ui/textarea"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Badge } from "@/components/ui/badge"
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip"
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog"
import { Search, Trash2, Lock, RefreshCw, AlertCircle, Pencil, Layers } from "lucide-react"
import type { Network } from "@/lib/types"
import { useDeleteNetwork, useRetryNetwork, useUpdateNetwork } from "@/lib/queries"
import { formatDistanceToNow } from "date-fns"
import { ConfirmDialog } from "@/components/shared/confirm-dialog"
import { toast } from "sonner"

interface NetworkTableProps {
  networks: Network[]
}

function StatusBadge({ network }: { network: Network }) {
  switch (network.status) {
    case "active":
      return <Badge variant="default" className="bg-green-600">Active</Badge>
    case "provisioning":
      return <Badge variant="default" className="bg-yellow-600 animate-pulse">Provisioning</Badge>
    case "error":
      return (
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger asChild>
              <Badge variant="destructive" className="cursor-help gap-1">
                <AlertCircle className="h-3 w-3" />
                Error
              </Badge>
            </TooltipTrigger>
            <TooltipContent className="max-w-xs">
              <p>{network.error_message || "Unknown error"}</p>
            </TooltipContent>
          </Tooltip>
        </TooltipProvider>
      )
    case "pending":
      return <Badge variant="secondary">Pending</Badge>
    case "deleting":
      return <Badge variant="secondary" className="animate-pulse">Deleting</Badge>
    default:
      return <Badge variant="secondary">{network.status}</Badge>
  }
}

function TypeBadge({ type }: { type: string }) {
  switch (type) {
    case "nat":
      return <Badge variant="default">NAT</Badge>
    case "bridged":
      return <Badge variant="secondary">Bridged</Badge>
    case "isolated":
      return <Badge variant="outline">Isolated</Badge>
    case "vxlan":
      return <Badge variant="default" className="bg-purple-600 gap-1"><Layers className="h-3 w-3" />VXLAN</Badge>
    default:
      return <Badge variant="secondary">{type}</Badge>
  }
}

export function NetworkTable({ networks }: NetworkTableProps) {
  const [searchQuery, setSearchQuery] = useState("")
  const [typeFilter, setTypeFilter] = useState<string>("all")
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false)
  const [networkToDelete, setNetworkToDelete] = useState<Network | null>(null)
  const [editDialogOpen, setEditDialogOpen] = useState(false)
  const [networkToEdit, setNetworkToEdit] = useState<Network | null>(null)
  const [editName, setEditName] = useState("")
  const [editDescription, setEditDescription] = useState("")

  const deleteNetwork = useDeleteNetwork()
  const retryNetwork = useRetryNetwork()
  const updateNetwork = useUpdateNetwork()

  // Handle successful deletion
  useEffect(() => {
    if (deleteNetwork.isSuccess) {
      toast.success("Network deleted successfully", {
        description: `Network "${networkToDelete?.name}" has been deleted.`
      })
      setDeleteDialogOpen(false)
      setNetworkToDelete(null)
      deleteNetwork.reset()
    }
  }, [deleteNetwork.isSuccess])

  // Handle deletion error
  useEffect(() => {
    if (deleteNetwork.isError) {
      const errorMessage = deleteNetwork.error instanceof Error
        ? deleteNetwork.error.message
        : "An unexpected error occurred"
      toast.error("Failed to delete network", {
        description: errorMessage
      })
    }
  }, [deleteNetwork.isError])

  const handleDeleteClick = (network: Network) => {
    if (network.vm_count > 0) {
      toast.error("Cannot delete network", {
        description: "This network has active VMs attached to it."
      })
      return
    }
    setNetworkToDelete(network)
    setDeleteDialogOpen(true)
  }

  const handleDeleteConfirm = () => {
    if (networkToDelete) {
      deleteNetwork.mutate(networkToDelete.id)
    }
  }

  const handleRetry = (network: Network) => {
    retryNetwork.mutate(network.id, {
      onSuccess: () => {
        toast.success("Retrying network provisioning", {
          description: `Retrying provisioning for "${network.name}"...`
        })
      },
      onError: (error) => {
        toast.error("Failed to retry", {
          description: error instanceof Error ? error.message : "An unexpected error occurred"
        })
      },
    })
  }

  const handleEditClick = (network: Network) => {
    setNetworkToEdit(network)
    setEditName(network.name)
    setEditDescription(network.description || "")
    setEditDialogOpen(true)
  }

  const handleEditSave = () => {
    if (!networkToEdit) return
    if (!editName.trim()) {
      toast.error("Name is required")
      return
    }
    updateNetwork.mutate(
      { id: networkToEdit.id, params: { name: editName, description: editDescription || undefined } },
      {
        onSuccess: () => {
          toast.success("Network updated", {
            description: `Network "${editName}" has been updated.`
          })
          setEditDialogOpen(false)
          setNetworkToEdit(null)
        },
        onError: (error) => {
          toast.error("Failed to update network", {
            description: error instanceof Error ? error.message : "An unexpected error occurred"
          })
        },
      }
    )
  }

  const filteredNetworks = networks.filter((network) => {
    const matchesSearch =
      network.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      network.bridge_name.toLowerCase().includes(searchQuery.toLowerCase())
    const matchesType = typeFilter === "all" || network.type === typeFilter
    return matchesSearch && matchesType
  })

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-4">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            placeholder="Search networks..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-9"
          />
        </div>
        <Select value={typeFilter} onValueChange={setTypeFilter}>
          <SelectTrigger className="w-40">
            <SelectValue placeholder="Type" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All Types</SelectItem>
            <SelectItem value="nat">NAT</SelectItem>
            <SelectItem value="bridged">Bridged</SelectItem>
            <SelectItem value="isolated">Isolated</SelectItem>
            <SelectItem value="vxlan">VXLAN</SelectItem>
          </SelectContent>
        </Select>
      </div>

      <div className="rounded-lg border border-border">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Name</TableHead>
              <TableHead>Type</TableHead>
              <TableHead>VLAN/VNI</TableHead>
              <TableHead>Status</TableHead>
              <TableHead>Bridge</TableHead>
              <TableHead>CIDR</TableHead>
              <TableHead>Host</TableHead>
              <TableHead>VMs</TableHead>
              <TableHead>Created</TableHead>
              <TableHead className="text-right">Actions</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {filteredNetworks.length === 0 ? (
              <TableRow>
                <TableCell colSpan={10} className="text-center py-8 text-muted-foreground">
                  No networks found
                </TableCell>
              </TableRow>
            ) : (
              filteredNetworks.map((network) => (
                <TableRow key={network.id}>
                  <TableCell className="font-medium">
                    <div className="flex items-center gap-2">
                      {!network.managed && (
                        <TooltipProvider>
                          <Tooltip>
                            <TooltipTrigger asChild>
                              <Lock className="h-3.5 w-3.5 text-muted-foreground flex-shrink-0" />
                            </TooltipTrigger>
                            <TooltipContent>
                              <p>Installer-created network (read-only)</p>
                            </TooltipContent>
                          </Tooltip>
                        </TooltipProvider>
                      )}
                      {network.name}
                    </div>
                  </TableCell>
                  <TableCell>
                    <TypeBadge type={network.type} />
                  </TableCell>
                  <TableCell>
                    {network.vni ? (
                      <code className="text-xs bg-muted px-1.5 py-0.5 rounded" title="VNI">VNI {network.vni}</code>
                    ) : network.vlan_id ? (
                      <code className="text-xs bg-muted px-1.5 py-0.5 rounded">{network.vlan_id}</code>
                    ) : (
                      <span className="text-muted-foreground text-sm">-</span>
                    )}
                  </TableCell>
                  <TableCell>
                    <StatusBadge network={network} />
                  </TableCell>
                  <TableCell>
                    <code className="text-xs bg-muted px-1.5 py-0.5 rounded">{network.bridge_name}</code>
                  </TableCell>
                  <TableCell>
                    {network.cidr ? (
                      <code className="text-xs bg-muted px-1.5 py-0.5 rounded">{network.cidr}</code>
                    ) : (
                      <span className="text-muted-foreground text-sm">-</span>
                    )}
                  </TableCell>
                  <TableCell className="text-sm text-muted-foreground">
                    {network.type === "vxlan" ? (
                      <span>{network.host_name || (network.host_id ? network.host_id.slice(0, 8) : "-")}{network.participating_hosts != null && ` + ${Math.max(0, network.participating_hosts - 1)}`}</span>
                    ) : (
                      network.host_name || (network.host_id ? network.host_id.slice(0, 8) : "-")
                    )}
                  </TableCell>
                  <TableCell>
                    <Badge variant="secondary">{network.vm_count} VMs</Badge>
                  </TableCell>
                  <TableCell className="text-sm text-muted-foreground">
                    {formatDistanceToNow(new Date(network.created_at), { addSuffix: true })}
                  </TableCell>
                  <TableCell className="text-right">
                    <div className="flex items-center justify-end gap-1">
                      {network.status === "error" && (
                        <TooltipProvider>
                          <Tooltip>
                            <TooltipTrigger asChild>
                              <Button
                                variant="ghost"
                                size="icon"
                                onClick={() => handleRetry(network)}
                                disabled={retryNetwork.isPending}
                              >
                                <RefreshCw className="h-4 w-4" />
                              </Button>
                            </TooltipTrigger>
                            <TooltipContent>Retry provisioning</TooltipContent>
                          </Tooltip>
                        </TooltipProvider>
                      )}
                      <TooltipProvider>
                        <Tooltip>
                          <TooltipTrigger asChild>
                            <Button
                              variant="ghost"
                              size="icon"
                              onClick={() => handleEditClick(network)}
                            >
                              <Pencil className="h-4 w-4" />
                            </Button>
                          </TooltipTrigger>
                          <TooltipContent>Edit name & description</TooltipContent>
                        </Tooltip>
                      </TooltipProvider>
                      <TooltipProvider>
                        <Tooltip>
                          <TooltipTrigger asChild>
                            <Button
                              variant="ghost"
                              size="icon"
                              onClick={() => handleDeleteClick(network)}
                              disabled={deleteNetwork.isPending || (!network.managed && network.type === "bridged")}
                            >
                              <Trash2 className="h-4 w-4" />
                            </Button>
                          </TooltipTrigger>
                          <TooltipContent>
                            {!network.managed && network.type === "bridged"
                              ? "Installer-created networks cannot be deleted"
                              : "Delete network"}
                          </TooltipContent>
                        </Tooltip>
                      </TooltipProvider>
                    </div>
                  </TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </div>

      {/* Edit Dialog */}
      <Dialog open={editDialogOpen} onOpenChange={setEditDialogOpen}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>Edit Network</DialogTitle>
            <DialogDescription>
              Update the name and description for this network. Infrastructure settings cannot be changed after creation.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-2">
            <div className="space-y-2">
              <Label htmlFor="edit-name">Name</Label>
              <Input
                id="edit-name"
                value={editName}
                onChange={(e) => setEditName(e.target.value)}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="edit-description">Description</Label>
              <Textarea
                id="edit-description"
                value={editDescription}
                onChange={(e) => setEditDescription(e.target.value)}
                rows={3}
                placeholder="Optional description"
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setEditDialogOpen(false)}>
              Cancel
            </Button>
            <Button onClick={handleEditSave} disabled={updateNetwork.isPending}>
              {updateNetwork.isPending ? "Saving..." : "Save"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <ConfirmDialog
        open={deleteDialogOpen}
        onOpenChange={setDeleteDialogOpen}
        title="Delete Network"
        description={`Are you sure you want to delete "${networkToDelete?.name}"?${networkToDelete?.managed ? " This will tear down the bridge, DHCP, and firewall rules on the host." : ""} This action cannot be undone.`}
        confirmText="Delete"
        cancelText="Cancel"
        onConfirm={handleDeleteConfirm}
        variant="destructive"
        isLoading={deleteNetwork.isPending}
      />
    </div>
  )
}
