"use client"

import * as React from "react"
import * as CheckboxPrimitive from "@radix-ui/react-checkbox"
import { CheckIcon } from "lucide-react"

import { cn } from "@/lib/utils"

function Checkbox({
  className,
  ...props
}: React.ComponentProps<typeof CheckboxPrimitive.Root>) {
  return (
    <CheckboxPrimitive.Root
      data-slot="checkbox"
      // className={cn(
      //   "peer border-input dark:bg-input/30 data-[state=checked]:bg-primary data-[state=checked]:text-primary-foreground dark:data-[state=checked]:bg-primary data-[state=checked]:border-primary focus-visible:border-ring focus-visible:ring-ring/50 aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive size-4 shrink-0 rounded-[4px] border shadow-xs transition-shadow outline-none focus-visible:ring-[3px] disabled:cursor-not-allowed disabled:opacity-50",
      //   className
      // )}
      className={cn(
        // size + shape
        "peer h-4 w-4 shrink-0 rounded-md bg-background",
        // base border + extra visible outline (works on light/dark)
        "border border-input shadow-[0_0_0_1px_hsl(var(--border))]",
        // focus ring
        "ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
        // states
        "disabled:cursor-not-allowed disabled:opacity-60",
        // hover: sedikit tebalkan garis
        "hover:shadow-[0_0_0_1.5px_hsl(var(--foreground)/0.25)]",
        // checked: pakai warna primary & tebalkan sedikit
        "data-[state=checked]:bg-primary data-[state=checked]:text-primary-foreground",
        "data-[state=checked]:border-primary data-[state=checked]:shadow-[0_0_0_1.5px_hsl(var(--primary))]",
        className
      )}
      {...props}
    >
      <CheckboxPrimitive.Indicator
        data-slot="checkbox-indicator"
        className="flex items-center justify-center text-current transition-none"
      >
        <CheckIcon className="size-3.5" />
      </CheckboxPrimitive.Indicator>
    </CheckboxPrimitive.Root>
  )
}

export { Checkbox }
