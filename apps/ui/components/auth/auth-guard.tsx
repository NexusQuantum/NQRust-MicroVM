"use client"

import { useEffect, useState } from "react"
import { useRouter, usePathname } from "next/navigation"
import { useAuthStore } from "@/lib/auth/store"

const PUBLIC_ROUTES = ["/"]

export function AuthGuard({ children }: { children: React.ReactNode }) {
  const router = useRouter()
  const pathname = usePathname()
  const { isAuthenticated, token } = useAuthStore()
  const [isChecking, setIsChecking] = useState(true)

  const isPublicRoute = PUBLIC_ROUTES.includes(pathname || "")
  const isProtectedRoute = !isPublicRoute

  useEffect(() => {
    // Small delay to ensure localStorage is loaded
    const timer = setTimeout(() => {
      setIsChecking(false)
    }, 100)

    return () => clearTimeout(timer)
  }, [])

  useEffect(() => {
    if (isChecking) return

    // Redirect to login if trying to access protected route without auth
    if (isProtectedRoute && (!isAuthenticated || !token)) {
      router.replace("/")
    }

    // Redirect to dashboard if trying to access login while authenticated
    if (isPublicRoute && isAuthenticated && token) {
      router.replace("/dashboard")
    }
  }, [isAuthenticated, token, pathname, router, isProtectedRoute, isPublicRoute, isChecking])

  // Show loading spinner while checking auth
  if (isChecking) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-orange-600"></div>
      </div>
    )
  }

  // Show loading spinner for protected routes without auth
  if (isProtectedRoute && (!isAuthenticated || !token)) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-orange-600"></div>
      </div>
    )
  }

  return <>{children}</>
}


