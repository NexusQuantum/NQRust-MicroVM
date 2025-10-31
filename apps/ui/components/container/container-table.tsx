"use client"

import { useState } from "react"
import Link from "next/link"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { StatusBadge } from "@/components/shared/status-badge"
import { Play, Square, RotateCw, FileText, Terminal, Trash2, Search } from "lucide-react"
import { formatDuration } from "@/lib/utils/format"
import type { Container } from "@/lib/types"
import { useStartContainer, useStopContainer, useRestartContainer, useDeleteContainer } from "@/lib/queries"

interface ContainerTableProps {
  containers: Container[]
}

export function ContainerTable({ containers }: ContainerTableProps) {
  const [searchQuery, setSearchQuery] = useState("")
  const [statusFilter, setStatusFilter] = useState<string>("all")

  const startContainer = useStartContainer()
  const stopContainer = useStopContainer()
  const restartContainer = useRestartContainer()
  const deleteContainer = useDeleteContainer()

  const filteredContainers = containers.filter((container) => {
    const matchesSearch =
      container.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      container.image.toLowerCase().includes(searchQuery.toLowerCase())
    const matchesStatus = statusFilter === "all" || container.state === statusFilter
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
            <SelectItem value="creating">Creating</SelectItem>
            <SelectItem value="booting">Booting</SelectItem>
            <SelectItem value="initializing">Initializing</SelectItem>
            <SelectItem value="paused">Paused</SelectItem>
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
                    <StatusBadge status={container.state} />
                  </TableCell>
                  <TableCell className="text-sm text-muted-foreground">
                    {container.uptime_seconds ? formatDuration(container.uptime_seconds) : "N/A"}
                  </TableCell>
                  <TableCell className="text-sm">
                    {container.cpu_limit !== undefined ? `${container.cpu_limit} vCPU` : "N/A"}
                  </TableCell>
                  <TableCell className="text-sm">
                    {container.memory_limit_mb !== undefined ? `${container.memory_limit_mb} MB` : "N/A"}
                  </TableCell>
                  <TableCell className="text-xs">
                    {container.port_mappings && container.port_mappings.length > 0 ? (
                      container.port_mappings.map((p, i) => (
                        <div key={i} className="font-mono">
                          {p.host}:{p.container} ({p.protocol})
                        </div>
                      ))
                    ) : (
                      <span className="text-muted-foreground">No ports</span>
                    )}
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
                      {container.state === "stopped" && (
                        <Button
                          variant="ghost"
                          size="icon"
                          title="Start"
                          onClick={() => startContainer.mutate(container.id)}
                          disabled={startContainer.isPending}
                        >
                          <Play className="h-4 w-4" />
                        </Button>
                      )}
                      {container.state === "running" && (
                        <>
                          <Button
                            variant="ghost"
                            size="icon"
                            title="Restart"
                            onClick={() => restartContainer.mutate(container.id)}
                            disabled={restartContainer.isPending}
                          >
                            <RotateCw className="h-4 w-4" />
                          </Button>
                          <Button
                            variant="ghost"
                            size="icon"
                            title="Stop"
                            onClick={() => stopContainer.mutate(container.id)}
                            disabled={stopContainer.isPending}
                          >
                            <Square className="h-4 w-4" />
                          </Button>
                        </>
                      )}
                      <Button
                        variant="ghost"
                        size="icon"
                        title="Delete"
                        onClick={() => {
                          if (confirm(`Are you sure you want to delete container "${container.name}"?`)) {
                            deleteContainer.mutate(container.id)
                          }
                        }}
                        disabled={deleteContainer.isPending}
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
    </div>
  )
}
