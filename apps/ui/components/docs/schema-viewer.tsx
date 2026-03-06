"use client"

import { useState } from "react"
import { ChevronRight } from "lucide-react"
import { cn } from "@/lib/utils"
import type { ResolvedSchema } from "@/lib/docs/openapi-types"

function formatType(schema: ResolvedSchema): string {
  if (schema.type === "array" && schema.items) {
    return `${formatType(schema.items)}[]`
  }
  let t = schema.type ?? "any"
  if (schema.format) t += ` (${schema.format})`
  if (schema.nullable) t += " | null"
  return t
}

function SchemaProperty({
  name,
  schema,
  required,
  depth,
}: {
  name: string
  schema: ResolvedSchema
  required: boolean
  depth: number
}) {
  const [expanded, setExpanded] = useState(depth < 1)
  const hasChildren =
    schema.properties ||
    (schema.type === "array" && schema.items?.properties) ||
    (schema.additionalProperties && typeof schema.additionalProperties === "object")

  const childSchema =
    schema.type === "array" && schema.items?.properties
      ? schema.items
      : schema

  return (
    <div className={cn("border-l border-border", depth > 0 && "ml-4")}>
      <div
        className={cn(
          "flex items-start gap-2 py-1.5 pl-3 pr-2",
          hasChildren && "cursor-pointer hover:bg-muted/50"
        )}
        onClick={hasChildren ? () => setExpanded(!expanded) : undefined}
      >
        {hasChildren ? (
          <ChevronRight
            className={cn(
              "mt-0.5 h-4 w-4 shrink-0 transition-transform",
              expanded && "rotate-90"
            )}
          />
        ) : (
          <span className="mt-0.5 h-4 w-4 shrink-0" />
        )}
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <code className="text-sm font-semibold text-foreground">{name}</code>
            <span className="text-xs text-muted-foreground">{formatType(schema)}</span>
            {required && (
              <span className="text-[10px] font-medium text-orange-500">required</span>
            )}
          </div>
          {schema.description && (
            <p className="mt-0.5 text-xs text-muted-foreground">{schema.description}</p>
          )}
          {schema.enum && (
            <div className="mt-1 flex flex-wrap gap-1">
              {schema.enum.map((v) => (
                <code
                  key={v}
                  className="rounded bg-muted px-1.5 py-0.5 text-[10px] text-muted-foreground"
                >
                  {v}
                </code>
              ))}
            </div>
          )}
        </div>
      </div>
      {hasChildren && expanded && childSchema.properties && (
        <div className="ml-2">
          {Object.entries(childSchema.properties).map(([key, val]) => (
            <SchemaProperty
              key={key}
              name={key}
              schema={val}
              required={childSchema.required?.includes(key) ?? false}
              depth={depth + 1}
            />
          ))}
        </div>
      )}
    </div>
  )
}

export function SchemaViewer({
  schema,
  title,
}: {
  schema: ResolvedSchema
  title?: string
}) {
  if (!schema.properties && !schema.items?.properties) {
    return (
      <div className="text-sm text-muted-foreground">
        <code>{formatType(schema)}</code>
      </div>
    )
  }

  const target = schema.items?.properties ? schema.items : schema

  return (
    <div>
      {title && (
        <h4 className="mb-2 text-sm font-semibold text-foreground">{title}</h4>
      )}
      <div className="rounded-lg border border-border">
        {target.properties &&
          Object.entries(target.properties).map(([key, val]) => (
            <SchemaProperty
              key={key}
              name={key}
              schema={val}
              required={target.required?.includes(key) ?? false}
              depth={0}
            />
          ))}
      </div>
    </div>
  )
}
