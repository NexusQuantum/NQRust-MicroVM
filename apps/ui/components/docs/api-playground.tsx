"use client"

import { useState, useCallback, useMemo, useEffect } from "react"
import { ChevronRight, Play, Loader2 } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { cn } from "@/lib/utils"
import { CopyButton } from "./copy-button"
import { MethodBadge } from "./method-badge"
import { HighlightedCode, useHighlighter } from "./code-example"
import { buildSampleObject } from "@/lib/docs/sample-values"
import type { ParsedEndpoint } from "@/lib/docs/openapi-types"

const TOKEN_KEY = "nqr-docs-token"
const BASE_URL_KEY = "nqr-docs-base-url"

function getStoredToken(): string {
  if (typeof window === "undefined") return ""
  return localStorage.getItem(TOKEN_KEY) ?? ""
}

function getStoredBaseUrl(): string {
  if (typeof window === "undefined") return ""
  return localStorage.getItem(BASE_URL_KEY) ?? ""
}

function getDefaultBaseUrl(): string {
  if (typeof window === "undefined") return "http://localhost:18080"
  if (process.env.NEXT_PUBLIC_API_BASE_URL) {
    // Remove /v1 suffix if present
    return process.env.NEXT_PUBLIC_API_BASE_URL.replace(/\/v1\/?$/, "")
  }
  const { protocol, hostname } = window.location
  return `${protocol}//${hostname}:18080`
}

