import * as fs from "fs"
import * as path from "path"
import * as yaml from "js-yaml"
import type {
  ParsedEndpoint,
  ParsedParam,
  ParsedResponse,
  ParsedTag,
  ResolvedSchema,
  NavIndex,
  NavTag,
} from "../lib/docs/openapi-types"

const SPEC_PATH = path.resolve(__dirname, "../../../openapi/manager/openapi.yaml")
const OUTPUT_DIR = path.resolve(__dirname, "../content/api")

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type Spec = any

function loadSpec(): Spec {
  const raw = fs.readFileSync(SPEC_PATH, "utf-8")
  return yaml.load(raw) as Spec
}

function resolveRef(spec: Spec, ref: string): Spec | null {
  // "#/components/schemas/Foo" → spec.components.schemas.Foo
  const parts = ref.replace("#/", "").split("/")
  let current = spec
  for (const p of parts) {
    current = current[p]
    if (!current) {
      console.warn(`Warning: Cannot resolve $ref: ${ref}, treating as opaque`)
      return null
    }
  }
  return current
}

function resolveSchema(spec: Spec, raw: Spec): ResolvedSchema {
  if (!raw) return {}

  if (raw.$ref) {
    const resolved = resolveRef(spec, raw.$ref)
    if (!resolved) {
      // Unresolvable ref — extract name and treat as string type
      const name = raw.$ref.split("/").pop() ?? "unknown"
      return { type: "string", description: `(${name})` }
    }
    return resolveSchema(spec, resolved)
  }

  if (raw.allOf) {
    // Merge allOf entries — handle $ref + nullable pattern from utoipa
    let merged: ResolvedSchema = {}
    for (const entry of raw.allOf) {
      const resolved = resolveSchema(spec, entry)
      merged = { ...merged, ...resolved }
      // Merge required arrays
      if (resolved.required && merged.required) {
        merged.required = [...new Set([...merged.required, ...resolved.required])]
      }
      // Merge properties
      if (resolved.properties && merged.properties) {
        merged.properties = { ...merged.properties, ...resolved.properties }
      }
    }
    if (raw.nullable) merged.nullable = true
    if (raw.description) merged.description = raw.description
    return merged
  }

  const schema: ResolvedSchema = {}
  if (raw.type) schema.type = raw.type
  if (raw.format) schema.format = raw.format
  if (raw.nullable) schema.nullable = true
  if (raw.description) schema.description = raw.description
  if (raw.enum) schema.enum = raw.enum
  if (raw.minimum !== undefined) schema.minimum = raw.minimum
  if (raw.maximum !== undefined) schema.maximum = raw.maximum

  if (raw.items) {
    schema.items = resolveSchema(spec, raw.items)
  }

  if (raw.properties) {
    schema.properties = {}
    for (const [key, val] of Object.entries(raw.properties)) {
      schema.properties[key] = resolveSchema(spec, val)
    }
  }

  if (raw.additionalProperties) {
    schema.additionalProperties =
      typeof raw.additionalProperties === "boolean"
        ? raw.additionalProperties
        : resolveSchema(spec, raw.additionalProperties)
  }

  if (raw.required) schema.required = raw.required

  if (raw.oneOf) {
    schema.oneOf = raw.oneOf.map((s: Spec) => resolveSchema(spec, s))
  }

  return schema
}

function makeSlug(method: string, urlPath: string): string {
  // /v1/vms/{id}/drives/{drive_id} → vms-id-drives-drive_id
  const cleaned = urlPath
    .replace(/^\/v1\//, "")
    .replace(/\{([^}]+)\}/g, "$1")
    .replace(/\//g, "-")
    .replace(/-+/g, "-")
    .replace(/-$/, "")
  return `${method}-${cleaned}`
}

function tagSlug(name: string): string {
  return name.toLowerCase().replace(/\s+/g, "-")
}

/** Fix utoipa bug: params marked as in:path that don't appear in URL template */
function fixParamLocation(params: ParsedParam[], urlPath: string): ParsedParam[] {
  const pathSegments = urlPath.match(/\{([^}]+)\}/g)?.map((s) => s.slice(1, -1)) ?? []
  return params.map((p) => {
    if (p.in === "path" && !pathSegments.includes(p.name)) {
      return { ...p, in: "query", required: false }
    }
    return p
  })
}

