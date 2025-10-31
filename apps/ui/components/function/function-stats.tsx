"use client"

import { MetricsChart } from "@/components/shared/metrics-chart"
import type { Function as FnType } from "@/lib/types"

interface FunctionStatsProps {
  functionData?: FnType
}

export function FunctionStats({ functionData }: FunctionStatsProps) {
  // Use the same MetricsChart component as VMs and containers, passing the VM ID
  // MetricsChart uses the VM metrics websocket endpoint
  if (!functionData?.vm_id) {
    return (
      <div className="text-center text-muted-foreground py-8">
        No VM associated with this function
      </div>
    )
  }

  return <MetricsChart resourceId={functionData.vm_id} resourceType="function" />
}
