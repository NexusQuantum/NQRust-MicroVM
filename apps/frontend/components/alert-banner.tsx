"use client"

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { AlertTriangle, CheckCircle, Info, XCircle } from "lucide-react"
import { cn } from "@/lib/utils"

interface AlertBannerProps {
  type: "success" | "warning" | "error" | "info"
  title?: string
  message: string
  className?: string
}

export function AlertBanner({ type, title, message, className }: AlertBannerProps) {
  const config = {
    success: {
      icon: CheckCircle,
      className:
        "border-green-200 bg-green-50 text-green-800 dark:border-green-800 dark:bg-green-950 dark:text-green-200",
    },
    warning: {
      icon: AlertTriangle,
      className:
        "border-yellow-200 bg-yellow-50 text-yellow-800 dark:border-yellow-800 dark:bg-yellow-950 dark:text-yellow-200",
    },
    error: {
      icon: XCircle,
      className: "border-red-200 bg-red-50 text-red-800 dark:border-red-800 dark:bg-red-950 dark:text-red-200",
    },
    info: {
      icon: Info,
      className: "border-blue-200 bg-blue-50 text-blue-800 dark:border-blue-800 dark:bg-blue-950 dark:text-blue-200",
    },
  }

  const { icon: Icon, className: typeClassName } = config[type]

  return (
    <Alert className={cn(typeClassName, className)}>
      <Icon className="h-4 w-4" />
      {title && <AlertTitle>{title}</AlertTitle>}
      <AlertDescription>{message}</AlertDescription>
    </Alert>
  )
}
