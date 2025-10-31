"use client"

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Plus, Loader2 } from "lucide-react"
import Link from "next/link"
import { ContainerTable } from "@/components/container/container-table"
import Image from "next/image"
import { useContainers } from "@/lib/queries"
import { Alert, AlertDescription } from "@/components/ui/alert"

export default function ContainersPage() {
  const { data: containers = [], isLoading, error, refetch } = useContainers()

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
