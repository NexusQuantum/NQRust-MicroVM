export interface ResolvedSchema {
  type?: string
  format?: string
  nullable?: boolean
  description?: string
  enum?: string[]
  items?: ResolvedSchema
  properties?: Record<string, ResolvedSchema>
  additionalProperties?: ResolvedSchema | boolean
  required?: string[]
  allOf?: ResolvedSchema[]
  oneOf?: ResolvedSchema[]
  minimum?: number
  maximum?: number
}

export interface ParsedParam {
  name: string
  in: "path" | "query" | "header" | "cookie"
  required: boolean
  description?: string
  schema: ResolvedSchema
}

export interface ParsedResponse {
  status: string
  description: string
  schema?: ResolvedSchema
}

export interface ParsedEndpoint {
  tag: string
  path: string
  method: string
  slug: string
  operationId?: string
  summary?: string
  description?: string
  security?: boolean
  parameters: ParsedParam[]
  requestBody?: ResolvedSchema
  requestBodyRequired?: boolean
  responses: ParsedResponse[]
}

export interface ParsedTag {
  name: string
  description?: string
  endpoints: ParsedEndpoint[]
}

export interface NavEndpoint {
  slug: string
  method: string
  path: string
  summary?: string
}

export interface NavTag {
  name: string
  slug: string
  description?: string
  endpoints: NavEndpoint[]
}

export interface NavIndex {
  tags: NavTag[]
}
