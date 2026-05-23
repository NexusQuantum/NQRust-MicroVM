// Install global demo-mode interceptors. Idempotent: safe to call more than
// once per page session.
//
// What it patches:
//   1. `apiClient.request` — short-circuits to the mock router.
//   2. `window.fetch` — for the small set of `fetch(...)` calls inside the app
//      that bypass apiClient (avatar upload, image upload, etc.).
//   3. `window.WebSocket` — stubs out shell/metrics/log streams with a fake
//      socket that emits canned events.

import { DEMO_MODE } from "./flag"
import { handleMockRequest } from "./router"
import { apiClient } from "@/lib/api/http"

const INSTALLED_FLAG = "__nqr_demo_installed__"

export function installDemoMode() {
  if (!DEMO_MODE) return
  if (typeof window === "undefined") return
  if ((window as any)[INSTALLED_FLAG]) return
  ;(window as any)[INSTALLED_FLAG] = true

  patchApiClient()
  patchFetch()
  patchWebSocket()

  // Make sure the auth/token check inside the SPA never explodes — the
  // bootstrap component below also seeds an auth token. We just guard
  // against double-init here.
}

function patchApiClient() {
  const original = apiClient.request.bind(apiClient)
  ;(apiClient as any).request = async function patched(endpoint: string, options: RequestInit = {}) {
    const method = (options.method || "GET").toUpperCase()
    let body: any = undefined
    if (typeof options.body === "string") {
      try { body = JSON.parse(options.body) } catch { body = options.body }
    } else if (options.body) {
      body = options.body
    }
    // Simulate a small but non-zero latency so loading states still flash.
    await delay(80 + Math.random() * 160)
    try {
      const result = await handleMockRequest({ method, path: endpoint, body })
      return result as any
    } catch (e) {
      throw e
    }
  }
  // keep `original` reachable for debugging
  ;(apiClient as any)._originalRequest = original
}

function patchFetch() {
  const original = window.fetch.bind(window)
  window.fetch = (async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = typeof input === "string" ? input : (input instanceof URL ? input.toString() : input.url)
    const base = apiClient.baseURL
    if (url.startsWith(base)) {
      const path = url.slice(base.length)
      const method = (init?.method || "GET").toUpperCase()
      let body: any = undefined
      if (init?.body && typeof init.body === "string") {
        try { body = JSON.parse(init.body) } catch { body = init.body }
      }
      await delay(120 + Math.random() * 160)
      const data = await handleMockRequest({ method, path, body })
      return new Response(JSON.stringify(data ?? {}), { status: 200, headers: { "Content-Type": "application/json" } })
    }
    return original(input as any, init)
  }) as any
}

function patchWebSocket() {
  const NativeWS = window.WebSocket
  class DemoWebSocket {
    url: string
    readyState = 0
    onopen: ((e: any) => void) | null = null
    onmessage: ((e: any) => void) | null = null
    onerror: ((e: any) => void) | null = null
    onclose: ((e: any) => void) | null = null
    static CONNECTING = 0
    static OPEN = 1
    static CLOSING = 2
    static CLOSED = 3
    private _interval: any = null
    private _kind: "metrics" | "logs" | "shell" | "unknown"
    constructor(url: string) {
      this.url = url
      // Classify by URL substring.
      if (url.includes("/metrics")) this._kind = "metrics"
      else if (url.includes("/logs")) this._kind = "logs"
      else if (url.includes("/shell")) this._kind = "shell"
      else this._kind = "unknown"
      setTimeout(() => {
        this.readyState = 1
        this.onopen?.({})
        this._start()
      }, 80)
    }
    _start() {
      const emit = (data: any) => this.onmessage?.({ data: typeof data === "string" ? data : JSON.stringify(data) })
      if (this._kind === "metrics") {
        this._interval = setInterval(() => {
          emit({
            timestamp: new Date().toISOString(),
            cpu_usage_percent: 20 + Math.random() * 60,
            memory_usage_percent: 30 + Math.random() * 50,
            memory_used_kb: 1024 * 1024,
            memory_total_kb: 2 * 1024 * 1024,
            load_average: 0.5 + Math.random(),
          })
        }, 2000)
      } else if (this._kind === "logs") {
        const samples = [
          "[info] demo mode: log stream is simulated",
          "[info] GET /healthz -> 200",
          "[debug] cache miss key=user:42",
          "[info] worker accepted job #1284",
        ]
        let i = 0
        this._interval = setInterval(() => {
          emit({
            container_id: "demo",
            timestamp: new Date().toISOString(),
            stream: i % 4 === 3 ? "stderr" : "stdout",
            message: samples[i % samples.length],
          })
          i++
        }, 1500)
      } else if (this._kind === "shell") {
        emit("\x1b[1;33mDemo mode\x1b[0m: interactive shell is disabled.\r\n")
        emit("Try the live build of NQR-MicroVM to get a real terminal.\r\n$ ")
      }
    }
    send(_: any) {
      // ignore in demo mode
    }
    close() {
      this.readyState = 3
      if (this._interval) clearInterval(this._interval)
      this.onclose?.({})
    }
    addEventListener(name: string, cb: (e: any) => void) {
      if (name === "open") this.onopen = cb
      else if (name === "message") this.onmessage = cb
      else if (name === "error") this.onerror = cb
      else if (name === "close") this.onclose = cb
    }
    removeEventListener() { /* noop */ }
  }
  // Replace globally so any new WebSocket(...) uses our shim. Keep the native
  // class around in case something explicitly references it later.
  ;(window as any)._NativeWebSocket = NativeWS
  ;(window as any).WebSocket = DemoWebSocket
}

function delay(ms: number) {
  return new Promise((r) => setTimeout(r, ms))
}
