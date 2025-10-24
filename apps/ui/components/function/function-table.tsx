"use client"

import { useState } from "react"
import Link from "next/link"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Badge } from "@/components/ui/badge"
import { FileText, Play, Trash2, Search } from "lucide-react"
import { formatRelativeTime } from "@/lib/utils/format"
import type { Function } from "@/lib/types"

interface FunctionTableProps {
  functions: Function[]
}

export function FunctionTable({ functions }: FunctionTableProps) {
  const [searchQuery, setSearchQuery] = useState("")
  const [runtimeFilter, setRuntimeFilter] = useState<string>("all")

  const filteredFunctions = functions.filter((fn) => {
    const matchesSearch = fn.name.toLowerCase().includes(searchQuery.toLowerCase())
    const matchesRuntime = runtimeFilter === "all" || fn.runtime === runtimeFilter
    return matchesSearch && matchesRuntime
  })

  const getRuntimeBadge = (runtime: string) => {
    const colors = {
      node: "bg-green-100 text-green-700 border-green-200",
      python: "bg-blue-100 text-blue-700 border-blue-200",
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

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-4">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            placeholder="Search functions..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-9"
          />
        </div>
        <Select value={runtimeFilter} onValueChange={setRuntimeFilter}>
          <SelectTrigger className="w-40">
            <SelectValue placeholder="Runtime" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All Runtimes</SelectItem>
            <SelectItem value="node">Node.js</SelectItem>
            <SelectItem value="python">Python</SelectItem>
          </SelectContent>
        </Select>
      </div>

      <div className="rounded-lg border border-border">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Name</TableHead>
              <TableHead>Runtime</TableHead>
              <TableHead>Last Invoked</TableHead>
              <TableHead>24h Invocations</TableHead>
              <TableHead>Avg Duration</TableHead>
              <TableHead>Memory</TableHead>
              <TableHead className="text-right">Actions</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {filteredFunctions.length === 0 ? (
              <TableRow>
                <TableCell colSpan={7} className="text-center py-8 text-muted-foreground">
                  No functions found
                </TableCell>
              </TableRow>
            ) : (
              filteredFunctions.map((fn) => (
                <TableRow key={fn.id}>
                  <TableCell>
                    <Link href={`/functions/${fn.id}`} className="font-medium hover:underline">
                      {fn.name}
                    </Link>
                  </TableCell>
                  <TableCell>{getRuntimeBadge(fn.runtime)}</TableCell>
                  <TableCell className="text-sm text-muted-foreground">
                    {fn.last_invoked_at ? formatRelativeTime(fn.last_invoked_at) : "Never"}
                  </TableCell>
                  <TableCell className="text-sm">{fn.invocation_count_24h?.toLocaleString('en-US') || 0}</TableCell>
                  <TableCell className="text-sm">{fn.avg_duration_ms ? `${fn.avg_duration_ms}ms` : "N/A"}</TableCell>
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
                      <Button variant="ghost" size="icon" title="Delete">
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
    </div>
  )
}
