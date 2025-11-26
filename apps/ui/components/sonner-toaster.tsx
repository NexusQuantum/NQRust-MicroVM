"use client"

import { Toaster } from "sonner"
import { useTheme } from "next-themes"

export function SonnerToaster() {
  const { theme } = useTheme()

  return (
    <Toaster
      theme={theme === "dark" ? "dark" : "light"}
      position="bottom-right"
      richColors
      closeButton
      expand={false}
      duration={4000}
    />
  )
}
