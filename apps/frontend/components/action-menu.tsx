"use client"

import type { VM } from "@/types/firecracker"
import type { Vm } from "@/types/nexus"
import { Button } from "@/components/ui/button"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { Play, Square, Pause, MoreHorizontal, Settings, Camera, Trash2, RotateCcw, Terminal, Download, Monitor } from "lucide-react"
import { useVmStatePatch } from "@/lib/queries"
import Link from "next/link"
import { useRouter } from "next/navigation"

interface ActionMenuProps {
  vm: VM | Vm
  variant?: "button" | "icon"
}

export function ActionMenu({ vm, variant = "icon" }: ActionMenuProps) {
  const facadeActions = useVmStatePatch()
  const router = useRouter()

  const handleFacadeAction = (action: 'start'|'pause'|'resume'|'stop'|'ctrl_alt_del'|'flush_metrics') => {
    facadeActions.mutate({ id: vm.id, action })
  }

  const canStart = vm.state === "not_started" || vm.state === "stopped"
  const canStop = vm.state === "running"
  const canPause = vm.state === "running"
  const canResume = vm.state === "paused"
  const canSendCtrlAltDel = vm.state === "running"

  const TriggerButton =
    variant === "button" ? (
      <Button variant="outline" size="sm">
        Actions
        <MoreHorizontal className="h-4 w-4 ml-1" />
      </Button>
    ) : (
      <Button variant="ghost" size="icon" className="h-8 w-8">
        <MoreHorizontal className="h-4 w-4" />
        <span className="sr-only">Open actions menu</span>
      </Button>
    )

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>{TriggerButton}</DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-48">
        {/* VM Control Actions */}
        {canStart && (
          <DropdownMenuItem onClick={() => handleFacadeAction("start")} disabled={facadeActions.isPending}>
            <Play className="h-4 w-4" />
            Start VM
          </DropdownMenuItem>
        )}

        {canStop && (
          <DropdownMenuItem onClick={() => handleFacadeAction("stop")} disabled={facadeActions.isPending}>
            <Square className="h-4 w-4" />
            Stop VM
          </DropdownMenuItem>
        )}

        {canPause && (
          <DropdownMenuItem onClick={() => handleFacadeAction("pause")} disabled={facadeActions.isPending}>
            <Pause className="h-4 w-4" />
            Pause VM
          </DropdownMenuItem>
        )}

        {canResume && (
          <DropdownMenuItem onClick={() => handleFacadeAction("resume")} disabled={facadeActions.isPending}>
            <Play className="h-4 w-4" />
            Resume VM
          </DropdownMenuItem>
        )}

        {(canStart || canStop || canPause || canResume) && <DropdownMenuSeparator />}

        {/* VM Management Actions */}
        {canSendCtrlAltDel && (
          <DropdownMenuItem onClick={() => handleFacadeAction("ctrl_alt_del")} disabled={facadeActions.isPending}>
            <Terminal className="h-4 w-4" />
            Send Ctrl+Alt+Del
          </DropdownMenuItem>
        )}

        {vm.state === "running" && (
          <DropdownMenuItem onClick={() => handleFacadeAction("flush_metrics")} disabled={facadeActions.isPending}>
            <Download className="h-4 w-4" />
            Flush Metrics
          </DropdownMenuItem>
        )}

        {canSendCtrlAltDel && <DropdownMenuSeparator />}

        {/* Navigation Actions */}
        <DropdownMenuItem asChild>
          <Link href={`/vms/${vm.id}`}>
            <Settings className="h-4 w-4" />
            Configure VM
          </Link>
        </DropdownMenuItem>

        <DropdownMenuItem onClick={() => router.push(`/vms/${vm.id}/shell`)}>
          <Monitor className="h-4 w-4" />
          Open Shell
        </DropdownMenuItem>

        <DropdownMenuItem>
          <Camera className="h-4 w-4" />
          Create Snapshot
        </DropdownMenuItem>

        <DropdownMenuItem>
          <RotateCcw className="h-4 w-4" />
          Restore Snapshot
        </DropdownMenuItem>

        <DropdownMenuSeparator />

        {/* Destructive Actions */}
        <DropdownMenuItem className="text-destructive focus:text-destructive">
          <Trash2 className="h-4 w-4" />
          Delete VM
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
