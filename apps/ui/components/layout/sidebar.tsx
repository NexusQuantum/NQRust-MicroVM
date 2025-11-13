"use client"

import React from "react"
import Link from "next/link"
import { usePathname, useRouter } from "next/navigation"
import { cn } from "@/lib/utils"
import { useAuthStore } from "@/lib/auth/store"
import {
  LayoutDashboard,
  Server,
  Zap,
  Container,
  Database,
  FileCode,
  Settings,
  ServerCog,
  Network,
  HardDrive,
  User,
  BookOpen,
  LogOut,
  Users
} from "lucide-react"
import Image from "next/image"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { Button } from "@/components/ui/button"
import { Avatar } from "@/components/user/avatar"
import { useProfile } from "@/lib/queries"

type IconType = React.ComponentType<{ className?: string }>
export type NavItem = { name: string; href: string; icon: IconType }

const MAIN: NavItem[] = [
  { name: "Dashboard", href: "/dashboard", icon: LayoutDashboard },
  { name: "Virtual Machines", href: "/vms", icon: Server },
  { name: "Functions", href: "/functions", icon: Zap },
  { name: "Containers", href: "/containers", icon: Container },
  { name: "Templates", href: "/templates", icon: FileCode },
  { name: "Registry", href: "/registry", icon: Database },
]

const HOST: NavItem[] = [
  { name: "Hosts", href: "/hosts", icon: ServerCog },
  { name: "Networks", href: "/networks", icon: Network },
  { name: "Volumes", href: "/volumes", icon: HardDrive },
]

const BOTTOM: NavItem[] = [
  { name: "Users", href: "/users", icon: Users },
  { name: "Settings", href: "/settings", icon: Settings }
]

/** Akurat: aktif jika path sama persis, atau startsWith dengan batas slash */
function isPathActive(pathname: string, href: string) {
  if (pathname === href) return true
  return pathname.startsWith(href.endsWith("/") ? href : `${href}/`)
}

const SidebarItem = React.memo(function SidebarItem({
  item,
  pathname,
  collapsed,
}: {
  item: NavItem
  pathname: string
  collapsed: boolean
}) {
  const active = isPathActive(pathname, item.href)
  return (
    <Link
      href={item.href}
      prefetch={false}
      className={cn(
        "flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-colors",
        active
          ? "bg-orange-500 text-white dark:bg-orange-600"
          : "text-muted-foreground hover:bg-orange-500/10 hover:text-orange-600 dark:hover:bg-orange-500/20 dark:hover:text-orange-500",
        collapsed && "justify-center text-muted-foreground"
      )}
      title={collapsed ? item.name : undefined}
      aria-current={active ? "page" : undefined}
      aria-label={item.name}
    >
      <item.icon className="h-5 w-5 flex-shrink-0 dark:text-foreground" />
      {!collapsed && <span className="truncate dark:text-foreground">{item.name}</span>}
    </Link>
  )
})

const SidebarSection = React.memo(function SidebarSection({
  title,
  items,
  collapsed,
  withDivider,
  pathname,
}: {
  title: string
  items: NavItem[]
  collapsed: boolean
  withDivider?: boolean
  pathname: string
}) {
  return (
    <div className={cn(withDivider && "border-t border-border pt-2")}>
      {!collapsed && (
        <div className="px-4 pt-3 pb-1 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
          {title}
        </div>
      )}
      <nav className="space-y-1 p-4 pt-2">
        {items.map((it) => (
          <SidebarItem key={it.href} item={it} pathname={pathname} collapsed={collapsed} />
        ))}
      </nav>
    </div>
  )
})

