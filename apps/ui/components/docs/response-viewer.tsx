"use client"

import { useMemo } from "react"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { CopyButton } from "./copy-button"
import { SchemaViewer } from "./schema-viewer"
import { HighlightedCode, useHighlighter } from "./code-example"
import { buildSampleObject, sampleValue } from "@/lib/docs/sample-values"
import type { ParsedResponse, ResolvedSchema } from "@/lib/docs/openapi-types"
import { cn } from "@/lib/utils"

function statusColor(status: string) {
  const code = parseInt(status)
  if (code >= 200 && code < 300) return "text-green-500"
  if (code >= 300 && code < 400) return "text-blue-500"
  if (code >= 400 && code < 500) return "text-amber-500"
  return "text-red-500"
}

function buildExample(schema: ResolvedSchema): string {
  if (schema.type === "array" && schema.items) {
    return JSON.stringify([sampleValue("item", schema.items)], null, 2)
  }
  if (schema.properties) {
    return JSON.stringify(buildSampleObject(schema), null, 2)
  }
  return JSON.stringify(sampleValue("value", schema), null, 2)
}

export function ResponseViewer({ responses }: { responses: ParsedResponse[] }) {
  const highlighter = useHighlighter()

  const defaultTab = responses.find((r) => r.status.startsWith("2"))?.status ?? responses[0]?.status

  const examples = useMemo(() => {
    const map: Record<string, string> = {}
    for (const r of responses) {
      if (r.schema) {
        map[r.status] = buildExample(r.schema)
      }
    }
    return map
  }, [responses])

  if (responses.length === 0) return null

  return (
    <div>
      <h4 className="mb-2 text-sm font-semibold text-foreground">Response</h4>
      <Tabs defaultValue={defaultTab} className="w-full">
        <TabsList className="h-8 w-full justify-start rounded-b-none border border-b-0 border-border bg-muted/50 p-0">
          {responses.map((r) => (
            <TabsTrigger
              key={r.status}
              value={r.status}
              className="rounded-none border-b-2 border-transparent px-3 py-1 text-xs data-[state=active]:border-orange-500 data-[state=active]:bg-transparent data-[state=active]:shadow-none"
            >
              <span className={cn("font-mono font-bold", statusColor(r.status))}>
                {r.status}
              </span>
            </TabsTrigger>
          ))}
        </TabsList>
        {responses.map((r) => (
          <TabsContent key={r.status} value={r.status} className="mt-0">
            <div className="rounded-b-lg border border-t-0 border-border p-4">
              <p className="mb-3 text-xs text-muted-foreground">{r.description}</p>
              {r.schema && (
                <div className="space-y-4">
                  <SchemaViewer schema={r.schema} title="Schema" />
                  {examples[r.status] && (
                    <div>
                      <div className="mb-1 flex items-center justify-between">
                        <h5 className="text-xs font-medium text-muted-foreground">
                          Example
                        </h5>
                        <CopyButton value={examples[r.status]} />
                      </div>
                      <HighlightedCode
                        code={examples[r.status]}
                        lang="json"
                        highlighter={highlighter}
                      />
                    </div>
                  )}
                </div>
              )}
            </div>
          </TabsContent>
        ))}
      </Tabs>
    </div>
  )
}
