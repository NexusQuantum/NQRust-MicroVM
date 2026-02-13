import { generateRequestId } from "@/lib/utils/format"

// Determine API base URL at runtime
// Priority: env var > same-host with manager port > localhost fallback
function getApiBaseUrl(): string {
  // If explicitly set via env var, use that
  if (process.env.NEXT_PUBLIC_API_BASE_URL) {
    return process.env.NEXT_PUBLIC_API_BASE_URL
  }

  // In browser, use same hostname but manager port (18080)
  if (typeof window !== "undefined") {
    const hostname = window.location.hostname
    const protocol = window.location.protocol
    return `${protocol}//${hostname}:18080/v1`
  }

  // Server-side fallback
  return "http://localhost:18080/v1"
}

const API_BASE_URL = getApiBaseUrl()

export interface FacadeError {
  error: string
  fault_message?: string
  status: number
  suggestion?: string
  request_id: string
}

export class ApiClient {
  private baseUrl: string
  private timeout: number
  private getToken: (() => string | null) | null = null

  constructor(baseUrl: string = API_BASE_URL, timeout: number = 30000) {
    this.baseUrl = baseUrl
    this.timeout = timeout
  }

  setTokenGetter(getToken: () => string | null) {
    this.getToken = getToken
  }

  get baseURL(): string {
    return this.baseUrl
  }

  async request<T>(endpoint: string, options: RequestInit = {}): Promise<T> {
    const requestId = generateRequestId()
    const url = `${this.baseUrl}${endpoint}`

    const controller = new AbortController()
    const timeoutId = setTimeout(() => controller.abort(), this.timeout)

    const headers: Record<string, string> = {
      "Content-Type": "application/json",
      "X-Request-Id": requestId,
      ...(options.headers as Record<string, string>),
    }

    // Add auth token if available
    if (this.getToken) {
      const token = this.getToken()
      if (token) {
        headers["Authorization"] = `Bearer ${token}`
      }
    }

    const config: RequestInit = {
      ...options,
      headers,
      signal: controller.signal,
    }

    const startTime = Date.now()

    try {
      const response = await fetch(url, config)
      const latency = Date.now() - startTime

      clearTimeout(timeoutId)

      // Log request details in development
      if (process.env.NODE_ENV === "development") {
        console.log({
          request_id: requestId,
          path: endpoint,
          status: response.status,
          latency_ms: latency,
        })
      }

      if (!response.ok) {
        const errorData: Partial<FacadeError> = await response
          .json()
          .catch(() => ({}))

        const facadeError: FacadeError = {
          error: errorData.error || "Unknown error",
          fault_message: errorData.fault_message,
          status: response.status,
          suggestion: errorData.suggestion,
          request_id: errorData.request_id || requestId,
        }

        throw new Error(JSON.stringify(facadeError))
      }

      // Handle 204 No Content responses
      if (response.status === 204) {
        return undefined as T
      }

      return await response.json()
    } catch (error) {
      clearTimeout(timeoutId)

      if (error instanceof Error) {
        // Try to parse as FacadeError first
        let facadeError: FacadeError | null = null
        try {
          facadeError = JSON.parse(error.message) as FacadeError
        } catch {
          // Not a valid JSON, will handle below
        }

        // If it's a valid FacadeError with status code, re-throw as-is
        if (facadeError && facadeError.status !== undefined && facadeError.status > 0) {
          throw error
        }

        // Handle abort/timeout
        if (error.name === "AbortError") {
          const timeoutError: FacadeError = {
            error: "Request timeout",
            fault_message: `Request to ${endpoint} timed out after ${this.timeout}ms`,
            status: 408,
            suggestion: "Check your connection and try again",
            request_id: requestId,
          }
          throw new Error(JSON.stringify(timeoutError))
        }

        // Handle network errors
        const networkError: FacadeError = {
          error: "Network error",
          fault_message: error.message,
          status: 0,
          suggestion: "Check your connection and API server status",
          request_id: requestId,
        }
        throw new Error(JSON.stringify(networkError))
      }

      throw error
    }
  }

  // HTTP method helpers
  async get<T>(endpoint: string, options?: RequestInit): Promise<T> {
    return this.request<T>(endpoint, { ...options, method: "GET" })
  }

  async post<T>(
    endpoint: string,
    data?: unknown,
    options?: RequestInit
  ): Promise<T> {
    return this.request<T>(endpoint, {
      ...options,
      method: "POST",
      body: data ? JSON.stringify(data) : undefined,
    })
  }

  async put<T>(
    endpoint: string,
    data?: unknown,
    options?: RequestInit
  ): Promise<T> {
    return this.request<T>(endpoint, {
      ...options,
      method: "PUT",
      body: data ? JSON.stringify(data) : undefined,
    })
  }

  async patch<T>(
    endpoint: string,
    data?: unknown,
    options?: RequestInit
  ): Promise<T> {
    return this.request<T>(endpoint, {
      ...options,
      method: "PATCH",
      body: data ? JSON.stringify(data) : undefined,
    })
  }

  async delete<T>(endpoint: string, options?: RequestInit): Promise<T> {
    return this.request<T>(endpoint, { ...options, method: "DELETE" })
  }
}

// Default client instance
export const apiClient = new ApiClient()

// Helper to parse facade errors from caught exceptions
export function parseFacadeError(error: unknown): FacadeError | null {
  if (error instanceof Error) {
    try {
      const parsed = JSON.parse(error.message) as FacadeError
      // Check if it's a valid FacadeError with status code
      if (parsed.status !== undefined && typeof parsed.status === "number") {
        return parsed
      }
    } catch {
      // Not a valid facade error
    }
  }
  return null
}