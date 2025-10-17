"use client"

import type { VM } from "@/types/firecracker"
import type { Vm } from "@/types/nexus"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Separator } from "@/components/ui/separator"
import { Cpu, HardDrive, Zap, Tag, Server, Play, Square, Pause, Terminal, Download } from "lucide-react"
import { formatBytes } from "@/lib/utils"
import { useVmStatePatch, type VmStateAction } from "@/lib/queries"

interface VMOverviewTabProps {
  vm: VM | Vm // Support both old and new VM types
}

export function VMOverviewTab({ vm }: VMOverviewTabProps) {
  const facadeActions = useVmStatePatch()

  const normalizedState = typeof vm.state === "string" ? vm.state.toLowerCase() : vm.state
  const canStart = normalizedState === "not_started" || normalizedState === "stopped"
  const canStop = normalizedState === "running"
  const canPause = normalizedState === "running"
  const canResume = normalizedState === "paused"
  const canSendCtrlAltDel = normalizedState === "running"
  const canFlushMetrics = normalizedState === "running"

  const triggerAction = (action: VmStateAction) => {
    facadeActions.mutate({ id: vm.id, action })
  }

  // Adapt to new backend structure - VM data is flatter
  const newVm = vm as Vm
  const oldVm = vm as VM
  
  // For new backend, VM fields are directly on the VM object
  const machine = {
    vcpu_count: newVm.vcpu || oldVm.config?.machine?.vcpu_count,
    mem_size_mib: newVm.mem_mib || oldVm.config?.machine?.mem_size_mib,
    smt: false, // Not available in new backend
    cpu_template: "None", // Not available in new backend
  }
  
  const boot = {
    kernel_image_path: newVm.kernel_path || oldVm.config?.boot?.kernel_image_path,
    initrd_path: undefined, // Not available in new backend
    boot_args: undefined, // Not available in new backend
  }

  return (
    <div className="grid gap-6 md:grid-cols-2">
      {/* VM Configuration */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Server className="h-5 w-5" />
            VM Configuration
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          {machine && (
            <div className="grid gap-3">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <Cpu className="h-4 w-4 text-muted-foreground" />
                  <span className="text-sm font-medium">vCPUs</span>
                </div>
                <Badge variant="outline">{machine.vcpu_count}</Badge>
              </div>

              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <HardDrive className="h-4 w-4 text-muted-foreground" />
                  <span className="text-sm font-medium">Memory</span>
                </div>
                <Badge variant="outline">{formatBytes((machine.mem_size_mib || 0) * 1024 * 1024)}</Badge>
              </div>

              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <Zap className="h-4 w-4 text-muted-foreground" />
                  <span className="text-sm font-medium">SMT</span>
                </div>
                <Badge variant={machine.smt ? "default" : "secondary"}>
                  {machine.smt ? "Enabled" : "Disabled"}
                </Badge>
              </div>

              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <Cpu className="h-4 w-4 text-muted-foreground" />
                  <span className="text-sm font-medium">CPU Template</span>
                </div>
                <Badge variant="outline">{machine.cpu_template}</Badge>
              </div>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Boot Configuration */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Play className="h-5 w-5" />
            Boot Configuration
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          {boot && (
            <div className="space-y-3">
              <div>
                <label className="text-sm font-medium text-muted-foreground">Kernel Image</label>
                <p className="text-sm font-mono bg-muted p-2 rounded mt-1 break-all">
                  {boot.kernel_image_path}
                </p>
              </div>

              {boot.initrd_path && (
                <div>
                  <label className="text-sm font-medium text-muted-foreground">Initial RAM Disk</label>
                  <p className="text-sm font-mono bg-muted p-2 rounded mt-1 break-all">{boot.initrd_path}</p>
                </div>
              )}

              {boot.boot_args && (
                <div>
                  <label className="text-sm font-medium text-muted-foreground">Boot Arguments</label>
                  <p className="text-sm font-mono bg-muted p-2 rounded mt-1 break-all">{boot.boot_args}</p>
                </div>
              )}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Quick Actions */}
      <Card>
        <CardHeader>
          <CardTitle>Quick Actions</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          <div className="grid gap-2">
            {canStart && (
              <Button
                variant="outline"
                onClick={() => triggerAction("start")}
                disabled={facadeActions.isPending}
                className="w-full justify-start"
              >
                <Play className="h-4 w-4" />
                Start VM
              </Button>
            )}

            {canPause && (
              <Button
                variant="outline"
                onClick={() => triggerAction("pause")}
                disabled={facadeActions.isPending}
                className="w-full justify-start"
              >
                <Pause className="h-4 w-4" />
                Pause VM
              </Button>
            )}

            {canResume && (
              <Button
                variant="outline"
                onClick={() => triggerAction("resume")}
                disabled={facadeActions.isPending}
                className="w-full justify-start"
              >
                <Play className="h-4 w-4" />
                Resume VM
              </Button>
            )}

            {canStop && (
              <Button
                variant="outline"
                onClick={() => triggerAction("stop")}
                disabled={facadeActions.isPending}
                className="w-full justify-start"
              >
                <Square className="h-4 w-4" />
                Stop VM
              </Button>
            )}

            {(canStart || canPause || canResume || canStop) && <Separator />}

            <Button
              variant="outline"
              onClick={() => triggerAction("ctrl_alt_del")}
              disabled={facadeActions.isPending || !canSendCtrlAltDel}
              className="w-full justify-start"
            >
              <Terminal className="h-4 w-4" />
              Send Ctrl+Alt+Del
            </Button>

            <Button
              variant="outline"
              onClick={() => triggerAction("flush_metrics")}
              disabled={facadeActions.isPending || !canFlushMetrics}
              className="w-full justify-start"
            >
              <Download className="h-4 w-4" />
              Flush Metrics
            </Button>
          </div>
        </CardContent>
      </Card>

      {/* Tags & Metadata */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Tag className="h-5 w-5" />
            Tags & Metadata
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* New backend doesn't have tags, show other metadata */}
          <div className="space-y-2">
            <div className="flex items-center justify-between text-sm">
              <span className="text-muted-foreground">Host ID</span>
              <code className="text-xs bg-muted px-1 py-0.5 rounded">{newVm.host_id || 'N/A'}</code>
            </div>
            
            <div className="flex items-center justify-between text-sm">
              <span className="text-muted-foreground">Host Address</span>
              <code className="text-xs bg-muted px-1 py-0.5 rounded">{newVm.host_addr || 'N/A'}</code>
            </div>

            <div className="flex items-center justify-between text-sm">
              <span className="text-muted-foreground">API Socket</span>
              <code className="text-xs bg-muted px-1 py-0.5 rounded">{newVm.api_sock || oldVm.socket_path || 'N/A'}</code>
            </div>

            {newVm.http_port && (
              <div className="flex items-center justify-between text-sm">
                <span className="text-muted-foreground">HTTP Port</span>
                <code className="text-xs bg-muted px-1 py-0.5 rounded">{newVm.http_port}</code>
              </div>
            )}

            {newVm.tap && (
              <div className="flex items-center justify-between text-sm">
                <span className="text-muted-foreground">TAP Interface</span>
                <code className="text-xs bg-muted px-1 py-0.5 rounded">{newVm.tap}</code>
              </div>
            )}

            {newVm.fc_unit && (
              <div className="flex items-center justify-between text-sm">
                <span className="text-muted-foreground">Firecracker Unit</span>
                <code className="text-xs bg-muted px-1 py-0.5 rounded">{newVm.fc_unit}</code>
              </div>
            )}
          </div>
        </CardContent>
      </Card>
    </div>
  )
}
