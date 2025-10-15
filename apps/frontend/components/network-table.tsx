"use client"

import { useMemo, useState } from "react"
import type { Vm, VmNic } from "@/types/nexus"
import { useVMNics, useDeleteVMNic } from "@/lib/queries"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Skeleton } from "@/components/ui/skeleton"
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
  vm: Vm
}

export function NetworkTable({ vm }: NetworkTableProps) {
  const { data: interfaces = [], isLoading } = useVMNics(vm.id)
  const deleteNic = useDeleteVMNic()
  const [selectedInterface, setSelectedInterface] = useState<VmNic | null>(null)
  const [isEditorOpen, setIsEditorOpen] = useState(false)
  const [editorMode, setEditorMode] = useState<"create" | "edit" | "rate-limit">("create")

  const canCreateInterfaces = true // NICs are database-backed, can be managed at any time
  const canEditRateLimits = true // Rate limiters can be updated at any time

  const nextIfaceId = useMemo(() => {
    const ids = interfaces.map((iface) => iface.iface_id.toLowerCase())
    let index = 1
    while (ids.includes(`eth${index}`)) {
      index += 1
    }
    return `eth${index}`
  }, [interfaces])

  const nextHostDevice = useMemo(() => {
    const existingHosts = interfaces.map((iface) => iface.host_dev_name.toLowerCase())
    const ifaceSuffix = nextIfaceId.replace(/^eth/i, "") || "1"

    const generate = (attempt: number) => {
      const extra = attempt === 0 ? "" : `${attempt}`
      const suffix = `${ifaceSuffix}${extra}`
      const maxPrefixLen = Math.max(0, 15 - suffix.length)
      const prefix = vm.tap.slice(0, maxPrefixLen)
      return `${prefix}${suffix}`.slice(0, 15)
    }

    let attempt = 0
    while (attempt < 100) {
      const candidate = generate(attempt)
      if (!existingHosts.includes(candidate.toLowerCase()) && candidate.length >= 3) {
        return candidate
      }
      attempt += 1
    }
    return vm.tap.slice(0, 15)
  }, [interfaces, nextIfaceId, vm.tap])

  const handleCreateInterface = () => {
    setSelectedInterface(null)
    setEditorMode("create")
    setIsEditorOpen(true)
  }

  const handleEditInterface = (iface: VmNic) => {
    setSelectedInterface(iface)
    setEditorMode("edit")
    setIsEditorOpen(true)
  }

  const handleEditRateLimits = (iface: VmNic) => {
    setSelectedInterface(iface)
    setEditorMode("rate-limit")
    setIsEditorOpen(true)
  }

  const handleDeleteInterface = (iface: VmNic) => {
    if (confirm(`Are you sure you want to delete interface "${iface.iface_id}"?`)) {
      deleteNic.mutate({ vmId: vm.id, nicId: iface.id })
    }
  }

  const generateMacAddress = () => {
    const mac = Array.from({ length: 6 }, () =>
      Math.floor(Math.random() * 256)
        .toString(16)
        .padStart(2, "0"),
    ).join(":")
    return `02:FC:${mac.slice(6)}`
  }

  if (isLoading) {
    return (
      <Card>
        <CardHeader>
          <div className="flex items-center gap-2">
            <Network className="h-5 w-5" />
            <CardTitle>Network Interfaces</CardTitle>
          </div>
        </CardHeader>
        <CardContent className="space-y-4">
          <Skeleton className="h-12 w-full" />
          <Skeleton className="h-12 w-full" />
        </CardContent>
      </Card>
    )
  }

  return (
    <div className="space-y-6">
      {/* Info Alert */}
      {(vm.state === "running" || vm.state === "paused") && (
        <AlertBanner
          type="info"
          title="Network Changes on Restart"
          message="Network interfaces are stored in the database and will be attached when the VM starts. Any changes you make will take effect on the next VM restart."
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
                      <Badge variant="secondary">N/A</Badge>
                    </TableCell>
                    <TableCell>
                      {iface.rx_rate_limiter ? (
                        <div className="text-xs">
                          {iface.rx_rate_limiter.size ? formatBytes(iface.rx_rate_limiter.size) + "/s" : "Configured"}
                        </div>
                      ) : (
                        <span className="text-muted-foreground text-sm">None</span>
                      )}
                    </TableCell>
                    <TableCell>
                      {iface.tx_rate_limiter ? (
                        <div className="text-xs">
                          {iface.tx_rate_limiter.size ? formatBytes(iface.tx_rate_limiter.size) + "/s" : "Configured"}
                        </div>
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
                              <DropdownMenuItem
                                className="text-destructive"
                                onClick={() => handleDeleteInterface(iface)}
                              >
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
        networkInterface={editorMode === "create"
          ? {
              iface_id: nextIfaceId,
              host_dev_name: nextHostDevice,
              guest_mac: "",
              allow_mmds_requests: true,
              rx_rate_limiter: {
                size: 125000000,
                one_time_burst: 125000000,
                refill_time: 1000,
              },
              tx_rate_limiter: {
                size: 125000000,
                one_time_burst: 125000000,
                refill_time: 1000,
              },
            }
          : (selectedInterface as any)}
        mode={editorMode}
        vmState={vm.state as any}
        vmId={vm.id}
        onGenerateMac={generateMacAddress}
      />
    </div>
  )
}
