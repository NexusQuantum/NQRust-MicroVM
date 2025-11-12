"use client"

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Loader2, Server } from "lucide-react"
import { useHosts } from "@/lib/queries"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { HostTable } from "@/components/host/host-table"

export default function HostsPage() {
  const { data: hosts = [], isLoading, error, refetch } = useHosts()

  return (
    <div className="space-y-6">
      <div className="relative overflow-hidden rounded-xl border border-border bg-gradient-to-br from-blue-50 to-blue-100/50 dark:from-blue-950/30 dark:to-blue-900/20 p-8">
        <div className="relative z-10 flex items-center justify-between">
          <div className="max-w-xl">
            <h1 className="text-3xl font-bold text-foreground">Hosts</h1>
            <p className="mt-2 text-muted-foreground">
              Monitor and manage compute hosts running the agent service. View resource metrics and VM distribution.
            </p>
          </div>
          <div className="hidden lg:flex items-center justify-center h-32 w-32 rounded-full bg-gradient-to-br from-blue-500/20 to-blue-600/20 dark:from-blue-700/30 dark:to-blue-800/20">
            <Server className="h-16 w-16 text-blue-600 dark:text-blue-400" />
          </div>
        </div>
        <div className="absolute right-0 top-0 h-64 w-64 translate-x-32 -translate-y-32 rounded-full bg-gradient-to-br from-blue-400/30 to-blue-600/30 dark:from-blue-500/20 dark:to-blue-600/10 blur-3xl" />
      </div>

      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <CardTitle>All Hosts</CardTitle>
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
                Failed to load hosts. Please try again.
              </AlertDescription>
            </Alert>
          ) : hosts.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              <p>No hosts found. Register an agent to get started.</p>
            </div>
          ) : (
            <HostTable hosts={hosts} />
          )}
        </CardContent>
      </Card>
    </div>
  )
}
