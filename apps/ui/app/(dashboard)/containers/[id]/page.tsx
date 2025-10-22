import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { ContainerOverview } from "@/components/container/container-overview"
import { ContainerConfig } from "@/components/container/container-config"
import { ContainerLogs } from "@/components/container/container-logs"
import { XTermWrapper } from "@/components/shared/xterm-wrapper"
import { MetricsChart } from "@/components/shared/metrics-chart"
import { Play, Square, RotateCw, Trash2, ArrowLeft } from "lucide-react"
import Link from "next/link"
import { use } from "react"

// Mock data
const mockContainer = {
  id: "ct-1",
  name: "postgres-main",
  image: "postgres:15",
  status: "running" as const,
  uptime_seconds: 345600,
  cpu_percent: 32.1,
  memory_used_mb: 512,
  memory_limit_mb: 2048,
  port_mappings: [{ host: 5432, container: 5432, protocol: "tcp" as const }],
  env_vars: {
    POSTGRES_USER: "admin",
    POSTGRES_DB: "myapp",
    POSTGRES_PASSWORD: "***",
  },
  volumes: [{ host: "/data/postgres", container: "/var/lib/postgresql/data" }],
  command: "postgres",
  restart_policy: "always",
  created_at: new Date(Date.now() - 86400000 * 4).toISOString(),
  started_at: new Date(Date.now() - 345600000).toISOString(),
}

const getStatusColor = (status: string) => {
  switch (status) {
    case "running":
      return "bg-green-500/10 text-green-700 border-green-200"
    case "stopped":
      return "bg-gray-500/10 text-gray-700 border-gray-200"
    case "error":
      return "bg-red-500/10 text-red-700 border-red-200"
    default:
      return "bg-blue-500/10 text-blue-700 border-blue-200"
  }
}

export default function ContainerDetailPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = use(params)
  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <Link href="/containers">
            <Button variant="ghost" size="icon">
              <ArrowLeft className="h-4 w-4" />
            </Button>
          </Link>
          <div>
            <div className="flex items-center gap-3">
              <h1 className="text-3xl font-bold text-foreground">{mockContainer.name}</h1>
              <Badge className={getStatusColor(mockContainer.status)}>{mockContainer.status}</Badge>
            </div>
            <p className="text-sm text-muted-foreground mt-1">
              {mockContainer.image} • ID: {mockContainer.id}
            </p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="outline" size="sm">
            <Play className="mr-2 h-4 w-4" />
            Start
          </Button>
          <Button variant="outline" size="sm">
            <Square className="mr-2 h-4 w-4" />
            Stop
          </Button>
          <Button variant="outline" size="sm">
            <RotateCw className="mr-2 h-4 w-4" />
            Restart
          </Button>
          <Button variant="destructive" size="sm">
            <Trash2 className="mr-2 h-4 w-4" />
            Delete
          </Button>
        </div>
      </div>

      <Tabs defaultValue="overview" className="space-y-4">
        <TabsList className="bg-muted/50">
          <TabsTrigger value="overview">Overview</TabsTrigger>
          <TabsTrigger value="logs">Logs</TabsTrigger>
          <TabsTrigger value="shell">Shell</TabsTrigger>
          <TabsTrigger value="stats">Stats</TabsTrigger>
          <TabsTrigger value="config">Config</TabsTrigger>
          <TabsTrigger value="events">Events</TabsTrigger>
        </TabsList>

        <TabsContent value="overview" className="space-y-4">
          <ContainerOverview container={mockContainer} />
        </TabsContent>

        <TabsContent value="logs" className="space-y-4">
          <ContainerLogs containerId={mockContainer.id} />
        </TabsContent>

        <TabsContent value="shell" className="space-y-4">
          <XTermWrapper containerId={mockContainer.id} />
        </TabsContent>

        <TabsContent value="stats" className="space-y-4">
          <MetricsChart resourceId={mockContainer.id} resourceType="container" />
        </TabsContent>

        <TabsContent value="config" className="space-y-4">
          <ContainerConfig container={mockContainer} />
        </TabsContent>

        <TabsContent value="events" className="space-y-4">
          <div className="rounded-lg border border-border bg-card">
            <div className="p-6 space-y-4">
              <h3 className="text-lg font-semibold">Container Events</h3>
              <div className="space-y-2">
                {[
                  { time: "2 minutes ago", event: "Container started", status: "success" },
                  { time: "1 hour ago", event: "Health check passed", status: "success" },
                  { time: "3 hours ago", event: "Container restarted", status: "warning" },
                  { time: "1 day ago", event: "Container created", status: "info" },
                ].map((event, i) => (
                  <div key={i} className="flex items-center justify-between py-2 border-b last:border-0">
                    <div className="flex items-center gap-3">
                      <div
                        className={`h-2 w-2 rounded-full ${
                          event.status === "success"
                            ? "bg-green-500"
                            : event.status === "warning"
                              ? "bg-yellow-500"
                              : "bg-blue-500"
                        }`}
                      />
                      <span className="text-sm">{event.event}</span>
                    </div>
                    <span className="text-sm text-muted-foreground">{event.time}</span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </TabsContent>
      </Tabs>
    </div>
  )
}
