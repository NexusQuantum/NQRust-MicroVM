"use client"

import { MetricsChart } from "@/components/shared/metrics-chart"

interface ContainerStatsProps {
  containerId: string
  vmId?: string | null
  containerState?: string
}

export function ContainerStats({ containerId, vmId, containerState }: ContainerStatsProps) {
  // Use the same MetricsChart component as VMs, passing the VM ID
  // MetricsChart uses the VM metrics websocket endpoint
  if (!vmId) {
    return (
      <div className="text-center text-muted-foreground py-8">
        No VM associated with this container
      </div>
    )
  }

  return <MetricsChart resourceId={vmId} resourceType="container" />
}
