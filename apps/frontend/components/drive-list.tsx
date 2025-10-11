"use client"

import { useState } from "react"
import type { VM, DriveConfig } from "@/types/firecracker"
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
import { DriveEditorDialog } from "@/components/drive-editor-dialog"
import { HardDrive, Plus, MoreHorizontal, Edit, Trash2, Settings, Database } from "lucide-react"
import { formatBytes } from "@/lib/utils"

interface DriveListProps {
  vm: VM
}

// Mock drive data - in real implementation this would come from the VM config
const mockDrives: DriveConfig[] = [
  {
    drive_id: "root",
    path_on_host: "/images/ubuntu-22.04-rootfs.ext4",
    is_root_device: true,
    is_read_only: false,
    cache_type: "Unsafe",
    io_engine: "Async",
    rate_limiter: {
      bandwidth: {
        size: 1000000,
        one_time_burst: 1000000,
        refill_time: 100,
      },
      ops: {
        size: 1000,
        one_time_burst: 1000,
        refill_time: 100,
      },
    },
  },
  {
    drive_id: "data",
    path_on_host: "/data/vm-data.ext4",
    is_root_device: false,
    is_read_only: false,
    cache_type: "Writeback",
    io_engine: "Sync",
  },
]

export function DriveList({ vm }: DriveListProps) {
  const [drives] = useState<DriveConfig[]>(mockDrives)
  const [selectedDrive, setSelectedDrive] = useState<DriveConfig | null>(null)
  const [isEditorOpen, setIsEditorOpen] = useState(false)
  const [editorMode, setEditorMode] = useState<"create" | "edit" | "rate-limit">("create")

  const canCreateDrives = vm.state === "stopped"
  const canEditRateLimits = vm.state === "running"

  const handleCreateDrive = () => {
    setSelectedDrive(null)
    setEditorMode("create")
    setIsEditorOpen(true)
  }

  const handleEditDrive = (drive: DriveConfig) => {
    setSelectedDrive(drive)
    setEditorMode("edit")
    setIsEditorOpen(true)
  }

  const handleEditRateLimits = (drive: DriveConfig) => {
    setSelectedDrive(drive)
    setEditorMode("rate-limit")
    setIsEditorOpen(true)
  }

  return (
    <div className="space-y-6">
      {/* Guardrail Alerts */}
      {vm.state === "running" && (
        <AlertBanner
          type="info"
          title="Runtime Mode"
          message="VM is running. You can only modify rate limiters. Stop the VM to create or modify drives."
        />
      )}

      {vm.state === "paused" && (
        <AlertBanner
          type="warning"
          title="VM Paused"
          message="VM is paused. Stop the VM to modify storage configuration."
        />
      )}

      {/* Header */}
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <HardDrive className="h-5 w-5" />
              <CardTitle>Storage Drives</CardTitle>
            </div>
            <Button onClick={handleCreateDrive} disabled={!canCreateDrives} size="sm">
              <Plus className="h-4 w-4 mr-2" />
              Add Drive
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          {drives.length === 0 ? (
            <div className="text-center py-8">
              <Database className="h-12 w-12 text-muted-foreground mx-auto mb-4" />
              <h3 className="text-lg font-medium">No drives configured</h3>
              <p className="text-muted-foreground mb-4">Add storage drives to provide persistent storage for your VM</p>
              <Button onClick={handleCreateDrive} disabled={!canCreateDrives}>
                <Plus className="h-4 w-4 mr-2" />
                Add First Drive
              </Button>
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Drive ID</TableHead>
                  <TableHead>Path</TableHead>
                  <TableHead>Type</TableHead>
                  <TableHead>Cache</TableHead>
                  <TableHead>I/O Engine</TableHead>
                  <TableHead>Rate Limits</TableHead>
                  <TableHead className="w-[50px]"></TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {drives.map((drive) => (
                  <TableRow key={drive.drive_id}>
                    <TableCell className="font-medium">
                      <div className="flex items-center gap-2">
                        {drive.drive_id}
                        {drive.is_root_device && (
                          <Badge variant="default" className="text-xs">
                            Root
                          </Badge>
                        )}
                        {drive.is_read_only && (
                          <Badge variant="secondary" className="text-xs">
                            Read-only
                          </Badge>
                        )}
                      </div>
                    </TableCell>
                    <TableCell>
                      <code className="text-xs bg-muted px-1 py-0.5 rounded break-all">{drive.path_on_host}</code>
                    </TableCell>
                    <TableCell>
                      <Badge variant="outline">{drive.is_root_device ? "Root Device" : "Data Drive"}</Badge>
                    </TableCell>
                    <TableCell>
                      <Badge variant="secondary">{drive.cache_type}</Badge>
                    </TableCell>
                    <TableCell>
                      <Badge variant="secondary">{drive.io_engine}</Badge>
                    </TableCell>
                    <TableCell>
                      {drive.rate_limiter ? (
                        <div className="space-y-1">
                          {drive.rate_limiter.bandwidth && (
                            <div className="text-xs">BW: {formatBytes(drive.rate_limiter.bandwidth.size)}/s</div>
                          )}
                          {drive.rate_limiter.ops && <div className="text-xs">IOPS: {drive.rate_limiter.ops.size}</div>}
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
                            <DropdownMenuItem onClick={() => handleEditRateLimits(drive)}>
                              <Settings className="h-4 w-4" />
                              Edit Rate Limits
                            </DropdownMenuItem>
                          )}
                          {canCreateDrives && (
                            <>
                              <DropdownMenuItem onClick={() => handleEditDrive(drive)}>
                                <Edit className="h-4 w-4" />
                                Edit Drive
                              </DropdownMenuItem>
                              <DropdownMenuSeparator />
                              <DropdownMenuItem className="text-destructive">
                                <Trash2 className="h-4 w-4" />
                                Remove Drive
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

      {/* Drive Editor Dialog */}
      <DriveEditorDialog
        open={isEditorOpen}
        onOpenChange={setIsEditorOpen}
        drive={selectedDrive}
        mode={editorMode}
        vmState={vm.state}
      />
    </div>
  )
}
