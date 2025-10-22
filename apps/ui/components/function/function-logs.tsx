"use client"

import { useState } from "react"
import { Card } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { StatusBadge } from "@/components/shared/status-badge"
import { ChevronDown, ChevronRight, Copy } from "lucide-react"
import { formatRelativeTime } from "@/lib/utils/format"
import type { FunctionInvocation } from "@/lib/types"

interface FunctionLogsProps {
  logs: FunctionInvocation[]
}

export function FunctionLogs({ logs }: FunctionLogsProps) {
  const [statusFilter, setStatusFilter] = useState<string>("all")
  const [expandedLogs, setExpandedLogs] = useState<Set<string>>(new Set())

  const filteredLogs = logs.filter((log) => {
    return statusFilter === "all" || log.status === statusFilter
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

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text)
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-4">
        <Select value={statusFilter} onValueChange={setStatusFilter}>
          <SelectTrigger className="w-40">
            <SelectValue placeholder="Status" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All Status</SelectItem>
            <SelectItem value="success">Success</SelectItem>
            <SelectItem value="error">Error</SelectItem>
            <SelectItem value="timeout">Timeout</SelectItem>
          </SelectContent>
        </Select>
        <div className="text-sm text-muted-foreground">
          {filteredLogs.length} invocation{filteredLogs.length !== 1 ? "s" : ""}
        </div>
      </div>

      <div className="space-y-3">
        {filteredLogs.map((log) => {
          const isExpanded = expandedLogs.has(log.id)
          return (
            <Card key={log.id} className="p-4">
              <div className="flex items-start justify-between">
                <div className="flex items-start gap-3 flex-1">
                  <Button variant="ghost" size="icon" className="h-6 w-6 mt-0.5" onClick={() => toggleExpanded(log.id)}>
                    {isExpanded ? <ChevronDown className="h-4 w-4" /> : <ChevronRight className="h-4 w-4" />}
                  </Button>
                  <div className="flex-1 space-y-2">
                    <div className="flex items-center gap-3">
                      <StatusBadge status={log.status} />
                      <span className="text-sm text-muted-foreground">{formatRelativeTime(log.invoked_at)}</span>
                      <span className="text-sm text-muted-foreground">{log.duration_ms}ms</span>
                      <span className="text-sm text-muted-foreground">{log.memory_used_mb}MB</span>
                    </div>
                    <div className="flex items-center gap-2">
                      <code className="text-xs bg-muted px-2 py-1 rounded">{log.request_id}</code>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-6 w-6"
                        onClick={() => copyToClipboard(log.request_id)}
                      >
                        <Copy className="h-3 w-3" />
                      </Button>
                    </div>

                    {isExpanded && (
                      <div className="space-y-3 mt-4">
                        <div>
                          <div className="text-sm font-medium mb-1">Event</div>
                          <pre className="bg-muted p-3 rounded text-xs overflow-auto">
                            {JSON.stringify(log.event, null, 2)}
                          </pre>
                        </div>

                        {log.response && (
                          <div>
                            <div className="text-sm font-medium mb-1">Response</div>
                            <pre className="bg-muted p-3 rounded text-xs overflow-auto">
                              {JSON.stringify(log.response, null, 2)}
                            </pre>
                          </div>
                        )}

                        {log.error && (
                          <div>
                            <div className="text-sm font-medium mb-1 text-destructive">Error</div>
                            <div className="bg-destructive/10 text-destructive p-3 rounded text-xs">{log.error}</div>
                          </div>
                        )}

                        <div>
                          <div className="text-sm font-medium mb-1">Logs</div>
                          <div className="bg-muted p-3 rounded text-xs space-y-1">
                            {log.logs.map((line, i) => (
                              <div key={i}>{line}</div>
                            ))}
                          </div>
                        </div>
                      </div>
                    )}
                  </div>
                </div>
              </div>
            </Card>
          )
        })}
      </div>
    </div>
  )
}
