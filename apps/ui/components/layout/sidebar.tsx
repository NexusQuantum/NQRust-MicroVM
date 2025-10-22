"use client"

import Link from "next/link"
import { usePathname } from "next/navigation"
import { cn } from "@/lib/utils"
import { LayoutDashboard, Server, Zap, Container, Database, FileCode, Settings } from "lucide-react"
import Image from "next/image"

const mainNavigation = [
  { name: "Dashboard", href: "/dashboard", icon: LayoutDashboard },
  { name: "Virtual Machines", href: "/vms", icon: Server },
  { name: "Functions", href: "/functions", icon: Zap },
  { name: "Containers", href: "/containers", icon: Container },
  { name: "Registry", href: "/registry", icon: Database },
  { name: "Templates", href: "/templates", icon: FileCode },
]

const bottomNavigation = [{ name: "Settings", href: "/settings", icon: Settings }]

export function Sidebar({ isCollapsed }: { isCollapsed: boolean }) {
  const pathname = usePathname()

  return (
    <div
      className={cn(
        "flex h-full flex-col border-r border-border bg-card transition-all duration-300",
        isCollapsed ? "w-16" : "w-64",
      )}
    >
      <div className="flex h-16 items-center justify-center border-b border-border px-4">
        {!isCollapsed && (
          <Link href="/dashboard" className="flex items-center">
            <Image
              src="/nqr-logo-full.png"
              alt="NQR-MicroVM"
              width={220}
              height={60}
              className="h-12 w-auto"
              priority
            />
          </Link>
        )}
        {isCollapsed && (
          <Link href="/dashboard" className="flex items-center">
            <Image src="/nqr-icon.png" alt="NQR" width={40} height={40} className="h-10 w-auto" priority />
          </Link>
        )}
      </div>

      <nav className="flex-1 space-y-1 p-4">
        {mainNavigation.map((item) => {
          const isActive = pathname.startsWith(item.href)
          return (
            <Link
              key={item.name}
              href={item.href}
              className={cn(
                "flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-colors",
                isActive
                  ? "bg-primary text-primary-foreground"
                  : "text-muted-foreground hover:bg-accent hover:text-accent-foreground",
                isCollapsed && "justify-center",
              )}
              title={isCollapsed ? item.name : undefined}
            >
              <item.icon className="h-5 w-5 flex-shrink-0" />
              {!isCollapsed && <span>{item.name}</span>}
            </Link>
          )
        })}
      </nav>

      <div className="border-t border-border">
        <nav className="space-y-1 p-4">
          {bottomNavigation.map((item) => {
            const isActive = pathname.startsWith(item.href)
            return (
              <Link
                key={item.name}
                href={item.href}
                className={cn(
                  "flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-colors",
                  isActive
                    ? "bg-primary text-primary-foreground"
                    : "text-muted-foreground hover:bg-accent hover:text-accent-foreground",
                  isCollapsed && "justify-center",
                )}
                title={isCollapsed ? item.name : undefined}
              >
                <item.icon className="h-5 w-5 flex-shrink-0" />
                {!isCollapsed && <span>{item.name}</span>}
              </Link>
            )
          })}
        </nav>
      </div>

      {!isCollapsed && (
        <div className="border-t border-border p-4">
          <div className="rounded-lg bg-muted/50 p-3">
            <p className="text-xs font-medium text-foreground">Platform Status</p>
            <div className="mt-2 flex items-center gap-2">
              <div className="h-2 w-2 rounded-full bg-green-500 animate-pulse" />
              <p className="text-xs text-muted-foreground">All systems operational</p>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
