"use client"

import { useMemo } from "react"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { RefreshCw, Loader2, Clock, Info, AlertCircle, CheckCircle, Zap } from "lucide-react"
import { useFunctionLogs } from "@/lib/queries"
import type { Function as FunctionType } from "@/lib/types"

interface FunctionEventProps {
  functionData?: FunctionType
}

interface Event {
  timestamp: string
  type: "info" | "warning" | "error" | "success"
  message: string
  source: "lifecycle" | "invocation"
}

const getEventIcon = (type: Event["type"]) => {
  switch (type) {
    case "error":
      return <AlertCircle className="h-4 w-4 text-red-500" />
    case "warning":
      return <AlertCircle className="h-4 w-4 text-yellow-500" />
    case "success":
      return <CheckCircle className="h-4 w-4 text-green-500" />
    default:
      return <Info className="h-4 w-4 text-blue-500" />
  }
}

const getEventBadgeColor = (type: Event["type"]) => {
  switch (type) {
    case "error":
      return "bg-red-500/10 text-red-700 border-red-200"
    case "warning":
      return "bg-yellow-500/10 text-yellow-700 border-yellow-200"
    case "success":
      return "bg-green-500/10 text-green-700 border-green-200"
    default:
      return "bg-blue-500/10 text-blue-700 border-blue-200"
  }
}

const getLifecycleEventType = (state: string): Event["type"] => {
  switch (state) {
    case "error":
      return "error"
    case "ready":
      return "success"
    case "creating":
    case "booting":
    case "deploying":
      return "info"
    default:
      return "info"
  }
}

const getLifecycleMessage = (state: string): string => {
  switch (state) {
    case "creating":
      return "Function created"
    case "booting":
      return "VM booting"
    case "deploying":
      return "Function deploying"
    case "ready":
      return "Function ready"
    case "error":
      return "Function error occurred"
    default:
      return `Function state: ${state}`
  }
}

export function FunctionEvent({ functionData }: FunctionEventProps) {
  const { data: logsResp, isLoading, refetch, isFetching } = useFunctionLogs(functionData?.id || "", {
    limit: 50,
  })

  const events = useMemo<Event[]>(() => {
    const result: Event[] = []

    // Add lifecycle events from function state
    if (functionData) {
      // Function created
      if (functionData.created_at) {
        result.push({
          timestamp: functionData.created_at,
          type: "info",
          message: "Function created",
          source: "lifecycle",
        })
      }

      // Current state
      if (functionData.state) {
        result.push({
          timestamp: functionData.updated_at || functionData.created_at || new Date().toISOString(),
          type: getLifecycleEventType(functionData.state),
          message: getLifecycleMessage(functionData.state),
          source: "lifecycle",
        })
      }
    }

    // Add invocation events
    if (logsResp?.items) {
      logsResp.items.forEach((inv) => {
        let type: Event["type"] = "success"
        let message = `Function invoked (${inv.duration_ms}ms)`
        if (inv.status === "error") {
          type = "error"
          message = `Function invocation failed: ${inv.error || "Unknown error"}`
        } else if (inv.status === "timeout") {
          type = "warning"
          message = `Function invocation timed out after ${inv.duration_ms}ms`
        }
        result.push({
          timestamp: inv.invoked_at,
          type,
          message,
          source: "invocation",
        })
      })
    }

    // Sort by timestamp (newest first)
    return result.sort((a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime())
  }, [functionData, logsResp])

  const handleRefresh = () => {
    refetch()
  }

  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between">
        <CardTitle>Function Events</CardTitle>
        <Button onClick={handleRefresh} variant="outline" size="sm" disabled={isFetching}>
          {isFetching ? (
            <>
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              Refreshing...
            </>
          ) : (
            <>
              <RefreshCw className="mr-2 h-4 w-4" />
              Refresh
            </>
          )}
        </Button>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <div className="flex items-center justify-center py-12">
            <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
          </div>
        ) : events.length === 0 ? (
          <div className="text-center py-12 text-muted-foreground">No events found</div>
        ) : (
          <div className="space-y-4">
            {events.map((event, i) => (
              <div key={i} className="flex gap-4 pb-4 border-b border-border last:border-0 last:pb-0">
                <div className="flex-shrink-0 mt-1">
                  {event.source === "invocation" ? (
                    <Zap className="h-4 w-4 text-purple-500" />
                  ) : (
                    getEventIcon(event.type)
                  )}
                </div>
                <div className="flex-1 space-y-1">
                  <div className="flex items-center gap-2">
                    <Badge className={getEventBadgeColor(event.type)}>{event.type}</Badge>
                    <Badge variant="outline" className="text-xs">
                      {event.source === "invocation" ? "Invocation" : "Lifecycle"}
                    </Badge>
                    <span className="text-xs text-muted-foreground flex items-center gap-1">
                      <Clock className="h-3 w-3" />
                      {new Date(event.timestamp).toLocaleString()}
                    </span>
                  </div>
                  <p className="text-sm text-foreground">{event.message}</p>
                </div>
              </div>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  )
}