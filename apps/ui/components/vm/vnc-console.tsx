"use client"

import { useEffect, useRef, useState } from "react"
// @ts-expect-error - @novnc/novnc ships untyped ESM
import RFB from "@novnc/novnc/lib/rfb"

interface VncConsoleProps {
  vmId: string
  /** Optional base URL override; defaults to current page host on the manager port. */
  managerBase?: string
  /** Fit-to-viewport. Defaults to true. */
  scaleViewport?: boolean
}

/**
 * In-browser VNC console powered by noVNC. Connects to the manager's
 * WebSocket-multiplexed VNC bridge at `/v1/vms/:id/console/vnc/ws`, which
 * in turn proxies to the agent's `/agent/v1/vmm/:id/console/vnc/ws` and
 * onward to QEMU's per-VM VNC UDS.
 *
 * Used to drive Windows install Setup, graphical Linux installers, or
 * any other graphical workload inside a QEMU VM. Headless Linux VMs
 * should use the serial console widget instead.
 */
export function VncConsole({
  vmId,
  managerBase,
  scaleViewport = true,
}: VncConsoleProps) {
  const containerRef = useRef<HTMLDivElement | null>(null)
  const rfbRef = useRef<any>(null)
  const [status, setStatus] = useState<"connecting" | "connected" | "disconnected" | "error">("connecting")
  const [message, setMessage] = useState<string>("")

  useEffect(() => {
    if (!containerRef.current) return

    // Derive the WebSocket URL. The browser is talking to the Next.js
    // dev server on :3000; the manager listens on :18080. Allow operator
    // override via prop or NEXT_PUBLIC_API_BASE_URL.
    const base =
      managerBase ||
      process.env.NEXT_PUBLIC_API_BASE_URL?.replace(/\/v1\/?$/, "") ||
      `${window.location.protocol}//${window.location.hostname}:18080`
    const wsBase = base.replace(/^http/, "ws")
    const url = `${wsBase}/v1/vms/${vmId}/console/vnc/ws`

    let rfb: any
    try {
      rfb = new RFB(containerRef.current, url, {
        wsProtocols: ["binary"],
      })
      rfb.viewOnly = false
      rfb.scaleViewport = scaleViewport
      rfb.resizeSession = false
      rfb.background = "rgb(20,20,20)"

      rfb.addEventListener("connect", () => {
        setStatus("connected")
        setMessage("")
      })
      rfb.addEventListener("disconnect", (e: any) => {
        setStatus("disconnected")
        setMessage(e?.detail?.reason || "Disconnected")
      })
      rfb.addEventListener("securityfailure", (e: any) => {
        setStatus("error")
        setMessage(`Security failure: ${e?.detail?.reason || "unknown"}`)
      })
      rfbRef.current = rfb
    } catch (err: any) {
      setStatus("error")
      setMessage(err?.message || String(err))
    }

    return () => {
      try {
        rfbRef.current?.disconnect?.()
      } catch {}
      rfbRef.current = null
    }
  }, [vmId, managerBase, scaleViewport])

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between text-xs text-muted-foreground">
        <span>VNC console for VM {vmId.slice(0, 8)}</span>
        <span
          className={
            status === "connected"
              ? "text-green-600"
              : status === "error"
                ? "text-red-600"
                : "text-amber-600"
          }
        >
          {status}
          {message ? `: ${message}` : ""}
        </span>
      </div>
      <div
        ref={containerRef}
        className="relative h-[600px] w-full overflow-hidden rounded-lg border border-border bg-black"
        // Make sure focusable so keyboard input reaches the guest.
        tabIndex={0}
      />
      <div className="flex gap-2 text-xs">
        <button
          type="button"
          className="rounded border border-border px-2 py-1 hover:bg-muted"
          onClick={() => rfbRef.current?.sendCtrlAltDel?.()}
        >
          Send Ctrl+Alt+Del
        </button>
        <button
          type="button"
          className="rounded border border-border px-2 py-1 hover:bg-muted"
          onClick={() =>
            rfbRef.current?.machineShutdown?.() // best-effort; QEMU ignores
          }
        >
          Power off (signal)
        </button>
      </div>
    </div>
  )
}
