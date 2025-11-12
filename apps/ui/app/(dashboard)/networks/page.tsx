"use client"

import { useState } from "react"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Loader2, Network, Plus } from "lucide-react"
import { useNetworks } from "@/lib/queries"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { NetworkTable } from "@/components/network/network-table"
import { NetworkCreateDialog } from "@/components/network/network-create-dialog"

export default function NetworksPage() {
  const { data: networks = [], isLoading, error, refetch } = useNetworks()
  const [showCreateDialog, setShowCreateDialog] = useState(false)

  return (
    <div className="space-y-6">
      <div className="relative overflow-hidden rounded-xl border border-border bg-gradient-to-br from-teal-50 to-teal-100/50 dark:from-teal-950/30 dark:to-teal-900/20 p-8">
        <div className="relative z-10 flex items-center justify-between">
          <div className="max-w-xl">
            <h1 className="text-3xl font-bold text-foreground">Networks</h1>
            <p className="mt-2 text-muted-foreground">
              Manage virtual networks for VM isolation. Create bridge networks or VLAN-based networks for advanced segmentation.
            </p>
          </div>
          <div className="hidden lg:flex items-center justify-center h-32 w-32 rounded-full bg-gradient-to-br from-teal-500/20 to-teal-600/20 dark:from-teal-700/30 dark:to-teal-800/20">
            <Network className="h-16 w-16 text-teal-600 dark:text-teal-400" />
          </div>
        </div>
        <div className="absolute right-0 top-0 h-64 w-64 translate-x-32 -translate-y-32 rounded-full bg-gradient-to-br from-teal-400/30 to-teal-600/30 dark:from-teal-500/20 dark:to-teal-600/10 blur-3xl" />
      </div>

      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <CardTitle>All Networks</CardTitle>
          <div className="flex gap-2">
            <Button onClick={() => setShowCreateDialog(true)}>
              <Plus className="mr-2 h-4 w-4" />
              Create Network
            </Button>
            <Button variant="outline" size="sm" onClick={() => refetch()}>
              Refresh
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
            </div>
          ) : error ? (
            <Alert variant="destructive">
              <AlertDescription>
                Failed to load networks. Please try again.
              </AlertDescription>
            </Alert>
          ) : networks.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              <p>No networks found.</p>
              <p className="text-sm mt-1">Networks are automatically registered when VMs are created.</p>
            </div>
          ) : (
            <NetworkTable networks={networks} />
          )}
        </CardContent>
      </Card>

      <NetworkCreateDialog open={showCreateDialog} onOpenChange={setShowCreateDialog} />
    </div>
  )
}
