"use client"

import { MethodBadge } from "./method-badge"
import { ParamTable } from "./param-table"
import { SchemaViewer } from "./schema-viewer"
import { CodeExample } from "./code-example"
import { ResponseViewer } from "./response-viewer"
import { ApiPlayground } from "./api-playground"
import { Shield } from "lucide-react"
import type { ParsedEndpoint } from "@/lib/docs/openapi-types"

function HighlightedPath({ path }: { path: string }) {
  // Highlight {params} in orange
  const parts = path.split(/(\{[^}]+\})/)
  return (
    <span className="font-mono text-lg font-semibold">
      {parts.map((part, i) =>
        part.startsWith("{") ? (
          <span key={i} className="text-orange-500">
            {part}
          </span>
        ) : (
          <span key={i}>{part}</span>
        )
      )}
    </span>
  )
}

export function ApiEndpoint({ endpoint }: { endpoint: ParsedEndpoint }) {
  return (
    <div className="flex flex-col gap-8 py-8 lg:flex-row">
      {/* Left column — descriptions & params */}
      <div className="flex-1 space-y-6 lg:max-w-[60%]">
        {/* Header */}
        <div>
          <div className="flex items-center gap-3">
            <MethodBadge method={endpoint.method} />
            <HighlightedPath path={endpoint.path} />
          </div>
          {endpoint.security && (
            <div className="mt-2 flex items-center gap-1.5 text-xs text-muted-foreground">
              <Shield className="h-3.5 w-3.5" />
              <span>Requires authentication</span>
            </div>
          )}
        </div>

        {/* Description */}
        {(endpoint.summary || endpoint.description) && (
          <div className="prose prose-sm dark:prose-invert max-w-none">
            {endpoint.summary && <p className="text-base">{endpoint.summary}</p>}
            {endpoint.description && endpoint.description !== endpoint.summary && (
              <p>{endpoint.description}</p>
            )}
          </div>
        )}

        {/* Parameters */}
        {endpoint.parameters.length > 0 && (
          <div>
            <h3 className="mb-3 text-sm font-semibold text-foreground">Parameters</h3>
            <ParamTable params={endpoint.parameters} />
          </div>
        )}

        {/* Request Body */}
        {endpoint.requestBody && (
          <div>
            <h3 className="mb-3 text-sm font-semibold text-foreground">
              Request Body
              {endpoint.requestBodyRequired && (
                <span className="ml-2 text-xs font-normal text-orange-500">required</span>
              )}
            </h3>
            <SchemaViewer schema={endpoint.requestBody} />
          </div>
        )}

        {/* Responses (mobile only — also shown in right column on desktop) */}
        <div className="lg:hidden">
          <ResponseViewer responses={endpoint.responses} />
        </div>
      </div>

      {/* Right column — code examples, responses, playground */}
      <div className="w-full space-y-6 lg:w-[40%] lg:min-w-[380px]">
        <div className="lg:sticky lg:top-[72px]">
          <div className="space-y-6">
            <CodeExample endpoint={endpoint} />
            <div className="hidden lg:block">
              <ResponseViewer responses={endpoint.responses} />
            </div>
            <ApiPlayground endpoint={endpoint} />
          </div>
        </div>
      </div>
    </div>
  )
}
