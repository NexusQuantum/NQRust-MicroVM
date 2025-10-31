"use client"

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { ExternalLink } from "lucide-react"
import Link from "next/link"
import type { Function as FnType } from "@/lib/types"

interface FunctionOverviewProps {
  functionData?: FnType
}

export function FunctionOverview({ functionData }: FunctionOverviewProps) {
  console.log(functionData)
  return (
    <div className="space-y-6">
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Image</CardTitle>
          </CardHeader>
          <CardContent>
            <code className="text-sm font-medium">ngix:alpine</code>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">CPU</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {functionData?.vcpu ? `${functionData.vcpu} vCPU` : "N/A"}
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Memory</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {functionData?.memory_mb} MB
            </div>
          </CardContent>
        </Card>


        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Function ID</CardTitle>
          </CardHeader>
          <CardContent>
            <code className="text-sm font-medium">{functionData?.id}</code>
          </CardContent>
        </Card>
      </div>

      {/* Container VM */}
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
              <code className="text-xs bg-muted px-2 py-1 rounded">{functionData?.vm_id}</code>
            </div>
            <Button variant="outline" asChild>
              <Link href={`/vms/${functionData?.vm_id}`}>
                <ExternalLink className="mr-2 h-4 w-4" />
                View VM
              </Link>
            </Button>
          </div>
        </CardContent>
      </Card>

      {/* Network & Access */}
      <Card>
        <CardHeader>
          <CardTitle>Network & Access</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-2">
            {functionData?.guest_ip && (
              <div className="space-y-2">
                <div className="text-sm font-medium text-muted-foreground">VM IP Address</div>
                <code className="bg-muted px-2 py-1 rounded text-sm font-medium">{functionData?.guest_ip}</code>
              </div>
            )}
            {functionData?.port && (
              <div className="space-y-2">
                <div className="text-sm font-medium text-muted-foreground">Port</div>
                <code className="bg-muted px-2 py-1 rounded text-sm font-medium">{functionData?.port}</code>
              </div>
            )}
          </div>
        </CardContent>
      </Card>

      {/* Environment */}
      <Card>
        <CardHeader>
          <CardTitle>Environment Variables</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-2">
            TEST_VAR: hello
          </div>
        </CardContent>
      </Card>

      {/* Volumes */}
      <Card>
        <CardHeader>
          <CardTitle>Volumes</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-2">
            /tmp/nginx-test → /usr/share/nginx/html
          </div>
        </CardContent>
      </Card>
    </div>
  )
}