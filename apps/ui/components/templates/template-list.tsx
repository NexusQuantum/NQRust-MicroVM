"use client"

import { useState } from "react"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Play, Edit, Copy, Trash2, Search, Server, Cpu, HardDrive } from "lucide-react"
import { formatRelativeTime } from "@/lib/utils/format"
import type { Template } from "@/lib/types"

interface TemplateListProps {
  templates: Template[]
}

export function TemplateList({ templates }: TemplateListProps) {
  const [searchQuery, setSearchQuery] = useState("")

  const filteredTemplates = templates.filter((template) => {
    return (
      template.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      template.description?.toLowerCase().includes(searchQuery.toLowerCase())
    )
  })

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-4">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            placeholder="Search templates..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-9"
          />
        </div>
      </div>

      {filteredTemplates.length === 0 && searchQuery === "" ? (
        <div className="flex flex-col items-center justify-center rounded-lg border-2 border-dashed border-border bg-muted/20 py-16">
          <div className="mb-4 rounded-full bg-muted p-4">
            <Server className="h-8 w-8 text-muted-foreground" />
          </div>
          <h3 className="mb-2 text-lg font-semibold">No templates yet</h3>
          <p className="mb-4 text-sm text-muted-foreground">Create your first VM template to get started</p>
          <Button>
            <Play className="mr-2 h-4 w-4" />
            Create Template
          </Button>
        </div>
      ) : (
        <div className="rounded-lg border border-border">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Name</TableHead>
                <TableHead>Description</TableHead>
                <TableHead>vCPU</TableHead>
                <TableHead>Memory</TableHead>
                <TableHead>Usage</TableHead>
                <TableHead>Created</TableHead>
                <TableHead className="text-right">Actions</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {filteredTemplates.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={7} className="text-center py-8 text-muted-foreground">
                    No templates found
                  </TableCell>
                </TableRow>
              ) : (
                filteredTemplates.map((template) => (
                  <TableRow key={template.id}>
                    <TableCell>
                      <div className="flex items-center gap-3">
                        <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-gradient-to-br from-blue-500/10 to-blue-600/10">
                          <Server className="h-5 w-5 text-blue-600" />
                        </div>
                        <span className="font-medium">{template.name}</span>
                      </div>
                    </TableCell>
                    <TableCell className="text-sm text-muted-foreground max-w-md truncate">
                      {template.description || "—"}
                    </TableCell>
                    <TableCell className="text-sm">
                      <div className="flex items-center gap-1">
                        <Cpu className="h-3 w-3 text-muted-foreground" />
                        {template.vcpu}
                      </div>
                    </TableCell>
                    <TableCell className="text-sm">
                      <div className="flex items-center gap-1">
                        <HardDrive className="h-3 w-3 text-muted-foreground" />
                        {template.mem_mib} MiB
                      </div>
                    </TableCell>
                    <TableCell className="text-sm">{template.usage_count || 0} VMs</TableCell>
                    <TableCell className="text-sm text-muted-foreground">
                      {formatRelativeTime(template.created_at)}
                    </TableCell>
                    <TableCell className="text-right">
                      <div className="flex justify-end gap-1">
                        <Button variant="default" size="sm">
                          <Play className="mr-2 h-4 w-4" />
                          Deploy
                        </Button>
                        <Button variant="ghost" size="icon" title="Edit">
                          <Edit className="h-4 w-4" />
                        </Button>
                        <Button variant="ghost" size="icon" title="Clone">
                          <Copy className="h-4 w-4" />
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
      )}
    </div>
  )
}
