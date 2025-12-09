"use client"

import { createContext, useContext, useState, useEffect, ReactNode } from "react"

export type Role = "admin" | "user" | "viewer"

export interface User {
  id: string
  username: string
  role: Role
  created_at: string
}

interface AuthState {
  token: string | null
  user: User | null
  isAuthenticated: boolean
  avatarRefreshKey: number
  setAuth: (token: string, user: User) => void
  clearAuth: () => void
  setUser: (user: User) => void
  refreshAvatar: () => void
}

const AuthContext = createContext<AuthState | undefined>(undefined)

const STORAGE_KEY = "auth-storage"

function loadFromStorage(): { token: string | null; user: User | null } {
  if (typeof window === "undefined") {
    return { token: null, user: null }
  }
  try {
    const stored = localStorage.getItem(STORAGE_KEY)
    if (stored) {
      const parsed = JSON.parse(stored)
      return {
        token: parsed.state?.token || null,
        user: parsed.state?.user || null,
      }
    }
  } catch {
    // Ignore errors
  }
  return { token: null, user: null }
}

function saveToStorage(token: string | null, user: User | null) {
  if (typeof window === "undefined") return
  try {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        state: { token, user },
      })
    )
  } catch {
    // Ignore errors
  }
}

export function AuthProvider({ children }: { children: ReactNode }) {
  const { token: initialToken, user: initialUser } = loadFromStorage()
  const [token, setToken] = useState<string | null>(initialToken)
  const [user, setUserState] = useState<User | null>(initialUser)
  const [avatarRefreshKey, setAvatarRefreshKey] = useState(0)

  useEffect(() => {
    saveToStorage(token, user)
  }, [token, user])

  const setAuth = (newToken: string, newUser: User) => {
    setToken(newToken)
    setUserState(newUser)
  }

  const clearAuth = () => {
    setToken(null)
    setUserState(null)
    if (typeof window !== "undefined") {
      localStorage.removeItem(STORAGE_KEY)
      // Also clear any other auth-related storage
      localStorage.removeItem("token")
      localStorage.removeItem("user")
    }
  }

  const setUser = (newUser: User) => {
    setUserState(newUser)
  }

  const refreshAvatar = () => {
    setAvatarRefreshKey((prev) => prev + 1)
  }

  return (
    <AuthContext.Provider
      value={{
        token,
        user,
        isAuthenticated: !!token && !!user,
        avatarRefreshKey,
        setAuth,
        clearAuth,
        setUser,
        refreshAvatar,
      }}
    >
      {children}
    </AuthContext.Provider>
  )
}

// Store reference for token getter
let storeRef: AuthState | null = null

export function useAuthStore(): AuthState {
  const context = useContext(AuthContext)
  if (!context) {
    throw new Error("useAuthStore must be used within AuthProvider")
  }
  // Update ref when context changes
  storeRef = context
  return context
}

// Export getter for API client
export function getAuthToken(): string | null {
  // First try the store ref
  if (storeRef?.token) {
    return storeRef.token
  }

  // Fallback to reading directly from localStorage
  // This handles cases where the store hasn't been initialized yet
  if (typeof window !== "undefined") {
    try {
      const stored = localStorage.getItem(STORAGE_KEY)
      if (stored) {
        const parsed = JSON.parse(stored)
        return parsed.state?.token || null
      }
    } catch {
      // Ignore errors
    }
  }

  return null
}

// ========================================
// Permission Helper Functions
// ========================================

/**
 * Check if user can create resources (VMs, functions, containers, etc.)
 * Admin and User can create, Viewer cannot
 */
export function canCreateResource(user: User | null): boolean {
  if (!user) return false
  return user.role === "admin" || user.role === "user"
}

/**
 * Check if user can view a specific resource
 * Admin can view all, Viewer can view all (read-only), User can view own resources
 */
export function canViewResource(user: User | null, ownerId?: string | null): boolean {
  if (!user) return false

  // Admin and Viewer can view everything
  if (user.role === "admin" || user.role === "viewer") {
    return true
  }

  // User can view resources without an owner (legacy/system resources)
  if (!ownerId) {
    return true
  }

  // User can view their own resources
  return user.role === "user" && ownerId === user.id
}

/**
 * Check if user can modify a specific resource
 * Admin can modify all, User can modify own, Viewer cannot modify
 */
export function canModifyResource(user: User | null, ownerId?: string | null): boolean {
  if (!user) return false

  // Viewer cannot modify anything
  if (user.role === "viewer") {
    return false
  }

  // Admin can modify everything
  if (user.role === "admin") {
    return true
  }

  // User can only modify their own resources
  return user.role === "user" && ownerId === user.id
}

/**
 * Check if user can delete a specific resource
 * Same rules as modify
 */
export function canDeleteResource(user: User | null, ownerId?: string | null): boolean {
  return canModifyResource(user, ownerId)
}

/**
 * Check if user can manage other users
 * Only admin can manage users
 */
export function canManageUsers(user: User | null): boolean {
  if (!user) return false
  return user.role === "admin"
}

/**
 * Check if user can view audit logs
 * Only admin can view audit logs
 */
export function canViewAuditLogs(user: User | null): boolean {
  if (!user) return false
  return user.role === "admin"
}

/**
 * Check if user is admin
 */
export function isAdmin(user: User | null): boolean {
  if (!user) return false
  return user.role === "admin"
}

/**
 * Check if user is viewer (read-only)
 */
export function isViewer(user: User | null): boolean {
  if (!user) return false
  return user.role === "viewer"
}

/**
 * Get a user-friendly role display name
 */
export function getRoleDisplayName(role: Role): string {
  switch (role) {
    case "admin":
      return "Administrator"
    case "user":
      return "User"
    case "viewer":
      return "Viewer (Read-Only)"
    default:
      return role
  }
}

