"use client"

import { useState, useEffect, useRef } from "react"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Play, Square } from "lucide-react"
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, Legend } from "recharts"

interface MetricsChartProps {
  resourceId: string
  resourceType?: "vm" | "container" | "function"
}

interface VMMetrics {
  cpu_usage_percent?: number
  memory_usage_percent?: number
  memory_used_mb?: number
  memory_total_mb?: number
  network_in_bytes?: number
  network_out_bytes?: number
  disk_read_bytes?: number
  disk_write_bytes?: number
}

export function MetricsChart({ resourceId, resourceType = "vm" }: MetricsChartProps) {
  const [isMonitoring, setIsMonitoring] = useState(false)
  const [metrics, setMetrics] = useState<any[]>([])
  const wsRef = useRef<WebSocket | null>(null)

  useEffect(() => {
    if (!isMonitoring || !resourceId) return

    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:"
    const wsUrl = `${protocol}//${window.location.hostname}:18080/v1/vms/${resourceId}/metrics/ws`

    try {
      const ws = new WebSocket(wsUrl)
      wsRef.current = ws

      ws.onopen = () => {
        console.log("WebSocket connected to VM metrics:", wsUrl)
      }

      ws.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data) as VMMetrics
          const timestamp = new Date().toLocaleTimeString()
          const newMetric = {
            time: timestamp,
            cpu: data.cpu_usage_percent || 0,
            memory: data.memory_usage_percent || 0,
            network: ((data.network_in_bytes || 0) + (data.network_out_bytes || 0)) / 1024, // Total KB/s
            disk: ((data.disk_read_bytes || 0) + (data.disk_write_bytes || 0)) / 1024, // Total KB/s
            networkIn: (data.network_in_bytes || 0) / 1024,
            networkOut: (data.network_out_bytes || 0) / 1024,
            diskRead: (data.disk_read_bytes || 0) / 1024,
            diskWrite: (data.disk_write_bytes || 0) / 1024,
          }

          setMetrics((prev) => {
            const updated = [...prev, newMetric]
            return updated.slice(-60) // Keep last 60 data points
          })
        } catch (e) {
          console.error("Failed to parse metrics:", e)
        }
      }

      ws.onerror = (error) => {
        console.error("WebSocket error:", error)
      }

      ws.onclose = () => {
        console.log("WebSocket disconnected")
      }

      return () => {
        if (wsRef.current) {
          wsRef.current.close()
          wsRef.current = null
        }
      }
    } catch (error) {
      console.error("Failed to connect WebSocket:", error)
    }
  }, [isMonitoring, resourceId])

  const toggleMonitoring = () => {
    setIsMonitoring(!isMonitoring)
    if (isMonitoring) {
      setMetrics([])
    }
  }

  const latestMetrics = metrics[metrics.length - 1]

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <CardTitle>Real-time Metrics</CardTitle>
          <Button onClick={toggleMonitoring} variant={isMonitoring ? "destructive" : "default"}>
            {isMonitoring ? (
              <>
                <Square className="mr-2 h-4 w-4" />
                Stop Monitoring
              </>
            ) : (
              <>
                <Play className="mr-2 h-4 w-4" />
                Start Monitoring
              </>
            )}
          </Button>
        </CardHeader>
        <CardContent>
          {!isMonitoring && metrics.length === 0 ? (
            <div className="text-center py-12 text-muted-foreground">
              Click Start Monitoring to view real-time metrics
            </div>
          ) : (
            <div className="space-y-6">
              <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
                <Card>
                  <CardHeader className="pb-2">
                    <CardTitle className="text-sm font-medium text-muted-foreground">CPU Usage</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="text-2xl font-bold">{latestMetrics ? `${latestMetrics.cpu.toFixed(1)}%` : "—"}</div>
                  </CardContent>
                </Card>

                <Card>
                  <CardHeader className="pb-2">
                    <CardTitle className="text-sm font-medium text-muted-foreground">Memory Usage</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="text-2xl font-bold">
                      {latestMetrics ? `${latestMetrics.memory.toFixed(1)}%` : "—"}
                    </div>
                  </CardContent>
                </Card>

                <Card>
                  <CardHeader className="pb-2">
                    <CardTitle className="text-sm font-medium text-muted-foreground">Network I/O</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="text-2xl font-bold">
                      {latestMetrics ? `${latestMetrics.network.toFixed(0)} KB/s` : "—"}
                    </div>
                  </CardContent>
                </Card>

                <Card>
                  <CardHeader className="pb-2">
                    <CardTitle className="text-sm font-medium text-muted-foreground">Disk I/O</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="text-2xl font-bold">
                      {latestMetrics ? `${latestMetrics.disk.toFixed(0)} KB/s` : "—"}
                    </div>
                  </CardContent>
                </Card>
              </div>

              {metrics.length > 0 && (
                <>
                  <Card>
                    <CardHeader>
                      <CardTitle className="text-sm">CPU & Memory Usage</CardTitle>
                    </CardHeader>
                    <CardContent>
                      <ResponsiveContainer width="100%" height={200}>
                        <LineChart data={metrics}>
                          <CartesianGrid strokeDasharray="3 3" />
                          <XAxis dataKey="time" tick={{ fontSize: 12 }} />
                          <YAxis tick={{ fontSize: 12 }} />
                          <Tooltip />
                          <Legend />
                          <Line
                            type="monotone"
                            dataKey="cpu"
                            stroke="#f97316"
                            name="CPU %"
                            strokeWidth={2}
                            dot={false}
                          />
                          <Line
                            type="monotone"
                            dataKey="memory"
                            stroke="#3b82f6"
                            name="Memory %"
                            strokeWidth={2}
                            dot={false}
                          />
                        </LineChart>
                      </ResponsiveContainer>
                    </CardContent>
                  </Card>

                  <Card>
                    <CardHeader>
                      <CardTitle className="text-sm">Network & Disk I/O</CardTitle>
                    </CardHeader>
                    <CardContent>
                      <ResponsiveContainer width="100%" height={200}>
                        <LineChart data={metrics}>
                          <CartesianGrid strokeDasharray="3 3" />
                          <XAxis dataKey="time" tick={{ fontSize: 12 }} />
                          <YAxis tick={{ fontSize: 12 }} />
                          <Tooltip />
                          <Legend />
                          <Line
                            type="monotone"
                            dataKey="network"
                            stroke="#10b981"
                            name="Network KB/s"
                            strokeWidth={2}
                            dot={false}
                          />
                          <Line
                            type="monotone"
                            dataKey="disk"
                            stroke="#8b5cf6"
                            name="Disk KB/s"
                            strokeWidth={2}
                            dot={false}
                          />
                        </LineChart>
                      </ResponsiveContainer>
                    </CardContent>
                  </Card>
                </>
              )}

              <div className="text-sm text-muted-foreground">
                Monitoring for {metrics.length} seconds • {isMonitoring ? "Connected" : "Disconnected"}
              </div>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  )
}
