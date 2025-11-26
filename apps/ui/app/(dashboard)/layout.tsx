"use client"

import type React from "react"
import { Sidebar } from "@/components/layout/sidebar"
import { Topbar } from "@/components/layout/topbar"
import { FunctionStateMonitorProvider } from "@/components/providers/function-state-monitor-provider"
import { useState } from "react"

export default function DashboardLayout({
  children,
}: {
  children: React.ReactNode
}) {
  const [isSidebarCollapsed, setIsSidebarCollapsed] = useState(false)

  return (
    <div className="flex h-screen overflow-hidden">
      <FunctionStateMonitorProvider />
      <Sidebar isCollapsed={isSidebarCollapsed} />
      <div className="flex flex-1 flex-col overflow-hidden">
        <Topbar isCollapsed={isSidebarCollapsed} onToggle={() => setIsSidebarCollapsed(!isSidebarCollapsed)} />
        <main className="flex-1 overflow-y-auto bg-background p-6">{children}</main>
      </div>
    </div>
  )
}
