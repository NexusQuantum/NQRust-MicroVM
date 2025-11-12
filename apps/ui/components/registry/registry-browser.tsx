"use client"

import { useState } from "react"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Badge } from "@/components/ui/badge"
import { Skeleton } from "@/components/ui/skeleton"
import { ArrowLeft, Search, HardDrive, Cpu, Download } from "lucide-react"
import { useRegistryImages, useImportRegistryImage, useCreateRegistryVolume, useDeleteRegistryItem, useRenameRegistryItem, useUploadRegistryFile } from "@/lib/queries"
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog"
import { useDateFormat } from "@/lib/hooks/use-date-format"

interface RegistryBrowserProps {
  type: "kernel" | "rootfs"
  onSelect: (id: string) => void
  onCancel: () => void
}

export function RegistryBrowser({ type, onSelect, onCancel }: RegistryBrowserProps) {
  const [searchQuery, setSearchQuery] = useState("")
  const { data: images, isLoading } = useRegistryImages()
  const importImage = useImportRegistryImage()
  const createVolume = useCreateRegistryVolume()
  const [importOpen, setImportOpen] = useState(false)
  const [volumeOpen, setVolumeOpen] = useState(false)
  const [importPath, setImportPath] = useState("")
  const [importName, setImportName] = useState("")
  const [volumeName, setVolumeName] = useState("")
  const [volumeSizeMb, setVolumeSizeMb] = useState(1024)
  const deleteItem = useDeleteRegistryItem()
  const dateFormat = useDateFormat()
  const renameItem = useRenameRegistryItem()
  const [renameOpen, setRenameOpen] = useState<string | null>(null)
  const [renameValue, setRenameValue] = useState("")
  const upload = useUploadRegistryFile()
  const [isDragging, setIsDragging] = useState(false)
  const [urlOpen, setUrlOpen] = useState(false)
  const [importUrl, setImportUrl] = useState("")

  const filteredImages = images?.filter((image: any) => {
    const matchesType =
      type === "kernel"
        ? image.kind === "kernel"
        : image.kind === "rootfs"

    const matchesSearch =
      !searchQuery ||
      image.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      image.project?.toLowerCase().includes(searchQuery.toLowerCase())

    return matchesType && matchesSearch
  })

  const getImageIcon = (imageKind: string) => {
    return imageKind === "kernel" ? Cpu : HardDrive
  }

  const formatSize = (bytes: number) => {
    if (bytes >= 1024 * 1024 * 1024) {
      return `${(bytes / 1024 / 1024 / 1024).toFixed(1)} GB`
    } else if (bytes >= 1024 * 1024) {
      return `${(bytes / 1024 / 1024).toFixed(1)} MB`
    } else if (bytes >= 1024) {
      return `${(bytes / 1024).toFixed(1)} KB`
    }
    return `${bytes} B`
  }

  return (
    <div className="max-w-4xl mx-auto space-y-6">
      <div className="flex items-center gap-4">
        <Button variant="ghost" size="icon" onClick={onCancel}>
          <ArrowLeft className="h-4 w-4" />
        </Button>
        <div>
          <h1 className="text-2xl font-bold tracking-tight">
            Browse {type === "kernel" ? "Kernel Images" : "Root Filesystems"}
          </h1>
          <p className="text-muted-foreground">
            Select a {type === "kernel" ? "kernel image" : "root filesystem"} from the registry
          </p>
        </div>
      </div>

      <Card>
        <CardHeader>
          <div className="flex items-center gap-4">
            <div className="relative flex-1">
              <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-muted-foreground" />
              <Input
                placeholder={`Search ${type === "kernel" ? "kernel images" : "root filesystems"}...`}
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="pl-10"
              />
            </div>
            <div className="flex items-center gap-2">
              <Dialog open={importOpen} onOpenChange={setImportOpen}>
                <DialogTrigger asChild>
                  <Button variant="outline">Import by Path</Button>
                </DialogTrigger>
                <DialogContent>
                  <DialogHeader>
                    <DialogTitle>Import {type === 'kernel' ? 'Kernel' : 'RootFS'} Image</DialogTitle>
                  </DialogHeader>
                  <div className="space-y-3">
                    <Input placeholder="/absolute/path/to/file" value={importPath} onChange={(e) => setImportPath(e.target.value)} />
                    <Input placeholder="Optional name (keeps filename if empty)" value={importName} onChange={(e) => setImportName(e.target.value)} />
                    <div className="flex justify-end gap-2">
                      <Button variant="outline" onClick={() => setImportOpen(false)}>Cancel</Button>
                      <Button onClick={async () => {
                        await importImage.mutateAsync({ type, path: importPath, name: importName || undefined })
                        setImportOpen(false); setImportPath(''); setImportName('')
                      }}>Import</Button>
                    </div>
                  </div>
                </DialogContent>
              </Dialog>
              <Dialog open={urlOpen} onOpenChange={setUrlOpen}>
                <DialogTrigger asChild>
                  <Button variant="outline">Import by URL</Button>
                </DialogTrigger>
                <DialogContent>
                  <DialogHeader>
                    <DialogTitle>Import from URL</DialogTitle>
                  </DialogHeader>
                  <div className="space-y-3">
                    <Input placeholder="https://..." value={importUrl} onChange={(e) => setImportUrl(e.target.value)} />
                    <div className="flex justify-end gap-2">
                      <Button variant="outline" onClick={() => setUrlOpen(false)}>Cancel</Button>
                      <Button onClick={async () => { await importImage.mutateAsync({ type, url: importUrl }); setUrlOpen(false); setImportUrl('') }}>Import</Button>
                    </div>
                  </div>
                </DialogContent>
              </Dialog>
              {type === 'rootfs' && (
                <Dialog open={volumeOpen} onOpenChange={setVolumeOpen}>
                  <DialogTrigger asChild>
                    <Button>Create Volume</Button>
                  </DialogTrigger>
                  <DialogContent>
                    <DialogHeader>
                      <DialogTitle>Create RootFS Volume</DialogTitle>
                    </DialogHeader>
                    <div className="space-y-3">
                      <Input placeholder="Name (e.g., rootfs-1.img)" value={volumeName} onChange={(e) => setVolumeName(e.target.value)} />
                      <div className="flex items-center gap-2">
                        <Input type="number" min={10} step={10} value={volumeSizeMb} onChange={(e) => setVolumeSizeMb(parseInt(e.target.value || '0'))} />
                        <span className="text-sm text-muted-foreground">MB</span>
                      </div>
                      <div className="flex justify-end gap-2">
                        <Button variant="outline" onClick={() => setVolumeOpen(false)}>Cancel</Button>
                        <Button onClick={async () => {
                          await createVolume.mutateAsync({ name: volumeName || 'volume', size_bytes: volumeSizeMb * 1024 * 1024, type: 'rootfs' })
                          setVolumeOpen(false); setVolumeName('');
                        }}>Create</Button>
                      </div>
                    </div>
                  </DialogContent>
                </Dialog>
              )}
            </div>
          </div>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="space-y-4">
              {Array.from({ length: 3 }).map((_, i) => (
                <div key={i} className="flex items-center space-x-4 p-4 border rounded-lg">
                  <Skeleton className="h-12 w-12 rounded" />
                  <div className="flex-1 space-y-2">
                    <Skeleton className="h-4 w-48" />
                    <Skeleton className="h-3 w-32" />
                  </div>
                  <Skeleton className="h-9 w-20" />
                </div>
              ))}
            </div>
          ) : !filteredImages?.length ? (
            <div
              className={`text-center py-12 border-2 border-dashed rounded-lg ${isDragging ? 'border-primary bg-primary/5' : 'border-muted'}`}
              onDragOver={(e) => { e.preventDefault(); setIsDragging(true) }}
              onDragLeave={() => setIsDragging(false)}
              onDrop={async (e) => {
                e.preventDefault(); setIsDragging(false);
                const file = e.dataTransfer.files?.[0];
                if (file) await upload.mutateAsync({ type, file });
              }}
            >
              <div className="mx-auto w-12 h-12 bg-muted rounded-full flex items-center justify-center mb-4">
                {type === "kernel" ? (
                  <Cpu className="h-6 w-6 text-muted-foreground" />
                ) : (
                  <HardDrive className="h-6 w-6 text-muted-foreground" />
                )}
              </div>
              <h3 className="text-lg font-semibold mb-2">Drop a file here to upload</h3>
              <p className="text-muted-foreground mb-4">or click to select a file</p>
              <label className="inline-block">
                <input type="file" className="hidden" onChange={async (e) => {
                  const f = e.target.files?.[0];
                  if (f) await upload.mutateAsync({ type, file: f })
                }} />
                <Button type="button" variant="outline">Choose File</Button>
              </label>
            </div>
          ) : (
            <div className="space-y-4">
              {filteredImages.map((image: any) => {
                const Icon = getImageIcon(image.kind)

                return (
                  <div
                    key={image.id}
                    className="flex items-center space-x-4 p-4 border rounded-xl hover:bg-muted/50 transition-colors min-h-24"
                  >
                    <div className="flex-shrink-0">
                      <div className="w-12 h-12 rounded-lg flex items-center justify-center brand-gradient">
                        <Icon className="h-6 w-6 text-white" />
                      </div>
                    </div>

                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2 mb-1">
                        <h3 className="font-semibold truncate">{image.name}</h3>
                        <Badge variant="outline" className="text-xs">
                          {image.kind}
                        </Badge>
                        {image.project && (
                          <Badge variant="secondary" className="text-xs">
                            {image.project}
                          </Badge>
                        )}
                      </div>

                      <div className="flex items-center gap-4 text-xs text-muted-foreground">
                        <span>Size: {formatSize(image.size)}</span>
                        <span>Updated: {dateFormat.formatDate(image.updated_at)}</span>
                        <span>SHA256: {image.sha256.substring(0, 12)}...</span>
                      </div>
                    </div>

                    <div className="flex-shrink-0">
                      <div className="flex items-center gap-2">
                        {type === "kernel" ? (
                          <Button onClick={() => onSelect(image.id)} variant="outline" className="gap-2 text-success border-success hover:bg-success/10">
                            <Download className="h-4 w-4" />
                            Use
                          </Button>
                        ) : (
                          <Button onClick={() => onSelect(image.id)} className="gap-2 bg-primary text-primary-foreground hover:outline hover:outline-2 hover:outline-offset-2 hover:[outline-color:hsl(var(--success))]">
                            <Download className="h-4 w-4" />
                            Mount
                          </Button>
                        )}
                        <Button variant="destructive" onClick={async () => { if (confirm(`Delete ${image.name}?`)) await deleteItem.mutateAsync(image.id) }}>Delete</Button>
                      </div>
                    </div>
                  </div>
                )
              })}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  )
}