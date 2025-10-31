"use client"

import { useEffect, useRef, useState, useCallback } from "react"

const WS_BASE_URL = process.env.NEXT_PUBLIC_WS_BASE_URL || "ws://localhost:18080"

export interface VMMetrics {
  cpu_usage_percent?: number
  memory_usage_percent?: number
  memory_used_mb?: number
  memory_total_mb?: number
  network_in_bytes?: number
  network_out_bytes?: number
  disk_read_bytes?: number
  disk_write_bytes?: number
}

export interface WebSocketState {
  isConnected: boolean
  isConnecting: boolean
  error: string | null
  lastMessage: VMMetrics | null
}

export class MetricsWebSocketClient {
  private ws: WebSocket | null = null
  private vmId: string
  private onMessage: (metrics: VMMetrics) => void
  private onStateChange: (state: Partial<WebSocketState>) => void
  private reconnectAttempts = 0
  private maxReconnectAttempts = 5
  private reconnectDelay = 1000 // Start with 1 second
  private reconnectTimer: NodeJS.Timeout | null = null
  private isManuallyDisconnected = false

  constructor(
    vmId: string,
    onMessage: (metrics: VMMetrics) => void,
    onStateChange: (state: Partial<WebSocketState>) => void,
  ) {
    this.vmId = vmId
    this.onMessage = onMessage
    this.onStateChange = onStateChange
  }

  connect() {
    if (this.ws?.readyState === WebSocket.OPEN) {
      return
    }

    this.isManuallyDisconnected = false
    this.onStateChange({ isConnecting: true, error: null })

    try {
      const wsUrl = `${WS_BASE_URL}/v1/vms/${this.vmId}/metrics/ws`
      this.ws = new WebSocket(wsUrl)

      this.ws.onopen = () => {
        console.log(`[v0] WebSocket connected to ${wsUrl}`)
        this.reconnectAttempts = 0
        this.reconnectDelay = 1000
        this.onStateChange({
          isConnected: true,
          isConnecting: false,
          error: null,
        })
      }

      this.ws.onmessage = (event) => {
        try {
          const metrics: VMMetrics = JSON.parse(event.data)
          console.log(`[v0] Received metrics for VM ${this.vmId}:`, metrics)
          this.onMessage(metrics)
          this.onStateChange({ lastMessage: metrics })
        } catch (error) {
          console.error("[v0] Failed to parse WebSocket message:", error)
          this.onStateChange({ error: "Failed to parse metrics data" })
        }
      }

      this.ws.onclose = (event) => {
        console.log(`[v0] WebSocket closed for VM ${this.vmId}:`, event.code, event.reason)
        this.onStateChange({
          isConnected: false,
          isConnecting: false,
        })

        // Only attempt reconnection if not manually disconnected
        if (!this.isManuallyDisconnected && this.reconnectAttempts < this.maxReconnectAttempts) {
          this.scheduleReconnect()
        } else if (this.reconnectAttempts >= this.maxReconnectAttempts) {
          this.onStateChange({
            error: "Maximum reconnection attempts reached",
          })
        }
      }

      this.ws.onerror = (error) => {
        console.error(`[v0] WebSocket error for VM ${this.vmId}:`, error)
        this.onStateChange({
          error: "WebSocket connection error",
          isConnecting: false,
        })
      }
    } catch (error) {
      console.error(`[v0] Failed to create WebSocket for VM ${this.vmId}:`, error)
      this.onStateChange({
        error: "Failed to create WebSocket connection",
        isConnecting: false,
      })
    }
  }

  private scheduleReconnect() {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer)
    }

    this.reconnectAttempts++
    const delay = Math.min(this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1), 30000)

    console.log(`[v0] Scheduling reconnect attempt ${this.reconnectAttempts} in ${delay}ms`)

    this.reconnectTimer = setTimeout(() => {
      if (!this.isManuallyDisconnected) {
        this.connect()
      }
    }, delay)
  }

  disconnect() {
    this.isManuallyDisconnected = true

    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer)
      this.reconnectTimer = null
    }

    if (this.ws) {
      this.ws.close()
      this.ws = null
    }

    this.onStateChange({
      isConnected: false,
      isConnecting: false,
    })
  }

  pause() {
    this.disconnect()
  }

  resume() {
    this.connect()
  }
}

export function useMetricsWebSocket(vmId: string) {
  const [isConnected, setIsConnected] = useState(false)
  const clientRef = useRef<MetricsWebSocketClient | null>(null)

  const connect = useCallback(
    (onMessage: (data: any) => void) => {
      if (clientRef.current) {
        clientRef.current.disconnect()
      }

      const handleStateChange = (state: Partial<WebSocketState>) => {
        setIsConnected(state.isConnected ?? false)
      }

      clientRef.current = new MetricsWebSocketClient(vmId, onMessage, handleStateChange)
      clientRef.current.connect()
    },
    [vmId],
  )

  const disconnect = useCallback(() => {
    if (clientRef.current) {
      clientRef.current.disconnect()
      clientRef.current = null
    }
    setIsConnected(false)
  }, [])

  useEffect(() => {
    return () => {
      if (clientRef.current) {
        clientRef.current.disconnect()
      }
    }
  }, [])

  return {
    connect,
    disconnect,
    isConnected,
  }
}


