"use client"

import { useEffect, useRef, useState } from "react"
import { useRouter, useSearchParams } from "next/navigation"
import { useAuthStore } from "@/lib/auth/store"

export default function SsoCallbackPage() {
  const router = useRouter()
  const searchParams = useSearchParams()
  const { setAuth } = useAuthStore()
  const [error, setError] = useState<string | null>(null)
  const processedRef = useRef(false)

  useEffect(() => {
    // Ensure this effect only runs once. Without this guard, calling
    // window.history.replaceState to strip the token from the URL triggers
    // a second re-run where searchParams is empty, which would briefly
    // flash "Missing authentication data" before the redirect lands.
    if (processedRef.current) return
    processedRef.current = true

    const token = searchParams.get("token")
    const userParam = searchParams.get("user")

    if (!token || !userParam) {
      setError("Missing authentication data. Please try signing in again.")
      return
    }

    try {
      const user = JSON.parse(decodeURIComponent(userParam))
      setAuth(token, user)

      // Clear sensitive data from URL
      window.history.replaceState({}, "", "/sso/callback")

      // Wait a tick so the auth store + localStorage write settle
      // before the AuthGuard on /dashboard re-reads them.
      setTimeout(() => {
        router.replace("/dashboard")
      }, 150)
    } catch {
      setError("Failed to process authentication response. Please try again.")
    }
  }, [searchParams, setAuth, router])

  if (error) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <div className="text-center space-y-4">
          <p className="text-destructive font-medium">{error}</p>
          <a href="/" className="text-sm text-muted-foreground hover:underline">
            Return to login
          </a>
        </div>
      </div>
    )
  }

  return (
    <div className="flex items-center justify-center min-h-screen">
      <div className="text-center space-y-2">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-orange-600 mx-auto" />
        <p className="text-sm text-muted-foreground">Completing sign in...</p>
      </div>
    </div>
  )
}
