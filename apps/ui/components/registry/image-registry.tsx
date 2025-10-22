"use client"

import { useState } from "react"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Badge } from "@/components/ui/badge"
import { Download, Copy, Trash2, Search, HardDrive, Database } from "lucide-react"
import { formatBytes, formatRelativeTime } from "@/lib/utils/format"
import type { Image } from "@/lib/types"

interface ImageRegistryProps {
  images: Image[]
}

export function ImageRegistry({ images }: ImageRegistryProps) {
  const [searchQuery, setSearchQuery] = useState("")
  const [typeFilter, setTypeFilter] = useState<string>("all")

  const filteredImages = images.filter((image) => {
    const matchesSearch = image.name.toLowerCase().includes(searchQuery.toLowerCase())
    const matchesType = typeFilter === "all" || image.kind === typeFilter
    return matchesSearch && matchesType
  })

  const getTypeBadge = (kind: string) => {
    const colors = {
      kernel: "bg-blue-100 text-blue-700 border-blue-200",
      rootfs: "bg-green-100 text-green-700 border-green-200",
    }
    return (
      <Badge variant="outline" className={colors[kind as keyof typeof colors]}>
        {kind.toUpperCase()}
      </Badge>
    )
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-4">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            placeholder="Search images..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-9 shadow-none"
          />
        </div>
        <Select value={typeFilter} onValueChange={setTypeFilter}>
          <SelectTrigger className="w-40 shadow-none">
            <SelectValue placeholder="Type" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All Types</SelectItem>
            <SelectItem value="kernel">Kernel</SelectItem>
            <SelectItem value="rootfs">Rootfs</SelectItem>
          </SelectContent>
        </Select>
      </div>

      {filteredImages.length === 0 && searchQuery === "" ? (
        <div className="flex flex-col items-center justify-center rounded-lg border-2 border-dashed border-border bg-muted/20 py-16">
          <div className="mb-4 rounded-full bg-muted p-4">
            <Database className="h-8 w-8 text-muted-foreground" />
          </div>
          <h3 className="mb-2 text-lg font-semibold">No images yet</h3>
          <p className="mb-4 text-sm text-muted-foreground">Import your first kernel or rootfs image</p>
          <Button>
            <Download className="mr-2 h-4 w-4" />
            Import Image
          </Button>
        </div>
      ) : (
        <div className="rounded-lg border border-border">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Name</TableHead>
                <TableHead>Type</TableHead>
                <TableHead>Size</TableHead>
                <TableHead>Project</TableHead>
                <TableHead>Path</TableHead>
                <TableHead>Usage</TableHead>
                <TableHead>Created</TableHead>
                <TableHead className="text-right">Actions</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {filteredImages.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={8} className="text-center py-8 text-muted-foreground">
                    No images found
                  </TableCell>
                </TableRow>
              ) : (
                filteredImages.map((image) => (
                  <TableRow key={image.id}>
                    <TableCell>
                      <div className="flex items-center gap-3">
                        <div
                          className={`flex h-10 w-10 items-center justify-center rounded-lg ${
                            image.kind === "kernel"
                              ? "bg-gradient-to-br from-blue-500/10 to-blue-600/10"
                              : "bg-gradient-to-br from-green-500/10 to-green-600/10"
                          }`}
                        >
                          <HardDrive
                            className={`h-5 w-5 ${image.kind === "kernel" ? "text-blue-600" : "text-green-600"}`}
                          />
                        </div>
                        <span className="font-medium">{image.name}</span>
                      </div>
                    </TableCell>
                    <TableCell>{getTypeBadge(image.kind)}</TableCell>
                    <TableCell className="text-sm">
                      {image.size_bytes ? formatBytes(image.size_bytes) : "N/A"}
                    </TableCell>
                    <TableCell className="text-sm">{image.project || "—"}</TableCell>
                    <TableCell>
                      <code className="text-xs bg-muted px-1.5 py-0.5 rounded">{image.path}</code>
                    </TableCell>
                    <TableCell className="text-sm">{image.usage_count || 0} VMs</TableCell>
                    <TableCell className="text-sm text-muted-foreground">
                      {formatRelativeTime(image.created_at)}
                    </TableCell>
                    <TableCell className="text-right">
                      <div className="flex justify-end gap-1">
                        <Button variant="ghost" size="icon" title="Clone">
                          <Copy className="h-4 w-4" />
                        </Button>
                        <Button variant="ghost" size="icon" title="Download">
                          <Download className="h-4 w-4" />
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
