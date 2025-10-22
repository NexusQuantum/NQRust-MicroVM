"use client"

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Plus, Edit, Trash2 } from "lucide-react"
import { useVMNics } from "@/lib/queries"
import { Skeleton } from "@/components/ui/skeleton"
import { AlertCircle } from "lucide-react"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"

interface VMNetworkProps {
  vmId: string
}

export function VMNetwork({ vmId }: VMNetworkProps) {
  const { data: nics = [], isLoading, error } = useVMNics(vmId)

  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between">
        <CardTitle>Network Interfaces</CardTitle>
        <Button>
          <Plus className="mr-2 h-4 w-4" />
          Add NIC
        </Button>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <div className="space-y-4">
            {[...Array(1)].map((_, i) => (
              <div key={i} className="flex items-center space-x-4 p-4 border rounded">
                <Skeleton className="h-4 w-20" />
                <Skeleton className="h-4 w-32" />
                <Skeleton className="h-4 w-24" />
                <Skeleton className="h-8 w-20 ml-auto" />
              </div>
            ))}
          </div>
        ) : error ? (
          <Alert variant="destructive">
            <AlertCircle className="h-4 w-4" />
            <AlertTitle>Error</AlertTitle>
            <AlertDescription>
              Failed to load VM network interfaces. Please try again later.
            </AlertDescription>
          </Alert>
        ) : nics.length === 0 ? (
          <div className="text-center py-8 text-muted-foreground">
            No network interfaces configured for this VM.
          </div>
        ) : (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Interface ID</TableHead>
                <TableHead>Guest MAC</TableHead>
                <TableHead>Host Device</TableHead>
                <TableHead className="text-right">Actions</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {nics.map((nic) => (
                <TableRow key={nic.iface_id}>
                  <TableCell className="font-mono text-sm">{nic.iface_id}</TableCell>
                  <TableCell className="font-mono text-sm">{nic.guest_mac}</TableCell>
                  <TableCell className="font-mono text-sm">{nic.host_dev_name}</TableCell>
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