export function ApiPlayground({ endpoint }: { endpoint: ParsedEndpoint }) {
  const highlighter = useHighlighter()
  const [expanded, setExpanded] = useState(false)
  const [token, setToken] = useState("")
  const [baseUrl, setBaseUrl] = useState("")
  const [paramValues, setParamValues] = useState<Record<string, string>>({})
  const [body, setBody] = useState("")
  const [loading, setLoading] = useState(false)
  const [response, setResponse] = useState<{
    status: number
    statusText: string
    body: string
    time: number
  } | null>(null)

  // Initialize state from localStorage on mount
  useEffect(() => {
    setToken(getStoredToken())
    setBaseUrl(getStoredBaseUrl() || getDefaultBaseUrl())
  }, [])

  // Initialize body from endpoint schema
  useEffect(() => {
    if (endpoint.requestBody) {
      const sample = buildSampleObject(endpoint.requestBody)
      setBody(JSON.stringify(sample, null, 2))
    } else {
      setBody("")
    }
  }, [endpoint])

  const pathParams = useMemo(
    () => endpoint.parameters.filter((p) => p.in === "path"),
    [endpoint]
  )
  const queryParams = useMemo(
    () => endpoint.parameters.filter((p) => p.in === "query"),
    [endpoint]
  )

  const handleTokenChange = useCallback((val: string) => {
    setToken(val)
    if (typeof window !== "undefined") localStorage.setItem(TOKEN_KEY, val)
  }, [])

  const handleBaseUrlChange = useCallback((val: string) => {
    setBaseUrl(val)
    if (typeof window !== "undefined") localStorage.setItem(BASE_URL_KEY, val)
  }, [])

  const handleSend = useCallback(async () => {
    setLoading(true)
    setResponse(null)

    let url = endpoint.path
    for (const p of pathParams) {
      url = url.replace(`{${p.name}}`, paramValues[p.name] ?? p.name)
    }

    const qp = queryParams.filter((p) => paramValues[p.name])
    if (qp.length > 0) {
      url += "?" + qp.map((p) => `${p.name}=${encodeURIComponent(paramValues[p.name])}`).join("&")
    }

    const fullUrl = `${baseUrl}${url}`
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
    }
    if (token && endpoint.security) {
      headers["Authorization"] = `Bearer ${token}`
    }

    const start = Date.now()
    try {
      const res = await fetch(fullUrl, {
        method: endpoint.method,
        headers,
        body: body && ["POST", "PUT", "PATCH"].includes(endpoint.method) ? body : undefined,
      })
      const elapsed = Date.now() - start
      const text = await res.text()
      let formatted = text
      try {
        formatted = JSON.stringify(JSON.parse(text), null, 2)
      } catch {
        // not JSON
      }
      setResponse({
        status: res.status,
        statusText: res.statusText,
        body: formatted,
        time: elapsed,
      })
    } catch (err) {
      const elapsed = Date.now() - start
      setResponse({
        status: 0,
        statusText: "Network Error",
        body: err instanceof Error ? err.message : "Request failed",
        time: elapsed,
      })
    } finally {
      setLoading(false)
    }
  }, [endpoint, paramValues, body, token, baseUrl, pathParams, queryParams])

  return (
    <div className="rounded-lg border border-border">
      <button
        className="flex w-full items-center gap-2 px-4 py-3 text-sm font-semibold text-foreground hover:bg-muted/50"
        onClick={() => setExpanded(!expanded)}
      >
        <ChevronRight
          className={cn(
            "h-4 w-4 transition-transform",
            expanded && "rotate-90"
          )}
        />
        <Play className="h-4 w-4 text-orange-500" />
        Try it
      </button>
      {expanded && (
        <div className="space-y-4 border-t border-border p-4">
          {/* Base URL */}
          <div>
            <Label className="text-xs text-muted-foreground">Base URL</Label>
            <Input
              value={baseUrl}
              onChange={(e) => handleBaseUrlChange(e.target.value)}
              className="mt-1 h-8 font-mono text-xs"
            />
          </div>

          {/* Auth */}
          {endpoint.security && (
            <div>
              <Label className="text-xs text-muted-foreground">Bearer Token</Label>
              <Input
                type="password"
                value={token}
                onChange={(e) => handleTokenChange(e.target.value)}
                placeholder="Enter your API token..."
                className="mt-1 h-8 font-mono text-xs"
              />
            </div>
          )}

          {/* Path params */}
          {pathParams.length > 0 && (
            <div className="space-y-2">
              <Label className="text-xs text-muted-foreground">Path Parameters</Label>
              {pathParams.map((p) => (
                <div key={p.name} className="flex items-center gap-2">
                  <code className="min-w-[80px] text-xs font-medium">{p.name}</code>
                  <Input
                    value={paramValues[p.name] ?? ""}
                    onChange={(e) =>
                      setParamValues((prev) => ({ ...prev, [p.name]: e.target.value }))
                    }
                    placeholder={p.schema.format === "uuid" ? "UUID" : p.name}
                    className="h-8 font-mono text-xs"
                  />
                </div>
              ))}
            </div>
          )}

          {/* Query params */}
          {queryParams.length > 0 && (
            <div className="space-y-2">
              <Label className="text-xs text-muted-foreground">Query Parameters</Label>
              {queryParams.map((p) => (
                <div key={p.name} className="flex items-center gap-2">
                  <code className="min-w-[80px] text-xs font-medium">{p.name}</code>
                  <Input
                    value={paramValues[p.name] ?? ""}
                    onChange={(e) =>
                      setParamValues((prev) => ({ ...prev, [p.name]: e.target.value }))
                    }
                    placeholder={p.name}
                    className="h-8 font-mono text-xs"
                  />
                </div>
              ))}
            </div>
          )}

          {/* Request body */}
          {endpoint.requestBody && (
            <div>
              <Label className="text-xs text-muted-foreground">Request Body</Label>
              <textarea
                value={body}
                onChange={(e) => setBody(e.target.value)}
                className="mt-1 w-full rounded-md border border-border bg-zinc-900 p-3 font-mono text-xs text-zinc-100 focus:outline-none focus:ring-1 focus:ring-orange-500"
                rows={Math.min(body.split("\n").length + 1, 15)}
              />
            </div>
          )}

          {/* Send button */}
          <Button
            onClick={handleSend}
            disabled={loading}
            className="w-full bg-orange-500 text-white hover:bg-orange-600"
          >
            {loading ? (
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            ) : (
              <Play className="mr-2 h-4 w-4" />
            )}
            Send Request
          </Button>

          {/* Response */}
          {response && (
            <div className="space-y-2">
              <div className="flex items-center gap-3">
                <MethodBadge
                  method={response.status.toString()}
                  className={cn(
                    "font-mono",
                    response.status >= 200 && response.status < 300
                      ? "bg-green-100 text-green-700 dark:bg-green-900/40 dark:text-green-400"
                      : response.status >= 400
                        ? "bg-red-100 text-red-700 dark:bg-red-900/40 dark:text-red-400"
                        : "bg-muted text-muted-foreground"
                  )}
                />
                <span className="text-xs text-muted-foreground">
                  {response.statusText}
                </span>
                <span className="ml-auto text-xs text-muted-foreground">
                  {response.time}ms
                </span>
              </div>
              <div className="relative">
                <CopyButton
                  value={response.body}
                  className="absolute right-2 top-2 text-zinc-400 hover:text-white"
                />
                <HighlightedCode
                  code={response.body}
                  lang="json"
                  highlighter={highlighter}
                />
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  )
}
