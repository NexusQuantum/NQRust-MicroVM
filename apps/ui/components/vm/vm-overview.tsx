import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { StatusBadge } from "@/components/shared/status-badge"
import { Play, Square, Pause, Trash2, Power } from "lucide-react"
import { formatRelativeTime, formatPercentage } from "@/lib/utils/format"
import type { VM } from "@/lib/types"

interface VMOverviewProps {
  vm: VM
}

export function VMOverview({ vm }: VMOverviewProps) {
  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle>Status & Actions</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <span className="text-sm font-medium">Current State:</span>
              <StatusBadge status={vm.state} />
            </div>
            <div className="flex gap-2">
              {vm.state === "stopped" && (
                <Button>
                  <Play className="mr-2 h-4 w-4" />
                  Start
                </Button>
              )}
              {vm.state === "running" && (
                <>
                  <Button variant="outline">
                    <Pause className="mr-2 h-4 w-4" />
                    Pause
                  </Button>
                  <Button variant="outline">
                    <Square className="mr-2 h-4 w-4" />
                    Stop
                  </Button>
                  <Button variant="outline">
                    <Power className="mr-2 h-4 w-4" />
                    Ctrl-Alt-Del
                  </Button>
                </>
              )}
              {vm.state === "paused" && (
                <Button>
                  <Play className="mr-2 h-4 w-4" />
                  Resume
                </Button>
              )}
              <Button variant="destructive">
                <Trash2 className="mr-2 h-4 w-4" />
                Delete
              </Button>
            </div>
          </div>
        </CardContent>
      </Card>

      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">vCPU</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{vm.vcpu}</div>
            {vm.cpu_usage_percent !== undefined && (
              <p className="text-xs text-muted-foreground mt-1">{formatPercentage(vm.cpu_usage_percent)} usage</p>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Memory</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{vm.mem_mib} MiB</div>
            {vm.memory_usage_percent !== undefined && (
              <p className="text-xs text-muted-foreground mt-1">{formatPercentage(vm.memory_usage_percent)} usage</p>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Host</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-lg font-medium">{vm.host_addr}</div>
            <p className="text-xs text-muted-foreground mt-1">ID: {vm.host_id}</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Guest IP</CardTitle>
          </CardHeader>
          <CardContent>
            <code className="text-lg font-medium">{vm.guest_ip || "N/A"}</code>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Information</CardTitle>
        </CardHeader>
        <CardContent>
          <dl className="grid grid-cols-2 gap-4">
            <div>
              <dt className="text-sm font-medium text-muted-foreground">VM ID</dt>
              <dd className="mt-1 text-sm font-mono">{vm.id}</dd>
            </div>
            <div>
              <dt className="text-sm font-medium text-muted-foreground">Name</dt>
              <dd className="mt-1 text-sm">{vm.name}</dd>
            </div>
            <div>
              <dt className="text-sm font-medium text-muted-foreground">Created</dt>
              <dd className="mt-1 text-sm">{formatRelativeTime(vm.created_at)}</dd>
            </div>
            <div>
              <dt className="text-sm font-medium text-muted-foreground">Last Updated</dt>
              <dd className="mt-1 text-sm">{formatRelativeTime(vm.updated_at)}</dd>
            </div>
          </dl>
        </CardContent>
      </Card>
    </div>
  )
}
