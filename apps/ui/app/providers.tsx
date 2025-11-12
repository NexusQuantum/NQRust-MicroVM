"use client"

import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { ReactQueryDevtools } from "@tanstack/react-query-devtools"
import { useState, useEffect } from "react"
import { AuthGuard } from "@/components/auth/auth-guard"
import { AuthProvider, useAuthStore, getAuthToken } from "@/lib/auth/store"
import { authApi, setAuthTokenGetter } from "@/lib/api/auth"
import { ThemeProvider } from "@/components/theme-provider"
import { ThemeSyncProvider } from "@/components/theme-sync-provider"

function AuthInitializer({ children }: { children: React.ReactNode }) {
  const { token, user, setUser, clearAuth } = useAuthStore()

  // Set token getter for API client
  useEffect(() => {
    setAuthTokenGetter(getAuthToken)
  }, [])

  // Validate token and fetch user on mount if token exists
  // Only validate if we have token but no user data
  useEffect(() => {
    if (token && !user) {
      authApi.getCurrentUser()
        .then((fetchedUser) => {
          setUser(fetchedUser)
        })
        .catch((error) => {
          // Only clear auth if it's a 401 Unauthorized (invalid token)
          // For other errors (like endpoint not found), keep the user logged in
          if (error && typeof error === 'object' && 'status' in error) {
            const status = (error as any).status
            if (status === 401) {
              console.log("Token is invalid, clearing auth")
              clearAuth()
            } else {
              console.log("Auth validation endpoint not available, keeping user logged in")
            }
          } else {
            // Network error or endpoint doesn't exist - keep user logged in
            console.log("Cannot validate token (endpoint may not exist), keeping user logged in")
          }
        })
    }
  }, [token, user, setUser, clearAuth])

  return <>{children}</>
}

export function Providers({ children }: { children: React.ReactNode }) {
  const [queryClient] = useState(
    () =>
      new QueryClient({
        defaultOptions: {
          queries: {
            staleTime: 60 * 1000, // 1 minute
            retry: (failureCount, error) => {
              // Don't retry on 4xx errors
              if (error && typeof error === 'object' && 'status' in error) {
                const status = error.status as number
                if (status >= 400 && status < 500) {
                  return false
                }
              }
              return failureCount < 3
            },
          },
        },
      })
  )

  return (
    <QueryClientProvider client={queryClient}>
      <ThemeProvider
        attribute="class"
        defaultTheme="system"
        enableSystem
        disableTransitionOnChange
        storageKey="nqr-microvm-theme"
      >
        <AuthProvider>
          <AuthInitializer>
            <ThemeSyncProvider>
              <AuthGuard>
                {children}
              </AuthGuard>
            </ThemeSyncProvider>
          </AuthInitializer>
          <ReactQueryDevtools initialIsOpen={false} />
        </AuthProvider>
      </ThemeProvider>
    </QueryClientProvider>
  )
}
