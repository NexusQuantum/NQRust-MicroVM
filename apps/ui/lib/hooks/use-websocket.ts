"use client"

import { useEffect, useRef, useState } from "react"

interface UseWebSocketOptions {
  onMessage?: (data: any) => void
  onOpen?: () => void
  onClose?: () => void
  onError?: (error: Event) => void
  reconnect?: boolean
  reconnectInterval?: number
}

export function useWebSocket(url: string | null, options: UseWebSocketOptions = {}) {
  const { onMessage, onOpen, onClose, onError, reconnect = true, reconnectInterval = 3000 } = options

  const [isConnected, setIsConnected] = useState(false)
  const [lastMessage, setLastMessage] = useState<any>(null)
  const wsRef = useRef<WebSocket | null>(null)
  const reconnectTimeoutRef = useRef<NodeJS.Timeout>()

  useEffect(() => {
    if (!url) return

    const connect = () => {
      try {
        const ws = new WebSocket(url)
        wsRef.current = ws

        ws.onopen = () => {
          setIsConnected(true)
          onOpen?.()
        }

        ws.onmessage = (event) => {
          try {
            const data = JSON.parse(event.data)
            setLastMessage(data)
            onMessage?.(data)
          } catch {
            setLastMessage(event.data)
            onMessage?.(event.data)
          }
        }

        ws.onclose = () => {
          setIsConnected(false)
          onClose?.()

          if (reconnect) {
            reconnectTimeoutRef.current = setTimeout(() => {
              connect()
            }, reconnectInterval)
          }
        }

        ws.onerror = (error) => {
          onError?.(error)
        }
      } catch (error) {
        console.error("WebSocket connection error:", error)
      }
    }

    connect()

    return () => {
      if (reconnectTimeoutRef.current) {
        clearTimeout(reconnectTimeoutRef.current)
      }
      if (wsRef.current) {
        wsRef.current.close()
      }
    }
  }, [url, onMessage, onOpen, onClose, onError, reconnect, reconnectInterval])

  const sendMessage = (data: any) => {
    if (wsRef.current && wsRef.current.readyState === WebSocket.OPEN) {
      wsRef.current.send(typeof data === "string" ? data : JSON.stringify(data))
    }
  }

  return {
    isConnected,
    lastMessage,
    sendMessage,
  }
}
