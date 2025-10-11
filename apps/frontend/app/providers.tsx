"use client"

import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { ReactQueryDevtools } from "@tanstack/react-query-devtools"
import { ThemeProvider } from "@/components/theme-provider"
import { Toaster } from "sonner"
import { useState } from "react"

export function Providers({ children }: { children: React.ReactNode }) {
  const [queryClient] = useState(
    () =>
      new QueryClient({
        defaultOptions: {
          queries: {
            staleTime: 30 * 1000, // 30 seconds
            retry: (failureCount, error) => {
              // Don't retry on 4xx errors
              if (error instanceof Error) {
                try {
                  const parsedError = JSON.parse(error.message)
                  if (parsedError.status >= 400 && parsedError.status < 500) {
                    return false
                  }
                } catch {
                  // Not a facade error, continue with default retry logic
                }
              }
              return failureCount < 3
            },
            refetchOnWindowFocus: false,
          },
          mutations: {
            retry: false,
          },
        },
      })
  )

  return (
    <QueryClientProvider client={queryClient}>
      <ThemeProvider
        attribute="class"
        defaultTheme={process.env.NEXT_PUBLIC_BRAND_PRESET || "dark"}
        enableSystem
        disableTransitionOnChange
      >
        {children}
        <Toaster
          position="top-right"
          expand
          richColors
          closeButton
        />
      </ThemeProvider>
      {process.env.NODE_ENV === "development" && (
        <ReactQueryDevtools initialIsOpen={false} />
      )}
    </QueryClientProvider>
  )
}