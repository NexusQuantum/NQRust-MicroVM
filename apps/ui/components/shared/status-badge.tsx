import { Badge } from "@/components/ui/badge"
import { cn } from "@/lib/utils"

interface StatusBadgeProps {
  status: "running" | "stopped" | "paused" | "idle" | "executing" | "error" | "restarting" | "success" | "timeout"
  className?: string
}

export function StatusBadge({ status, className }: StatusBadgeProps) {
  const variants = {
    running: "bg-emerald-100 text-emerald-700 border-emerald-200",
    stopped: "bg-red-100 text-red-600 border-red-200",
    paused: "bg-amber-100 text-amber-700 border-amber-200",
    idle: "bg-slate-100 text-slate-600 border-slate-200",
    executing: "bg-blue-100 text-blue-700 border-blue-200",
    error: "bg-red-100 text-red-700 border-red-200",
    restarting: "bg-amber-100 text-amber-700 border-amber-200",
    success: "bg-emerald-100 text-emerald-700 border-emerald-200",
    timeout: "bg-orange-100 text-orange-700 border-orange-200",
  }

  return (
    <Badge variant="outline" className={cn("font-medium capitalize", variants[status], className)}>
      {status}
    </Badge>
  )
}
