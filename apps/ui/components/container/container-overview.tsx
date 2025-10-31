import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { ExternalLink } from "lucide-react"
import type { Container } from "@/lib/types"
import Link from "next/link"

interface ContainerOverviewProps {
  container: Container
  vmId?: string | null
}

export function ContainerOverview({ container, vmId }: ContainerOverviewProps) {
  return (
    <div className="space-y-6">
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
            <CardTitle className="text-sm font-medium text-muted-foreground">CPU</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {container.cpu_limit !== undefined ? `${container.cpu_limit} vCPU` : "N/A"}
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Memory</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {container.memory_limit_mb !== undefined ? `${container.memory_limit_mb} MB` : "N/A"}
            </div>
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

      {vmId && (
        <Card>
          <CardHeader>
            <CardTitle>Container VM</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex items-center justify-between">
              <div className="space-y-1">
                <p className="text-sm text-muted-foreground">
                  This container is running in a dedicated microVM
                </p>
                <code className="text-xs bg-muted px-2 py-1 rounded">{container.container_runtime_id}</code>
              </div>
              <Button variant="outline" asChild>
                <Link href={`/vms/${vmId}`}>
                  <ExternalLink className="mr-2 h-4 w-4" />
                  View VM
                </Link>
              </Button>
            </div>
          </CardContent>
        </Card>
      )}

      <Card>
        <CardHeader>
          <CardTitle>Network & Access</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          {container.guest_ip && (
            <div className="space-y-2">
              <div className="text-sm font-medium text-muted-foreground">VM IP Address</div>
              <code className="bg-muted px-2 py-1 rounded text-sm font-medium">{container.guest_ip}</code>
            </div>
          )}
          
          {container.port_mappings && container.port_mappings.length > 0 && (
            <div className="space-y-2">
              <div className="text-sm font-medium text-muted-foreground">Port Mappings</div>
              <div className="space-y-2">
                {container.port_mappings.map((mapping, i) => {
                  const connectionInfo = container.guest_ip 
                    ? `${container.guest_ip}:${mapping.host}`
                    : `localhost:${mapping.host}`;
                  return (
                    <div key={i} className="flex flex-col gap-1">
                      <div className="flex items-center gap-2 text-sm">
                        <code className="bg-muted px-2 py-1 rounded">
                          {mapping.host}:{mapping.container} ({mapping.protocol})
                        </code>
                      </div>
                      {container.state === "running" && container.guest_ip && (
                        <div className="text-xs text-muted-foreground pl-1">
                          Access at: <code className="bg-muted px-1.5 py-0.5 rounded">{connectionInfo}</code>
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>
            </div>
          )}
          
          {!container.guest_ip && (!container.port_mappings || container.port_mappings.length === 0) && (
            <p className="text-sm text-muted-foreground">
              No network information available
            </p>
          )}
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
