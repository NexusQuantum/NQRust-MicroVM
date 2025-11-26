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
import { Play, Square, Pause, Trash2, Search } from "lucide-react"
import { formatPercentage } from "@/lib/utils/format"
import type { Vm } from "@/lib/types"
import { useVmStatePatch, useDeleteVM } from "@/lib/queries"
import { useAuthStore, canModifyResource, canDeleteResource } from "@/lib/auth/store"
import { useDateFormat } from "@/lib/hooks/use-date-format"
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip"
import { toast } from "sonner"

interface VMTableProps {
  vms: Vm[]
}

const ITEMS_PER_PAGE = 10

export function VMTable({ vms }: VMTableProps) {
  const [searchQuery, setSearchQuery] = useState("")
  const [stateFilter, setStateFilter] = useState<string>("all")
  const [currentPage, setCurrentPage] = useState(1)
  const [deleteDialog, setDeleteDialog] = useState<{ open: boolean; vmId: string; vmName: string }>({
    open: false,
    vmId: "",
    vmName: "",
  })
  const { user } = useAuthStore()
  const dateFormat = useDateFormat()

  const filteredVMs = vms.filter((vm) => {
    const vmName = vm.name || vm.vm_name || `VM-${vm.id}`
    const matchesSearch = vmName.toLowerCase().includes(searchQuery.toLowerCase())
    const matchesState = stateFilter === "all" || vm.state === stateFilter

    // Filter by ownership for non-admin/non-viewer users
    const canView = user?.role === "admin" || user?.role === "viewer" ||
                    !(vm as any).created_by_user_id ||
                    (vm as any).created_by_user_id === user?.id

    return matchesSearch && matchesState && canView
  })

  const totalPages = Math.ceil(filteredVMs.length / ITEMS_PER_PAGE)
  const startIndex = (currentPage - 1) * ITEMS_PER_PAGE
  const paginatedVMs = filteredVMs.slice(startIndex, startIndex + ITEMS_PER_PAGE)

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
      deleteMutation.mutate(deleteDialog.vmId, {
        onSuccess: () => {
          toast.success("VM Deleted", {
            description: `${deleteDialog.vmName} has been deleted`,
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
                const vmName = vm.name || vm.vm_name || `VM-${vm.id}`
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
                    {(vm as any).created_by_user_id ? (
                      (vm as any).created_by_user_id === user?.id ? (
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
                      {canModifyResource(user, (vm as any).created_by_user_id) && (
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
                      {canDeleteResource(user, (vm as any).created_by_user_id) && (
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

      {totalPages > 1 && (
        <Pagination>
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

      <ConfirmDialog
        open={deleteDialog.open}
        onOpenChange={(open) => setDeleteDialog({ ...deleteDialog, open })}
        title="Delete VM"
        description={`Are you sure you want to delete ${deleteDialog.vmName}? This action cannot be undone.`}
        confirmText="Delete"
        onConfirm={() => handleDelete()}
        variant="destructive"
      />
    </div>
  )
}
