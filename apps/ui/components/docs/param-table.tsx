"use client"

import type { ParsedParam } from "@/lib/docs/openapi-types"
import { cn } from "@/lib/utils"

function formatType(schema: { type?: string; format?: string; nullable?: boolean; enum?: string[] }): string {
  let t = schema.type ?? "any"
  if (schema.format) t += ` (${schema.format})`
  if (schema.nullable) t += " | null"
  return t
}

function ParamRow({ param }: { param: ParsedParam }) {
  return (
    <tr className="border-b border-border last:border-0">
      <td className="px-3 py-2.5 align-top">
        <div className="flex items-center gap-2">
          <code className="text-sm font-semibold text-foreground">{param.name}</code>
          {param.required && (
            <span className="h-1.5 w-1.5 rounded-full bg-orange-500" title="Required" />
          )}
        </div>
      </td>
      <td className="px-3 py-2.5 align-top">
        <code className="text-xs text-muted-foreground">{formatType(param.schema)}</code>
      </td>
      <td className="px-3 py-2.5 align-top">
        <span
          className={cn(
            "inline-flex rounded px-1.5 py-0.5 text-[10px] font-medium",
            param.in === "path"
              ? "bg-orange-100 text-orange-700 dark:bg-orange-900/40 dark:text-orange-400"
              : param.in === "query"
                ? "bg-blue-100 text-blue-700 dark:bg-blue-900/40 dark:text-blue-400"
                : "bg-muted text-muted-foreground"
          )}
        >
          {param.in}
        </span>
      </td>
      <td className="px-3 py-2.5 align-top text-xs text-muted-foreground">
        {param.description}
        {param.schema.enum && (
          <div className="mt-1 flex flex-wrap gap-1">
            {param.schema.enum.map((v) => (
              <code key={v} className="rounded bg-muted px-1 py-0.5 text-[10px]">
                {v}
              </code>
            ))}
          </div>
        )}
      </td>
    </tr>
  )
}

export function ParamTable({ params }: { params: ParsedParam[] }) {
  if (params.length === 0) return null

  const pathParams = params.filter((p) => p.in === "path")
  const queryParams = params.filter((p) => p.in === "query")
  const headerParams = params.filter((p) => p.in === "header")

  const groups = [
    { label: "Path Parameters", items: pathParams },
    { label: "Query Parameters", items: queryParams },
    { label: "Header Parameters", items: headerParams },
  ].filter((g) => g.items.length > 0)

  return (
    <div className="space-y-4">
      {groups.map((group) => (
        <div key={group.label}>
          <h4 className="mb-2 text-sm font-semibold text-foreground">{group.label}</h4>
          <div className="overflow-hidden rounded-lg border border-border">
            <table className="w-full text-left">
              <thead>
                <tr className="border-b border-border bg-muted/50">
                  <th className="px-3 py-2 text-xs font-medium text-muted-foreground">Name</th>
                  <th className="px-3 py-2 text-xs font-medium text-muted-foreground">Type</th>
                  <th className="px-3 py-2 text-xs font-medium text-muted-foreground">In</th>
                  <th className="px-3 py-2 text-xs font-medium text-muted-foreground">Description</th>
                </tr>
              </thead>
              <tbody>
                {group.items.map((p) => (
                  <ParamRow key={p.name} param={p} />
                ))}
              </tbody>
            </table>
          </div>
        </div>
      ))}
    </div>
  )
}
