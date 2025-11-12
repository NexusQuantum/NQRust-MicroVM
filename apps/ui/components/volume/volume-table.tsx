"use client"

import { useState } from "react"
import Link from "next/link"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Badge } from "@/components/ui/badge"
import { Search, Link as LinkIcon, Settings } from "lucide-react"
import type { Volume } from "@/lib/types"
import { formatDistanceToNow } from "date-fns"

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B"

  const k = 1024
  const sizes = ["B", "KB", "MB", "GB", "TB"]
  const i = Math.floor(Math.log(bytes) / Math.log(k))

  // Show 2 decimal places for precision
  const value = bytes / Math.pow(k, i)
  const formatted = i === 0 ? value.toString() : value.toFixed(2)

  return `${formatted} ${sizes[i]}`
}

interface VolumeTableProps {
  volumes: Volume[]
}

function getStatusColor(status: string) {
  switch (status) {
    case "available":
      return "bg-green-100 text-green-800 border-green-200"
    case "attached":
      return "bg-blue-100 text-blue-800 border-blue-200"
    case "creating":
      return "bg-yellow-100 text-yellow-800 border-yellow-200"
    case "error":
      return "bg-red-100 text-red-800 border-red-200"
    default:
      return "bg-gray-100 text-gray-800 border-gray-200"
  }
}

export function VolumeTable({ volumes }: VolumeTableProps) {
  const [searchQuery, setSearchQuery] = useState("")
  const [statusFilter, setStatusFilter] = useState<string>("all")

  const filteredVolumes = volumes.filter((volume) => {
    const matchesSearch =
      volume.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      (volume.attached_to_vm_name && volume.attached_to_vm_name.toLowerCase().includes(searchQuery.toLowerCase()))
    const matchesStatus = statusFilter === "all" || volume.status === statusFilter
    return matchesSearch && matchesStatus
  })

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-4">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            placeholder="Search volumes..."
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
            <SelectItem value="available">Available</SelectItem>
            <SelectItem value="attached">Attached</SelectItem>
            <SelectItem value="creating">Creating</SelectItem>
            <SelectItem value="error">Error</SelectItem>
          </SelectContent>
        </Select>
      </div>

      <div className="rounded-lg border border-border">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Name</TableHead>
              <TableHead>Size</TableHead>
              <TableHead>Type</TableHead>
              <TableHead>Status</TableHead>
              <TableHead>Host</TableHead>
              <TableHead>Attached To</TableHead>
              <TableHead>Created</TableHead>
              <TableHead className="text-right">Actions</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {filteredVolumes.length === 0 ? (
              <TableRow>
                <TableCell colSpan={8} className="text-center py-8 text-muted-foreground">
                  No volumes found
                </TableCell>
              </TableRow>
            ) : (
              filteredVolumes.map((volume) => (
                <TableRow key={volume.id}>
                  <TableCell className="font-medium">{volume.name}</TableCell>
                  <TableCell className="text-sm">{formatBytes(volume.size_bytes)}</TableCell>
                  <TableCell>
                    <Badge variant="outline">{volume.type.toUpperCase()}</Badge>
                  </TableCell>
                  <TableCell>
                    <Badge variant="outline" className={getStatusColor(volume.status)}>
                      {volume.status}
                    </Badge>
                  </TableCell>
                  <TableCell className="text-sm text-muted-foreground">
                    {volume.host_name || volume.host_id.slice(0, 8)}
                  </TableCell>
                  <TableCell>
                    {volume.attached_to_vm_id ? (
                      <div className="flex flex-col gap-0.5">
                        <Link
                          href={`/vms/${volume.attached_to_vm_id}`}
                          className="flex items-center gap-1 text-sm text-blue-600 hover:underline"
                        >
                          <LinkIcon className="h-3 w-3" />
                          {volume.attached_to_vm_name || "Unknown VM"}
                        </Link>
                        <span className="text-xs text-muted-foreground font-mono">
                          {volume.attached_to_vm_id.slice(0, 8)}...
                        </span>
                      </div>
                    ) : (
                      <span className="text-muted-foreground text-sm">Not attached</span>
                    )}
                  </TableCell>
                  <TableCell className="text-sm text-muted-foreground">
                    {formatDistanceToNow(new Date(volume.created_at), { addSuffix: true })}
                  </TableCell>
                  <TableCell className="text-right">
                    {volume.status === "attached" && volume.attached_to_vm_id ? (
                      <Button
                        variant="ghost"
                        size="sm"
                        title="Manage in VM Storage"
                        asChild
                      >
                        <Link href={`/vms/${volume.attached_to_vm_id}?tab=storage`}>
                          <Settings className="h-4 w-4 mr-2" />
                          Manage Storage
                        </Link>
                      </Button>
                    ) : (
                      <span className="text-xs text-muted-foreground">-</span>
                    )}
                  </TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </div>
    </div>
  )
}
