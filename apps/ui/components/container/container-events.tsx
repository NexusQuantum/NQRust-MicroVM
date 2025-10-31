"use client"

import { useState, useEffect } from "react"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { RefreshCw, Loader2, Clock, Info, AlertCircle, CheckCircle } from "lucide-react"
import { useContainerLogs } from "@/lib/queries"

interface ContainerEventsProps {
  containerId: string
}

interface Event {
  timestamp: string
  type: "info" | "warning" | "error" | "success"
  message: string
  stream: string
}

const getEventType = (message: string, stream: string): Event["type"] => {
  const lowerMessage = message.toLowerCase()
  if (stream === "stderr" || lowerMessage.includes("error") || lowerMessage.includes("fail")) {
    return "error"
  }
  if (lowerMessage.includes("warn")) {
    return "warning"
  }
  if (lowerMessage.includes("success") || lowerMessage.includes("started") || lowerMessage.includes("ready")) {
    return "success"
  }
  return "info"
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

export function ContainerEvents({ containerId }: ContainerEventsProps) {
  const [events, setEvents] = useState<Event[]>([])
  const { data: logsResp, isLoading, refetch, isFetching } = useContainerLogs(containerId)

  useEffect(() => {
    if (logsResp?.items) {
      const newEvents: Event[] = logsResp.items.map((log) => ({
        timestamp: log.timestamp,
        type: getEventType(log.message, log.stream),
        message: log.message,
        stream: log.stream,
      }))
      setEvents(newEvents)
    }
  }, [logsResp])

  const handleRefresh = () => {
    refetch()
  }

  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between">
        <CardTitle>Container Events</CardTitle>
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
                <div className="flex-shrink-0 mt-1">{getEventIcon(event.type)}</div>
                <div className="flex-1 space-y-1">
                  <div className="flex items-center gap-2">
                    <Badge className={getEventBadgeColor(event.type)}>{event.type}</Badge>
                    <span className="text-xs text-muted-foreground flex items-center gap-1">
                      <Clock className="h-3 w-3" />
                      {new Date(event.timestamp).toLocaleString()}
                    </span>
                  </div>
                  <p className="text-sm text-foreground font-mono break-all">{event.message}</p>
                  {event.stream && (
                    <span className="text-xs text-muted-foreground">Stream: {event.stream}</span>
                  )}
                </div>
              </div>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  )
}
