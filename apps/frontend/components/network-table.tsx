"use client"

import { useState } from "react"
import type { VM, NetworkConfig } from "@/types/firecracker"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { AlertBanner } from "@/components/alert-banner"
import { NicEditorDialog } from "@/components/nic-editor-dialog"
import { Network, Plus, MoreHorizontal, Edit, Trash2, Settings, Wifi } from "lucide-react"
import { formatBytes } from "@/lib/utils"

interface NetworkTableProps {
  vm: VM
}

// Mock network interface data
const mockNetworkInterfaces: NetworkConfig[] = [
  {
    iface_id: "eth0",
    host_dev_name: "tap0",
    guest_mac: "02:FC:00:00:00:05",
    allow_mmds_requests: true,
    rx_rate_limiter: {
      size: 1000000,
      one_time_burst: 1000000,
      refill_time: 100,
    },
    tx_rate_limiter: {
      size: 1000000,
      one_time_burst: 1000000,
      refill_time: 100,
    },
  },
]

export function NetworkTable({ vm }: NetworkTableProps) {
  const [interfaces] = useState<NetworkConfig[]>(mockNetworkInterfaces)
  const [selectedInterface, setSelectedInterface] = useState<NetworkConfig | null>(null)
  const [isEditorOpen, setIsEditorOpen] = useState(false)
  const [editorMode, setEditorMode] = useState<"create" | "edit" | "rate-limit">("create")

  const canCreateInterfaces = vm.state === "stopped"
  const canEditRateLimits = vm.state === "running"

  const handleCreateInterface = () => {
    setSelectedInterface(null)
    setEditorMode("create")
    setIsEditorOpen(true)
  }

  const handleEditInterface = (iface: NetworkConfig) => {
    setSelectedInterface(iface)
    setEditorMode("edit")
    setIsEditorOpen(true)
  }

  const handleEditRateLimits = (iface: NetworkConfig) => {
    setSelectedInterface(iface)
    setEditorMode("rate-limit")
    setIsEditorOpen(true)
  }

  const generateMacAddress = () => {
    const mac = Array.from({ length: 6 }, () =>
      Math.floor(Math.random() * 256)
        .toString(16)
        .padStart(2, "0"),
    ).join(":")
    return `02:FC:${mac.slice(6)}`
  }

  return (
    <div className="space-y-6">
      {/* Guardrail Alerts */}
      {vm.state === "running" && (
        <AlertBanner
          type="info"
          title="Runtime Mode"
          message="VM is running. You can only modify rate limiters. Stop the VM to create or modify network interfaces."
        />
      )}

      {vm.state === "paused" && (
        <AlertBanner
          type="warning"
          title="VM Paused"
          message="VM is paused. Stop the VM to modify network configuration."
        />
      )}

      {/* Header */}
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Network className="h-5 w-5" />
              <CardTitle>Network Interfaces</CardTitle>
            </div>
            <Button onClick={handleCreateInterface} disabled={!canCreateInterfaces} size="sm">
              <Plus className="h-4 w-4 mr-2" />
              Add Interface
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          {interfaces.length === 0 ? (
            <div className="text-center py-8">
              <Wifi className="h-12 w-12 text-muted-foreground mx-auto mb-4" />
              <h3 className="text-lg font-medium">No network interfaces</h3>
              <p className="text-muted-foreground mb-4">Add network interfaces to enable VM connectivity</p>
              <Button onClick={handleCreateInterface} disabled={!canCreateInterfaces}>
                <Plus className="h-4 w-4 mr-2" />
                Add First Interface
              </Button>
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Interface ID</TableHead>
                  <TableHead>Host Device</TableHead>
                  <TableHead>Guest MAC</TableHead>
                  <TableHead>MMDS</TableHead>
                  <TableHead>RX Rate Limit</TableHead>
                  <TableHead>TX Rate Limit</TableHead>
                  <TableHead className="w-[50px]"></TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {interfaces.map((iface) => (
                  <TableRow key={iface.iface_id}>
                    <TableCell className="font-medium">{iface.iface_id}</TableCell>
                    <TableCell>
                      <code className="text-xs bg-muted px-1 py-0.5 rounded">{iface.host_dev_name}</code>
                    </TableCell>
                    <TableCell>
                      <code className="text-xs bg-muted px-1 py-0.5 rounded font-mono">
                        {iface.guest_mac || "Auto-generated"}
                      </code>
                    </TableCell>
                    <TableCell>
                      <Badge variant={iface.allow_mmds_requests ? "default" : "secondary"}>
                        {iface.allow_mmds_requests ? "Enabled" : "Disabled"}
                      </Badge>
                    </TableCell>
                    <TableCell>
                      {iface.rx_rate_limiter ? (
                        <div className="text-xs">{formatBytes(iface.rx_rate_limiter.size)}/s</div>
                      ) : (
                        <span className="text-muted-foreground text-sm">None</span>
                      )}
                    </TableCell>
                    <TableCell>
                      {iface.tx_rate_limiter ? (
                        <div className="text-xs">{formatBytes(iface.tx_rate_limiter.size)}/s</div>
                      ) : (
                        <span className="text-muted-foreground text-sm">None</span>
                      )}
                    </TableCell>
                    <TableCell>
                      <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                          <Button variant="ghost" size="icon" className="h-8 w-8">
                            <MoreHorizontal className="h-4 w-4" />
                            <span className="sr-only">Open menu</span>
                          </Button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end">
                          {canEditRateLimits && (
                            <DropdownMenuItem onClick={() => handleEditRateLimits(iface)}>
                              <Settings className="h-4 w-4" />
                              Edit Rate Limits
                            </DropdownMenuItem>
                          )}
                          {canCreateInterfaces && (
                            <>
                              <DropdownMenuItem onClick={() => handleEditInterface(iface)}>
                                <Edit className="h-4 w-4" />
                                Edit Interface
                              </DropdownMenuItem>
                              <DropdownMenuSeparator />
                              <DropdownMenuItem className="text-destructive">
                                <Trash2 className="h-4 w-4" />
                                Remove Interface
                              </DropdownMenuItem>
                            </>
                          )}
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      {/* Network Interface Editor Dialog */}
      <NicEditorDialog
        open={isEditorOpen}
        onOpenChange={setIsEditorOpen}
        networkInterface={selectedInterface}
        mode={editorMode}
        vmState={vm.state}
        onGenerateMac={generateMacAddress}
      />
    </div>
  )
}
