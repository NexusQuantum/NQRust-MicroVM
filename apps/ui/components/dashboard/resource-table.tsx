"use client"

import { useState, useMemo, useEffect } from "react"
import Link from "next/link"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { ReusableTabs, TabItem } from "@/components/dashboard/tabs-new"
import { StatusBadge } from "@/components/shared/status-badge"
import { Server, Zap, Container, Play, Trash2, Search, Square, Pause, FileText, Check, AlertTriangle, Loader2, Clipboard } from "lucide-react"
import type { UnifiedResource } from "@/lib/api/dashboard"
import { useVmStatePatch, useDeleteVM, useInvokeFunction } from "@/lib/queries"
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip"
import { ConfirmDialog } from "@/components/shared/confirm-dialog"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { useAuthStore, canModifyResource, canDeleteResource } from "@/lib/auth/store"
import { useDateFormat } from "@/lib/hooks/use-date-format"
import { useTheme } from "next-themes"
import dynamic from "next/dynamic"
import { toast } from "sonner"

const Editor = dynamic(() => import("@monaco-editor/react"), { ssr: false })

interface ResourceTableProps {
  resources: UnifiedResource[]
}

const DEFAULT_PAYLOAD = `{
  "key1": 10,
  "key2": 5
}`

export function ResourceTable({ resources }: ResourceTableProps) {
  const dateFormat = useDateFormat()
  const { user } = useAuthStore()
  const { theme } = useTheme()
  const [searchQuery, setSearchQuery] = useState("")
  const [typeFilter, setTypeFilter] = useState<string>("all")
  const [stateFilter, setStateFilter] = useState<string>("all")
  const [currentPage, setCurrentPage] = useState(1)
  const itemsPerPage = 10

  // ---- invoke function dialog ----
  const invokeMutation = useInvokeFunction()
  const [perFunctionPayload, setPerFunctionPayload] = useState<Record<string, string>>({})
  const [showInvokeDialog, setShowInvokeDialog] = useState<{ open: boolean; fnId: string; fnName: string }>({
    open: false,
    fnId: "",
    fnName: "",
  })
  const [payloadText, setPayloadText] = useState(DEFAULT_PAYLOAD)
  const [jsonError, setJsonError] = useState<string>("")
  const [invokeOutput, setInvokeOutput] = useState<string>("")

  // Function invoke handlers
  const openInvokeDialog = ({ id, name }: { id: string; name: string }) => {
    const cached = perFunctionPayload[id] ?? DEFAULT_PAYLOAD
    setPayloadText(cached)
    setJsonError("")
    setInvokeOutput("")
    setShowInvokeDialog({ open: true, fnId: id, fnName: name })
  }

  const closeInvokeDialog = () => setShowInvokeDialog((s) => ({ ...s, open: false }))

  const onPayloadChange = (value?: string) => {
    const v = value ?? ""
    setPayloadText(v)
    try {
      JSON.parse(v || "{}")
      setJsonError("")
    } catch (e: any) {
      setJsonError(e?.message || "Invalid JSON")
    }
  }

  const formatPayload = () => {
    try {
      const obj = JSON.parse(payloadText || "{}")
      const pretty = JSON.stringify(obj, null, 2)
      setPayloadText(pretty)
      setJsonError("")
    } catch (e: any) {
      setJsonError(e?.message || "Invalid JSON")
    }
  }

  const handleCopyOutput = async () => {
    try {
      await navigator.clipboard.writeText(invokeOutput)
      toast.success("Copied", {
        description: "Response copied to clipboard.",
      })
    } catch {
      toast.error("Copy failed", {
        description: "Could not copy to clipboard.",
      })
    }
  }

  const onInvokeClick = async () => {
    let parsed: unknown
    try {
      parsed = JSON.parse(payloadText || "{}")
      setJsonError("")
    } catch (e: any) {
      setJsonError(e?.message || "Invalid JSON")
      return
    }

    setPerFunctionPayload((prev) => ({ ...prev, [showInvokeDialog.fnId]: payloadText }))

    try {
      const res = await invokeMutation.mutateAsync({
        fnId: showInvokeDialog.fnId,
        payload: { event: parsed as any },
      })

      const printable = JSON.stringify((res as any)?.response ?? res, null, 2)
      setInvokeOutput(printable)

      toast.success("Invoke Succeeded", {
        description: `Function "${showInvokeDialog.fnName}" invoked successfully.`,
      })
    } catch (error: any) {
      const errPayload = {
        error: error?.message ?? "Unknown error",
        data: error,
      }
      setInvokeOutput(JSON.stringify(errPayload, null, 2))

      toast.error("Invoke Failed", {
        description: error?.message ?? "Unknown error",
      })
    }
  }

  // Hitung jumlah resource per tipe
  const resourceCounts = useMemo(() => {
    return {
      all: resources.length,
      vm: resources.filter(r => r.type === "vm").length,
      function: resources.filter(r => r.type === "function").length,
      container: resources.filter(r => r.type === "container").length,
    }
  }, [resources])

  const filteredResources = useMemo(() => {
    return resources.filter((resource) => {
      const matchesSearch = resource.name?.toLowerCase().includes(searchQuery.toLowerCase()) || false
      const matchesType = typeFilter === "all" || resource.type === typeFilter
      const matchesState = stateFilter === "all" || resource.state === stateFilter
      return matchesSearch && matchesType && matchesState
    })
  }, [resources, searchQuery, typeFilter, stateFilter])

  // Reset ke halaman pertama ketika filter berubah
  useEffect(() => {
    setCurrentPage(1)
  }, [searchQuery, typeFilter, stateFilter])

  // Pagination logic
  const totalPages = Math.ceil(filteredResources.length / itemsPerPage)
  const paginatedResources = useMemo(() => {
    const startIndex = (currentPage - 1) * itemsPerPage
    const endIndex = startIndex + itemsPerPage
    return filteredResources.slice(startIndex, endIndex)
  }, [filteredResources, currentPage, itemsPerPage])

  const goToPage = (page: number) => {
    setCurrentPage(Math.max(1, Math.min(page, totalPages)))
  }

  const [deleteDialog, setDeleteDialog] = useState<{ open: boolean; vmId: string; vmName: string }>({
    open: false,
    vmId: "",
    vmName: "",
  })

  const getTypeIcon = (type: string) => {
    switch (type) {
      case "vm":
        return <Server className="h-4 w-4" />
      case "function":
        return <Zap className="h-4 w-4" />
      case "container":
        return <Container className="h-4 w-4" />
      default:
        return null
    }
  }

  const getTypeBadge = (type: string) => {
    const colors = {
      vm: "bg-blue-100 text-blue-700 border-blue-200 dark:bg-input/50 dark:border-blue-500",
      function: "bg-purple-100 text-purple-700 border-purple-200 dark:bg-input/50 dark:border-purple-500",
      container: "bg-green-100 text-green-700 border-green-200 dark:bg-input/50 dark:border-green-500",
    }
    return (
      <Badge variant="outline" className={colors[type as keyof typeof colors]}>
        <span className="flex items-center gap-1">
          {getTypeIcon(type)}
          {type.toUpperCase()}
        </span>
      </Badge>
    )
  }

  const getResourceLink = (resource: UnifiedResource) => {
    switch (resource.type) {
      case "vm":
        return `/vms/${resource.id}`
      case "function":
        return `/functions/${resource.id}`
      case "container":
        return `/containers/${resource.id}`
      default:
        return "#"
    }
  }

  const vmStatePatch = useVmStatePatch()
  const deleteMutation = useDeleteVM()
  const handleAction = (id: string, action: "start" | "stop" | "resume" | "ctrl_alt_del" | "pause") => {
    vmStatePatch.mutate({ id, action })
  }

  const handleDelete = () => {
    if (deleteDialog.vmId && deleteDialog.vmName) {
      deleteMutation.mutate(deleteDialog.vmId, {
        onSuccess: () => {
          toast.success("Resource Deleted", {
            description: `${deleteDialog.vmName} has been deleted`,
          })
          setDeleteDialog({ open: false, vmId: "", vmName: "" })
        },
        onError: (error) => {
          toast.error("Delete Failed", {
            description: `Failed to delete ${deleteDialog.vmName}: ${error.message}`,
          })
        }
      })
    }
  }

  // Define tabs with badges
  const tabs: TabItem[] = useMemo(() => [
    {
      value: "all",
      label: "All",
      badge: (
        <Badge variant="secondary" className="bg-background text-foreground px-1.5 py-0 text-xs dark:text-primary dark:bg-card">
          {resourceCounts.all}
        </Badge>
      ),
    },
    {
      value: "vm",
      label: "VMs",
      icon: <Server className="h-4 w-4" />,
    },
    {
      value: "function",
      label: "Functions",
      icon: <Zap className="h-4 w-4" />,
    },
    {
      value: "container",
      label: "Containers",
      icon: <Container className="h-4 w-4" />,
    },
  ], [resourceCounts])

  // console.log('Resource: ', resources)

  return (
    <div className="space-y-4">
      {/* Type Filter Tabs */}
      <ReusableTabs
        tabs={tabs}
        value={typeFilter}
        onValueChange={setTypeFilter}
      />

      {/* Search and State Filter */}
      <div className="flex items-center gap-4">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            placeholder="Search resources..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-9 shadow-none"
          />
        </div>
        <Select value={stateFilter} onValueChange={setStateFilter}>
          <SelectTrigger className="w-40 shadow-none">
            <SelectValue placeholder="State" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All States</SelectItem>
            <SelectItem value="running">Running</SelectItem>
            <SelectItem value="stopped">Stopped</SelectItem>
            <SelectItem value="idle">Idle</SelectItem>
            <SelectItem value="error">Error</SelectItem>
          </SelectContent>
        </Select>
      </div>

      <div className="rounded-lg border border-border">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Name</TableHead>
              <TableHead>Type</TableHead>
              <TableHead>State</TableHead>
              <TableHead>Metrics</TableHead>
              <TableHead>Owner</TableHead>
              <TableHead className="text-right">Actions</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {paginatedResources.length === 0 ? (
              <TableRow>
                <TableCell colSpan={6} className="text-center py-8 text-muted-foreground">
                  No resources found
                </TableCell>
              </TableRow>
            ) : (
              paginatedResources.map((resource) => (
                <TableRow key={`${resource.type}-${resource.id}`}>
                  <TableCell>
                    <Link href={getResourceLink(resource)} className="font-medium hover:underline">
                      {resource.name || 'Unknown'}
                    </Link>
                  </TableCell>
                  <TableCell>{getTypeBadge(resource.type)}</TableCell>
                  <TableCell>
                    <StatusBadge status={resource.state as any} />
                  </TableCell>
                  <TableCell>
                    <div className="text-sm text-muted-foreground flex gap-1 flex-wrap flex-col">
                      {resource.metrics?.cpu !== undefined && resource.metrics.cpu > 0 && (
                        <span className="text-primary font-bold">CPU: {resource.metrics.cpu} vCPU</span>
                      )}
                      {resource.metrics?.memory !== undefined && resource.metrics.memory > 0 && (
                        <span>
                          Memory: {resource.type === "vm"
                            ? `${resource.metrics.memory} MiB`
                            : `${resource.metrics.memory} MB`}
                        </span>
                      )}
                      {resource.metrics?.lastInvoked && (
                        <span>Last: {dateFormat.formatRelative(resource.metrics.lastInvoked)}</span>
                      )}
                      {(!resource.metrics?.cpu || resource.metrics.cpu === 0) &&
                        (!resource.metrics?.memory || resource.metrics.memory === 0) &&
                        !resource.metrics?.lastInvoked && (
                          <span className="text-muted-foreground/50">—</span>
                        )}
                    </div>
                  </TableCell>
                  <TableCell className="text-sm text-muted-foreground">
                    {(resource as any).created_by_user_id ? (
                      (resource as any).created_by_user_id === user?.id ? (
                        <span className="text-primary font-medium">You</span>
                      ) : (
                        <span className="text-muted-foreground">Other User</span>
                      )
                    ) : (
                      <span className="text-muted-foreground italic">System</span>
                    )}
                  </TableCell>
                  <TableCell className="text-right">
                    {!canModifyResource(user, (resource as any).created_by_user_id) &&
                      !canDeleteResource(user, (resource as any).created_by_user_id) ? (
                      <span className="text-muted-foreground text-sm">Not permitted</span>
                    ) : (
                      <div className="flex justify-end gap-1">
                        {/* Function-specific actions */}
                        {resource.type === "function" && (
                          <>
                            <Button
                              variant="ghost"
                              size="icon"
                              title="Invoke"
                              onClick={() => openInvokeDialog({ id: resource.id, name: resource.name || 'Unknown' })}
                            >
                              <Play className="h-4 w-4" />
                            </Button>
                            <Button variant="ghost" size="icon" title="Logs" asChild>
                              <Link href={`/functions/${resource.id}?tab=logs`}>
                                <FileText className="h-4 w-4" />
                              </Link>
                            </Button>
                          </>
                        )}

                        {/* VM-specific actions */}
                        {resource.type === "vm" && canModifyResource(user, (resource as any).created_by_user_id) && (
                          <>
                            {resource.state === "stopped" && (
                              <Button variant="ghost" size="icon" onClick={() => handleAction(resource.id, "start")}>
                                <Play className="h-4 w-4 " />
                              </Button>
                            )}
                            {resource.state === "running" && (
                              <>
                                <Button variant="ghost" size="icon" onClick={() => handleAction(resource.id, "pause")}>
                                  <Pause className="h-4 w-4" />
                                </Button>
                                <Button variant="ghost" size="icon" onClick={() => handleAction(resource.id, "stop")}>
                                  <Square className="h-4 w-4" />
                                </Button>
                              </>
                            )}
                            {resource.state === "paused" && (
                              <Button variant="ghost" size="icon" onClick={() => handleAction(resource.id, "resume")}>
                                <Play className="h-4 w-4" />
                              </Button>
                            )}
                          </>
                        )}

                        {/* Delete action for all types */}
                        {canDeleteResource(user, (resource as any).created_by_user_id) && (
                          <>
                            {resource.type === "vm" && resource.state === "running" ? (
                              <TooltipProvider>
                                <Tooltip>
                                  <TooltipTrigger asChild>
                                    <Button variant="ghost" size="icon" disabled>
                                      <Trash2 className="h-4 w-4" />
                                    </Button>
                                  </TooltipTrigger>
                                  <TooltipContent>
                                    <p>Cannot delete running VM. Stop the VM first.</p>
                                  </TooltipContent>
                                </Tooltip>
                              </TooltipProvider>
                            ) : (
                              <Button variant="ghost" size="icon" onClick={() => setDeleteDialog({ open: true, vmId: resource.id, vmName: resource.name || 'Unknown' })}>
                                <Trash2 className="h-4 w-4" />
                              </Button>
                            )}
                          </>
                        )}
                      </div>
                    )}
                  </TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </div>

      {/* Pagination */}
      {totalPages > 1 && (
        <div className="flex items-center justify-between px-2">
          <div className="text-sm text-muted-foreground">
            Showing {((currentPage - 1) * itemsPerPage) + 1} to {Math.min(currentPage * itemsPerPage, filteredResources.length)} of {filteredResources.length} resources
          </div>
          <div className="flex items-center gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={() => goToPage(1)}
              disabled={currentPage === 1}
              className="h-8 px-3"
            >
              First
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => goToPage(currentPage - 1)}
              disabled={currentPage === 1}
              className="h-8 px-3"
            >
              Prev
            </Button>

            {/* Page Numbers */}
            <div className="flex items-center gap-1">
              {Array.from({ length: totalPages }, (_, i) => i + 1)
                .filter(page => {
                  // Show first page, last page, current page, and pages around current
                  if (page === 1 || page === totalPages) return true
                  if (Math.abs(page - currentPage) <= 1) return true
                  return false
                })
                .map((page, idx, arr) => {
                  // Add ellipsis
                  const prevPage = arr[idx - 1]
                  const showEllipsis = prevPage && page - prevPage > 1

                  return (
                    <div key={page} className="flex items-center gap-1">
                      {showEllipsis && <span className="px-1 text-muted-foreground">...</span>}
                      <Button
                        variant={currentPage === page ? "default" : "outline"}
                        size="sm"
                        onClick={() => goToPage(page)}
                        className="h-8 w-8 p-0"
                      >
                        {page}
                      </Button>
                    </div>
                  )
                })}
            </div>

            <Button
              variant="outline"
              size="sm"
              onClick={() => goToPage(currentPage + 1)}
              disabled={currentPage === totalPages}
              className="h-8 px-3"
            >
              Next
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => goToPage(totalPages)}
              disabled={currentPage === totalPages}
              className="h-8 px-3"
            >
              Last
            </Button>
          </div>
        </div>
      )}

      <ConfirmDialog
        open={deleteDialog.open}
        onOpenChange={(open) => setDeleteDialog({ ...deleteDialog, open })}
        title="Delete VM"
        description={`Are you sure you want to delete ${deleteDialog.vmName}? This action cannot be undone.`}
        confirmText="Delete"
        onConfirm={() => handleDelete()}
        variant="destructive"
      />

      {/* Invoke Function Dialog */}
      <Dialog open={showInvokeDialog.open} onOpenChange={(open) => setShowInvokeDialog({ ...showInvokeDialog, open })}>
        <DialogContent className="space-y-3 w-[min(92vw,900px)] sm:max-w-[900px] overflow-hidden">
          <DialogHeader>
            <DialogTitle>Invoke Function</DialogTitle>
            <DialogDescription>
              {showInvokeDialog.fnName ? (
                <span>
                  Target: <span className="font-medium">{showInvokeDialog.fnName}</span>
                </span>
              ) : (
                "Provide JSON payload to invoke this function."
              )}
            </DialogDescription>
          </DialogHeader>

          {/* Editor */}
          <div className="border rounded-lg overflow-hidden w-full max-w-full min-w-0">
            <Editor
              className="!w-full"
              height="180px"
              language="json"
              value={payloadText}
              onChange={onPayloadChange}
              theme={theme === "dark" ? "vs-dark" : "light"}
              options={{
                minimap: { enabled: false },
                fontSize: 12,
                lineNumbers: "on",
                scrollBeyondLastLine: false,
                automaticLayout: true,
                wordWrap: "on",
                wordWrapColumn: 100,
                wrappingIndent: "same",
                scrollBeyondLastColumn: 0,
              }}
            />
          </div>

          {/* JSON status */}
          {jsonError ? (
            <div className="flex items-start gap-2 text-red-600 text-xs min-w-0">
              <AlertTriangle className="h-4 w-4 mt-0.5" />
              <span className="break-words">{jsonError}</span>
            </div>
          ) : (
            <div className="flex items-center gap-2 text-emerald-600 text-xs min-w-0">
              <Check className="h-4 w-4" />
              <span>JSON valid</span>
            </div>
          )}

          {/* Response panel */}
          <div className="space-y-2 min-w-0">
            <div className="flex items-center justify-between min-w-0">
              <span className="text-sm font-medium">Response</span>
              <div className="flex gap-2">
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={handleCopyOutput}
                  disabled={!invokeOutput}
                  title="Copy response"
                >
                  <Clipboard className="mr-2 h-4 w-4" />
                  Copy
                </Button>
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={() => setInvokeOutput("")}
                  disabled={!invokeOutput}
                  title="Clear response"
                >
                  Clear
                </Button>
              </div>
            </div>

            <div className="border rounded-lg bg-muted/30 max-h-56 overflow-auto w-full max-w-full min-w-0">
              <pre className="p-3 text-xs leading-relaxed whitespace-pre-wrap break-words break-all overflow-x-hidden">
                {invokeOutput ? invokeOutput : "// Response akan ditampilkan di sini setelah Invoke."}
              </pre>
            </div>
          </div>

          <DialogFooter className="gap-2 sm:gap-2 flex-wrap">
            <Button type="button" variant="outline" onClick={formatPayload} disabled={invokeMutation.isPending}>
              Format JSON
            </Button>
            <Button type="button" variant="outline" onClick={closeInvokeDialog} disabled={invokeMutation.isPending}>
              Cancel
            </Button>
            <Button
              onClick={onInvokeClick}
              disabled={!!jsonError || !showInvokeDialog.fnId || invokeMutation.isPending}
            >
              {invokeMutation.isPending ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  Invoking...
                </>
              ) : (
                <>
                  <Play className="mr-2 h-4 w-4" />
                  Invoke
                </>
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}
