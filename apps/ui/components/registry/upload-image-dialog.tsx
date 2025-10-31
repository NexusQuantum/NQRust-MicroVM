"use client"

import { useState, useRef } from "react"
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Upload, Loader2, File as FileIcon, X } from "lucide-react"
import { useUploadImage } from "@/lib/queries"

interface UploadImageDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  defaultKind?: "docker" | "kernel" | "rootfs"
}

export function UploadImageDialog({ open, onOpenChange, defaultKind = "docker" }: UploadImageDialogProps) {
  const uploadImage = useUploadImage()
  const fileInputRef = useRef<HTMLInputElement>(null)

  const [selectedFile, setSelectedFile] = useState<File | null>(null)
  const [kind, setKind] = useState<"docker" | "kernel" | "rootfs">(defaultKind)
  const [name, setName] = useState("")
  const [project, setProject] = useState("")

  const handleFileSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    if (file) {
      setSelectedFile(file)
      if (!name) {
        // Auto-fill name from filename
        setName(file.name.replace(/\.[^/.]+$/, ""))
      }
    }
  }

  const handleUpload = () => {
    if (!selectedFile) return

    uploadImage.mutate(
      {
        file: selectedFile,
        kind,
        name: name.trim() || undefined,
        project: project.trim() || undefined,
      },
      {
        onSuccess: () => {
          onOpenChange(false)
          resetForm()
        },
      }
    )
  }

  const resetForm = () => {
    setSelectedFile(null)
    setName("")
    setProject("")
    if (fileInputRef.current) {
      fileInputRef.current.value = ""
    }
  }

  const formatFileSize = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>Upload Image</DialogTitle>
          <DialogDescription>
            Upload a Docker image tarball, kernel, or rootfs image
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4">
          <div className="space-y-2">
            <Label htmlFor="image-kind">Image Type</Label>
            <Select value={kind} onValueChange={(v) => setKind(v as any)}>
              <SelectTrigger id="image-kind">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="docker">Docker Image (.tar)</SelectItem>
                <SelectItem value="kernel">VM Kernel</SelectItem>
                <SelectItem value="rootfs">VM Rootfs</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div className="space-y-2">
            <Label>File</Label>
            <div className="flex items-center gap-2">
              <Input
                ref={fileInputRef}
                type="file"
                onChange={handleFileSelect}
                accept={kind === "docker" ? ".tar,.tar.gz" : "*"}
                className="hidden"
                id="file-upload"
              />
              <Button
                variant="outline"
                onClick={() => fileInputRef.current?.click()}
                className="w-full"
                type="button"
              >
                <Upload className="mr-2 h-4 w-4" />
                {selectedFile ? "Change File" : "Select File"}
              </Button>
            </div>
            {selectedFile && (
              <div className="flex items-center gap-2 p-2 bg-muted rounded-lg">
                <FileIcon className="h-4 w-4 text-muted-foreground" />
                <div className="flex-1 min-w-0">
                  <p className="text-sm font-medium truncate">{selectedFile.name}</p>
                  <p className="text-xs text-muted-foreground">{formatFileSize(selectedFile.size)}</p>
                </div>
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={() => {
                    setSelectedFile(null)
                    if (fileInputRef.current) fileInputRef.current.value = ""
                  }}
                >
                  <X className="h-4 w-4" />
                </Button>
              </div>
            )}
          </div>

          <div className="space-y-2">
            <Label htmlFor="image-name">
              Name <span className="text-xs text-muted-foreground">(optional)</span>
            </Label>
            <Input
              id="image-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Auto-detected from file"
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="image-project">
              Project <span className="text-xs text-muted-foreground">(optional)</span>
            </Label>
            <Input
              id="image-project"
              value={project}
              onChange={(e) => setProject(e.target.value)}
              placeholder="e.g., production, testing"
            />
          </div>

          {kind === "docker" && (
            <div className="rounded-lg bg-blue-50 dark:bg-blue-950 p-3 text-sm text-blue-900 dark:text-blue-100">
              <p className="font-medium mb-1">Docker Image Format</p>
              <p className="text-xs">
                Export Docker images using: <code className="bg-blue-100 dark:bg-blue-900 px-1 rounded">docker save -o image.tar image:tag</code>
              </p>
            </div>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={uploadImage.isPending}>
            Cancel
          </Button>
          <Button onClick={handleUpload} disabled={!selectedFile || uploadImage.isPending}>
            {uploadImage.isPending ? (
              <>
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                Uploading...
              </>
            ) : (
              <>
                <Upload className="mr-2 h-4 w-4" />
                Upload
              </>
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
