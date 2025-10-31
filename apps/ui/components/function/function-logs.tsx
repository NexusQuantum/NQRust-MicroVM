"use client"

import { useState, useEffect, useRef } from "react"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Switch } from "@/components/ui/switch"
import { Label } from "@/components/ui/label"
import { Download, Loader2, ChevronDown, ChevronRight, Copy } from "lucide-react"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Badge } from "@/components/ui/badge"
import { useFunctionLogs } from "@/lib/queries"
import type { FunctionInvocation } from "@/lib/types"
import { formatRelativeTime } from "@/lib/utils/format"

interface FunctionLogsProps {
  functionId: string
}

export function FunctionLogs({ functionId }: FunctionLogsProps) {
  const [statusFilter, setStatusFilter] = useState<string>("all")
  const [expandedLogs, setExpandedLogs] = useState<Set<string>>(new Set())
  const [autoScroll, setAutoScroll] = useState(true)
  const logsEndRef = useRef<HTMLDivElement>(null)
  const { data: logsResp, isLoading, refetch } = useFunctionLogs(functionId, {
    status: statusFilter !== "all" ? statusFilter : undefined,
    limit: 100,
  })

  const invocations = logsResp?.items || []

  const filteredInvocations = invocations.filter((inv) => {
    return statusFilter === "all" || inv.status === statusFilter
  })

  const toggleExpanded = (id: string) => {
    const newExpanded = new Set(expandedLogs)
    if (newExpanded.has(id)) {
      newExpanded.delete(id)
    } else {
      newExpanded.add(id)
    }
    setExpandedLogs(newExpanded)
  }

  const copyToClipboard = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text)
    } catch (e) {
      console.error("Failed to copy:", e)
    }
  }

  const downloadLogs = () => {
    const logText = filteredInvocations
      .map((inv) => {
        const lines = [
          `[${new Date(inv.invoked_at).toLocaleString()}] Invocation ${inv.id}`,
          `  Status: ${inv.status}`,
          `  Duration: ${inv.duration_ms}ms`,
          `  Memory: ${inv.memory_used_mb}MB`,
          `  Request ID: ${inv.request_id}`,
          `  Event: ${JSON.stringify(inv.event, null, 2)}`,
        ]
        if (inv.response) {
          lines.push(`  Response: ${JSON.stringify(inv.response, null, 2)}`)
        }
        if (inv.error) {
          lines.push(`  Error: ${inv.error}`)
        }
        if (inv.logs && inv.logs.length > 0) {
          lines.push(`  Logs:`)
          inv.logs.forEach((log) => lines.push(`    ${log}`))
        }
        return lines.join("\n")
      })
      .join("\n\n")
    const blob = new Blob([logText], { type: "text/plain" })
    const url = URL.createObjectURL(blob)
    const a = document.createElement("a")
    a.href = url
    a.download = `function-${functionId}-invocations.txt`
    document.body.appendChild(a)
    a.click()
    document.body.removeChild(a)
    URL.revokeObjectURL(url)
  }

  useEffect(() => {
    if (autoScroll && logsEndRef.current) {
      logsEndRef.current.scrollIntoView({ behavior: "smooth" })
    }
  }, [filteredInvocations, autoScroll])

  useEffect(() => {
    refetch()
  }, [statusFilter])

  const getStatusColor = (status: string) => {
    switch (status) {
      case "success":
        return "bg-green-500/10 text-green-700 border-green-200"
      case "error":
        return "bg-red-500/10 text-red-700 border-red-200"
      case "timeout":
        return "bg-yellow-500/10 text-yellow-700 border-yellow-200"
      default:
        return "bg-gray-500/10 text-gray-700 border-gray-200"
    }
  }

  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between">
        <CardTitle>Function Invocations</CardTitle>
        <div className="flex items-center gap-4">
          <Select value={statusFilter} onValueChange={setStatusFilter}>
            <SelectTrigger className="w-[140px]">
              <SelectValue placeholder="Filter by status" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All Status</SelectItem>
              <SelectItem value="success">Success</SelectItem>
              <SelectItem value="error">Error</SelectItem>
              <SelectItem value="timeout">Timeout</SelectItem>
            </SelectContent>
          </Select>
          <div className="flex items-center gap-2">
            <Switch id="auto-scroll" checked={autoScroll} onCheckedChange={setAutoScroll} />
            <Label htmlFor="auto-scroll" className="text-sm">
              Auto-scroll
            </Label>
          </div>
          <Button variant="outline" size="sm" onClick={downloadLogs} disabled={filteredInvocations.length === 0}>
            <Download className="mr-2 h-4 w-4" />
            Download
          </Button>
        </div>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <div className="flex items-center justify-center py-12">
            <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
          </div>
        ) : filteredInvocations.length === 0 ? (
          <div className="text-muted-foreground text-center py-4">
            {invocations.length === 0 ? "No invocations found" : "No invocations match the selected filter"}
          </div>
        ) : (
          <div className="space-y-4 max-h-[600px] overflow-auto">
            {filteredInvocations.map((inv) => {
              const isExpanded = expandedLogs.has(inv.id)
              return (
                <div key={inv.id} className="border rounded-lg p-4 space-y-2">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-3">
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-6 w-6"
                        onClick={() => toggleExpanded(inv.id)}
                      >
                        {isExpanded ? (
                          <ChevronDown className="h-4 w-4" />
                        ) : (
                          <ChevronRight className="h-4 w-4" />
                        )}
                      </Button>
                      <Badge className={getStatusColor(inv.status)}>{inv.status}</Badge>
                      <span className="text-sm text-muted-foreground">
                        {formatRelativeTime(inv.invoked_at)}
                      </span>
                      <span className="text-xs text-muted-foreground">Duration: {inv.duration_ms}ms</span>
                      <span className="text-xs text-muted-foreground">Memory: {inv.memory_used_mb}MB</span>
                    </div>
                    <div className="flex items-center gap-2">
                      <span className="text-xs text-muted-foreground font-mono">ID: {inv.request_id}</span>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-6 w-6"
                        onClick={() => copyToClipboard(JSON.stringify(inv, null, 2))}
                      >
                        <Copy className="h-3 w-3" />
                      </Button>
                    </div>
                  </div>
                  {isExpanded && (
                    <div className="ml-8 space-y-3 pt-2 border-t">
                      <div>
                        <div className="text-xs font-semibold mb-1">Event:</div>
                        <pre className="bg-muted p-2 rounded text-xs overflow-x-auto">
                          {JSON.stringify(inv.event, null, 2)}
                        </pre>
                      </div>
                      {inv.response && (
                        <div>
                          <div className="text-xs font-semibold mb-1">Response:</div>
                          <pre className="bg-muted p-2 rounded text-xs overflow-x-auto">
                            {JSON.stringify(inv.response, null, 2)}
                          </pre>
                        </div>
                      )}
                      {inv.error && (
                        <div>
                          <div className="text-xs font-semibold mb-1 text-red-600">Error:</div>
                          <pre className="bg-red-500/10 p-2 rounded text-xs overflow-x-auto text-red-600">
                            {inv.error}
                          </pre>
                        </div>
                      )}
                      {inv.logs && inv.logs.length > 0 && (
                        <div>
                          <div className="text-xs font-semibold mb-1">Logs:</div>
                          <div className="bg-muted p-2 rounded text-xs font-mono space-y-1">
                            {inv.logs.map((log, i) => (
                              <div key={i}>{log}</div>
                            ))}
                          </div>
                        </div>
                      )}
                    </div>
                  )}
                  <div ref={logsEndRef} />
                </div>
              )
            })}
          </div>
        )}
      </CardContent>
    </Card>
  )
}
