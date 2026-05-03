"use client";

import { useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { HardDrive, Loader2, Plus, RotateCw } from "lucide-react";
import { useStorageBackends } from "@/lib/queries";
import { BackendTable } from "@/components/storage/backend-table";
import { BackendCreateDialog } from "@/components/storage/backend-create-dialog";

export default function StoragePage() {
  const { data: backends = [], isLoading, error, refetch, isFetching } = useStorageBackends();
  const [showCreate, setShowCreate] = useState(false);

  const active = backends.filter((b) => !b.deleted_at);

  return (
    <div className="space-y-6">
      <div className="relative overflow-hidden rounded-xl border border-border bg-gradient-to-br from-blue-50 to-blue-100/50 dark:from-blue-950/30 dark:to-blue-900/20 p-8">
        <div className="relative z-10 flex items-center justify-between">
          <div className="max-w-xl">
            <h1 className="text-3xl font-bold text-foreground">Storage backends</h1>
            <p className="mt-2 text-muted-foreground">
              Where VM disks live. Configure local disk, NFS shares, iSCSI targets (generic or
              TrueNAS), or SPDK lvol pools. Each VM is created on a chosen backend.
            </p>
            <Button onClick={() => setShowCreate(true)} className="mt-4">
              <Plus className="mr-2 h-4 w-4" />
              Add backend
            </Button>
          </div>
          <div className="hidden lg:flex items-center justify-center h-32 w-32 rounded-full bg-gradient-to-br from-blue-500/20 to-blue-600/20 dark:from-blue-700/30 dark:to-blue-800/20">
            <HardDrive className="h-16 w-16 text-blue-600 dark:text-blue-400" />
          </div>
        </div>
        <div className="absolute right-0 top-0 h-64 w-64 translate-x-32 -translate-y-32 rounded-full bg-gradient-to-br from-blue-400/30 to-blue-600/30 dark:from-blue-500/20 dark:to-blue-600/10 blur-3xl" />
      </div>

      <Card>
        <CardHeader className="flex items-center justify-between">
          <CardTitle>Configured backends</CardTitle>
          <Button
            variant="outline"
            onClick={() => refetch()}
            disabled={isFetching}
            title="Refresh backend list"
          >
            {isFetching ? (
              <>
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                Refreshing...
              </>
            ) : (
              <>
                <RotateCw className="mr-2 h-4 w-4" />
                Refresh
              </>
            )}
          </Button>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
            </div>
          ) : error ? (
            <Alert variant="destructive">
              <AlertDescription>Failed to load storage backends. Please try again.</AlertDescription>
            </Alert>
          ) : active.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              <p>No storage backends configured yet.</p>
              <p className="text-sm mt-1">Add an external backend to host VM disks on NFS, iSCSI, or TrueNAS.</p>
            </div>
          ) : (
            <BackendTable backends={active} />
          )}
        </CardContent>
      </Card>

      <BackendCreateDialog open={showCreate} onOpenChange={setShowCreate} />
    </div>
  );
}
