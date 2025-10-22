"use client"

import { MockTerminal } from "./mock-terminal"

interface XTermWrapperProps {
  vmId?: string
  containerId?: string
}

export function XTermWrapper({ vmId, containerId }: XTermWrapperProps) {
  return <MockTerminal vmId={vmId} containerId={containerId} />
}
