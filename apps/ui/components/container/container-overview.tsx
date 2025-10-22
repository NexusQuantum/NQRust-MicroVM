import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { StatusBadge } from "@/components/shared/status-badge"
import { Play, Square, RotateCw, Trash2 } from "lucide-react"
import { formatDuration, formatPercentage } from "@/lib/utils/format"
import type { Container } from "@/lib/types"

interface ContainerOverviewProps {
  container: Container
}

export function ContainerOverview({ container }: ContainerOverviewProps) {
  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle>Status & Actions</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <span className="text-sm font-medium">Current Status:</span>
              <StatusBadge status={container.status} />
              {container.uptime_seconds && (
                <span className="text-sm text-muted-foreground">
                  Uptime: {formatDuration(container.uptime_seconds)}
                </span>
              )}
            </div>
            <div className="flex gap-2">
              {container.status === "stopped" && (
                <Button>
                  <Play className="mr-2 h-4 w-4" />
                  Start
                </Button>
              )}
              {container.status === "running" && (
                <>
                  <Button variant="outline">
                    <RotateCw className="mr-2 h-4 w-4" />
                    Restart
                  </Button>
                  <Button variant="outline">
                    <Square className="mr-2 h-4 w-4" />
                    Stop
                  </Button>
                </>
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
            <CardTitle className="text-sm font-medium text-muted-foreground">Image</CardTitle>
          </CardHeader>
          <CardContent>
            <code className="text-sm font-medium">{container.image}</code>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">CPU Usage</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {container.cpu_percent !== undefined ? formatPercentage(container.cpu_percent) : "N/A"}
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Memory</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {container.memory_used_mb}/{container.memory_limit_mb} MB
            </div>
            {container.memory_used_mb && container.memory_limit_mb && (
              <p className="text-xs text-muted-foreground mt-1">
                {formatPercentage((container.memory_used_mb / container.memory_limit_mb) * 100)} usage
              </p>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Container ID</CardTitle>
          </CardHeader>
          <CardContent>
            <code className="text-sm font-medium">{container.id}</code>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Port Mappings</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-2">
            {container.port_mappings.map((mapping, i) => (
              <div key={i} className="flex items-center gap-2 text-sm">
                <code className="bg-muted px-2 py-1 rounded">
                  {mapping.host}:{mapping.container} ({mapping.protocol})
                </code>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>

      {container.env_vars && Object.keys(container.env_vars).length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle>Environment Variables</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-2">
              {Object.entries(container.env_vars).map(([key, value]) => (
                <div key={key} className="flex items-center gap-2 text-sm">
                  <code className="font-medium">{key}:</code>
                  <code className="bg-muted px-2 py-1 rounded">{value}</code>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      )}

      {container.volumes && container.volumes.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle>Volumes</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-2">
              {container.volumes.map((volume, i) => (
                <div key={i} className="text-sm">
                  <code className="bg-muted px-2 py-1 rounded">
                    {volume.host} → {volume.container}
                  </code>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  )
}
