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
import { useDeleteRegistryItem } from "@/lib/queries"
import { toast } from "sonner"
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog"

interface ImageRegistryProps {
  images: Image[]
}

export function ImageRegistry({ images }: ImageRegistryProps) {
  const [searchQuery, setSearchQuery] = useState("")
  const [typeFilter, setTypeFilter] = useState<string>("all")
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false)
  const [imageToDelete, setImageToDelete] = useState<Image | null>(null)

  const deleteImage = useDeleteRegistryItem()

  const filteredImages = images.filter((image) => {
    const matchesSearch = image.name.toLowerCase().includes(searchQuery.toLowerCase())
    const matchesType = typeFilter === "all" || image.kind === typeFilter
    return matchesSearch && matchesType
  })

  const handleCopyPath = (path: string) => {
    navigator.clipboard.writeText(path)
    toast.success("Path copied to clipboard")
  }

  const handleDownload = (image: Image) => {
    // For Docker images (tarballs), we could implement a download endpoint
    // For now, copy the path which can be used for manual operations
    handleCopyPath(image.host_path)
    toast.info("Image path copied", {
      description: "Use this path to access the image file on the server",
    })
  }

  const handleDeleteClick = (image: Image) => {
    setImageToDelete(image)
    setDeleteDialogOpen(true)
  }

  const handleDeleteConfirm = () => {
    if (!imageToDelete) return

    deleteImage.mutate(imageToDelete.id, {
      onSuccess: () => {
        setDeleteDialogOpen(false)
        setImageToDelete(null)
      },
    })
  }

  const getTypeBadge = (kind: string) => {
    const colors = {
      kernel: "bg-blue-100 text-blue-700 border-blue-200",
      rootfs: "bg-green-100 text-green-700 border-green-200",
      docker: "bg-purple-100 text-purple-700 border-purple-200",
    }
    return (
      <Badge variant="outline" className={colors[kind as keyof typeof colors] || "bg-gray-100 text-gray-700 border-gray-200"}>
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
            <SelectItem value="docker">Docker</SelectItem>
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
                            image.kind === "docker"
                              ? "bg-gradient-to-br from-purple-500/10 to-purple-600/10"
                              : image.kind === "kernel"
                              ? "bg-gradient-to-br from-blue-500/10 to-blue-600/10"
                              : "bg-gradient-to-br from-green-500/10 to-green-600/10"
                          }`}
                        >
                          <HardDrive
                            className={`h-5 w-5 ${
                              image.kind === "docker"
                                ? "text-purple-600"
                                : image.kind === "kernel"
                                ? "text-blue-600"
                                : "text-green-600"
                            }`}
                          />
                        </div>
                        <span className="font-medium">{image.name}</span>
                      </div>
                    </TableCell>
                    <TableCell>{getTypeBadge(image.kind)}</TableCell>
                    <TableCell className="text-sm">
                      {image.size ? formatBytes(image.size) : "N/A"}
                    </TableCell>
                    <TableCell className="text-sm">{image.project || "—"}</TableCell>
                    <TableCell>
                      <code className="text-xs bg-muted px-1.5 py-0.5 rounded">{image.host_path}</code>
                    </TableCell>
                    <TableCell className="text-sm">—</TableCell>
                    <TableCell className="text-sm text-muted-foreground">
                      {formatRelativeTime(image.created_at)}
                    </TableCell>
                    <TableCell className="text-right">
                      <div className="flex justify-end gap-1">
                        <Button
                          variant="ghost"
                          size="icon"
                          title="Copy Path"
                          onClick={() => handleCopyPath(image.host_path)}
                        >
                          <Copy className="h-4 w-4" />
                        </Button>
                        <Button
                          variant="ghost"
                          size="icon"
                          title="Download / Copy Path"
                          onClick={() => handleDownload(image)}
                        >
                          <Download className="h-4 w-4" />
                        </Button>
                        <Button
                          variant="ghost"
                          size="icon"
                          title="Delete"
                          onClick={() => handleDeleteClick(image)}
                          disabled={deleteImage.isPending}
                        >
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

      <AlertDialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete Image</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to delete <strong>{imageToDelete?.name}</strong>?
              <br />
              This action cannot be undone. The image file will be removed from the registry.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={deleteImage.isPending}>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={handleDeleteConfirm}
              disabled={deleteImage.isPending}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              {deleteImage.isPending ? "Deleting..." : "Delete"}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  )
}
