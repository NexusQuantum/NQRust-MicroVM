"use client"

import { useEffect, useState } from "react"
import { useTheme } from "next-themes"
import { usePreferences, useUpdatePreferences } from "@/lib/queries"
import { useAuthStore } from "@/lib/auth/store"

/**
 * ThemeSyncProvider synchronizes the theme between:
 * 1. next-themes (local state)
 * 2. Backend user preferences
 *
 * When user is logged in:
 * - On mount: Load theme from backend preferences
 * - On theme change: Save to backend preferences
 */
export function ThemeSyncProvider({ children }: { children: React.ReactNode }) {
  const { theme, setTheme, resolvedTheme } = useTheme()
  const { isAuthenticated } = useAuthStore()
  const { data: preferences, isLoading } = usePreferences()
  const updatePreferencesMutation = useUpdatePreferences()
  const [hasLoadedFromBackend, setHasLoadedFromBackend] = useState(false)

  // Load theme from backend preferences when user logs in (only once)
  useEffect(() => {
    if (!isAuthenticated || isLoading || !preferences || hasLoadedFromBackend) return

    // If backend has a theme preference, sync it
    if (preferences.theme) {
      setTheme(preferences.theme)
      setHasLoadedFromBackend(true)
    } else {
      // If no backend preference, mark as loaded to prevent further attempts
      setHasLoadedFromBackend(true)
    }
  }, [isAuthenticated, preferences, isLoading, setTheme, hasLoadedFromBackend])

  // Save theme to backend when it changes (debounced)
  useEffect(() => {
    if (!isAuthenticated || !theme || !hasLoadedFromBackend) return

    // Don't update if already in sync
    if (preferences?.theme === theme) return

    // Debounce the update to avoid too many API calls
    const timeoutId = setTimeout(() => {
      updatePreferencesMutation.mutate({
        theme: theme,
      })
    }, 500)

    return () => clearTimeout(timeoutId)
  }, [theme, isAuthenticated, preferences?.theme, updatePreferencesMutation, hasLoadedFromBackend])

  // Debug: Log theme changes to help diagnose issues
  useEffect(() => {
    if (process.env.NODE_ENV === 'development') {
      console.log('Theme state:', { theme, resolvedTheme, backendTheme: preferences?.theme })
    }
  }, [theme, resolvedTheme, preferences?.theme])

  return <>{children}</>
}
