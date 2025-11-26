import { useEffect, useRef } from 'react'
import { useQuery } from '@tanstack/react-query'
import { facadeApi } from '@/lib/api/facade'
import { useNotificationStore } from '@/lib/stores/notification-store'
import type { Function } from '@/lib/types'

interface FunctionStateMonitorOptions {
  functionId: string
  enabled?: boolean
  onStateChange?: (oldState: string, newState: string) => void
}

/**
 * Hook to monitor function state changes and send notifications
 * Monitors state transitions: creating -> deploying -> ready/error
 */
export function useFunctionStateMonitor({
  functionId,
  enabled = true,
  onStateChange,
}: FunctionStateMonitorOptions) {
  const { addNotification } = useNotificationStore()
  const previousStateRef = useRef<string | null>(null)

  // Poll function every 3 seconds to detect state changes
  const { data: functionData } = useQuery({
    queryKey: ['function-state-monitor', functionId],
    queryFn: () => facadeApi.getFunction(functionId),
    enabled: enabled && !!functionId,
    refetchInterval: (data) => {
      // Stop polling when function reaches terminal/stable state
      const state = (data as Function)?.state

      // Stop polling for all terminal states (stable + error)
      // Provider will continue monitoring recently created functions
      // to catch any state transitions, but polling itself can stop
      const terminalStates = ['ready', 'stopped', 'inactive', 'failed', 'error', 'crashed']

      if (terminalStates.includes(state)) {
        return false // Stop polling for all terminal states
      }

      return 3000 // Poll every 3 seconds for transitional states
    },
    staleTime: 0, // Always fetch fresh data
  })

  useEffect(() => {
    if (!functionData) return

    const currentState = functionData.state
    const previousState = previousStateRef.current

    // If state hasn't changed, skip
    if (previousState === currentState) {
      return
    }

    // If this is first render (previousState is null)
    if (!previousState) {
      // Just set initial state and skip notification
      previousStateRef.current = currentState
      return
    }

    // Skip notification for error states
    const errorStates = ['error', 'failed', 'crashed']
    if (errorStates.includes(currentState)) {
      // Update state but don't send notification
      previousStateRef.current = currentState
      onStateChange?.(previousState, currentState)
      return
    }

    // Call optional callback
    onStateChange?.(previousState, currentState)

    // Send notification for state changes (except error states)
    // Define notification messages for all possible states
    const stateMessages: Record<string, { title: string; message: string; type: 'info' | 'success' | 'error' | 'warning' }> = {
      creating: {
        title: 'Function Creating',
        message: `Function "${functionData.name}" is being created`,
        type: 'info',
      },
      booting: {
        title: 'Function Booting',
        message: `Function "${functionData.name}" is booting up`,
        type: 'info',
      },
      deploying: {
        title: 'Function Deploying',
        message: `Function "${functionData.name}" is being deployed`,
        type: 'info',
      },
      development: {
        title: 'Function In Development',
        message: `Function "${functionData.name}" is in development mode`,
        type: 'info',
      },
      starting: {
        title: 'Function Starting',
        message: `Function "${functionData.name}" is starting`,
        type: 'info',
      },
      ready: {
        title: 'Function Ready',
        message: `Function "${functionData.name}" is now ready to use`,
        type: 'success',
      },
      active: {
        title: 'Function Active',
        message: `Function "${functionData.name}" is now active`,
        type: 'success',
      },
      running: {
        title: 'Function Running',
        message: `Function "${functionData.name}" is running`,
        type: 'success',
      },
      stopping: {
        title: 'Function Stopping',
        message: `Function "${functionData.name}" is stopping`,
        type: 'warning',
      },
      stopped: {
        title: 'Function Stopped',
        message: `Function "${functionData.name}" has been stopped`,
        type: 'warning',
      },
      paused: {
        title: 'Function Paused',
        message: `Function "${functionData.name}" has been paused`,
        type: 'warning',
      },
      inactive: {
        title: 'Function Inactive',
        message: `Function "${functionData.name}" is now inactive`,
        type: 'warning',
      },
      error: {
        title: 'Function Error',
        message: `Function "${functionData.name}" encountered an error`,
        type: 'error',
      },
      failed: {
        title: 'Function Failed',
        message: `Function "${functionData.name}" deployment failed`,
        type: 'error',
      },
      crashed: {
        title: 'Function Crashed',
        message: `Function "${functionData.name}" has crashed`,
        type: 'error',
      },
    }

    // Get notification config for current state, or create a generic one
    const notification = stateMessages[currentState] || {
      title: 'Function State Changed',
      message: `Function "${functionData.name}" state changed to: ${currentState}`,
      type: 'info' as const,
    }

    // Always send notification for state changes
    addNotification({
      ...notification,
      actionUrl: `/functions/${functionId}`,
      resourceType: 'function',
      resourceId: functionId,
    })

    // Update previous state
    previousStateRef.current = currentState
  }, [functionData, functionId, onStateChange, addNotification])

  return {
    currentState: functionData?.state,
    functionData,
  }
}
