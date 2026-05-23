"use client"

// DemoBootstrap mounts at the top of the providers tree when NEXT_PUBLIC_DEMO_MODE
// is enabled. Responsibilities:
//   1. Install the request/WebSocket interceptors before any data hooks fire.
//   2. Seed the auth store with a fake admin so AuthGuard passes.
//   3. Show a small floating banner so visitors know the data is synthetic.
//   4. Auto-redirect the landing page to /dashboard since there is no login flow.

import { useEffect } from "react"
import { useRouter, usePathname } from "next/navigation"
import { DEMO_MODE } from "@/lib/demo/flag"
import { installDemoMode } from "@/lib/demo/install"
import { useAuthStore } from "@/lib/auth/store"
import { Sparkles, RotateCcw } from "lucide-react"
import { resetState } from "@/lib/demo/state"

export function DemoBootstrap({ children }: { children: React.ReactNode }) {
  if (!DEMO_MODE) return <>{children}</>
  return <DemoBootstrapImpl>{children}</DemoBootstrapImpl>
}

function DemoBootstrapImpl({ children }: { children: React.ReactNode }) {
  const router = useRouter()
  const pathname = usePathname()
  const { isAuthenticated, setAuth } = useAuthStore()

  // Install interceptors as early as possible. Layout effect would be nicer but
  // useEffect runs before any TanStack queries fire children-deep.
  useEffect(() => {
    installDemoMode()
  }, [])

  // Seed the auth store on first paint so the AuthGuard lets us through.
  useEffect(() => {
    if (!isAuthenticated) {
      setAuth("demo-token", {
        id: "u-demo",
        username: "demo",
        role: "admin",
        email: "demo@nqr-microvm.com",
        created_at: new Date().toISOString(),
      })
    }
  }, [isAuthenticated, setAuth])

  // Skip the login screen — go straight to the dashboard.
  useEffect(() => {
    if (pathname === "/" || pathname === "") {
      router.replace("/dashboard")
    }
  }, [pathname, router])

  return (
    <>
      {children}
      <DemoBanner />
    </>
  )
}

function DemoBanner() {
  const onReset = () => {
    resetState()
    if (typeof window !== "undefined") {
      window.location.reload()
    }
  }
  return (
    <div className="fixed bottom-4 right-4 z-[9999] flex items-center gap-2 rounded-full border border-orange-500/30 bg-orange-500/95 px-4 py-2 text-xs font-medium text-white shadow-lg backdrop-blur-sm">
      <Sparkles className="h-3.5 w-3.5" />
      <span>Demo mode — data is simulated</span>
      <button
        type="button"
        onClick={onReset}
        className="ml-2 inline-flex items-center gap-1 rounded-full bg-white/15 px-2 py-0.5 text-[11px] font-medium hover:bg-white/25"
        title="Reset demo state"
      >
        <RotateCcw className="h-3 w-3" />
        Reset
      </button>
    </div>
  )
}
