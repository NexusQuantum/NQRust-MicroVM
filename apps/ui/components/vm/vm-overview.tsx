"use client"

import { useState } from "react"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Badge } from "@/components/ui/badge"
import { StatusBadge } from "@/components/shared/status-badge"
import { Pencil, Check, X } from "lucide-react"
import { formatPercentage } from "@/lib/utils/format"
import type { Vm } from "@/lib/types"
import { useDateFormat } from "@/lib/hooks/use-date-format"
import { useUpdateVM } from "@/lib/queries"
import { TagEditor } from "@/components/vm/tag-editor"
import { toast } from "sonner"

interface VMOverviewProps {
  vm: Vm
}

export function VMOverview({ vm }: VMOverviewProps) {
  const dateFormat = useDateFormat()
  const updateVM = useUpdateVM()
  const [isEditingName, setIsEditingName] = useState(false)
  const [editName, setEditName] = useState(vm.name)

  const handleSaveName = () => {
    const trimmed = editName.trim()
    if (!trimmed) {
      toast.error("Name cannot be empty")
      return
    }
    if (trimmed === vm.name) {
      setIsEditingName(false)
      return
    }
    updateVM.mutate(
      { id: vm.id, data: { name: trimmed } },
      {
        onSuccess: () => {
          toast.success("VM renamed", { description: `Renamed to "${trimmed}"` })
          setIsEditingName(false)
        },
        onError: (error) => {
          toast.error("Failed to rename VM", {
            description: error instanceof Error ? error.message : "An unexpected error occurred",
          })
        },
      }
    )
  }

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle>Status & Actions</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <span className="text-sm font-medium">Current State:</span>
              <StatusBadge status={vm.state} />
            </div>

          </div>
        </CardContent>
      </Card>

      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">vCPU</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{vm.vcpu}</div>
            {vm.cpu_usage_percent !== undefined && (
              <p className="text-xs text-muted-foreground mt-1">{formatPercentage(vm.cpu_usage_percent)} usage</p>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Memory</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{vm.mem_mib} MiB</div>
            {vm.memory_usage_percent !== undefined && (
              <p className="text-xs text-muted-foreground mt-1">{formatPercentage(vm.memory_usage_percent)} usage</p>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Host</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-lg font-medium">{vm.host_addr}</div>
            <p className="text-xs text-muted-foreground mt-1">ID: {vm.host_id}</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Guest IP</CardTitle>
          </CardHeader>
          <CardContent>
            <code className="text-lg font-medium">{vm.guest_ip || "N/A"}</code>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Information</CardTitle>
        </CardHeader>
        <CardContent>
          <dl className="grid grid-cols-2 gap-4">
            <div>
              <dt className="text-sm font-medium text-muted-foreground">VM ID</dt>
              <dd className="mt-1 text-sm font-mono">{vm.id}</dd>
            </div>
            <div>
              <dt className="text-sm font-medium text-muted-foreground">Name</dt>
              <dd className="mt-1 text-sm">
                {isEditingName ? (
                  <div className="flex items-center gap-1">
                    <Input
                      value={editName}
                      onChange={(e) => setEditName(e.target.value)}
                      className="h-7 text-sm"
                      onKeyDown={(e) => {
                        if (e.key === "Enter") handleSaveName()
                        if (e.key === "Escape") { setIsEditingName(false); setEditName(vm.name) }
                      }}
                      autoFocus
                    />
                    <Button variant="ghost" size="icon" className="h-7 w-7" onClick={handleSaveName} disabled={updateVM.isPending}>
                      <Check className="h-3.5 w-3.5" />
                    </Button>
                    <Button variant="ghost" size="icon" className="h-7 w-7" onClick={() => { setIsEditingName(false); setEditName(vm.name) }}>
                      <X className="h-3.5 w-3.5" />
                    </Button>
                  </div>
                ) : (
                  <div className="flex items-center gap-1">
                    <span>{vm.name}</span>
                    <Button variant="ghost" size="icon" className="h-6 w-6" onClick={() => { setEditName(vm.name); setIsEditingName(true) }}>
                      <Pencil className="h-3 w-3" />
                    </Button>
                  </div>
                )}
              </dd>
            </div>
            <div>
              <dt className="text-sm font-medium text-muted-foreground">Created</dt>
              <dd className="mt-1 text-sm">{dateFormat.formatRelative(vm.created_at)}</dd>
            </div>
            <div>
              <dt className="text-sm font-medium text-muted-foreground">Last Updated</dt>
              <dd className="mt-1 text-sm">{dateFormat.formatRelative(vm.updated_at)}</dd>
            </div>
            <div className="col-span-2">
              <dt className="text-sm font-medium text-muted-foreground">Tags</dt>
              <dd className="mt-1">
                <TagEditor vmId={vm.id} tags={vm.tags || []} />
              </dd>
            </div>
            <div className="col-span-2">
              <dt className="text-sm font-medium text-muted-foreground">Rootfs Path</dt>
              <dd className="mt-1 text-sm font-mono break-all">{vm.rootfs_path}</dd>
            </div>
            <div className="col-span-2">
              <dt className="text-sm font-medium text-muted-foreground">Kernel Path</dt>
              <dd className="mt-1 text-sm font-mono break-all">{vm.kernel_path}</dd>
            </div>
            <div>
              <dt className="text-sm font-medium text-muted-foreground">Network TAP</dt>
              <dd className="mt-1 text-sm font-mono">{vm.tap || "N/A"}</dd>
            </div>
            <div>
              <dt className="text-sm font-medium text-muted-foreground">API Socket</dt>
              <dd className="mt-1 text-sm font-mono">{vm.api_sock}</dd>
            </div>
          </dl>
        </CardContent>
      </Card>
    </div>
  )
}
