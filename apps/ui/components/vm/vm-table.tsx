"use client"

import { useState } from "react"
import Link from "next/link"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { StatusBadge } from "@/components/shared/status-badge"
import { ConfirmDialog } from "@/components/shared/confirm-dialog"
import {
  Pagination,
  PaginationContent,
  PaginationItem,
  PaginationLink,
  PaginationNext,
  PaginationPrevious,
} from "@/components/ui/pagination"
import { Badge } from "@/components/ui/badge"
import { Play, Square, Pause, Trash2, Search } from "lucide-react"
import { formatPercentage } from "@/lib/utils/format"
import type { Vm } from "@/lib/types"
import { useVmStatePatch, useDeleteVM, useVolumes, useDeleteVolume } from "@/lib/queries"
import { useAuthStore, canModifyResource, canDeleteResource } from "@/lib/auth/store"
import { useDateFormat } from "@/lib/hooks/use-date-format"
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip"
import { Checkbox } from "@/components/ui/checkbox"
import { Label } from "@/components/ui/label"
import { toast } from "sonner"

interface VMTableProps {
  vms: Vm[]
}

export function VMTable({ vms }: VMTableProps) {
  const [searchQuery, setSearchQuery] = useState("")
  const [stateFilter, setStateFilter] = useState<string>("all")
  const [tagFilter, setTagFilter] = useState<string>("all")
  const [itemsPerPage, setItemsPerPage] = useState(10)
  const [currentPage, setCurrentPage] = useState(1)
  const [deleteDialog, setDeleteDialog] = useState<{ open: boolean; vmId: string; vmName: string }>({
    open: false,
    vmId: "",
    vmName: "",
  })
  const { user } = useAuthStore()
  const dateFormat = useDateFormat()

  // Extract unique tags for filter dropdown
  const allTags = Array.from(
    new Set(vms.flatMap((vm) => vm.tags || []).filter((t) => !t.startsWith("type:")))
  ).sort()

  const filteredVMs = vms.filter((vm) => {
    const vmName = vm.name || `VM-${vm.id}`
    const query = searchQuery.toLowerCase()
    const matchesSearch =
      vmName.toLowerCase().includes(query) ||
      (vm.guest_ip && vm.guest_ip.toLowerCase().includes(query)) ||
      vm.id.toLowerCase().includes(query) ||
      (vm.tags || []).some((tag) => tag.toLowerCase().includes(query))
    const matchesState = stateFilter === "all" || vm.state === stateFilter
    const matchesTag = tagFilter === "all" || (vm.tags || []).includes(tagFilter)

    // Filter by ownership for non-admin/non-viewer users
    const canView = user?.role === "admin" || user?.role === "viewer" ||
                    !vm.created_by_user_id ||
                    vm.created_by_user_id === user?.id

    return matchesSearch && matchesState && matchesTag && canView
  })

  const totalPages = Math.ceil(filteredVMs.length / itemsPerPage)
  const startIndex = (currentPage - 1) * itemsPerPage
  const paginatedVMs = filteredVMs.slice(startIndex, startIndex + itemsPerPage)

  const { data: allVolumes = [] } = useVolumes()
  const deleteVolumeMutation = useDeleteVolume()
  const [deleteVolumesChecked, setDeleteVolumesChecked] = useState(true)

  const attachedVolumes = allVolumes.filter(v => v.attached_to_vm_id === deleteDialog.vmId)

  const vmStatePatch = useVmStatePatch()
  const deleteMutation = useDeleteVM()
  const handleAction = (name: string, id: string, action: "start" | "stop" | "resume" | "ctrl_alt_del" | "pause") => {
    vmStatePatch.mutate({ id, action }, {
      onSuccess: () => {
        const actionMessages = {
          start: { title: "VM Started", description: `${name} has been started successfully` },
          stop: { title: "VM Stopped", description: `${name} has been stopped successfully` },
          pause: { title: "VM Paused", description: `${name} has been paused successfully` },
          resume: { title: "VM Resumed", description: `${name} has been resumed successfully` },
          ctrl_alt_del: { title: "Signal Sent", description: `Ctrl+Alt+Del signal sent to ${name}` },
        }

        const message = actionMessages[action]
        toast.success(message.title, {
          description: message.description,
        })
      },
      onError: (error) => {
        toast.error("Action Failed", {
          description: `Failed to ${action} ${name}: ${error.message}`,
        })
      }
    })
  }

  const handleDelete = () => {
    if (deleteDialog.vmId && deleteDialog.vmName) {
      const volumeIdsToDelete = deleteVolumesChecked ? attachedVolumes.map(v => v.id) : []
      deleteMutation.mutate(deleteDialog.vmId, {
        onSuccess: async () => {
          if (volumeIdsToDelete.length > 0) {
            await Promise.allSettled(
              volumeIdsToDelete.map(id => deleteVolumeMutation.mutateAsync(id))
            )
          }
          toast.success("VM Deleted", {
            description: `${deleteDialog.vmName} has been deleted${volumeIdsToDelete.length > 0 ? ` along with ${volumeIdsToDelete.length} volume${volumeIdsToDelete.length !== 1 ? "s" : ""}` : ""}`,
          })
          setDeleteDialog({ open: false, vmId: "", vmName: "" })
        },
        onError: (error) => {
          toast.error("Delete Failed", {
            description: `Failed to delete ${deleteDialog.vmName}: ${error.message}`,
          })
        }
      })
    }
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-4">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            placeholder="Search VMs..."
            value={searchQuery}
            onChange={(e) => {
              setSearchQuery(e.target.value)
              setCurrentPage(1)
            }}
            className="pl-9"
          />
        </div>
        <Select
          value={stateFilter}
          onValueChange={(value) => {
            setStateFilter(value)
            setCurrentPage(1)
          }}
        >
          <SelectTrigger className="w-40">
            <SelectValue placeholder="State" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All States</SelectItem>
            <SelectItem value="running">Running</SelectItem>
            <SelectItem value="stopped">Stopped</SelectItem>
            <SelectItem value="paused">Paused</SelectItem>
          </SelectContent>
        </Select>
        {allTags.length > 0 && (
          <Select
            value={tagFilter}
            onValueChange={(value) => {
              setTagFilter(value)
              setCurrentPage(1)
            }}
          >
            <SelectTrigger className="w-40">
              <SelectValue placeholder="Tag" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All Tags</SelectItem>
              {allTags.map((tag) => (
                <SelectItem key={tag} value={tag}>{tag}</SelectItem>
              ))}
            </SelectContent>
          </Select>
        )}
      </div>

      <div className="rounded-lg border border-border">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Name</TableHead>
              <TableHead>State</TableHead>
              <TableHead>CPU</TableHead>
              <TableHead>Memory</TableHead>
              <TableHead>Guest IP</TableHead>
              <TableHead>Host</TableHead>
              <TableHead>Owner</TableHead>
              <TableHead>Created</TableHead>
              <TableHead className="text-right">Actions</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {paginatedVMs.length === 0 ? (
              <TableRow>
                <TableCell colSpan={9} className="text-center py-8 text-muted-foreground">
                  No VMs found
                </TableCell>
              </TableRow>
            ) : (
              paginatedVMs.map((vm) => {
                const vmName = vm.name || `VM-${vm.id}`
                return (
                <TableRow key={vm.id}>
                  <TableCell>
                    <Link href={`/vms/${vm.id}`} className="font-medium hover:underline">
                      {vmName}
                    </Link>
                  </TableCell>
                  <TableCell>
                    <StatusBadge status={vm.state as any} />
                  </TableCell>
                  <TableCell>
                    <div className="text-sm">
                      <div className="font-medium">{vm.vcpu} vCPU</div>
                      {vm.cpu_usage_percent !== undefined && (
                        <div className="text-muted-foreground">{formatPercentage(vm.cpu_usage_percent)}</div>
                      )}
                    </div>
                  </TableCell>
                  <TableCell>
                    <div className="text-sm">
                      <div className="font-medium">{vm.mem_mib} MiB</div>
                      {vm.memory_usage_percent !== undefined && (
                        <div className="text-muted-foreground">{formatPercentage(vm.memory_usage_percent)}</div>
                      )}
                    </div>
                  </TableCell>
                  <TableCell>
                    <code className="text-xs bg-muted px-1.5 py-0.5 rounded">{vm.guest_ip || "N/A"}</code>
                  </TableCell>
                  <TableCell className="text-sm text-muted-foreground">{vm.host_addr}</TableCell>
                  <TableCell className="text-sm text-muted-foreground">
                    {vm.created_by_user_id ? (
                      vm.created_by_user_id === user?.id ? (
                        <span className="text-primary font-medium">You</span>
                      ) : (
                        <span className="text-muted-foreground">Other User</span>
                      )
                    ) : (
                      <span className="text-muted-foreground italic">System</span>
                    )}
                  </TableCell>
                  <TableCell className="text-sm text-muted-foreground">{dateFormat.formatRelative(vm.created_at)}</TableCell>
                  <TableCell className="text-right">
                    <div className="flex justify-end gap-1">
                      {canModifyResource(user, vm.created_by_user_id) && (
                        <>
                          {vm.state === "stopped" && (
                            <Button
                              variant="ghost"
                              size="icon"
                              title="Start"
                              onClick={() => handleAction(vmName, vm.id, "start")}
                            >
                              <Play className="h-4 w-4" />
                            </Button>
                          )}
                          {vm.state === "running" && (
                            <>
                              <Button
                                variant="ghost"
                                size="icon"
                                title="Pause"
                                onClick={() => handleAction(vmName, vm.id, "pause")}
                              >
                                <Pause className="h-4 w-4" />
                              </Button>
                              <Button
                                variant="ghost"
                                size="icon"
                                title="Stop"
                                onClick={() => handleAction(vmName, vm.id, "stop")}
                              >
                                <Square className="h-4 w-4" />
                              </Button>
                            </>
                          )}
                          {vm.state === "paused" && (
                            <Button
                              variant="ghost"
                              size="icon"
                              title="Resume"
                              onClick={() => handleAction(vmName, vm.id, "resume")}
                            >
                              <Play className="h-4 w-4" />
                            </Button>
                          )}
                        </>
                      )}
                      {canDeleteResource(user, vm.created_by_user_id) && (
                        <>
                          {vm.state === "running" ? (
                            <TooltipProvider>
                              <Tooltip>
                                <TooltipTrigger asChild>
                                  <Button variant="ghost" size="icon" disabled>
                                    <Trash2 className="h-4 w-4" />
                                  </Button>
                                </TooltipTrigger>
                                <TooltipContent>
                                  <p>Cannot delete running VM. Stop the VM first.</p>
                                </TooltipContent>
                              </Tooltip>
                            </TooltipProvider>
                          ) : (
                            <Button
                              variant="ghost"
                              size="icon"
                              title="Delete"
                              onClick={() => setDeleteDialog({ open: true, vmId: vm.id, vmName })}
                            >
                              <Trash2 className="h-4 w-4" />
                            </Button>
                          )}
                        </>
                      )}
                    </div>
                  </TableCell>
                </TableRow>
                )
              })
            )}
          </TableBody>
        </Table>
      </div>

      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          <span>Show</span>
          <Select
            value={String(itemsPerPage)}
            onValueChange={(value) => {
              setItemsPerPage(Number(value))
              setCurrentPage(1)
            }}
          >
            <SelectTrigger className="w-[70px] h-8">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="10">10</SelectItem>
              <SelectItem value="25">25</SelectItem>
              <SelectItem value="50">50</SelectItem>
              <SelectItem value="100">100</SelectItem>
            </SelectContent>
          </Select>
          <span>of {filteredVMs.length} VMs</span>
        </div>
        {totalPages > 1 && (
          <Pagination className="justify-end w-auto mx-0">
            <PaginationContent>
              <PaginationItem>
                <PaginationPrevious
                  onClick={() => setCurrentPage((p) => Math.max(1, p - 1))}
                  className={currentPage === 1 ? "pointer-events-none opacity-50" : "cursor-pointer"}
                />
              </PaginationItem>
              {Array.from({ length: totalPages }, (_, i) => i + 1).map((page) => (
                <PaginationItem key={page}>
                  <PaginationLink
                    onClick={() => setCurrentPage(page)}
                    isActive={currentPage === page}
                    className="cursor-pointer"
                  >
                    {page}
                  </PaginationLink>
                </PaginationItem>
              ))}
              <PaginationItem>
                <PaginationNext
                  onClick={() => setCurrentPage((p) => Math.min(totalPages, p + 1))}
                  className={currentPage === totalPages ? "pointer-events-none opacity-50" : "cursor-pointer"}
                />
              </PaginationItem>
            </PaginationContent>
          </Pagination>
        )}
      </div>

      <ConfirmDialog
        open={deleteDialog.open}
        onOpenChange={(open) => setDeleteDialog({ ...deleteDialog, open })}
        title="Delete VM"
        description={`Are you sure you want to delete ${deleteDialog.vmName}? This action cannot be undone.`}
        confirmText="Delete"
        onConfirm={() => handleDelete()}
        variant="destructive"
      >
        {attachedVolumes.length > 0 && (
          <div className="flex items-center space-x-2 py-2">
            <Checkbox
              id="delete-vm-volumes"
              checked={deleteVolumesChecked}
              onCheckedChange={(checked) => setDeleteVolumesChecked(checked as boolean)}
            />
            <Label htmlFor="delete-vm-volumes" className="text-sm cursor-pointer">
              Also delete {attachedVolumes.length} attached volume{attachedVolumes.length !== 1 ? "s" : ""}
            </Label>
          </div>
        )}
      </ConfirmDialog>
    </div>
  )
}
