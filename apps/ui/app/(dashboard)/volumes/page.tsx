"use client"

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Loader2, HardDrive } from "lucide-react"
import { useVolumes } from "@/lib/queries"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { VolumeTable } from "@/components/volume/volume-table"

export default function VolumesPage() {
  const { data: volumes = [], isLoading, error, refetch } = useVolumes()

  return (
    <div className="space-y-6">
      <div className="relative overflow-hidden rounded-xl border border-border bg-gradient-to-br from-indigo-50 to-indigo-100/50 dark:from-indigo-950/30 dark:to-indigo-900/20 p-8">
        <div className="relative z-10 flex items-center justify-between">
          <div className="max-w-xl">
            <h1 className="text-3xl font-bold text-foreground">Volumes</h1>
            <p className="mt-2 text-muted-foreground">
              Manage persistent block storage volumes. Create volumes independently and attach them to VMs as needed.
            </p>
          </div>
          <div className="hidden lg:flex items-center justify-center h-32 w-32 rounded-full bg-gradient-to-br from-indigo-500/20 to-indigo-600/20 dark:from-indigo-700/30 dark:to-indigo-800/20">
            <HardDrive className="h-16 w-16 text-indigo-600 dark:text-indigo-400" />
          </div>
        </div>
        <div className="absolute right-0 top-0 h-64 w-64 translate-x-32 -translate-y-32 rounded-full bg-gradient-to-br from-indigo-400/30 to-indigo-600/30 dark:from-indigo-500/20 dark:to-indigo-600/10 blur-3xl" />
      </div>

      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <CardTitle>All Volumes</CardTitle>
          <Button variant="outline" size="sm" onClick={() => refetch()}>
            Refresh
          </Button>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
            </div>
          ) : error ? (
            <Alert variant="destructive">
              <AlertDescription>
                Failed to load volumes. Please try again.
              </AlertDescription>
            </Alert>
          ) : volumes.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              <p>No volumes found. Create your first volume to get started.</p>
            </div>
          ) : (
            <VolumeTable volumes={volumes} />
          )}
        </CardContent>
      </Card>
    </div>
  )
}
