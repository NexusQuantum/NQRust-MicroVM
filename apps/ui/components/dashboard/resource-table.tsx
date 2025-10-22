"use client"

import { useState } from "react"
import Link from "next/link"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { StatusBadge } from "@/components/shared/status-badge"
import { Server, Zap, Container, Play, Square, Trash2, Search } from "lucide-react"
import { formatRelativeTime, formatPercentage } from "@/lib/utils/format"
import type { UnifiedResource } from "@/lib/api/dashboard"

interface ResourceTableProps {
  resources: UnifiedResource[]
}

export function   ResourceTable({ resources }: ResourceTableProps) {
  const [searchQuery, setSearchQuery] = useState("")
  const [typeFilter, setTypeFilter] = useState<string>("all")
  const [stateFilter, setStateFilter] = useState<string>("all")

  const filteredResources = resources.filter((resource) => {
    const matchesSearch = resource.name?.toLowerCase().includes(searchQuery.toLowerCase()) || false
    const matchesType = typeFilter === "all" || resource.type === typeFilter
    const matchesState = stateFilter === "all" || resource.state === stateFilter
    return matchesSearch && matchesType && matchesState
  })

  const getTypeIcon = (type: string) => {
    switch (type) {
      case "vm":
        return <Server className="h-4 w-4" />
      case "function":
        return <Zap className="h-4 w-4" />
      case "container":
        return <Container className="h-4 w-4" />
      default:
        return null
    }
  }

  const getTypeBadge = (type: string) => {
    const colors = {
      vm: "bg-blue-100 text-blue-700 border-blue-200",
      function: "bg-purple-100 text-purple-700 border-purple-200",
      container: "bg-green-100 text-green-700 border-green-200",
    }
    return (
      <Badge variant="outline" className={colors[type as keyof typeof colors]}>
        <span className="flex items-center gap-1">
          {getTypeIcon(type)}
          {type.toUpperCase()}
        </span>
      </Badge>
    )
  }

  const getResourceLink = (resource: UnifiedResource) => {
    switch (resource.type) {
      case "vm":
        return `/vms/${resource.id}`
      case "function":
        return `/functions/${resource.id}`
      case "container":
        return `/containers/${resource.id}`
      default:
        return "#"
    }
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-4">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            placeholder="Search resources..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-9 shadow-none"
          />
        </div>
        <Select value={typeFilter} onValueChange={setTypeFilter}>
          <SelectTrigger className="w-40 shadow-none">
            <SelectValue placeholder="Type" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All Types</SelectItem>
            <SelectItem value="vm">VMs</SelectItem>
            <SelectItem value="function">Functions</SelectItem>
            <SelectItem value="container">Containers</SelectItem>
          </SelectContent>
        </Select>
        <Select value={stateFilter} onValueChange={setStateFilter}>
          <SelectTrigger className="w-40 shadow-none">
            <SelectValue placeholder="State" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All States</SelectItem>
            <SelectItem value="running">Running</SelectItem>
            <SelectItem value="stopped">Stopped</SelectItem>
            <SelectItem value="idle">Idle</SelectItem>
            <SelectItem value="error">Error</SelectItem>
          </SelectContent>
        </Select>
      </div>

      <div className="rounded-lg border border-border">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Name</TableHead>
              <TableHead>Type</TableHead>
              <TableHead>State</TableHead>
              <TableHead>Metrics</TableHead>
              <TableHead className="text-right">Actions</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {filteredResources.length === 0 ? (
              <TableRow>
                <TableCell colSpan={5} className="text-center py-8 text-muted-foreground">
                  No resources found
                </TableCell>
              </TableRow>
            ) : (
              filteredResources.map((resource) => (
                <TableRow key={`${resource.type}-${resource.id}`}>
                  <TableCell>
                    <Link href={getResourceLink(resource)} className="font-medium hover:underline">
                      {resource.name || 'Unknown'}
                    </Link>
                  </TableCell>
                  <TableCell>{getTypeBadge(resource.type)}</TableCell>
                  <TableCell>
                    <StatusBadge status={resource.state as any} />
                  </TableCell>
                  <TableCell>
                    <div className="text-sm text-muted-foreground">
                      {resource.metrics?.cpu !== undefined && (
                        <span>CPU: {formatPercentage(resource.metrics.cpu)}</span>
                      )}
                      {resource.metrics?.memory !== undefined && (
                        <span className="ml-3">Memory: {formatPercentage(resource.metrics.memory)}</span>
                      )}
                      {resource.metrics?.lastInvoked && (
                        <span>Last: {formatRelativeTime(resource.metrics.lastInvoked)}</span>
                      )}
                    </div>
                  </TableCell>
                  <TableCell className="text-right">
                    <div className="flex justify-end gap-2">
                      {resource.state === "stopped" && (
                        <Button variant="ghost" size="icon">
                          <Play className="h-4 w-4" />
                        </Button>
                      )}
                      {resource.state === "running" && (
                        <Button variant="ghost" size="icon">
                          <Square className="h-4 w-4" />
                        </Button>
                      )}
                      <Button variant="ghost" size="icon">
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
