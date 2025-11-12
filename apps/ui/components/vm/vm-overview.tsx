import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { StatusBadge } from "@/components/shared/status-badge"
import { Play, Square, Pause, Trash2, Power } from "lucide-react"
import { formatPercentage } from "@/lib/utils/format"
import type { Vm } from "@/lib/types"
import { useDateFormat } from "@/lib/hooks/use-date-format"

interface VMOverviewProps {
  vm: Vm
}

export function VMOverview({ vm }: VMOverviewProps) {
  const dateFormat = useDateFormat()
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
              <dd className="mt-1 text-sm">{dateFormat.formatRelative(vm.created_at)}</dd>
            </div>
            <div>
              <dt className="text-sm font-medium text-muted-foreground">Last Updated</dt>
              <dd className="mt-1 text-sm">{dateFormat.formatRelative(vm.updated_at)}</dd>
            </div>
            <div className="col-span-2">
              <dt className="text-sm font-medium text-muted-foreground">Rootfs Path</dt>
              <dd className="mt-1 text-sm font-mono break-all">{vm.rootfs_path}</dd>
            </div>
            <div className="col-span-2">
              <dt className="text-sm font-medium text-muted-foreground">Kernel Path</dt>
              <dd className="mt-1 text-sm font-mono break-all">{vm.kernel_path}</dd>
            </div>
            <div>
              <dt className="text-sm font-medium text-muted-foreground">Network TAP</dt>
              <dd className="mt-1 text-sm font-mono">{vm.tap || "N/A"}</dd>
            </div>
            <div>
              <dt className="text-sm font-medium text-muted-foreground">API Socket</dt>
              <dd className="mt-1 text-sm font-mono">{vm.api_sock}</dd>
            </div>
          </dl>
        </CardContent>
      </Card>
    </div>
  )
}
