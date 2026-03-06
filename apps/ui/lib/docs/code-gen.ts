import type { ParsedEndpoint } from "./openapi-types"
import { buildSampleObject } from "./sample-values"

function buildUrl(endpoint: ParsedEndpoint): string {
  let url = endpoint.path
  // Replace path params with sample values
  for (const p of endpoint.parameters.filter((p) => p.in === "path")) {
    const sampleVal = p.schema.format === "uuid"
      ? "550e8400-e29b-41d4-a716-446655440000"
      : p.name
    url = url.replace(`{${p.name}}`, sampleVal)
  }
  return url
}

function buildQueryString(endpoint: ParsedEndpoint): string {
  const queryParams = endpoint.parameters.filter((p) => p.in === "query")
  if (queryParams.length === 0) return ""
  const parts = queryParams.map((p) => `${p.name}=value`)
  return "?" + parts.join("&")
}

function getBodyJson(endpoint: ParsedEndpoint): string | null {
  if (!endpoint.requestBody) return null
  const sample = buildSampleObject(endpoint.requestBody)
  return JSON.stringify(sample, null, 2)
}

export function generateCurl(endpoint: ParsedEndpoint, baseUrl = "http://localhost:18080"): string {
  const url = `${baseUrl}${buildUrl(endpoint)}${buildQueryString(endpoint)}`
  const parts: string[] = [`curl -X ${endpoint.method}`]

  if (endpoint.security) {
    parts.push(`  -H "Authorization: Bearer YOUR_TOKEN"`)
  }

  const body = getBodyJson(endpoint)
  if (body) {
    parts.push(`  -H "Content-Type: application/json"`)
    parts.push(`  -d '${body}'`)
  }

  parts.push(`  "${url}"`)
  return parts.join(" \\\n")
}

export function generateJavaScript(endpoint: ParsedEndpoint, baseUrl = "http://localhost:18080"): string {
  const url = `${baseUrl}${buildUrl(endpoint)}${buildQueryString(endpoint)}`
  const body = getBodyJson(endpoint)

  const headers: Record<string, string> = {}
  if (endpoint.security) headers["Authorization"] = "Bearer YOUR_TOKEN"
  if (body) headers["Content-Type"] = "application/json"

  const hasHeaders = Object.keys(headers).length > 0

  let code = `const response = await fetch("${url}", {\n`
  code += `  method: "${endpoint.method}",\n`

  if (hasHeaders) {
    code += `  headers: {\n`
    for (const [key, val] of Object.entries(headers)) {
      code += `    "${key}": "${val}",\n`
    }
    code += `  },\n`
  }

  if (body) {
    code += `  body: JSON.stringify(${body}),\n`
  }

  code += `});\n\n`
  code += `const data = await response.json();\n`
  code += `console.log(data);`

  return code
}

export function generatePython(endpoint: ParsedEndpoint, baseUrl = "http://localhost:18080"): string {
  const url = `${baseUrl}${buildUrl(endpoint)}${buildQueryString(endpoint)}`
  const body = getBodyJson(endpoint)

  let code = `import requests\n\n`

  if (endpoint.security) {
    code += `headers = {"Authorization": "Bearer YOUR_TOKEN"}\n`
  }

  const method = endpoint.method.toLowerCase()
  const args: string[] = [`"${url}"`]
  if (endpoint.security) args.push("headers=headers")
  if (body) args.push(`json=${body}`)

  code += `response = requests.${method}(\n`
  code += args.map((a) => `    ${a}`).join(",\n")
  code += `\n)\n\n`
  code += `print(response.json())`

  return code
}
