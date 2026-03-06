import { cn } from "@/lib/utils"

const METHOD_COLORS: Record<string, string> = {
  GET: "bg-green-100 text-green-700 dark:bg-green-900/40 dark:text-green-400",
  POST: "bg-blue-100 text-blue-700 dark:bg-blue-900/40 dark:text-blue-400",
  PUT: "bg-amber-100 text-amber-700 dark:bg-amber-900/40 dark:text-amber-400",
  PATCH: "bg-purple-100 text-purple-700 dark:bg-purple-900/40 dark:text-purple-400",
  DELETE: "bg-red-100 text-red-700 dark:bg-red-900/40 dark:text-red-400",
}

export function MethodBadge({
  method,
  className,
  size = "default",
}: {
  method: string
  className?: string
  size?: "sm" | "default"
}) {
  const colors = METHOD_COLORS[method.toUpperCase()] ?? "bg-muted text-muted-foreground"
  return (
    <span
      className={cn(
        "inline-flex items-center rounded font-mono font-bold uppercase",
        size === "sm" ? "px-1.5 py-0.5 text-[10px]" : "px-2 py-1 text-xs",
        colors,
        className
      )}
    >
      {method}
    </span>
  )
}
