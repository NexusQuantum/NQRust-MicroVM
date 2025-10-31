import { Badge } from "@/components/ui/badge"
import { cn } from "@/lib/utils"

interface StatusBadgeProps {
  status: "running" | "stopped" | "paused" | "idle" | "executing" | "error" | "restarting" | "success" | "timeout" | "creating" | "deploying" | "ready" | "booting"
  className?: string
}

export function StatusBadge({ status, className }: StatusBadgeProps) {
  const variants = {
    // VM states
    running: "bg-emerald-100 text-emerald-700 border-emerald-200",
    stopped: "bg-red-100 text-red-600 border-red-200",
    paused: "bg-amber-100 text-amber-700 border-amber-200",
    idle: "bg-slate-100 text-slate-600 border-slate-200",
    executing: "bg-blue-100 text-blue-700 border-blue-200",
    error: "bg-red-100 text-red-700 border-red-200",
    restarting: "bg-amber-100 text-amber-700 border-amber-200",
    success: "bg-emerald-100 text-emerald-700 border-emerald-200",
    timeout: "bg-orange-100 text-orange-700 border-orange-200",

    // Function states (matching function-table.tsx)
    creating: "bg-yellow-100 text-yellow-700 border-yellow-200",
    deploying: "bg-blue-100 text-blue-700 border-blue-200",
    ready: "bg-green-100 text-green-700 border-green-200",
    booting: "bg-gray-100 text-gray-700 border-gray-200",
  }

  const labels = {
    // VM states
    running: "Running",
    stopped: "Stopped",
    paused: "Paused",
    idle: "Idle",
    executing: "Executing",
    error: "Error",
    restarting: "Restarting",
    success: "Success",
    timeout: "Timeout",

    // Function states
    creating: "Creating",
    deploying: "Deploying",
    ready: "Ready",
    booting: "Booting",
  }

  return (
    <Badge variant="outline" className={cn("font-medium", variants[status], className)}>
      {labels[status]}
    </Badge>
  )
}
