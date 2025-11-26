"use client"

import { useMemo, useState } from "react"
import Link from "next/link"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Badge } from "@/components/ui/badge"
import { StatusBadge } from "@/components/shared/status-badge"
import { FileText, Play, Trash2, Search, Check, AlertTriangle, Loader2, Clipboard } from "lucide-react"
import {
  Pagination,
  PaginationContent,
  PaginationItem,
  PaginationLink,
  PaginationNext,
  PaginationPrevious,
} from "@/components/ui/pagination"
import type { Function } from "@/lib/types"
import { useDateFormat } from "@/lib/hooks/use-date-format"
import { ConfirmDialog } from "@/components/shared/confirm-dialog"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { useDeleteFunction, useInvokeFunction } from "@/lib/queries"
import { toast } from "sonner"
import dynamic from "next/dynamic"
import { useAuthStore, canDeleteResource } from "@/lib/auth/store"
import { useTheme } from "next-themes"

const Editor = dynamic(() => import("@monaco-editor/react"), { ssr: false })

interface FunctionTableProps {
  functions: Function[]
}

const ITEMS_PER_PAGE = 10
const DEFAULT_PAYLOAD = `{
// Test event payload
}`

export function FunctionTable({ functions }: FunctionTableProps) {
  const dateFormat = useDateFormat()
  const { user } = useAuthStore()
  const { theme } = useTheme()

  // ---- filters/pagination ----
  const [searchQuery, setSearchQuery] = useState("")
  const [runtimeFilter, setRuntimeFilter] = useState<string>("all")
  const [stateFilter, setStateFilter] = useState<string>("all")
  const [currentPage, setCurrentPage] = useState(1)

  const filteredFunctions = useMemo(() => {
    return functions.filter((fn) => {
      const matchesSearch = fn.name.toLowerCase().includes(searchQuery.toLowerCase())
      const matchesRuntime = runtimeFilter === "all" || fn.runtime === runtimeFilter
      const matchesState = stateFilter === "all" || fn.state === stateFilter

      // Filter by ownership for non-admin/non-viewer users
      const canView = user?.role === "admin" || user?.role === "viewer" ||
        !(fn as any).created_by_user_id ||
        (fn as any).created_by_user_id === user?.id

      return matchesSearch && matchesRuntime && matchesState && canView
    })
  }, [functions, searchQuery, runtimeFilter, stateFilter, user])

  const totalPages = Math.ceil(filteredFunctions.length / ITEMS_PER_PAGE)
  const startIndex = (currentPage - 1) * ITEMS_PER_PAGE
  const paginatedFunctions = filteredFunctions.slice(startIndex, startIndex + ITEMS_PER_PAGE)

  // ---- delete ----
  const deleteMutation = useDeleteFunction()
  const [deleteDialog, setDeleteDialog] = useState<{ open: boolean; fnId: string; fnName: string }>({
    open: false,
    fnId: "",
    fnName: "",
  })
  const handleDelete = () => {
    if (deleteDialog.fnId && deleteDialog.fnName) {
      deleteMutation.mutate(deleteDialog.fnId, {
        onSuccess: () => {
          toast.success("Function Deleted", {
            description: `${deleteDialog.fnName} has been deleted successfully`,
          })
          setDeleteDialog({ open: false, fnId: "", fnName: "" })
        },
        onError: (error: any) => {
          toast.error("Delete Failed", {
            description: `Failed to delete ${deleteDialog.fnName}: ${error?.message ?? "Unknown error"}`,
          })
        },
      })
    }
  }

  // ---- invoke ----
  const invokeMutation = useInvokeFunction()
  const [perFunctionPayload, setPerFunctionPayload] = useState<Record<string, string>>({})
  const [showDialog, setShowDialog] = useState<{ open: boolean; fnId: string; fnName: string }>({
    open: false,
    fnId: "",
    fnName: "",
  })
  const [payloadText, setPayloadText] = useState(DEFAULT_PAYLOAD)
  const [jsonError, setJsonError] = useState<string>("")
  const [invokeOutput, setInvokeOutput] = useState<string>("")

  // buka dialog + prefill payload + reset output
  const openInvokeDialog = ({ id, name }: { id: string; name: string }) => {
    const cached = perFunctionPayload[id] ?? DEFAULT_PAYLOAD
    setPayloadText(cached)
    setJsonError("")
    setInvokeOutput("")
    setShowDialog({ open: true, fnId: id, fnName: name })
  }

  const closeInvokeDialog = () => setShowDialog((s) => ({ ...s, open: false }))

  // editor onChange + validasi realtime
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

  // format JSON (biar rapi)
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

  // copy response helper
  const handleCopyOutput = async () => {
    try {
      await navigator.clipboard.writeText(invokeOutput)
      toast.success("Copied", {
        description: "Response copied to clipboard",
      })
    } catch {
      toast.error("Copy Failed", {
        description: "Could not copy to clipboard",
      })
    }
  }

  // Tombol Invoke — memanggil API via useInvokeFunction
  const onInvokeClick = async () => {
    // validasi terakhir
    let parsed: unknown
    try {
      parsed = JSON.parse(payloadText || "{}")
      setJsonError("")
    } catch (e: any) {
      setJsonError(e?.message || "Invalid JSON")
      return
    }

    // simpan payload per-function
    setPerFunctionPayload((prev) => ({ ...prev, [showDialog.fnId]: payloadText }))

    try {
      const res = await invokeMutation.mutateAsync({
        fnId: showDialog.fnId,
        payload: { event: parsed }, // struktur yang kamu pakai
      })

      // tampilkan res.response (fallback ke res jika tidak ada)
      const printable = JSON.stringify((res as any)?.response ?? res, null, 2)
      setInvokeOutput(printable)

      toast.success("Invoke Succeeded", {
        description: `Function "${showDialog.fnName}" invoked successfully`,
      })
      // closeInvokeDialog() // kalau mau menutup otomatis
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

  return (
    <div className="space-y-4">
      {/* Filters */}
      <div className="flex items-center gap-4">
        <div className="relative flex-1 min-w-0">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            placeholder="Search functions..."
            value={searchQuery}
            onChange={(e) => {
              setSearchQuery(e.target.value)
              setCurrentPage(1)
            }}
            className="pl-9"
          />
        </div>
        <Select
          value={runtimeFilter}
          onValueChange={(value) => {
            setRuntimeFilter(value)
            setCurrentPage(1)
          }}
        >
          <SelectTrigger className="w-40">
            <SelectValue placeholder="Runtime" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All Runtimes</SelectItem>
            <SelectItem value="node">Node.js</SelectItem>
            <SelectItem value="python">Python</SelectItem>
            <SelectItem value="deno">Deno</SelectItem>
            <SelectItem value="bun">Bun</SelectItem>
          </SelectContent>
        </Select>
        <Select
          value={stateFilter}
          onValueChange={(value) => {
            setStateFilter(value)
            setCurrentPage(1)
          }}
        >
          <SelectTrigger className="w-40">
            <SelectValue placeholder="State" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All States</SelectItem>
            <SelectItem value="ready">Ready</SelectItem>
            <SelectItem value="creating">Creating</SelectItem>
            <SelectItem value="deploying">Deploying</SelectItem>
            <SelectItem value="error">Error</SelectItem>
          </SelectContent>
        </Select>
      </div>

      {/* Table */}
      <div className="rounded-lg border border-border">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Name</TableHead>
              <TableHead>Runtime</TableHead>
              <TableHead>State</TableHead>
              <TableHead>Last Invoked</TableHead>
              <TableHead>24h Invocations</TableHead>
              <TableHead>Guest IP</TableHead>
              <TableHead>CPU</TableHead>
              <TableHead>Memory</TableHead>
              <TableHead>Owner</TableHead>
              <TableHead className="text-right">Actions</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {paginatedFunctions.length === 0 ? (
              <TableRow>
                <TableCell colSpan={10} className="text-center py-8 text-muted-foreground">
                  No functions found
                </TableCell>
              </TableRow>
            ) : (
              paginatedFunctions.map((fn) => (
                <TableRow key={fn.id}>
                  <TableCell>
                    <Link href={`/functions/${fn.id}`} className="font-medium hover:underline">
                      {fn.name}
                    </Link>
                  </TableCell>
                  <TableCell>
                    <StatusBadge status={fn.runtime as any} />
                  </TableCell>
                  <TableCell>
                    <StatusBadge status={fn.state as any} />
                  </TableCell>
                  <TableCell className="text-sm text-muted-foreground">
                    {fn.last_invoked_at ? dateFormat.formatRelative(fn.last_invoked_at) : "Never"}
                  </TableCell>
                  <TableCell className="text-sm">{fn.invocation_count_24h?.toLocaleString("en-US") || 0}</TableCell>
                  <TableCell className="text-sm font-mono">
                    {fn.guest_ip ? (fn.port ? `${fn.guest_ip}:${fn.port}` : fn.guest_ip) : (
                      <span className="text-muted-foreground">N/A</span>
                    )}
                  </TableCell>
                  <TableCell className="text-sm">{fn.vcpu ? `${fn.vcpu} vCPU` : "N/A"}</TableCell>
                  <TableCell className="text-sm">{fn.memory_mb} MB</TableCell>
                  <TableCell className="text-sm text-muted-foreground">
                    {(fn as any).created_by_user_id ? (
                      (fn as any).created_by_user_id === user?.id ? (
                        <span className="text-primary font-medium">You</span>
                      ) : (
                        <span className="text-muted-foreground">Other User</span>
                      )
                    ) : (
                      <span className="text-muted-foreground italic">System</span>
                    )}
                  </TableCell>
                  <TableCell className="text-right">
                    <div className="flex justify-end gap-1">
                      <Button variant="ghost" size="icon" title="Invoke" asChild>
                        <Button variant="outline" onClick={() => openInvokeDialog({ id: fn.id, name: fn.name })}>
                          <Play className="h-4 w-4" />
                        </Button>
                      </Button>
                      <Button variant="ghost" size="icon" title="Logs" asChild>
                        <Link href={`/functions/${fn.id}?tab=logs`}>
                          <FileText className="h-4 w-4" />
                        </Link>
                      </Button>
                      {canDeleteResource(user, (fn as any).created_by_user_id) && (
                        <Button
                          variant="ghost"
                          size="icon"
                          title="Delete"
                          onClick={() => setDeleteDialog({ open: true, fnId: fn.id, fnName: fn.name })}
                        >
                          <Trash2 className="h-4 w-4" />
                        </Button>
                      )}
                    </div>
                  </TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </div>

      {/* Pagination */}
      {totalPages > 1 && (
        <Pagination>
          <PaginationContent>
            <PaginationItem>
              <PaginationPrevious
                size={undefined}
                onClick={() => setCurrentPage((p) => Math.max(1, p - 1))}
                className={currentPage === 1 ? "pointer-events-none opacity-50" : "cursor-pointer"}
              />
            </PaginationItem>
            {Array.from({ length: totalPages }, (_, i) => i + 1).map((page) => (
              <PaginationItem key={page}>
                <PaginationLink
                  size={undefined}
                  onClick={() => setCurrentPage(page)}
                  isActive={currentPage === page}
                  className="cursor-pointer"
                >
                  {page}
                </PaginationLink>
              </PaginationItem>
            ))}
            <PaginationItem>
              <PaginationNext
                size={undefined}
                onClick={() => setCurrentPage((p) => Math.min(totalPages, p + 1))}
                className={currentPage === totalPages ? "pointer-events-none opacity-50" : "cursor-pointer"}
              />
            </PaginationItem>
          </PaginationContent>
        </Pagination>
      )}

      {/* Delete dialog */}
      <ConfirmDialog
        open={deleteDialog.open}
        onOpenChange={(open) => setDeleteDialog({ ...deleteDialog, open })}
        title="Delete Function"
        description={`Are you sure you want to delete ${deleteDialog.fnName}? This action cannot be undone.`}
        confirmText="Delete"
        onConfirm={handleDelete}
        variant="destructive"
      />

      {/* Invoke dialog */}
      <Dialog open={showDialog.open} onOpenChange={(open) => setShowDialog({ ...showDialog, open })}>
        <DialogContent
          className="space-y-3 w-[min(92vw,900px)] sm:max-w-[900px] overflow-hidden"
        >
          <DialogHeader>
            <DialogTitle>Invoke Function</DialogTitle>
            <DialogDescription>
              {showDialog.fnName ? (
                <span>
                  Target: <Link href={`/functions/${showDialog.fnId}?tab=details`}>
                    <span className="font-medium hover:underline text-primary">{showDialog.fnName}</span>
                  </Link>
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
              disabled={!!jsonError || !showDialog.fnId || invokeMutation.isPending}
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
