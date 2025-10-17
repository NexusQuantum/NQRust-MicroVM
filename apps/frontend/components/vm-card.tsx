"use client"

import type { VM } from "@/types/firecracker"
import type { Vm } from "@/types/nexus"
import { Card, CardContent, CardHeader, CardTitle, CardAction } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { Play, Square, Pause, MoreHorizontal, Settings, Camera, Trash2, Cpu, HardDrive, Clock, Check, StopCircle } from "lucide-react"
import { cn, formatBytes } from "@/lib/utils"
import { useVmStatePatch, useDeleteVM } from "@/lib/queries"
import Link from "next/link"

interface VMCardProps {
  vm: Vm | VM // Support both old and new VM types during transition
  onSelect?: (vmId: string) => void
  isSelected?: boolean
}

function formatRelativeTime(input: string | number | Date): string {
  const now = Date.now()
  const then = new Date(input).getTime()
  const diff = Math.max(0, Math.floor((now - then) / 1000))
  if (diff < 60) return `${diff}s ago`
  const m = Math.floor(diff / 60)
  if (m < 60) return `${m}m ago`
  const h = Math.floor(m / 60)
  if (h < 24) return `${h}h ago`
  const d = Math.floor(h / 24)
  if (d < 30) return `${d}d ago`
  const mo = Math.floor(d / 30)
  if (mo < 12) return `${mo}mo ago`
  const y = Math.floor(mo / 12)
  return `${y}y ago`
}

