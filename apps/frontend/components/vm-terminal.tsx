"use client"

import { useEffect, useRef, useState } from "react"
import { Terminal } from "xterm"
import { FitAddon } from "@xterm/addon-fit"
import { WebLinksAddon } from "@xterm/addon-web-links"
import "xterm/css/xterm.css"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { Badge } from "@/components/ui/badge"
import { Terminal as TerminalIcon, RefreshCw, Copy, Check } from "lucide-react"
import { facadeApi } from "@/lib/api/facade"
import type { Vm } from "@/types/nexus"

interface VMTerminalProps {
  vm: Vm
}

export function VMTerminal({ vm }: VMTerminalProps) {
  const terminalRef = useRef<HTMLDivElement>(null)
  const terminalInstance = useRef<Terminal | null>(null)
  const fitAddon = useRef<FitAddon | null>(null)
  const ws = useRef<WebSocket | null>(null)
  const [connectionState, setConnectionState] = useState<"connecting" | "connected" | "disconnected" | "error">("disconnected")
  const [credentials, setCredentials] = useState<{ username: string; password: string } | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [copiedField, setCopiedField] = useState<string | null>(null)

  // Fetch credentials
  useEffect(() => {
    const fetchCredentials = async () => {
      try {
        const creds = await facadeApi.getShellCredentials(vm.id)
        setCredentials(creds)
      } catch (err) {
        console.error("Failed to fetch shell credentials:", err)
        setError("Failed to fetch shell credentials. VM may not have shell access configured.")
      }
    }

    if (vm.id) {
      fetchCredentials()
    }
  }, [vm.id])

  const copyToClipboard = async (text: string, field: string) => {
    try {
      await navigator.clipboard.writeText(text)
      setCopiedField(field)
      setTimeout(() => setCopiedField(null), 2000)
    } catch (err) {
      console.error("Failed to copy:", err)
    }
  }

  const connect = () => {
    if (!terminalRef.current) return

    // Clean up existing connection
    disconnect()

    setConnectionState("connecting")
    setError(null)

    try {
      // Create terminal
      const terminal = new Terminal({
        cursorBlink: true,
        fontSize: 14,
        fontFamily: 'Menlo, Monaco, "Courier New", monospace',
        theme: {
          background: "#1e1e1e",
          foreground: "#d4d4d4",
          cursor: "#d4d4d4",
          selectionBackground: "#264f78",
          black: "#000000",
          red: "#cd3131",
          green: "#0dbc79",
          yellow: "#e5e510",
          blue: "#2472c8",
          magenta: "#bc3fbc",
          cyan: "#11a8cd",
          white: "#e5e5e5",
          brightBlack: "#666666",
          brightRed: "#f14c4c",
          brightGreen: "#23d18b",
          brightYellow: "#f5f543",
          brightBlue: "#3b8eea",
          brightMagenta: "#d670d6",
          brightCyan: "#29b8db",
          brightWhite: "#e5e5e5",
        },
        scrollback: 10000,
      })

      // Add addons
      const fit = new FitAddon()
      fitAddon.current = fit
      terminal.loadAddon(fit)
      terminal.loadAddon(new WebLinksAddon())

      // Open terminal
      terminal.open(terminalRef.current)
      fit.fit()

      // Handle resize
      const handleResize = () => {
        if (fit) {
          fit.fit()
        }
      }
      window.addEventListener("resize", handleResize)

      terminalInstance.current = terminal

      // Connect WebSocket
      const wsUrl = facadeApi.getShellWebSocketUrl(vm.id)
      const websocket = new WebSocket(wsUrl)
      ws.current = websocket

      websocket.onopen = () => {
        setConnectionState("connected")
        terminal.writeln("\x1b[1;32m✓ Connected to VM shell\x1b[0m")
        terminal.writeln("")

        if (credentials) {
          terminal.writeln("\x1b[1;36mLogin credentials:\x1b[0m")
          terminal.writeln(`  Username: \x1b[1m${credentials.username}\x1b[0m`)
          terminal.writeln(`  Password: \x1b[1m${credentials.password}\x1b[0m`)
          terminal.writeln("")
        }
      }

      websocket.onerror = (err) => {
        console.error("WebSocket error:", err)
        setConnectionState("error")
        setError("WebSocket connection failed. The VM may not have a serial console configured.")
        terminal.writeln("\x1b[1;31m✗ Connection error\x1b[0m")
      }

      websocket.onclose = () => {
        setConnectionState("disconnected")
        terminal.writeln("")
        terminal.writeln("\x1b[1;33m⚠ Connection closed\x1b[0m")
      }

      websocket.onmessage = (event) => {
        if (typeof event.data === "string") {
          terminal.write(event.data)
        } else if (event.data instanceof ArrayBuffer) {
          terminal.write(new Uint8Array(event.data))
        } else if (event.data instanceof Blob) {
          event.data.arrayBuffer().then((buffer) => {
            terminal.write(new Uint8Array(buffer))
          })
        }
      }

      // Send terminal input to WebSocket
      terminal.onData((data) => {
        if (websocket.readyState === WebSocket.OPEN) {
          websocket.send(data)
        }
      })

      // Cleanup function
      return () => {
        window.removeEventListener("resize", handleResize)
      }
    } catch (err) {
      console.error("Failed to create terminal:", err)
      setConnectionState("error")
      setError(`Failed to initialize terminal: ${err instanceof Error ? err.message : String(err)}`)
    }
  }

  const disconnect = () => {
    if (ws.current) {
      ws.current.close()
      ws.current = null
    }
    if (terminalInstance.current) {
      terminalInstance.current.dispose()
      terminalInstance.current = null
    }
    fitAddon.current = null
    setConnectionState("disconnected")
  }

  useEffect(() => {
    // Cleanup on unmount
    return () => {
      disconnect()
    }
  }, [])

  const getStateColor = () => {
    switch (connectionState) {
      case "connected":
        return "bg-green-500"
      case "connecting":
        return "bg-yellow-500"
      case "error":
        return "bg-red-500"
      default:
        return "bg-gray-500"
    }
  }

  const getStateText = () => {
    switch (connectionState) {
      case "connected":
        return "Connected"
      case "connecting":
        return "Connecting..."
      case "error":
        return "Error"
      default:
        return "Disconnected"
    }
  }

  return (
    <div className="space-y-4">
      {/* Connection Info Card */}
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <TerminalIcon className="h-5 w-5" />
              <div>
                <CardTitle>VM Console</CardTitle>
                <CardDescription>Interactive shell access to the virtual machine</CardDescription>
              </div>
            </div>
            <div className="flex items-center gap-2">
              <Badge variant="outline" className="gap-2">
                <div className={`h-2 w-2 rounded-full ${getStateColor()}`} />
                {getStateText()}
              </Badge>
              {connectionState === "disconnected" || connectionState === "error" ? (
                <Button onClick={connect} size="sm">
                  <TerminalIcon className="h-4 w-4 mr-2" />
                  Connect
                </Button>
              ) : (
                <Button onClick={disconnect} variant="outline" size="sm">
                  Disconnect
                </Button>
              )}
              {connectionState === "connected" && (
                <Button
                  onClick={() => {
                    disconnect()
                    setTimeout(connect, 100)
                  }}
                  variant="outline"
                  size="sm"
                >
                  <RefreshCw className="h-4 w-4" />
                </Button>
              )}
            </div>
          </div>
        </CardHeader>

        {credentials && (
          <CardContent>
            <div className="rounded-md border bg-muted/50 p-4">
              <h4 className="text-sm font-semibold mb-3">Login Credentials</h4>
              <div className="space-y-2">
                <div className="flex items-center justify-between">
                  <div className="text-sm">
                    <span className="text-muted-foreground">Username:</span>{" "}
                    <code className="bg-background px-2 py-1 rounded text-xs font-mono">
                      {credentials.username}
                    </code>
                  </div>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => copyToClipboard(credentials.username, "username")}
                  >
                    {copiedField === "username" ? (
                      <Check className="h-3 w-3" />
                    ) : (
                      <Copy className="h-3 w-3" />
                    )}
                  </Button>
                </div>
                <div className="flex items-center justify-between">
                  <div className="text-sm">
                    <span className="text-muted-foreground">Password:</span>{" "}
                    <code className="bg-background px-2 py-1 rounded text-xs font-mono">
                      {credentials.password}
                    </code>
                  </div>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => copyToClipboard(credentials.password, "password")}
                  >
                    {copiedField === "password" ? (
                      <Check className="h-3 w-3" />
                    ) : (
                      <Copy className="h-3 w-3" />
                    )}
                  </Button>
                </div>
              </div>
            </div>
          </CardContent>
        )}
      </Card>

      {/* Error Alert */}
      {error && (
        <Alert variant="destructive">
          <AlertDescription>
            {error}
            {error.includes("console") && (
              <div className="mt-2 text-sm">
                <strong>Note:</strong> This VM was created without a serial console socket. To enable
                terminal access, the VM needs to be recreated with console configuration, or you can
                configure the serial device via the VM Config tab.
              </div>
            )}
          </AlertDescription>
        </Alert>
      )}

      {/* VM State Warning */}
      {vm.state !== "running" && (
        <Alert>
          <AlertDescription>
            VM is currently <strong>{vm.state}</strong>. The VM must be running to access the
            console.
          </AlertDescription>
        </Alert>
      )}

      {/* Terminal Container */}
      <Card className="border-2">
        <CardContent className="p-0">
          <div
            ref={terminalRef}
            className="w-full h-[600px] bg-[#1e1e1e]"
            style={{ overflow: "hidden" }}
          />
        </CardContent>
      </Card>

      {/* Help Text */}
      <div className="text-sm text-muted-foreground space-y-1">
        <p>
          <strong>Tip:</strong> This is a direct console connection to the VM. Use the credentials
          above to log in.
        </p>
        <p>
          The terminal supports standard terminal features including copy/paste (Ctrl+Shift+C/V),
          scrollback, and clickable links.
        </p>
      </div>
    </div>
  )
}
