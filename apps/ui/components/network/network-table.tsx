"use client"

import { useState } from "react"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Badge } from "@/components/ui/badge"
import { Search, Trash2, Tag } from "lucide-react"
import type { Network } from "@/lib/types"
import { useDeleteNetwork } from "@/lib/queries"
import { formatDistanceToNow } from "date-fns"

interface NetworkTableProps {
  networks: Network[]
}

export function NetworkTable({ networks }: NetworkTableProps) {
  const [searchQuery, setSearchQuery] = useState("")
  const [typeFilter, setTypeFilter] = useState<string>("all")

  const deleteNetwork = useDeleteNetwork()

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
            <SelectItem value="bridge">Bridge</SelectItem>
            <SelectItem value="vlan">VLAN</SelectItem>
          </SelectContent>
        </Select>
      </div>

      <div className="rounded-lg border border-border">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Name</TableHead>
              <TableHead>Type</TableHead>
              <TableHead>Bridge</TableHead>
              <TableHead>VLAN ID</TableHead>
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
                <TableCell colSpan={9} className="text-center py-8 text-muted-foreground">
                  No networks found
                </TableCell>
              </TableRow>
            ) : (
              filteredNetworks.map((network) => (
                <TableRow key={network.id}>
                  <TableCell className="font-medium">{network.name}</TableCell>
                  <TableCell>
                    <Badge variant={network.type === "vlan" ? "default" : "secondary"}>
                      {network.type === "vlan" && <Tag className="h-3 w-3 mr-1" />}
                      {network.type.toUpperCase()}
                    </Badge>
                  </TableCell>
                  <TableCell>
                    <code className="text-xs bg-muted px-1.5 py-0.5 rounded">{network.bridge_name}</code>
                  </TableCell>
                  <TableCell>
                    {network.vlan_id ? (
                      <Badge variant="outline">{network.vlan_id}</Badge>
                    ) : (
                      <span className="text-muted-foreground text-sm">N/A</span>
                    )}
                  </TableCell>
                  <TableCell>
                    {network.cidr ? (
                      <code className="text-xs bg-muted px-1.5 py-0.5 rounded">{network.cidr}</code>
                    ) : (
                      <span className="text-muted-foreground text-sm">N/A</span>
                    )}
                  </TableCell>
                  <TableCell className="text-sm text-muted-foreground">
                    {network.host_name || network.host_id.slice(0, 8)}
                  </TableCell>
                  <TableCell>
                    <Badge variant="secondary">{network.vm_count} VMs</Badge>
                  </TableCell>
                  <TableCell className="text-sm text-muted-foreground">
                    {formatDistanceToNow(new Date(network.created_at), { addSuffix: true })}
                  </TableCell>
                  <TableCell className="text-right">
                    <Button
                      variant="ghost"
                      size="icon"
                      title="Delete"
                      onClick={() => {
                        if (network.vm_count > 0) {
                          alert("Cannot delete network with active VMs")
                          return
                        }
                        if (confirm(`Are you sure you want to delete network "${network.name}"?`)) {
                          deleteNetwork.mutate(network.id)
                        }
                      }}
                      disabled={deleteNetwork.isPending || network.vm_count > 0}
                    >
                      <Trash2 className="h-4 w-4" />
                    </Button>
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
