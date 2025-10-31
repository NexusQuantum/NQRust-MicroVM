import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import type { Container } from "@/lib/types"

interface ContainerConfigProps {
  container: Container
}

export function ContainerConfig({ container }: ContainerConfigProps) {
  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle>Basic Configuration</CardTitle>
        </CardHeader>
        <CardContent>
          <dl className="grid grid-cols-2 gap-4">
            <div>
              <dt className="text-sm font-medium text-muted-foreground">Name</dt>
              <dd className="mt-1 text-sm">{container.name}</dd>
            </div>
            <div>
              <dt className="text-sm font-medium text-muted-foreground">Image</dt>
              <dd className="mt-1 text-sm">
                <code className="bg-muted px-2 py-1 rounded">{container.image}</code>
              </dd>
            </div>
            <div>
              <dt className="text-sm font-medium text-muted-foreground">Command</dt>
              <dd className="mt-1 text-sm">
                <code className="bg-muted px-2 py-1 rounded">{container.command || "Default"}</code>
              </dd>
            </div>
            <div>
              <dt className="text-sm font-medium text-muted-foreground">Restart Policy</dt>
              <dd className="mt-1 text-sm">{container.restart_policy || "no"}</dd>
            </div>
          </dl>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Resource Limits</CardTitle>
        </CardHeader>
        <CardContent>
          <dl className="grid grid-cols-2 gap-4">
            <div>
              <dt className="text-sm font-medium text-muted-foreground">Memory Limit</dt>
              <dd className="mt-1 text-sm">{container.memory_limit_mb} MB</dd>
            </div>
            <div>
              <dt className="text-sm font-medium text-muted-foreground">CPU Limit</dt>
              <dd className="mt-1 text-sm">{container.cpu_limit ? `${container.cpu_limit} cores` : "Not configured"}</dd>
            </div>
          </dl>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Network Configuration</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-3">
            <div>
              <dt className="text-sm font-medium text-muted-foreground mb-2">Port Mappings</dt>
              {container.port_mappings.map((mapping, i) => (
                <dd key={i} className="text-sm mb-1">
                  <code className="bg-muted px-2 py-1 rounded">
                    {mapping.host}:{mapping.container} ({mapping.protocol})
                  </code>
                </dd>
              ))}
            </div>
            <div>
              <dt className="text-sm font-medium text-muted-foreground">Network Mode</dt>
              <dd className="mt-1 text-sm">bridge</dd>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  )
}
