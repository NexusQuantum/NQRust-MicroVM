"use client"

import { useState } from "react"
import Link from "next/link"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { StatusBadge } from "@/components/shared/status-badge"
import { Play, Square, RotateCw, FileText, Terminal, Trash2, Search } from "lucide-react"
import { formatDuration, formatPercentage } from "@/lib/utils/format"
import type { Container } from "@/lib/types"

interface ContainerTableProps {
  containers: Container[]
}

export function ContainerTable({ containers }: ContainerTableProps) {
  const [searchQuery, setSearchQuery] = useState("")
  const [statusFilter, setStatusFilter] = useState<string>("all")

  const filteredContainers = containers.filter((container) => {
    const matchesSearch =
      container.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      container.image.toLowerCase().includes(searchQuery.toLowerCase())
    const matchesStatus = statusFilter === "all" || container.status === statusFilter
    return matchesSearch && matchesStatus
  })

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-4">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            placeholder="Search containers..."
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
            <SelectItem value="running">Running</SelectItem>
            <SelectItem value="stopped">Stopped</SelectItem>
            <SelectItem value="restarting">Restarting</SelectItem>
            <SelectItem value="error">Error</SelectItem>
          </SelectContent>
        </Select>
      </div>

      <div className="rounded-lg border border-border">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Name</TableHead>
              <TableHead>Image</TableHead>
              <TableHead>Status</TableHead>
              <TableHead>Uptime</TableHead>
              <TableHead>CPU</TableHead>
              <TableHead>Memory</TableHead>
              <TableHead>Ports</TableHead>
              <TableHead className="text-right">Actions</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {filteredContainers.length === 0 ? (
              <TableRow>
                <TableCell colSpan={8} className="text-center py-8 text-muted-foreground">
                  No containers found
                </TableCell>
              </TableRow>
            ) : (
              filteredContainers.map((container) => (
                <TableRow key={container.id}>
                  <TableCell>
                    <Link href={`/containers/${container.id}`} className="font-medium hover:underline">
                      {container.name}
                    </Link>
                  </TableCell>
                  <TableCell>
                    <code className="text-xs bg-muted px-1.5 py-0.5 rounded">{container.image}</code>
                  </TableCell>
                  <TableCell>
                    <StatusBadge status={container.status} />
                  </TableCell>
                  <TableCell className="text-sm text-muted-foreground">
                    {container.uptime_seconds ? formatDuration(container.uptime_seconds) : "N/A"}
                  </TableCell>
                  <TableCell className="text-sm">
                    {container.cpu_percent !== undefined ? formatPercentage(container.cpu_percent) : "N/A"}
                  </TableCell>
                  <TableCell className="text-sm">
                    {container.memory_used_mb !== undefined
                      ? `${container.memory_used_mb}/${container.memory_limit_mb} MB`
                      : "N/A"}
                  </TableCell>
                  <TableCell className="text-xs">
                    {container.port_mappings.map((p, i) => (
                      <div key={i}>
                        {p.host}→{p.container}
                      </div>
                    ))}
                  </TableCell>
                  <TableCell className="text-right">
                    <div className="flex justify-end gap-1">
                      <Button variant="ghost" size="icon" title="Logs" asChild>
                        <Link href={`/containers/${container.id}?tab=logs`}>
                          <FileText className="h-4 w-4" />
                        </Link>
                      </Button>
                      <Button variant="ghost" size="icon" title="Shell" asChild>
                        <Link href={`/containers/${container.id}?tab=shell`}>
                          <Terminal className="h-4 w-4" />
                        </Link>
                      </Button>
                      {container.status === "stopped" && (
                        <Button variant="ghost" size="icon" title="Start">
                          <Play className="h-4 w-4" />
                        </Button>
                      )}
                      {container.status === "running" && (
                        <>
                          <Button variant="ghost" size="icon" title="Restart">
                            <RotateCw className="h-4 w-4" />
                          </Button>
                          <Button variant="ghost" size="icon" title="Stop">
                            <Square className="h-4 w-4" />
                          </Button>
                        </>
                      )}
                      <Button variant="ghost" size="icon" title="Delete">
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
    </div>
  )
}
