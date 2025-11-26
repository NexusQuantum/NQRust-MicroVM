"use client"

import { useEffect } from 'react'
import { useFunctions } from '@/lib/queries'
import { useFunctionStateMonitor } from '@/lib/hooks/use-function-state-monitor'

/**
 * Component to monitor individual function state
 */
function FunctionMonitor({ functionId }: { functionId: string }) {
  useFunctionStateMonitor({
    functionId,
    enabled: true,
  })
  return null
}

/**
 * Provider that monitors all functions that need state tracking
 * Monitors:
 * 1. Functions in transitional states (creating, deploying, etc.)
 * 2. Recently created functions (within 10 minutes) regardless of state
 *    to ensure we catch all state transitions including quick errors
 */
export function FunctionStateMonitorProvider() {
  const { data: functions = [] } = useFunctions(5000) // Refetch every 5 seconds

  // Stable states that typically don't transition
  const stableStates = ['ready', 'stopped', 'inactive']

  // Get current time
  const now = new Date()
  const TEN_MINUTES_MS = 10 * 60 * 1000 // 10 minutes in milliseconds

  // Get functions that need monitoring
  const activeFunctions = functions.filter((fn) => {
    // Monitor if NOT in stable state (including error states!)
    if (!stableStates.includes(fn.state)) {
      return true
    }

    // Also monitor recently created functions (within 10 minutes)
    // This ensures we catch state transitions for functions that error quickly
    if (fn.created_at) {
      const createdAt = new Date(fn.created_at)
      const ageMs = now.getTime() - createdAt.getTime()
      if (ageMs < TEN_MINUTES_MS) {
        return true
      }
    }

    return false
  })

  return (
    <>
      {activeFunctions.map((fn) => (
        <FunctionMonitor key={fn.id} functionId={fn.id} />
      ))}
    </>
  )
}
