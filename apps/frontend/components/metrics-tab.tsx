"use client"

import { useEffect, useState } from "react"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Activity, Cpu, HardDrive, Network, Play, Square } from "lucide-react"
import { useMetricsWebSocket } from "@/lib/ws"
import { Line } from "react-chartjs-2"
import {
  Chart as ChartJS,
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  Title,
  Tooltip,
  Legend,
  type ChartOptions,
} from "chart.js"
import type { VM } from "@/types/firecracker"

ChartJS.register(CategoryScale, LinearScale, PointElement, LineElement, Title, Tooltip, Legend)

interface MetricsTabProps {
  vm: any
}

interface MetricsData {
  timestamp: number
  cpu: number
  memory: number
  networkIn: number
  networkOut: number
  diskRead: number
  diskWrite: number
}

type IncomingMetrics = {
  cpu_usage_percent?: number
  memory_usage_percent?: number
  network_in_bytes?: number
  network_out_bytes?: number
  disk_read_bytes?: number
  disk_write_bytes?: number
}

const MAX_DATA_POINTS = 60 // Keep last 60 data points (1 minute at 1s intervals)

export function MetricsTab({ vm }: MetricsTabProps) {
  const [isStreaming, setIsStreaming] = useState(false)
  const [metricsData, setMetricsData] = useState<MetricsData[]>([])
  const { connect, disconnect, isConnected } = useMetricsWebSocket(vm.id)

  useEffect(() => {
    const handleMetrics = (data: IncomingMetrics) => {
      console.log("Received metrics:", data)
      const newMetric: MetricsData = {
        timestamp: Date.now(),
        cpu: data.cpu_usage_percent || 0,
        memory: data.memory_usage_percent || 0,
        networkIn: data.network_in_bytes || 0,
        networkOut: data.network_out_bytes || 0,
        diskRead: data.disk_read_bytes || 0,
        diskWrite: data.disk_write_bytes || 0,
      }
      console.log("New metric added:", newMetric)

      setMetricsData((prev) => {
        const updated = [...prev, newMetric]
        console.log("Total metrics data points:", updated.length)
        return updated.slice(-MAX_DATA_POINTS)
      })
    }

  if (isStreaming && vm.state === "running") {
      connect(handleMetrics)
    } else {
      disconnect()
    }

    return () => disconnect()
  }, [isStreaming, vm.state, connect, disconnect])

  const toggleStreaming = () => {
    setIsStreaming(!isStreaming)
    if (isStreaming) {
      setMetricsData([])
    }
  }

  const percentageChartOptions: ChartOptions<"line"> = {
    responsive: true,
    maintainAspectRatio: false,
    animation: false,
    plugins: {
      legend: {
        display: false,
      },
    },
    scales: {
      x: {
        display: false,
      },
      y: {
        beginAtZero: true,
        max: 100,
        grid: {
          display: true,
        },
        ticks: {
          stepSize: 25,
          callback: (value) => `${value}%`,
        },
      },
    },
    elements: {
      point: {
        radius: 2,
        hitRadius: 10,
      },
      line: {
        borderWidth: 2,
        tension: 0.4,
      },
    },
  }

  const bytesChartOptions: ChartOptions<"line"> = {
    responsive: true,
    maintainAspectRatio: false,
    animation: false,
    plugins: {
      legend: {
        display: true,
        position: "top",
        labels: {
          usePointStyle: true,
          boxWidth: 6,
        },
      },
    },
    scales: {
      x: {
        display: false,
      },
      y: {
        beginAtZero: true,
        grid: {
          display: true,
        },
        ticks: {
          callback: (value) => {
            const bytes = value as number
            if (bytes >= 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)}MB/s`
            if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)}KB/s`
            return `${bytes}B/s`
          },
        },
      },
    },
    elements: {
      point: {
        radius: 2,
        hitRadius: 10,
      },
      line: {
        borderWidth: 2,
        tension: 0.4,
      },
    },
  }

  const labels = metricsData.map((_, index) => index.toString())

  const cpuData = {
    labels,
    datasets: [
      {
        data: metricsData.map((d) => d.cpu),
        borderColor: "rgb(59, 130, 246)",
        backgroundColor: "rgba(59, 130, 246, 0.1)",
        fill: true,
      },
    ],
  }

  const memoryData = {
    labels,
    datasets: [
      {
        data: metricsData.map((d) => d.memory),
        borderColor: "rgb(34, 197, 94)",
        backgroundColor: "rgba(34, 197, 94, 0.1)",
        fill: true,
      },
    ],
  }

  const networkData = {
    labels,
    datasets: [
      {
        label: "In",
        data: metricsData.map((d) => d.networkIn),
        borderColor: "rgb(168, 85, 247)",
        backgroundColor: "rgba(168, 85, 247, 0.12)",
        fill: false,
      },
      {
        label: "Out",
        data: metricsData.map((d) => d.networkOut),
        borderColor: "rgba(168, 85, 247, 0.6)",
        backgroundColor: "rgba(168, 85, 247, 0.08)",
        borderDash: [4, 4] as unknown as number[],
        fill: false,
      },
    ],
  }

  const diskData = {
    labels,
    datasets: [
      {
        label: "Read",
        data: metricsData.map((d) => d.diskRead),
        borderColor: "rgb(234, 179, 8)",
        backgroundColor: "rgba(234, 179, 8, 0.12)",
        fill: false,
      },
      {
        label: "Write",
        data: metricsData.map((d) => d.diskWrite),
        borderColor: "rgba(234, 179, 8, 0.7)",
        backgroundColor: "rgba(234, 179, 8, 0.06)",
        borderDash: [4, 4] as unknown as number[],
        fill: false,
      },
    ],
  }

  const latestMetrics = metricsData[metricsData.length - 1]

  // Debug logging
  if (metricsData.length > 0) {
    console.log("Chart data prepared:", {
      dataPoints: metricsData.length,
      cpu: cpuData.datasets[0].data,
      memory: memoryData.datasets[0].data,
      networkIn: networkData.datasets[0].data,
      networkOut: networkData.datasets[1].data,
      diskRead: diskData.datasets[0].data,
      diskWrite: diskData.datasets[1].data,
    })
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-lg font-semibold">Real-time Metrics</h3>
          <p className="text-sm text-muted-foreground">Monitor VM performance and resource usage</p>
        </div>

        <div className="flex items-center gap-2">
          <Badge variant={isConnected ? "default" : "secondary"} className="gap-1">
            <Activity className="h-3 w-3" />
            {isConnected ? "Connected" : "Disconnected"}
          </Badge>
          <Button
            onClick={toggleStreaming}
            disabled={vm.state !== "running"}
            variant={isStreaming ? "destructive" : "default"}
            className="gap-2"
          >
            {isStreaming ? (
              <>
                <Square className="h-4 w-4" />
                Stop Monitoring
              </>
            ) : (
              <>
                <Play className="h-4 w-4" />
                Start Monitoring
              </>
            )}
          </Button>
        </div>
      </div>

  {vm.state !== "running" && (
        <Card>
          <CardContent className="flex items-center justify-center py-8">
            <div className="text-center">
              <Activity className="h-12 w-12 text-muted-foreground mx-auto mb-4" />
              <h3 className="text-lg font-semibold mb-2">VM Not Running</h3>
              <p className="text-muted-foreground">Start the VM to view real-time metrics</p>
            </div>
          </CardContent>
        </Card>
      )}

  {vm.state === "running" && (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          {/* CPU Usage */}
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-base flex items-center gap-2">
                <Cpu className="h-4 w-4" />
                CPU Usage
                {latestMetrics && (
                  <Badge variant="outline" className="ml-auto">
                    {latestMetrics.cpu.toFixed(1)}%
                  </Badge>
                )}
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="h-32">
                {metricsData.length > 0 ? (
                  <Line data={cpuData} options={percentageChartOptions} />
                ) : (
                  <div className="flex items-center justify-center h-full text-muted-foreground">
                    {isStreaming ? "Collecting data..." : "Start monitoring to view data"}
                  </div>
                )}
              </div>
            </CardContent>
          </Card>

          {/* Memory Usage */}
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-base flex items-center gap-2">
                <HardDrive className="h-4 w-4" />
                Memory Usage
                {latestMetrics && (
                  <Badge variant="outline" className="ml-auto">
                    {latestMetrics.memory.toFixed(1)}%
                  </Badge>
                )}
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="h-32">
                {metricsData.length > 0 ? (
                  <Line data={memoryData} options={percentageChartOptions} />
                ) : (
                  <div className="flex items-center justify-center h-full text-muted-foreground">
                    {isStreaming ? "Collecting data..." : "Start monitoring to view data"}
                  </div>
                )}
              </div>
            </CardContent>
          </Card>

          {/* Network I/O */}
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-base flex items-center gap-2">
                <Network className="h-4 w-4" />
                Network I/O
                {latestMetrics && (
                  <div className="ml-auto flex gap-2">
                    <Badge variant="outline" className="text-xs">
                      ↓ {(latestMetrics.networkIn / 1024).toFixed(1)}KB/s
                    </Badge>
                    <Badge variant="outline" className="text-xs">
                      ↑ {(latestMetrics.networkOut / 1024).toFixed(1)}KB/s
                    </Badge>
                  </div>
                )}
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="h-32">
                {metricsData.length > 0 ? (
                  <Line data={networkData} options={bytesChartOptions} />
                ) : (
                  <div className="flex items-center justify-center h-full text-muted-foreground">
                    {isStreaming ? "Collecting data..." : "Start monitoring to view data"}
                  </div>
                )}
              </div>
            </CardContent>
          </Card>

          {/* Disk I/O */}
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-base flex items-center gap-2">
                <HardDrive className="h-4 w-4" />
                Disk I/O
                {latestMetrics && (
                  <div className="ml-auto flex gap-2">
                    <Badge variant="outline" className="text-xs">
                      R {(latestMetrics.diskRead / 1024).toFixed(1)}KB/s
                    </Badge>
                    <Badge variant="outline" className="text-xs">
                      W {(latestMetrics.diskWrite / 1024).toFixed(1)}KB/s
                    </Badge>
                  </div>
                )}
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="h-32">
                {metricsData.length > 0 ? (
                  <Line data={diskData} options={bytesChartOptions} />
                ) : (
                  <div className="flex items-center justify-center h-full text-muted-foreground">
                    {isStreaming ? "Collecting data..." : "Start monitoring to view data"}
                  </div>
                )}
              </div>
            </CardContent>
          </Card>
        </div>
      )}
    </div>
  )
}
