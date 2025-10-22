"use client"

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Badge } from "@/components/ui/badge"
import { Plus, Edit, Trash2 } from "lucide-react"
import { useVMDrives } from "@/lib/queries"
import { Skeleton } from "@/components/ui/skeleton"
import { AlertCircle } from "lucide-react"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"

interface VMStorageProps {
  vmId: string
}

export function VMStorage({ vmId }: VMStorageProps) {
  const { data: drives = [], isLoading, error } = useVMDrives(vmId)

  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between">
        <CardTitle>Attached Drives</CardTitle>
        <Button>
          <Plus className="mr-2 h-4 w-4" />
          Add Drive
        </Button>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <div className="space-y-4">
            {[...Array(2)].map((_, i) => (
              <div key={i} className="flex items-center space-x-4 p-4 border rounded">
                <Skeleton className="h-4 w-20" />
                <Skeleton className="h-4 w-40" />
                <Skeleton className="h-4 w-16" />
                <Skeleton className="h-4 w-16" />
                <Skeleton className="h-8 w-20 ml-auto" />
              </div>
            ))}
          </div>
        ) : error ? (
          <Alert variant="destructive">
            <AlertCircle className="h-4 w-4" />
            <AlertTitle>Error</AlertTitle>
            <AlertDescription>
              Failed to load VM drives. Please try again later.
            </AlertDescription>
          </Alert>
        ) : drives.length === 0 ? (
          <div className="text-center py-8 text-muted-foreground">
            No drives attached to this VM.
          </div>
        ) : (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Drive ID</TableHead>
                <TableHead>Path</TableHead>
                <TableHead>Root Device</TableHead>
                <TableHead>Read Only</TableHead>
                <TableHead className="text-right">Actions</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {drives.map((drive) => (
                <TableRow key={drive.drive_id}>
                  <TableCell className="font-mono text-sm">{drive.drive_id}</TableCell>
                  <TableCell className="font-mono text-sm">{drive.path_on_host}</TableCell>
                  <TableCell>
                    {drive.is_root_device ? (
                      <Badge variant="outline" className="bg-blue-100 text-blue-700 border-blue-200">
                        Root
                      </Badge>
                    ) : (
                      <span className="text-muted-foreground">No</span>
                    )}
                  </TableCell>
                  <TableCell>{drive.is_read_only ? "Yes" : "No"}</TableCell>
                  <TableCell className="text-right">
                    <div className="flex justify-end gap-2">
                      <Button variant="ghost" size="icon">
                        <Edit className="h-4 w-4" />
                      </Button>
                      <Button variant="ghost" size="icon">
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    </div>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        )}
      </CardContent>
    </Card>
  )
}
