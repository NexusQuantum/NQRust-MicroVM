"use client"

import { PanelLeft, PanelRight, Sun, Moon, Monitor } from "lucide-react"
import { useTheme } from "next-themes"
import { useEffect, useState } from "react"
import { Button } from "@/components/ui/button"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { NotificationDropdown } from "./notification-dropdown"
import { SearchCommand } from "./search-command"

export function Topbar({ isCollapsed, onToggle }: { isCollapsed: boolean; onToggle: () => void }) {
  const { theme, setTheme } = useTheme()
  const [mounted, setMounted] = useState(false)

  // Prevent hydration mismatch by only rendering theme toggle after mount
  useEffect(() => {
    setMounted(true)
  }, [])

  const getThemeIcon = () => {
    if (!mounted) return <Sun className="h-5 w-5" />

    switch (theme) {
      case "light":
        return <Sun className="h-5 w-5" />
      case "dark":
        return <Moon className="h-5 w-5" />
      case "system":
        return <Monitor className="h-5 w-5" />
      default:
        return <Sun className="h-5 w-5" />
    }
  }

  return (
    <div className="flex h-16 items-center justify-between border-b border-border bg-card px-6">
      <div className="flex flex-1 items-center gap-4">
        <Button variant="ghost" size="icon" onClick={onToggle} className="h-9 w-9">
          {isCollapsed ? <PanelRight className="h-5 w-5" /> : <PanelLeft className="h-5 w-5" />}
        </Button>

        <SearchCommand />
      </div>

      <div className="flex items-center gap-2">
        {/* Theme Toggle */}
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="ghost" size="icon" className="h-9 w-9">
              {getThemeIcon()}
              <span className="sr-only">Toggle theme</span>
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem onClick={() => setTheme("light")}>
              <Sun className="mr-2 h-4 w-4" />
              <span>Light</span>
            </DropdownMenuItem>
            <DropdownMenuItem onClick={() => setTheme("dark")}>
              <Moon className="mr-2 h-4 w-4" />
              <span>Dark</span>
            </DropdownMenuItem>
            <DropdownMenuItem onClick={() => setTheme("system")}>
              <Monitor className="mr-2 h-4 w-4" />
              <span>System</span>
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>

        {/* Notifications */}
        <NotificationDropdown />
      </div>
    </div>
  )
}
