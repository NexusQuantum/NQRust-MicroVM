"use client"

import { useState } from "react"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Badge } from "@/components/ui/badge"
import { Search, Activity, HardDrive, Cpu, MemoryStick, Trash2 } from "lucide-react"
import type { Host } from "@/lib/types"
import { formatDistanceToNow } from "date-fns"
import { useDeleteHost } from "@/lib/queries"
import { ConfirmDialog } from "@/components/shared/confirm-dialog"

interface HostTableProps {
  hosts: Host[]
}

function getStatusColor(status: string) {
  switch (status) {
    case "healthy":
      return "bg-green-100 text-green-800 border-green-200"
    case "degraded":
      return "bg-yellow-100 text-yellow-800 border-yellow-200"
    case "offline":
      return "bg-red-100 text-red-800 border-red-200"
    default:
      return "bg-gray-100 text-gray-800 border-gray-200"
  }
}

export function HostTable({ hosts }: HostTableProps) {
  const [searchQuery, setSearchQuery] = useState("")
  const [statusFilter, setStatusFilter] = useState<string>("all")
  const [deleteDialog, setDeleteDialog] = useState<{ open: boolean; hostId: string; hostName: string }>({
    open: false,
    hostId: "",
    hostName: "",
  })
  const deleteHost = useDeleteHost()

  const filteredHosts = hosts.filter((host) => {
    const matchesSearch =
      host.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      host.addr.toLowerCase().includes(searchQuery.toLowerCase())
    const matchesStatus = statusFilter === "all" || host.status === statusFilter
    return matchesSearch && matchesStatus
  })

  console.log("Host: ", hosts)

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-4">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            placeholder="Search hosts..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-9"
          />
        </div>
        <Select value={statusFilter} onValueChange={setStatusFilter}>
          <SelectTrigger className="w-40">
            <SelectValue placeholder="Status" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All Status</SelectItem>
            <SelectItem value="healthy">Healthy</SelectItem>
            <SelectItem value="degraded">Degraded</SelectItem>
            <SelectItem value="offline">Offline</SelectItem>
          </SelectContent>
        </Select>
      </div>

      <div className="rounded-lg border border-border">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Name</TableHead>
              <TableHead>Address</TableHead>
              <TableHead>Status</TableHead>
              <TableHead>Resources</TableHead>
              <TableHead>Source Count</TableHead>
              <TableHead>Last Seen</TableHead>
              <TableHead className="w-20">Actions</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {filteredHosts.length === 0 ? (
              <TableRow>
                <TableCell colSpan={7} className="text-center py-8 text-muted-foreground">
                  No hosts found
                </TableCell>
              </TableRow>
            ) : (
              filteredHosts.map((host) => {
                const canDelete = host.status === "offline" || host.status === "degraded"
                return (
                  <TableRow key={host.id}>
                    <TableCell className="font-medium">{host.name}</TableCell>
                    <TableCell>
                      <code className="text-xs bg-muted px-1.5 py-0.5 rounded">{host.addr}</code>
                    </TableCell>
                    <TableCell>
                      <Badge variant="outline" className={getStatusColor(host.status)}>
                        <Activity className="h-3 w-3 mr-1" />
                        {host.status}
                      </Badge>
                    </TableCell>
                    <TableCell>
                      <div className="space-y-1 text-xs">
                        {host.total_cpus !== undefined && (
                          <div className="flex items-center gap-1.5 text-muted-foreground">
                            <Cpu className="h-3 w-3" />
                            <span>{host.total_cpus} vCPUs</span>
                          </div>
                        )}
                        {host.total_memory_mb !== undefined && (
                          <div className="flex items-center gap-1.5 text-muted-foreground">
                            <MemoryStick className="h-3 w-3" />
                            <span>{(host.total_memory_mb / 1024).toFixed(1)} GB</span>
                          </div>
                        )}
                        {host.total_disk_gb !== undefined && (
                          <div className="flex items-center gap-1.5 text-muted-foreground">
                            <HardDrive className="h-3 w-3" />
                            <span>
                              {host.used_disk_gb || 0}/{host.total_disk_gb} GB
                            </span>
                          </div>
                        )}
                      </div>
                    </TableCell>
                    <TableCell>
                      <Badge variant="secondary">{host.vm_count} Source</Badge>
                    </TableCell>
                    <TableCell className="text-sm text-muted-foreground">
                      {formatDistanceToNow(new Date(host.last_seen_at), { addSuffix: true })}
                    </TableCell>
                    <TableCell>
                      <Button
                        variant="ghost"
                        size="icon"
                        onClick={() => setDeleteDialog({ open: true, hostId: host.id, hostName: host.name })}
                        disabled={!canDelete || deleteHost.isPending}
                        title={canDelete ? "Delete host" : "Only dead hosts (offline/degraded) can be deleted"}
                      >
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    </TableCell>
                  </TableRow>
                )
              })
            )}
          </TableBody>
        </Table>
      </div>

      <ConfirmDialog
        open={deleteDialog.open}
        onOpenChange={(open) => setDeleteDialog({ open, hostId: "", hostName: "" })}
        onConfirm={() => {
          if (deleteDialog.hostId) {
            deleteHost.mutate(deleteDialog.hostId)
            setDeleteDialog({ open: false, hostId: "", hostName: "" })
          }
        }}
        title="Delete Host"
        description={`Are you sure you want to delete "${deleteDialog.hostName}"? This action cannot be undone. Only dead hosts (offline for more than 30 seconds) can be deleted.`}
        confirmText="Delete"
        variant="destructive"
        isLoading={deleteHost.isPending}
      />
    </div>
  )
}
