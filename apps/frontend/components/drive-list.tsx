"use client"

import { useState } from "react"
import type { Vm, VmDrive } from "@/types/nexus"
import { useVMDrives, useDeleteVMDrive } from "@/lib/queries"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Skeleton } from "@/components/ui/skeleton"
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
  vm: Vm
}

export function DriveList({ vm }: DriveListProps) {
  const { data: drives = [], isLoading } = useVMDrives(vm.id)
  const deleteDrive = useDeleteVMDrive()
  const [selectedDrive, setSelectedDrive] = useState<VmDrive | null>(null)
  const [isEditorOpen, setIsEditorOpen] = useState(false)
  const [editorMode, setEditorMode] = useState<"create" | "edit" | "rate-limit">("create")

  // Drives are database-backed and applied during VM start/restart
  const canCreateDrives = true // Can manage drives at any time - changes take effect on restart
  const canEditRateLimits = false // Rate limiters for drives not fully implemented yet

  const handleCreateDrive = () => {
    setSelectedDrive(null)
    setEditorMode("create")
    setIsEditorOpen(true)
  }

  const handleEditDrive = (drive: VmDrive) => {
    setSelectedDrive(drive)
    setEditorMode("edit")
    setIsEditorOpen(true)
  }

  const handleEditRateLimits = (drive: VmDrive) => {
    setSelectedDrive(drive)
    setEditorMode("rate-limit")
    setIsEditorOpen(true)
  }

  const handleDeleteDrive = (drive: VmDrive) => {
    if (confirm(`Are you sure you want to delete drive "${drive.drive_id}"?`)) {
      deleteDrive.mutate({ vmId: vm.id, driveId: drive.id })
    }
  }

  if (isLoading) {
    return (
      <Card>
        <CardHeader>
          <div className="flex items-center gap-2">
            <HardDrive className="h-5 w-5" />
            <CardTitle>Storage Drives</CardTitle>
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
          title="Drive Changes on Restart"
          message="Drives are stored in the database and will be attached when the VM starts. Any changes you make will take effect on the next VM restart."
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
                              <DropdownMenuItem
                                className="text-destructive"
                                onClick={() => handleDeleteDrive(drive)}
                              >
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
        drive={selectedDrive as any}
        mode={editorMode}
        vmState={vm.state as any}
        vmId={vm.id}
      />
    </div>
  )
}
