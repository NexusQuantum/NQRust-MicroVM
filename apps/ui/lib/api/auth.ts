import { apiClient } from "./http"
import { useAuthStore, type User } from "@/lib/auth/store"

export interface LoginRequest {
  username: string
  password: string
}

export interface LoginResponse {
  token: string
  user: User
}

export const authApi = {
  async login(credentials: LoginRequest): Promise<LoginResponse> {
    return apiClient.post<LoginResponse>("/auth/login", credentials)
  },

  async getCurrentUser(): Promise<User> {
    return apiClient.get<User>("/auth/me")
  },

  async logout() {
    // Logout is handled client-side by clearing the token
    // No API call needed unless we implement token revocation
  },
}

// Initialize API client with token getter
// This will be set when the auth store is available
let tokenGetter: (() => string | null) | null = null

export function setAuthTokenGetter(getter: () => string | null) {
  tokenGetter = getter
  apiClient.setTokenGetter(getter)
}

// Try to set token getter if auth store is available
if (typeof window !== "undefined") {
  // Defer to avoid circular dependency
  setTimeout(() => {
    try {
      const { useAuthStore } = require("@/lib/auth/store")
      // We'll set this in the AuthProvider component instead
    } catch {
      // Ignore
    }
  }, 0)
}