function parseEndpoint(
  spec: Spec,
  urlPath: string,
  method: string,
  op: Spec
): ParsedEndpoint {
  const tag = op.tags?.[0] ?? "Other"

  let parameters: ParsedParam[] = (op.parameters ?? []).map((p: Spec) => ({
    name: p.name,
    in: p.in,
    required: p.required ?? false,
    description: p.description,
    schema: resolveSchema(spec, p.schema ?? {}),
  }))

  parameters = fixParamLocation(parameters, urlPath)

  let requestBody: ResolvedSchema | undefined
  let requestBodyRequired: boolean | undefined
  if (op.requestBody) {
    const content = op.requestBody.content?.["application/json"]
    if (content?.schema) {
      requestBody = resolveSchema(spec, content.schema)
      requestBodyRequired = op.requestBody.required
    }
  }

  const responses: ParsedResponse[] = Object.entries(op.responses ?? {}).map(
    ([status, resp]: [string, Spec]) => {
      const r: ParsedResponse = { status, description: resp.description ?? "" }
      const content = resp.content?.["application/json"]
      if (content?.schema) {
        r.schema = resolveSchema(spec, content.schema)
      }
      return r
    }
  )

  const hasSecurity = op.security !== undefined
    ? op.security.length > 0
    : (spec.security ?? []).length > 0

  return {
    tag,
    path: urlPath,
    method: method.toUpperCase(),
    slug: makeSlug(method, urlPath),
    operationId: op.operationId,
    summary: op.summary,
    description: op.description,
    security: hasSecurity,
    parameters,
    requestBody,
    requestBodyRequired,
    responses,
  }
}

function main() {
  const spec = loadSpec()

  // Collect all endpoints grouped by tag
  const tagMap = new Map<string, ParsedEndpoint[]>()

  // Initialize with tag descriptions from spec
  const tagDescriptions = new Map<string, string>()
  for (const t of spec.tags ?? []) {
    tagDescriptions.set(t.name, t.description ?? "")
  }

  const METHODS = ["get", "post", "put", "patch", "delete"]

  for (const [urlPath, methods] of Object.entries(spec.paths ?? {})) {
    for (const method of METHODS) {
      const op = (methods as Spec)[method]
      if (!op) continue

      const endpoint = parseEndpoint(spec, urlPath, method, op)
      const existing = tagMap.get(endpoint.tag) ?? []
      existing.push(endpoint)
      tagMap.set(endpoint.tag, existing)
    }
  }

  // Ensure output dir exists
  fs.mkdirSync(OUTPUT_DIR, { recursive: true })

  // Write per-tag JSON
  const tags: ParsedTag[] = []
  for (const [name, endpoints] of tagMap) {
    const tag: ParsedTag = {
      name,
      description: tagDescriptions.get(name),
      endpoints,
    }
    tags.push(tag)
    const slug = tagSlug(name)
    fs.writeFileSync(
      path.join(OUTPUT_DIR, `${slug}.json`),
      JSON.stringify(tag, null, 2)
    )
  }

  // Write navigation index
  const navIndex: NavIndex = {
    tags: tags.map((t) => {
      const nav: NavTag = {
        name: t.name,
        slug: tagSlug(t.name),
        description: t.description,
        endpoints: t.endpoints.map((e) => ({
          slug: e.slug,
          method: e.method,
          path: e.path,
          summary: e.summary ?? e.responses[0]?.description,
        })),
      }
      return nav
    }),
  }

  fs.writeFileSync(
    path.join(OUTPUT_DIR, "_index.json"),
    JSON.stringify(navIndex, null, 2)
  )

  console.log(
    `Parsed ${tags.reduce((s, t) => s + t.endpoints.length, 0)} endpoints across ${tags.length} tags`
  )
}

main()
