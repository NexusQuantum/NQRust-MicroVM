"use client"

// DemoBootstrap mounts at the top of the providers tree when NEXT_PUBLIC_DEMO_MODE
// is enabled. Responsibilities:
//   1. Install the request/WebSocket interceptors before any data hooks fire.
//   2. Show a small floating banner so visitors know the data is synthetic.
//
// Auth: real login flow stays — the user must type admin/admin to enter. The
// mock /auth/login endpoint validates those credentials and rejects anything
// else, so visitors get the same experience as the real product.

import { useEffect } from "react"
import { DEMO_MODE } from "@/lib/demo/flag"
import { installDemoMode } from "@/lib/demo/install"
import { Sparkles, RotateCcw } from "lucide-react"
import { resetState } from "@/lib/demo/state"

export function DemoBootstrap({ children }: { children: React.ReactNode }) {
  if (!DEMO_MODE) return <>{children}</>
  return <DemoBootstrapImpl>{children}</DemoBootstrapImpl>
}

function DemoBootstrapImpl({ children }: { children: React.ReactNode }) {
  useEffect(() => {
    installDemoMode()
  }, [])

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
      // Clear the auth state too so visitors are kicked back to the login
      // page after a reset — matches the "fresh demo" expectation.
      try {
        localStorage.removeItem("auth-storage")
      } catch {
        /* ignore */
      }
      window.location.href = "/"
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
        title="Reset demo state and sign out"
      >
        <RotateCcw className="h-3 w-3" />
        Reset
      </button>
    </div>
  )
}
