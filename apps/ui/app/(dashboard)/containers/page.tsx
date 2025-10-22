import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Plus } from "lucide-react"
import Link from "next/link"
import { ContainerTable } from "@/components/container/container-table"
import Image from "next/image"

// Mock data
const mockContainers = [
  {
    id: "ct-1",
    name: "postgres-main",
    image: "postgres:15",
    status: "running" as const,
    uptime_seconds: 345600,
    cpu_percent: 32.1,
    memory_used_mb: 512,
    memory_limit_mb: 2048,
    port_mappings: [{ host: 5432, container: 5432, protocol: "tcp" as const }],
    created_at: new Date(Date.now() - 86400000 * 4).toISOString(),
    started_at: new Date(Date.now() - 345600000).toISOString(),
  },
  {
    id: "ct-2",
    name: "redis-cache",
    image: "redis:7-alpine",
    status: "running" as const,
    uptime_seconds: 172800,
    cpu_percent: 12.4,
    memory_used_mb: 128,
    memory_limit_mb: 512,
    port_mappings: [{ host: 6379, container: 6379, protocol: "tcp" as const }],
    created_at: new Date(Date.now() - 86400000 * 2).toISOString(),
    started_at: new Date(Date.now() - 172800000).toISOString(),
  },
  {
    id: "ct-3",
    name: "nginx-proxy",
    image: "nginx:latest",
    status: "stopped" as const,
    cpu_percent: 0,
    memory_used_mb: 0,
    port_mappings: [
      { host: 80, container: 80, protocol: "tcp" as const },
      { host: 443, container: 443, protocol: "tcp" as const },
    ],
    created_at: new Date(Date.now() - 86400000).toISOString(),
  },
]

export default function ContainersPage() {
  return (
    <div className="space-y-6">
      <div className="relative overflow-hidden rounded-xl border border-border bg-gradient-to-br from-purple-50 to-purple-100/50 p-8">
        <div className="relative z-10 flex items-center justify-between">
          <div className="max-w-xl">
            <h1 className="text-3xl font-bold text-foreground">Containers</h1>
            <p className="mt-2 text-muted-foreground">
              Deploy and orchestrate Docker containers with full control over networking and resources
            </p>
            <Button asChild className="mt-4">
              <Link href="/containers/new">
                <Plus className="mr-2 h-4 w-4" />
                Deploy Container
              </Link>
            </Button>
          </div>
          <div className="hidden lg:block">
            <Image src="/docker-containers-deployment-infrastructure-illust.jpg" alt="Containers" width={300} height={200} className="rounded-lg" />
          </div>
        </div>
        <div className="absolute right-0 top-0 h-64 w-64 translate-x-32 -translate-y-32 rounded-full bg-gradient-to-br from-purple-400/30 to-purple-600/30 blur-3xl" />
      </div>

      <Card>
        <CardHeader>
          <CardTitle>All Containers</CardTitle>
        </CardHeader>
        <CardContent>
          <ContainerTable containers={mockContainers} />
        </CardContent>
      </Card>
    </div>
  )
}
