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
import {
  Play,
  Square,
  Pause,
  MoreHorizontal,
  Settings,
  Camera,
  Trash2,
  RotateCcw,
  Terminal,
  Download,
} from "lucide-react"
import { useVmStatePatch } from "@/lib/queries"
import Link from "next/link"

interface ActionMenuProps {
  vm: VM | Vm
  variant?: "button" | "icon"
}

export function ActionMenu({ vm, variant = "icon" }: ActionMenuProps) {
  const facadeActions = useVmStatePatch()

  const handleFacadeAction = (action: 'start'|'pause'|'resume'|'stop') => {
    facadeActions.mutate({ id: vm.id, action })
  }

  const canStop = vm.state === "running"

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
        {canStop && (
          <DropdownMenuItem onClick={() => handleFacadeAction("stop")} disabled={facadeActions.isPending}>
            <Square className="h-4 w-4" />
            Stop VM
          </DropdownMenuItem>
        )}

        {(canStart || canStop || canPause || canResume) && <DropdownMenuSeparator />}

        {/* VM Management Actions */}
        {/* No Ctrl+Alt+Del or FlushMetrics in current backend */}

        <DropdownMenuSeparator />

        {/* Navigation Actions */}
        <DropdownMenuItem asChild>
          <Link href={`/vms/${vm.id}`}>
            <Settings className="h-4 w-4" />
            Configure VM
          </Link>
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
