"use client"

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Plus, Loader2 } from "lucide-react"
import Link from "next/link"
import { ContainerTable } from "@/components/container/container-table"
import { useContainers } from "@/lib/queries"
import { Alert, AlertDescription } from "@/components/ui/alert"

const ContainerFlowDiagram = () => (
  <svg width="300" height="200" viewBox="0 0 300 200" fill="none" xmlns="http://www.w3.org/2000/svg" className="drop-shadow-lg">
    {/* Host Machine */}
    <rect x="10" y="120" width="280" height="70" rx="8" fill="#faf5ff" stroke="#9333ea" strokeWidth="2"/>
    <text x="150" y="150" textAnchor="middle" fill="#581c87" fontWeight="600" fontSize="14">Host Machine (KVM)</text>
    <text x="150" y="165" textAnchor="middle" fill="#581c87" fontSize="11">Firecracker Hypervisor</text>

    {/* Container VM 1 */}
    <rect x="20" y="30" width="70" height="70" rx="6" fill="#faf5ff" stroke="#9333ea" strokeWidth="2"/>
    <circle cx="55" cy="50" r="12" fill="#a855f7" opacity="0.3"/>
    <rect x="43" y="42" width="24" height="16" rx="2" fill="#9333ea" opacity="0.2"/>
    <text x="55" y="74" textAnchor="middle" fill="#581c87" fontWeight="600" fontSize="10">VM 1</text>
    <text x="55" y="88" textAnchor="middle" fill="#581c87" fontSize="8">Docker</text>

    {/* Container VM 2 */}
    <rect x="115" y="30" width="70" height="70" rx="6" fill="#faf5ff" stroke="#9333ea" strokeWidth="2"/>
    <circle cx="150" cy="50" r="12" fill="#a855f7" opacity="0.3"/>
    <rect x="138" y="42" width="24" height="16" rx="2" fill="#9333ea" opacity="0.2"/>
    <text x="150" y="74" textAnchor="middle" fill="#581c87" fontWeight="600" fontSize="10">VM 2</text>
    <text x="150" y="88" textAnchor="middle" fill="#581c87" fontSize="8">Docker</text>

    {/* Container VM 3 */}
    <rect x="210" y="30" width="70" height="70" rx="6" fill="#faf5ff" stroke="#9333ea" strokeWidth="2"/>
    <circle cx="245" cy="50" r="12" fill="#a855f7" opacity="0.3"/>
    <rect x="233" y="42" width="24" height="16" rx="2" fill="#9333ea" opacity="0.2"/>
    <text x="245" y="74" textAnchor="middle" fill="#581c87" fontWeight="600" fontSize="10">VM 3</text>
    <text x="245" y="88" textAnchor="middle" fill="#581c87" fontSize="8">Docker</text>

    {/* Connection lines */}
    <line x1="55" y1="100" x2="55" y2="120" stroke="#9333ea" strokeWidth="2" strokeDasharray="4"/>
    <line x1="150" y1="100" x2="150" y2="120" stroke="#9333ea" strokeWidth="2" strokeDasharray="4"/>
    <line x1="245" y1="100" x2="245" y2="120" stroke="#9333ea" strokeWidth="2" strokeDasharray="4"/>

    {/* Arrows */}
    <polygon points="55,120 52,115 58,115" fill="#9333ea"/>
    <polygon points="150,120 147,115 153,115" fill="#9333ea"/>
    <polygon points="245,120 242,115 248,115" fill="#9333ea"/>

    {/* Container isolation indicator */}
    <text x="150" y="15" textAnchor="middle" fill="#581c87" fontSize="10" fontWeight="500">Container-per-VM Isolation</text>
  </svg>
)

export default function ContainersPage() {
  const { data: containers = [], isLoading, error, refetch } = useContainers()

  return (
    <div className="space-y-6">
      <div className="relative overflow-hidden rounded-xl border border-border bg-gradient-to-br from-purple-50 to-purple-100/50 dark:from-purple-950/30 dark:to-purple-900/20 p-8">
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
            <ContainerFlowDiagram />
          </div>
        </div>
        <div className="absolute right-0 top-0 h-64 w-64 translate-x-32 -translate-y-32 rounded-full bg-gradient-to-br from-purple-400/30 to-purple-600/30 dark:from-purple-500/20 dark:to-purple-600/10 blur-3xl" />
      </div>

      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <CardTitle>All Containers</CardTitle>
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
                Failed to load containers. Please try again.
              </AlertDescription>
            </Alert>
          ) : containers.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              <p>No containers found. Deploy your first container to get started.</p>
            </div>
          ) : (
            <ContainerTable containers={containers} />
          )}
        </CardContent>
      </Card>
    </div>
  )
}
