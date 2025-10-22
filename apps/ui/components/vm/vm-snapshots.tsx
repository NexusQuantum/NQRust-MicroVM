"use client"

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Badge } from "@/components/ui/badge"
import { Plus, RotateCcw, Trash2 } from "lucide-react"
import { formatRelativeTime } from "@/lib/utils/format"
import { useSnapshots } from "@/lib/queries"
import { Skeleton } from "@/components/ui/skeleton"
import { AlertCircle } from "lucide-react"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"

interface VMSnapshotsProps {
  vmId: string
}

export function VMSnapshots({ vmId }: VMSnapshotsProps) {
  const { data: snapshots = [], isLoading, error } = useSnapshots(vmId)

  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between">
        <CardTitle>Snapshots</CardTitle>
        <Button>
          <Plus className="mr-2 h-4 w-4" />
          Create Snapshot
        </Button>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <div className="space-y-4">
            {[...Array(2)].map((_, i) => (
              <div key={i} className="flex items-center space-x-4 p-4 border rounded">
                <Skeleton className="h-4 w-24" />
                <Skeleton className="h-6 w-16" />
                <Skeleton className="h-4 w-20" />
                <Skeleton className="h-8 w-24 ml-auto" />
              </div>
            ))}
          </div>
        ) : error ? (
          <Alert variant="destructive">
            <AlertCircle className="h-4 w-4" />
            <AlertTitle>Error</AlertTitle>
            <AlertDescription>
              Failed to load VM snapshots. Please try again later.
            </AlertDescription>
          </Alert>
        ) : snapshots.length === 0 ? (
          <div className="text-center py-8 text-muted-foreground">
            No snapshots available for this VM.
          </div>
        ) : (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Name</TableHead>
                <TableHead>Type</TableHead>
                <TableHead>Created</TableHead>
                <TableHead className="text-right">Actions</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {snapshots.map((snapshot) => (
                <TableRow key={snapshot.id}>
                  <TableCell className="font-mono text-sm">{snapshot.id}</TableCell>
                  <TableCell>
                    <Badge
                      variant="outline"
                      className={
                        snapshot.snapshot_type === "Full"
                          ? "bg-blue-100 text-blue-700 border-blue-200"
                          : "bg-purple-100 text-purple-700 border-purple-200"
                      }
                    >
                      {snapshot.snapshot_type}
                    </Badge>
                  </TableCell>
                  <TableCell className="text-sm text-muted-foreground">
                    {formatRelativeTime(snapshot.created_at)}
                  </TableCell>
                  <TableCell className="text-right">
                    <div className="flex justify-end gap-2">
                      <Button variant="outline" size="sm">
                        <RotateCcw className="mr-2 h-4 w-4" />
                        Restore
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
