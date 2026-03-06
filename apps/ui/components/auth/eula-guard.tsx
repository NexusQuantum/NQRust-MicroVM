"use client"

import { useEffect } from "react"
import { useRouter, usePathname } from "next/navigation"
import { useAuthStore } from "@/lib/auth/store"
import { useEulaStatus } from "@/lib/queries"
import { Loader2 } from "lucide-react"

// "/" is intentionally excluded so the login page is also gated by EULA
const SKIP_PATHS = ["/eula", "/setup/license"]

export function EulaGuard({ children }: { children: React.ReactNode }) {
    const router = useRouter()
    const pathname = usePathname()
    const { isAuthenticated, token } = useAuthStore()

    const isOnSkipPath =
        SKIP_PATHS.some((p) => pathname === p) || pathname.startsWith("/setup") || pathname.startsWith("/docs")

    // EULA is now app-level and the endpoint is public — always fetch when not on a skip path
    const { data: eulaStatus, isLoading } = useEulaStatus(!isOnSkipPath)

    useEffect(() => {
        if (isOnSkipPath) return
        if (!eulaStatus) return

        if (eulaStatus.needs_acceptance && pathname !== "/eula") {
            router.replace("/eula")
        } else if (!eulaStatus.needs_acceptance && pathname === "/eula") {
            // Redirect to login if not authenticated, dashboard if authenticated
            router.replace(isAuthenticated && token ? "/dashboard" : "/")
        }
    }, [isAuthenticated, token, pathname, isOnSkipPath, eulaStatus, router])

    if (isOnSkipPath) {
        return <>{children}</>
    }

    if (isLoading) {
        return (
            <div className="flex h-screen w-screen items-center justify-center bg-background">
                <Loader2 className="h-8 w-8 animate-spin text-primary" />
            </div>
        )
    }

    if (eulaStatus?.needs_acceptance) {
        return null
    }

    return <>{children}</>
}
