"use client"

import { useState } from "react"
import Link from "next/link"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Badge } from "@/components/ui/badge"
import { FileText, Play, Trash2, Search } from "lucide-react"
import {
  Pagination,
  PaginationContent,
  PaginationItem,
  PaginationLink,
  PaginationNext,
  PaginationPrevious,
} from "@/components/ui/pagination"
import { formatRelativeTime } from "@/lib/utils/format"
import type { Function } from "@/lib/types"
import { ConfirmDialog } from "@/components/shared/confirm-dialog"
import { useDeleteFunction } from "@/lib/queries"
import { useToast } from "@/hooks/use-toast"

interface FunctionTableProps {
  functions: Function[]
}

const ITEMS_PER_PAGE = 10

export function FunctionTable({ functions }: FunctionTableProps) {
  const [searchQuery, setSearchQuery] = useState("")
  const [runtimeFilter, setRuntimeFilter] = useState<string>("all")
  const [stateFilter, setStateFilter] = useState<string>("all")
  const [currentPage, setCurrentPage] = useState(1)

  const filteredFunctions = functions.filter((fn) => {
    const matchesSearch = fn.name.toLowerCase().includes(searchQuery.toLowerCase())
    const matchesRuntime = runtimeFilter === "all" || fn.runtime === runtimeFilter
    const matchesState = stateFilter === "all" || fn.state === stateFilter
    return matchesSearch && matchesRuntime && matchesState
  })

  const totalPages = Math.ceil(filteredFunctions.length / ITEMS_PER_PAGE)
  const startIndex = (currentPage - 1) * ITEMS_PER_PAGE
  const paginatedFunctions = filteredFunctions.slice(
    startIndex,
    startIndex + ITEMS_PER_PAGE
  )

  const [deleteDialog, setDeleteDialog] = useState<{ open: boolean; fnId: string; fnName: string }>({
    open: false,
    fnId: "",
    fnName: "",
  })

  const { toast } = useToast()
  const deleteMutation = useDeleteFunction()

  const handleDelete = () => {
    if (deleteDialog.fnId && deleteDialog.fnName) {
      deleteMutation.mutate(deleteDialog.fnId, {
        onSuccess: () => {
          toast({
            title: "Function Deleted",
            description: `${deleteDialog.fnName} has been deleted`,
            variant: "destructive",
          })
          setDeleteDialog({ open: false, fnId: "", fnName: "" })
        },
        onError: (error) => {
          toast({
            title: "Delete Failed",
            description: `Failed to delete ${deleteDialog.fnName}: ${error.message}`,
            variant: "destructive",
          })
        }
      })
    }
  }

  const getRuntimeBadge = (runtime: string) => {
    const colors = {
      node: "bg-[#6cc24a] text-black border-[#44883e]",
      python: "bg-[#ffde57] text-[#4584b6] border-[#4584b6]",
    }
    const labels = {
      node: "Node.js",
      python: "Python",
    }
    return (
      <Badge variant="outline" className={colors[runtime as keyof typeof colors]}>
        {labels[runtime as keyof typeof labels]}
      </Badge>
    )
  }

  const getStateBagde = (state: string) => {
    const colors = {
      creating: "bg-yellow-100 text-yellow-700 border-yellow-200",
      deploying: "bg-blue-100 text-blue-700 border-blue-200",
      ready: "bg-green-100 text-green-700 border-green-200",
      error: "bg-red-100 text-red-700 border-red-200",
      booting: "bg-gray-100 text-gray-700 border-gray-200",
    }

    const labels = {
      creating: "Creating",
      deploying: "Deploying",
      ready: "Ready",
      error: "Error",
      booting: "Booting",
    }

    return (
      <Badge variant="outline" className={colors[state as keyof typeof colors]}>
        {labels[state as keyof typeof labels]}
      </Badge>
    )
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-4">
        <div className="relative flex-1">
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
              <TableHead>Memory</TableHead>
              <TableHead className="text-right">Actions</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {paginatedFunctions.length === 0 ? (
              <TableRow>
                <TableCell colSpan={8} className="text-center py-8 text-muted-foreground">
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
                  <TableCell>{getRuntimeBadge(fn.runtime)}</TableCell>
                  <TableCell>{getStateBagde(fn.state)}</TableCell>
                  <TableCell className="text-sm text-muted-foreground">
                    {fn.last_invoked_at ? formatRelativeTime(fn.last_invoked_at) : "Never"}
                  </TableCell>
                  <TableCell className="text-sm">{fn.invocation_count_24h?.toLocaleString('en-US') || 0}</TableCell>
                  <TableCell className="text-sm">192.128.1.1</TableCell>
                  <TableCell className="text-sm">{fn.memory_mb} MB</TableCell>
                  <TableCell className="text-right">
                    <div className="flex justify-end gap-1">
                      <Button variant="ghost" size="icon" title="Invoke" asChild>
                        <Link href={`/functions/${fn.id}`}>
                          <Play className="h-4 w-4" />
                        </Link>
                      </Button>
                      <Button variant="ghost" size="icon" title="Logs" asChild>
                        <Link href={`/functions/${fn.id}/logs`}>
                          <FileText className="h-4 w-4" />
                        </Link>
                      </Button>
                      <Button variant="ghost" size="icon" title="Delete" onClick={() => setDeleteDialog({ open: true, fnId: fn.id, fnName: fn.name })}>
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    </div>
                  </TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </div>

      {totalPages > 1 && (
        <Pagination>
          <PaginationContent>
            <PaginationItem>
              <PaginationPrevious
                onClick={() => setCurrentPage((p) => Math.max(1, p - 1))}
                className={currentPage === 1 ? "pointer-events-none opacity-50" : "cursor-pointer"} size={undefined} />
            </PaginationItem>
            {Array.from({ length: totalPages }, (_, i) => i + 1).map((page) => (
              <PaginationItem key={page}>
                <PaginationLink
                  onClick={() => setCurrentPage(page)}
                  isActive={currentPage === page}
                  className="cursor-pointer"
                  size={undefined}
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

      <ConfirmDialog
        open={deleteDialog.open}
        onOpenChange={(open) => setDeleteDialog({ ...deleteDialog, open })}
        title="Delete Function"
        description={`Are you sure you want to delete ${deleteDialog.fnName}? This action cannot be undone.`}
        confirmText="Delete"
        onConfirm={() => handleDelete()}
        variant="destructive"
      />
    </div>
  )
}
