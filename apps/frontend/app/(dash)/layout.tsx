"use client"

import type React from "react"

import {
  SidebarProvider,
  Sidebar,
  SidebarContent,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuItem,
  SidebarMenuButton,
  SidebarInset,
  SidebarTrigger,
} from "@/components/ui/sidebar"
import { Button } from "@/components/ui/button"
import { LayoutDashboard, Server, Database, Plus, Settings, HelpCircle } from "lucide-react"
import Link from "next/link"
import { usePathname } from "next/navigation"
import { ThemeToggle } from "@/components/theme-toggle"
import Image from "next/image"
import type { Route } from "next"

const navigation: { title: string; href: Route; icon: any }[] = [
  {
    title: "Dashboard",
    href: "/dashboard",
    icon: LayoutDashboard,
  },
  {
    title: "Virtual Machines",
    href: "/vms",
    icon: Server,
  },
  {
    title: "Registry",
    href: "/registry",
    icon: Database,
  },
  {
    title: "Function",
    href: "/function",
    icon: Database,
  },

]

export default function DashboardLayout({
  children,
}: {
  children: React.ReactNode
}) {
  const pathname = usePathname()

  return (
    <SidebarProvider>
      <div className="flex min-h-screen w-full">
        <Sidebar>
          <SidebarHeader className="border-b border-sidebar-border">
            <div className="flex items-center gap-2 px-2 py-2">
              <Image src="/logo.png" alt="NexusRust logo" width={32} height={32} className="w-8 h-8 rounded-lg" />
              <div className="flex flex-col">
                <span className="font-semibold text-sm">NexusRust</span>
                <span className="text-xs text-muted-foreground">MicroVM Manager</span>
              </div>
            </div>
          </SidebarHeader>

          <SidebarContent>
            <div className="p-2">
              <Button asChild className="w-full justify-center">
                <Link href="/vms/create">
                  <Plus className="h-4 w-4" />
                  Create VM
                </Link>
              </Button>
            </div>

            <SidebarMenu>
              {navigation.map((item) => (
                <SidebarMenuItem key={item.href}>
                  <SidebarMenuButton asChild isActive={pathname === item.href || pathname.startsWith(item.href + "/")}>
                    <Link href={item.href}>
                      <item.icon className="h-4 w-4" />
                      <span>{item.title}</span>
                    </Link>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              ))}
            </SidebarMenu>

            <div className="mt-auto p-2">
              <SidebarMenu className="mb-2">
                <SidebarMenuItem>
                  <SidebarMenuButton asChild isActive={pathname === "/settings"} className={pathname === "/settings" ? "text-success" : undefined}>
                    <Link href="/settings">
                      <Settings className="h-4 w-4" />
                      <span>Settings</span>
                    </Link>
                  </SidebarMenuButton>
                </SidebarMenuItem>
                <SidebarMenuItem>
                  <SidebarMenuButton asChild isActive={pathname === "/help"} className={pathname === "/help" ? "text-success" : undefined}>
                    <Link href="/help">
                      <HelpCircle className="h-4 w-4" />
                      <span>Help & Support</span>
                    </Link>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              </SidebarMenu>
        <div className="flex items-center justify-between pt-2 px-2 text-xs text-muted-foreground">
                <div className="flex items-center gap-2">
          <Image src="/logo.png" alt="NexusRust logo" width={24} height={24} className="w-6 h-6 rounded-md" />
                  <span>NexusRust</span>
                </div>
                <span className="opacity-70">v1.0</span>
              </div>
            </div>
          </SidebarContent>
        </Sidebar>

        <SidebarInset className="flex-1">
          <header className="sticky top-0 z-10 flex h-14 items-center gap-4 border-b bg-background px-6">
            <SidebarTrigger />
            <div className="flex-1" />
            <ThemeToggle />
          </header>

          <main className="flex-1 p-6">{children}</main>
        </SidebarInset>
      </div>
    </SidebarProvider>
  )
}
