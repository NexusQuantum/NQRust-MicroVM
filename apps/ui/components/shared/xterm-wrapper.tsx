"use client"

import { VMTerminal } from "@/components/vm/vm-terminal"
import { useVM } from "@/lib/queries"
import { MockTerminal } from "./mock-terminal"

interface XTermWrapperProps {
  vmId?: string
  containerId?: string
}

export function XTermWrapper({ vmId, containerId }: XTermWrapperProps) {
  // If containerId is provided, show mock terminal (container support not implemented yet)
  if (containerId) {
    return <MockTerminal vmId={vmId} containerId={containerId} />
  }

  // For VMs, use the real terminal if we have a vmId
  if (vmId) {
    // We need the VM object for the terminal, so fetch it
    // eslint-disable-next-line react-hooks/rules-of-hooks
    const { data: vm, isLoading } = useVM(vmId)

    if (isLoading) {
      return <div className="flex items-center justify-center h-[600px]">Loading terminal...</div>
    }

    if (!vm) {
      return <div className="flex items-center justify-center h-[600px]">VM not found</div>
    }

    return <VMTerminal vm={vm} />
  }

  // Fallback to mock terminal
  return <MockTerminal vmId={vmId} containerId={containerId} />
}
