"use client"

import type { VMState } from "@/types/firecracker"
import { Badge } from "@/components/ui/badge"
import { cn } from "@/lib/utils"
import { Circle, Play, Square, Pause } from "lucide-react"

interface StatusIndicatorProps {
  state: VMState
  showIcon?: boolean
  size?: "sm" | "md" | "lg"
}

export function StatusIndicator({ state, showIcon = true, size = "md" }: StatusIndicatorProps) {
  const getStateConfig = (state: VMState) => {
    switch (state) {
      case "running":
        return {
          label: "Running",
          className: "text-success bg-success/10 border-success/20",
          icon: Play,
          dotColor: "bg-success",
        }
      case "stopped":
        return {
          label: "Stopped",
          className: "text-muted-foreground bg-muted border-border",
          icon: Square,
          dotColor: "bg-muted-foreground",
        }
      case "paused":
        return {
          label: "Paused",
          className: "text-warning bg-warning/10 border-warning/20",
          icon: Pause,
          dotColor: "bg-warning",
        }
      default:
        return {
          label: "Unknown",
          className: "text-muted-foreground bg-muted border-border",
          icon: Circle,
          dotColor: "bg-muted-foreground",
        }
    }
  }

  const config = getStateConfig(state)
  const Icon = config.icon

  const sizeClasses = {
    sm: "text-xs px-2 py-0.5",
    md: "text-xs px-2.5 py-1",
    lg: "text-sm px-3 py-1.5",
  }

  const iconSizes = {
    sm: "h-2.5 w-2.5",
    md: "h-3 w-3",
    lg: "h-3.5 w-3.5",
  }

  return (
    <Badge variant="outline" className={cn("font-medium border", config.className, sizeClasses[size])}>
      <div className="flex items-center gap-1.5">
        {showIcon && (
          <div className="relative">
            <Icon className={cn("shrink-0", iconSizes[size])} />
            {state === "running" && (
              <div
                className={cn(
                  "absolute -top-0.5 -right-0.5 rounded-full animate-pulse",
                  config.dotColor,
                  size === "sm" ? "h-1.5 w-1.5" : "h-2 w-2",
                )}
              />
            )}
          </div>
        )}
        <span>{config.label}</span>
      </div>
    </Badge>
  )
}
