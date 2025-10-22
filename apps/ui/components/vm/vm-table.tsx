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
import { formatRelativeTime, formatPercentage } from "@/lib/utils/format"
import { useToast } from "@/hooks/use-toast"
import type { VM } from "@/lib/types"

interface VMTableProps {
  vms: VM[]
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
  const { toast } = useToast()

  const filteredVMs = vms.filter((vm) => {
    const matchesSearch = vm.name.toLowerCase().includes(searchQuery.toLowerCase())
    const matchesState = stateFilter === "all" || vm.state === stateFilter
    return matchesSearch && matchesState
  })

  const totalPages = Math.ceil(filteredVMs.length / ITEMS_PER_PAGE)
  const startIndex = (currentPage - 1) * ITEMS_PER_PAGE
  const paginatedVMs = filteredVMs.slice(startIndex, startIndex + ITEMS_PER_PAGE)

  const handleAction = (action: string, vmName: string) => {
    toast({
      title: `VM ${action}`,
      description: `${vmName} has been ${action.toLowerCase()}`,
    })
  }

  const handleDelete = () => {
    toast({
      title: "VM Deleted",
      description: `${deleteDialog.vmName} has been deleted`,
      variant: "destructive",
    })
    setDeleteDialog({ open: false, vmId: "", vmName: "" })
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
              <TableHead>Created</TableHead>
              <TableHead className="text-right">Actions</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {paginatedVMs.length === 0 ? (
              <TableRow>
                <TableCell colSpan={8} className="text-center py-8 text-muted-foreground">
                  No VMs found
                </TableCell>
              </TableRow>
            ) : (
              paginatedVMs.map((vm) => (
                <TableRow key={vm.id}>
                  <TableCell>
                    <Link href={`/vms/${vm.id}`} className="font-medium hover:underline">
                      {vm.name}
                    </Link>
                  </TableCell>
                  <TableCell>
                    <StatusBadge status={vm.state} />
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
                  <TableCell className="text-sm text-muted-foreground">{formatRelativeTime(vm.created_at)}</TableCell>
                  <TableCell className="text-right">
                    <div className="flex justify-end gap-1">
                      {vm.state === "stopped" && (
                        <Button
                          variant="ghost"
                          size="icon"
                          title="Start"
                          onClick={() => handleAction("Started", vm.name)}
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
                            onClick={() => handleAction("Paused", vm.name)}
                          >
                            <Pause className="h-4 w-4" />
                          </Button>
                          <Button
                            variant="ghost"
                            size="icon"
                            title="Stop"
                            onClick={() => handleAction("Stopped", vm.name)}
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
                          onClick={() => handleAction("Resumed", vm.name)}
                        >
                          <Play className="h-4 w-4" />
                        </Button>
                      )}
                      <Button
                        variant="ghost"
                        size="icon"
                        title="Delete"
                        onClick={() => setDeleteDialog({ open: true, vmId: vm.id, vmName: vm.name })}
                      >
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    </div>
                  </TableCell>
                </TableRow>
              ))
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
        onConfirm={handleDelete}
        variant="destructive"
      />
    </div>
  )
}
