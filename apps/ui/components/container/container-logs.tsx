"use client"

import { useState, useEffect, useRef } from "react"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Switch } from "@/components/ui/switch"
import { Label } from "@/components/ui/label"
import { Download, Loader2, Play, Square } from "lucide-react"
import { useDateFormat } from "@/lib/hooks/use-date-format"

interface ContainerLogsProps {
  containerId: string
}

interface LogEntry {
  timestamp: string
  stream: string
  message: string
}

export function ContainerLogs({ containerId }: ContainerLogsProps) {
  const [autoScroll, setAutoScroll] = useState(true)
  const [logs, setLogs] = useState<LogEntry[]>([])
  const [isStreaming, setIsStreaming] = useState(false)
  const [isConnected, setIsConnected] = useState(false)
  const wsRef = useRef<WebSocket | null>(null)
  const logsEndRef = useRef<HTMLDivElement>(null)
  const dateFormat = useDateFormat()

  const getWebSocketUrl = () => {
    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:"
    const host = window.location.hostname
    const port = process.env.NEXT_PUBLIC_API_PORT || "18080"
    return `${protocol}//${host}:${port}/v1/containers/${containerId}/logs/stream`
  }

  const startStreaming = () => {
    if (wsRef.current?.readyState === WebSocket.OPEN) return

    const ws = new WebSocket(getWebSocketUrl())
    wsRef.current = ws

    ws.onopen = () => {
      setIsConnected(true)
      setIsStreaming(true)
    }

    ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data)
        if (data.error) {
          console.error("Log stream error:", data.error)
        } else if (data.info) {
          console.info("Log stream info:", data.info)
        } else {
          setLogs((prev) => [...prev, data])
        }
      } catch (e) {
        console.error("Failed to parse log message:", e)
      }
    }

    ws.onerror = (error) => {
      console.error("WebSocket error:", error)
      setIsConnected(false)
    }

    ws.onclose = () => {
      setIsConnected(false)
      setIsStreaming(false)
    }
  }

  const stopStreaming = () => {
    if (wsRef.current) {
      wsRef.current.close()
      wsRef.current = null
    }
    setIsStreaming(false)
    setIsConnected(false)
  }

  const downloadLogs = () => {
    const logText = logs.map((log) => `[${log.timestamp}] [${log.stream}] ${log.message}`).join("\n")
    const blob = new Blob([logText], { type: "text/plain" })
    const url = URL.createObjectURL(blob)
    const a = document.createElement("a")
    a.href = url
    a.download = `container-${containerId}-logs.txt`
    document.body.appendChild(a)
    a.click()
    document.body.removeChild(a)
    URL.revokeObjectURL(url)
  }

  useEffect(() => {
    if (autoScroll && logsEndRef.current) {
      logsEndRef.current.scrollIntoView({ behavior: "smooth" })
    }
  }, [logs, autoScroll])

  useEffect(() => {
    return () => {
      stopStreaming()
    }
  }, [])

  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between">
        <CardTitle>Container Logs</CardTitle>
        <div className="flex items-center gap-4">
          <Button
            variant="outline"
            size="sm"
            onClick={isStreaming ? stopStreaming : startStreaming}
            disabled={isStreaming && !isConnected}
          >
            {isStreaming && !isConnected ? (
              <>
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                Connecting...
              </>
            ) : isStreaming ? (
              <>
                <Square className="mr-2 h-4 w-4" />
                Stop Stream
              </>
            ) : (
              <>
                <Play className="mr-2 h-4 w-4" />
                Start Stream
              </>
            )}
          </Button>
          <div className="flex items-center gap-2">
            <Switch id="auto-scroll" checked={autoScroll} onCheckedChange={setAutoScroll} />
            <Label htmlFor="auto-scroll" className="text-sm">
              Auto-scroll
            </Label>
          </div>
          <Button variant="outline" size="sm" onClick={downloadLogs} disabled={logs.length === 0}>
            <Download className="mr-2 h-4 w-4" />
            Download
          </Button>
        </div>
      </CardHeader>
      <CardContent>
        <div className="bg-muted rounded-lg p-4 font-mono text-xs space-y-1 max-h-[600px] overflow-auto">
          {logs.length === 0 ? (
            <div className="text-muted-foreground text-center py-4">
              {isStreaming ? "Waiting for logs..." : "Click 'Start Stream' to begin streaming logs"}
            </div>
          ) : (
            logs.map((log, i) => (
              <div key={i} className={log.stream === "stderr" ? "text-red-500" : "text-foreground"}>
                <span className="text-muted-foreground">[{dateFormat.formatDateTime(log.timestamp)}]</span>{" "}
                <span className="text-blue-500">[{log.stream}]</span> {log.message}
              </div>
            ))
          )}
          <div ref={logsEndRef} />
        </div>
      </CardContent>
    </Card>
  )
}