export function VMCard({ vm, onSelect, isSelected }: VMCardProps) {
  const actionsMutation = useVmStatePatch()
  const deleteMutation = useDeleteVM()

  const handleAction = (actionType: 'start'|'stop'|'pause'|'resume') => {
    actionsMutation.mutate({ id: (vm as any).id, action: actionType })
  }

  const handleDelete = () => {
    const id = (vm as any).id
    const name = vm.name || id
    if (typeof window !== "undefined") {
      const confirmed = window.confirm(`Delete VM "${name}"? This cannot be undone.`)
      if (!confirmed) return
    }
    deleteMutation.mutate(id)
  }

  const isBusy = (actionsMutation as any).isPending || deleteMutation.isPending

  // Normalize state from backend (Running/Paused/NotStarted) to UI (running/paused/stopped)
  const normalizedState: "running" | "paused" | "stopped" = (
    vm?.state === "Running" || vm?.state === "running" ? "running" :
    vm?.state === "Paused" || vm?.state === "paused" ? "paused" :
    "stopped"
  )

  const canStart = normalizedState === "stopped"
  const canStop = normalizedState === "running"
  const canPause = normalizedState === "running"
  const canResume = normalizedState === "paused"

  return (
    <Card className={cn(
      "transition-colors rounded-xl shadow-md hover:shadow-lg focus-within:ring-2 focus-within:ring-ring",
      isSelected && "ring-2 ring-primary",
      normalizedState === "running" && "card-tint-running",
      normalizedState === "paused" && "card-tint-paused",
      normalizedState === "stopped" && "card-tint-stopped",
      "flex flex-col h-full"
    )}>
  <CardHeader className="px-5 pt-4 pb-4">
        <div className="flex items-start justify-between">
          <div className="flex-1 min-w-0">
            <CardTitle className="text-xl font-semibold line-clamp-1" title={vm.name || vm.id}>
              <Link href={`/vms/${vm.id}`} className="hover:text-primary transition-colors">
                {vm.name || vm.id}
              </Link>
            </CardTitle>
            {/* Status pill directly under title */}
            <div className="mt-1" aria-live="polite" data-testid="status-pill">
              {normalizedState === "running" && (
                <span className="inline-flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-xs font-medium"
                  style={{ backgroundColor: "var(--brand-emerald-50)", color: "var(--brand-emerald-700)", border: "1px solid var(--brand-emerald-200)" }}>
                  <Check className="h-3 w-3" /> Running
                </span>
              )}
              {normalizedState === "paused" && (
                <span className="inline-flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-xs font-medium"
                  style={{ backgroundColor: "var(--brand-amber-50)", color: "var(--brand-amber-700)", border: "1px solid var(--brand-amber-200)" }}>
                  <Pause className="h-3 w-3" /> Paused
                </span>
              )}
              {normalizedState === "stopped" && (
                <span className="inline-flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-xs font-medium"
                  style={{ backgroundColor: "var(--brand-danger-50)", color: "var(--brand-danger-700)", border: "1px solid var(--brand-danger-200)" }}>
                  <StopCircle className="h-3 w-3" /> Stopped
                </span>
              )}
            </div>
            {/* Description not available in new backend VM model */}
            {(vm as VM).description && (
              <p className="text-sm text-muted-foreground mt-1 line-clamp-2">{(vm as VM).description}</p>
            )}
          </div>
          <CardAction>
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="ghost" size="icon" className="h-8 w-8" aria-label={`Open actions for ${vm.name}`}> 
                  <MoreHorizontal className="h-4 w-4" />
                  <span className="sr-only">Open menu</span>
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                {canStart && (
                  <DropdownMenuItem onClick={() => handleAction("start")} aria-label={`Start ${vm.name}`}>
                    <Play className="h-4 w-4" />
                    Start VM
                  </DropdownMenuItem>
                )}
                {canStop && (
                  <DropdownMenuItem onClick={() => handleAction("stop")} aria-label={`Stop ${vm.name}`}>
                    <Square className="h-4 w-4" />
                    Stop VM
                  </DropdownMenuItem>
                )}
                {canPause && (
                  <DropdownMenuItem onClick={() => handleAction("pause")} aria-label={`Pause ${vm.name}`}>
                    <Pause className="h-4 w-4" />
                    Pause VM
                  </DropdownMenuItem>
                )}
                <DropdownMenuSeparator />
                <DropdownMenuItem asChild>
                  <Link href={`/vms/${vm.id}`}>
                    <Settings className="h-4 w-4" />
                    Configure
                  </Link>
                </DropdownMenuItem>
                <DropdownMenuItem>
                  <Camera className="h-4 w-4" />
                  Create Snapshot
                </DropdownMenuItem>
                <DropdownMenuSeparator />
                <DropdownMenuItem className="text-destructive" onClick={handleDelete} aria-label={`Delete ${vm.name}`} disabled={deleteMutation.isPending}>
                  <Trash2 className="h-4 w-4" />
                  Delete VM
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </CardAction>
        </div>
      </CardHeader>

      <CardContent className="px-5 pb-4 pt-0 flex flex-col flex-1">
          {/* Tags */}
          <div className="space-y-3">
          {/* Tags - only available in old VM model */}
          {(vm as VM)?.tags && (
            <div className="flex flex-wrap gap-1.5">
              {(!Array.isArray((vm as VM).tags) ? Object.entries((vm as VM).tags as Record<string, string>) : ((vm as VM).tags as string[]).map(t => [t, ""]))
                .slice(0, 3)
                .map(([key, value]) => {
                  const base = "text-xs rounded-full px-2.5 py-0.5 font-medium"
                  let cls = ""
                  if (!Array.isArray((vm as VM).tags)) {
                    if (key === "role") {
                      if (String(value).toLowerCase() === "web") cls = "border-[var(--brand-primary)] text-[color:var(--brand-primary)] bg-[var(--brand-primary-50)]"
                      else if (String(value).toLowerCase() === "database") cls = "border-[var(--brand-purple)] text-[color:var(--brand-purple)] bg-[var(--brand-purple-50)]"
                      else if (String(value).toLowerCase() === "dev") cls = "border-[var(--brand-emerald)] text-[color:var(--brand-emerald)] bg-[var(--brand-emerald-50)]"
                      return (
                        <Badge key={key} variant="outline" data-testid="chip-role" className={cn(base, "max-w-[160px] truncate", cls)}>
                          {key}: {String(value)}
                        </Badge>
                      )
                    }
                    if (key === "tier") {
                      if (String(value).toLowerCase() === "production") cls = "bg-[var(--brand-primary)] text-white"
                      else if (String(value).toLowerCase() === "staging") cls = "bg-[var(--brand-amber-100)] text-[color:var(--brand-amber-700)]"
                      else if (String(value).toLowerCase() === "development") cls = "bg-[var(--brand-neutral-50)] text-[color:var(--brand-neutral-700)]"
                      return (
                        <Badge key={key} data-testid="chip-tier" className={cn(base, "max-w-[160px] truncate", cls)}>
                          {key}: {String(value)}
                        </Badge>
                      )
                    }
                    return (
                      <Badge key={key} variant="secondary" className={cn(base, "max-w-[160px] truncate")}>
                        {key}: {String(value)}
                      </Badge>
                    )
                  }
                  // Array form: show tag label only
                  return (
                    <Badge key={key} variant="secondary" className={cn(base, "max-w-[160px] truncate")}>
                      {key}
                    </Badge>
                  )
                })}
              {(!Array.isArray((vm as VM).tags) ? Object.keys((vm as VM).tags).length : ((vm as VM).tags as string[]).length) > 3 && (
                <Badge variant="outline" className="text-xs rounded-full px-2.5 py-0.5 font-medium">
                  +{(!Array.isArray((vm as VM).tags) ? Object.keys((vm as VM).tags).length : ((vm as VM).tags as string[]).length) - 3} more
                </Badge>
              )}
            </div>
          )}

          {/* Specs - adapt to new backend structure */}
          <div className="flex items-center gap-4 text-sm text-muted-foreground">
            <div className="flex items-center gap-1.5">
              <Cpu className="h-4 w-4" />
              <span className="font-medium">
                {(vm as Vm).vcpu || (vm as VM)?.config?.machine?.vcpu_count || 0} vCPU
              </span>
            </div>
            <span aria-hidden>•</span>
            <div className="flex items-center gap-1.5">
              <HardDrive className="h-4 w-4" />
              <span className="font-medium">
                {formatBytes(((vm as Vm).mem_mib || (vm as VM)?.config?.machine?.mem_size_mib || 0) * 1024 * 1024)}
              </span>
            </div>
          </div>

          {/* Push the bottom section down */}
          <div className="mt-auto" />

          {/* Bottom metadata */}
          <div className="grid grid-cols-3 gap-4 text-xs text-muted-foreground">
            <div>
              <span className="block">Host</span>
              <span className="font-medium text-foreground/80 truncate" title={(vm as Vm).host_addr}>
                {(vm as Vm).host_addr || (vm as VM).owner || '-'}
              </span>
            </div>
            <div>
              <span className="block">State</span>
              <span className="font-medium capitalize text-foreground/80">
                {vm.state || (vm as VM).environment || '-'}
              </span>
            </div>
            <div className="flex items-center gap-1">
              <Clock className="h-3 w-3" />
              <time dateTime={vm.updated_at ? new Date(vm.updated_at).toISOString() : ''} title={vm.updated_at ? new Date(vm.updated_at).toLocaleString() : ''}>
                Updated {vm.updated_at ? formatRelativeTime(vm.updated_at) : '-'}
              </time>
            </div>
          </div>

          {/* Quick actions */}
          <div className={cn(
            "pt-3 grid gap-2",
            vm.state === "stopped" && "grid-cols-1",
            (vm.state === "running" || vm.state === "paused") && "grid-cols-2"
          )}>
            {normalizedState === "stopped" && (
              <Button
                size="sm"
                variant="ghost"
                onClick={() => handleAction("start")}
                disabled={(actionsMutation as any).isPending}
                aria-label={`Start ${vm.name}`}
                data-testid="btn-start"
                className="w-full bg-[var(--brand-emerald-50)] text-[color:var(--brand-emerald-700)] hover:bg-[var(--brand-emerald-700)] hover:text-white border border-[color:var(--brand-emerald-200)]"
              >
                {(actionsMutation as any).isPending ? (
                  <span className="inline-block size-3 rounded-full border-2 border-white/40 border-t-white animate-spin" aria-hidden />
                ) : (
                  <Play className="h-3 w-3" />
                )}
                Start
              </Button>
            )}
            {(normalizedState === "running" || normalizedState === "paused") && (
              <Button
                size="sm"
                variant="ghost"
                onClick={() => handleAction("stop")}
                disabled={(actionsMutation as any).isPending}
                aria-label={`Stop ${vm.name}`}
                data-testid="btn-stop"
                className="w-full bg-[var(--brand-danger-50)] text-[color:var(--brand-danger-700)] hover:bg-[var(--brand-danger-700)] hover:text-white border border-[color:var(--brand-danger-200)]"
              >
                {(actionsMutation as any).isPending ? (
                  <span className="inline-block size-3 rounded-full border-2 border-white/40 border-t-white animate-spin" aria-hidden />
                ) : (
                  <Square className="h-3 w-3" />
                )}
                Stop
              </Button>
            )}
            {normalizedState === "running" && (
              <Button
                size="sm"
                variant="ghost"
                onClick={() => handleAction("pause")}
                disabled={(actionsMutation as any).isPending}
                aria-label={`Pause ${vm.name}`}
                data-testid="btn-pause"
                className="w-full bg-[var(--brand-amber-50)] text-[color:var(--brand-amber-700)] hover:bg-[var(--brand-amber-700)] hover:text-white border border-[color:var(--brand-amber-200)]"
              >
                {(actionsMutation as any).isPending ? (
                  <span className="inline-block size-3 rounded-full border-2 border-white/40 border-t-white animate-spin" aria-hidden />
                ) : (
                  <Pause className="h-3 w-3" />
                )}
                Pause
              </Button>
            )}
            {normalizedState === "paused" && (
              <Button
                size="sm"
                variant="ghost"
                onClick={() => handleAction("resume")}
                disabled={(actionsMutation as any).isPending}
                aria-label={`Resume ${vm.name}`}
                data-testid="btn-resume"
                className="w-full bg-[var(--brand-emerald-50)] text-[color:var(--brand-emerald-700)] hover:bg-[var(--brand-emerald-700)] hover:text-white border border-[color:var(--brand-emerald-200)]"
              >
                {(actionsMutation as any).isPending ? (
                  <span className="inline-block size-3 rounded-full border-2 border-white/40 border-t-white animate-spin" aria-hidden />
                ) : (
                  <Play className="h-3 w-3" />
                )}
                Resume
              </Button>
            )}
          </div>
        </div>
      </CardContent>
    </Card>
  )
}
