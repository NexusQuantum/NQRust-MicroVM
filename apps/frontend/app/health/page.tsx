"use client"

import { useQuery } from "@tanstack/react-query"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { StatusIndicator } from "@/components/status-indicator"

export default function HealthPage() {
  const { data: healthData, isLoading } = useQuery({
    queryKey: ['health'],
    queryFn: async () => {
      const response = await fetch('/api/status')
      if (!response.ok) throw new Error('Health check failed')
      return response.json()
    },
    refetchInterval: 5000, // Refresh every 5 seconds
  })

  return (
    <div className="container mx-auto py-6">
      <div className="space-y-6">
        <div>
          <h1 className="text-3xl font-bold">System Health</h1>
          <p className="text-muted-foreground">
            Monitor the status of NexusRust services and connections
          </p>
        </div>

        <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <StatusIndicator 
                  status={healthData?.socket_available ? 'running' : 'stopped'} 
                />
                Firecracker Bridge
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="space-y-2">
                <div className="flex justify-between">
                  <span className="text-sm text-muted-foreground">Socket Available</span>
                  <Badge variant={healthData?.socket_available ? 'success' : 'destructive'}>
                    {healthData?.socket_available ? 'Available' : 'Unavailable'}
                  </Badge>
                </div>
                <div className="flex justify-between">
                  <span className="text-sm text-muted-foreground">Socket Path</span>
                  <span className="text-sm font-mono">
                    {healthData?.socket_path || '/tmp/firecracker.sock'}
                  </span>
                </div>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <StatusIndicator 
                  status={healthData?.status === 'available' ? 'running' : 'stopped'} 
                />
                API Status
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="space-y-2">
                <div className="flex justify-between">
                  <span className="text-sm text-muted-foreground">Status</span>
                  <Badge variant={healthData?.status === 'available' ? 'success' : 'destructive'}>
                    {healthData?.status || 'Unknown'}
                  </Badge>
                </div>
                <div className="flex justify-between">
                  <span className="text-sm text-muted-foreground">Last Check</span>
                  <span className="text-sm">
                    {healthData?.timestamp ? new Date(healthData.timestamp).toLocaleTimeString() : 'Never'}
                  </span>
                </div>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>Configuration</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="space-y-2">
                <div className="flex justify-between">
                  <span className="text-sm text-muted-foreground">API Base</span>
                  <span className="text-sm font-mono">
                    {process.env.NEXT_PUBLIC_API_BASE_URL || '/api'}
                  </span>
                </div>
                <div className="flex justify-between">
                  <span className="text-sm text-muted-foreground">WebSocket</span>
                  <span className="text-sm font-mono">
                    {process.env.NEXT_PUBLIC_WS_BASE_URL || 'ws://localhost:8000'}
                  </span>
                </div>
              </div>
            </CardContent>
          </Card>
        </div>

        {healthData?.message && (
          <Card>
            <CardContent className="pt-6">
              <p className="text-sm text-muted-foreground">{healthData.message}</p>
            </CardContent>
          </Card>
        )}
      </div>
    </div>
  )
}