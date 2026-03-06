"use client"

import { useEffect } from "react"
import { useRouter, usePathname } from "next/navigation"
import { useLicenseStatus } from "@/lib/queries"
import { AlertTriangle } from "lucide-react"

const SKIP_PATHS = ["/setup/license", "/eula", "/"]

export function LicenseGuard({ children }: { children: React.ReactNode }) {
    const router = useRouter()
    const pathname = usePathname()

    // License check is system-level — no auth required
    const shouldSkip = SKIP_PATHS.includes(pathname) || pathname.startsWith("/docs")

    const { data: licenseStatus } = useLicenseStatus(!shouldSkip)

    useEffect(() => {
        if (shouldSkip) return

        if (licenseStatus && !licenseStatus.is_licensed && !licenseStatus.is_grace_period) {
            router.replace("/setup/license")
        }
    }, [shouldSkip, licenseStatus, router])

    // On skip path, just render children
    if (shouldSkip) {
        return <>{children}</>
    }

    // Unlicensed and not grace period — block rendering while redirecting
    if (licenseStatus && !licenseStatus.is_licensed && !licenseStatus.is_grace_period) {
        return null
    }

    return (
        <>
            {/* Grace period warning banner */}
            {licenseStatus?.is_grace_period && pathname !== "/setup/license" && (
                <div className="fixed top-0 left-0 right-0 z-50 flex items-center justify-center gap-2 bg-yellow-500/90 px-4 py-1.5 text-xs font-medium text-yellow-950 shadow-sm">
                    <AlertTriangle className="h-3.5 w-3.5" />
                    License verification pending. Grace period: {licenseStatus.grace_days_remaining} days remaining.
                    <button
                        onClick={() => router.push("/setup/license")}
                        className="underline hover:no-underline ml-1"
                    >
                        Verify now
                    </button>
                </div>
            )}
            {children}
        </>
    )
}