/** User panel yang pindah dari Topbar ke Sidebar bawah */
function SidebarUser({ collapsed }: { collapsed: boolean }) {
  const { user, clearAuth } = useAuthStore()
  const router = useRouter()
  const { data: profile } = useProfile()

  if (!user) {
    return null
  }

  const handleLogout = () => {
    // Clear auth state and localStorage
    clearAuth()

    // Redirect to login page
    router.replace("/")

    // Optional: Show toast notification
    // Uncomment if you want to show logout message
    // toast({ title: "Logged out", description: "You have been successfully logged out." })
  }

  if (collapsed) {
    // Mode collapsed: hanya tombol avatar bundar
    return (
      <div className="border-t border-border p-3">
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
              className="h-10 w-10 rounded-full p-0"
              title={user.username}
            >
              <Avatar
                key={profile?.avatar_path || user.id}
                avatarPath={profile?.avatar_path}
                username={user.username}
                size="md"
                className="h-10 w-10"
              />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent side="right" align="start" className="w-56">
            <DropdownMenuLabel>
              <div className="flex flex-col space-y-1">
                <p className="text-sm font-medium">{user.username}</p>
                <p className="text-xs text-muted-foreground capitalize">{user.role}</p>
              </div>
            </DropdownMenuLabel>
            <DropdownMenuSeparator />
            <DropdownMenuItem asChild>
              <Link href="/settings">
                <User className="mr-2 h-4 w-4" />
                Profile & Settings
              </Link>
            </DropdownMenuItem>
            <DropdownMenuItem asChild>
              <a href="https://docs.example.com" target="_blank" rel="noreferrer">
                <BookOpen className="mr-2 h-4 w-4" />
                Documentation
              </a>
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem onClick={handleLogout}>
              <LogOut className="mr-2 h-4 w-4" />
              Logout
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    )
  }

  // Mode expanded: kartu kecil seperti pada screenshot
  return (
    <div className="border-t border-border p-4">
      <div className="flex items-center gap-3 rounded-lg bg-muted/50 p-3">
        <div className="relative">
          <Avatar
            key={profile?.avatar_path || user.id}
            avatarPath={profile?.avatar_path}
            username={user.username}
            size="md"
            className="h-9 w-9"
          />
          {/* status dot */}
          <span className="absolute -right-0.5 -bottom-0.5 inline-block h-2.5 w-2.5 rounded-full bg-green-500 ring-2 ring-white dark:ring-background" />
        </div>
        <div className="min-w-0 flex-1">
          <p className="truncate text-sm font-medium text-foreground">{user.username}</p>
          <p className="truncate text-xs text-muted-foreground capitalize">{user.role}</p>
        </div>

        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="outline" size="sm" className="shrink-0">
              Manage
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="w-56">
            <DropdownMenuLabel>Account</DropdownMenuLabel>
            <DropdownMenuSeparator />
            <DropdownMenuItem asChild>
              <Link href="/settings">
                <User className="mr-2 h-4 w-4" />
                Profile & Settings
              </Link>
            </DropdownMenuItem>
            <DropdownMenuItem asChild>
              <a href="https://docs.example.com" target="_blank" rel="noreferrer">
                <BookOpen className="mr-2 h-4 w-4" />
                Documentation
              </a>
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem onClick={handleLogout}>
              <LogOut className="mr-2 h-4 w-4" />
              Logout
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </div>
  )
}

export function Sidebar({ isCollapsed }: { isCollapsed: boolean }) {
  const pathname = usePathname()
  const { user } = useAuthStore()

  // Filter bottom items based on user role
  const bottomItems = React.useMemo(() => {
    if (!user) return BOTTOM
    // Only show Users tab for admin
    return BOTTOM.filter(item => {
      if (item.href === "/users") {
        return user.role === "admin"
      }
      return true
    })
  }, [user])

  const sections = React.useMemo(
    () => [
      { title: "Main", items: MAIN, withDivider: false },
      { title: "Host", items: HOST, withDivider: true },
    ],
    []
  )

  return (
    <div
      className={cn(
        "flex h-full flex-col border-r border-border bg-card transition-all duration-300",
        isCollapsed ? "w-18" : "w-64"
      )}
    >
      {/* Brand */}
      <div className="flex h-16 items-center justify-start border-b border-border px-4">
        {!isCollapsed ? (
          <Link href="/dashboard" className="flex items-center" prefetch={false}>
            <Image
              src="/nqr-logo-full.png"
              alt="NQR-MicroVM"
              width={220}
              height={60}
              className="h-12 w-auto"
              priority
            />
          </Link>
        ) : (
          <Link href="/dashboard" className="flex items-center" prefetch={false}>
            <Image src="/nq-logo.png" alt="NQR-MicroVM" width={40} height={40} className="h-10 w-auto" priority />
          </Link>
        )}
      </div>

      {/* Sections */}
      <div className="flex-1 overflow-y-auto">
        {sections.map((s) => (
          <div key={s.title}>
            {!isCollapsed && (
              <div className="px-4 pt-3 pb-1 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
                {s.title}
              </div>
            )}
            <nav className={cn("space-y-1 p-4 pt-2", s.withDivider && "border-t border-border pt-3")}>
              {s.items.map((it) => (
                <SidebarItem key={it.href} item={it} pathname={pathname} collapsed={isCollapsed} />
              ))}
            </nav>
          </div>
        ))}

        {/* Bottom simple links (Settings, etc) */}
        <div className="border-t border-border">
          <nav className="space-y-1 p-4">
            {bottomItems.map((it) => (
              <SidebarItem key={it.href} item={it} pathname={pathname} collapsed={isCollapsed} />
            ))}
          </nav>
        </div>
      </div>

      {/* User moved here */}
      <SidebarUser collapsed={isCollapsed} />
    </div>
  )
}
