"use client"

import type React from "react"

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { ArrowLeft, Upload, LinkIcon, FileUp } from "lucide-react"
import Link from "next/link"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { useState } from "react"
import { useToast } from "@/hooks/use-toast"
import { useRouter } from "next/navigation"

export default function ImportImagePage() {
  const { toast } = useToast()
  const router = useRouter()
  const [isUploading, setIsUploading] = useState(false)
  const [selectedFile, setSelectedFile] = useState<File | null>(null)
  const [formData, setFormData] = useState({
    name: "",
    kind: "",
    project: "",
    path: "/images/",
    url: "",
  })

  const handleFileSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    if (file) {
      setSelectedFile(file)
      if (!formData.name) {
        setFormData({ ...formData, name: file.name.replace(/\.[^/.]+$/, "") })
      }
    }
  }

  const handleUpload = async () => {
    if (!selectedFile || !formData.name || !formData.kind) {
      toast({
        title: "Missing information",
        description: "Please fill in all required fields.",
        variant: "destructive",
      })
      return
    }

    setIsUploading(true)
    await new Promise((resolve) => setTimeout(resolve, 2000))

    toast({
      title: "Image imported",
      description: `${formData.name} has been imported successfully.`,
    })

    setIsUploading(false)
    router.push("/registry")
  }

  const handleImportFromUrl = async () => {
    if (!formData.url || !formData.name || !formData.kind) {
      toast({
        title: "Missing information",
        description: "Please fill in all required fields.",
        variant: "destructive",
      })
      return
    }

    setIsUploading(true)
    await new Promise((resolve) => setTimeout(resolve, 2000))

    toast({
      title: "Image imported",
      description: `${formData.name} is being downloaded and imported.`,
    })

    setIsUploading(false)
    router.push("/registry")
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <Link href="/registry">
            <Button variant="ghost" size="icon">
              <ArrowLeft className="h-4 w-4" />
            </Button>
          </Link>
          <div>
            <h1 className="text-3xl font-bold text-foreground">Import Image</h1>
            <p className="text-sm text-muted-foreground mt-1">
              Upload or link a kernel or rootfs image to your registry
            </p>
          </div>
        </div>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Import Method</CardTitle>
        </CardHeader>
        <CardContent>
          <Tabs defaultValue="upload" className="space-y-4">
            <TabsList className="grid w-full grid-cols-2">
              <TabsTrigger value="upload">Upload File</TabsTrigger>
              <TabsTrigger value="url">From URL</TabsTrigger>
            </TabsList>

            <TabsContent value="upload" className="space-y-4">
              <div
                className="rounded-lg border-2 border-dashed border-border p-12 text-center hover:border-primary/50 transition-colors cursor-pointer"
                onClick={() => document.getElementById("file-input")?.click()}
              >
                {selectedFile ? (
                  <>
                    <FileUp className="mx-auto h-12 w-12 text-primary" />
                    <p className="mt-4 text-sm font-medium">{selectedFile.name}</p>
                    <p className="text-sm text-muted-foreground">{(selectedFile.size / 1024 / 1024).toFixed(2)} MB</p>
                  </>
                ) : (
                  <>
                    <Upload className="mx-auto h-12 w-12 text-muted-foreground" />
                    <p className="mt-4 text-sm text-muted-foreground">
                      Drag and drop your image file here, or click to browse
                    </p>
                    <p className="mt-2 text-xs text-muted-foreground">Supported: .ext4, .img, .qcow2, vmlinux</p>
                  </>
                )}
                <input
                  id="file-input"
                  type="file"
                  className="hidden"
                  accept=".ext4,.img,.qcow2"
                  onChange={handleFileSelect}
                />
              </div>
            </TabsContent>

            <TabsContent value="url" className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="url">
                  Image URL <span className="text-destructive">*</span>
                </Label>
                <Input
                  id="url"
                  type="url"
                  placeholder="https://example.com/images/ubuntu-22.04.ext4"
                  value={formData.url}
                  onChange={(e) => setFormData({ ...formData, url: e.target.value })}
                />
                <p className="text-sm text-muted-foreground">Provide a direct download link to the image file</p>
              </div>
            </TabsContent>
          </Tabs>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Image Configuration</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-2">
              <Label htmlFor="name">
                Image Name <span className="text-destructive">*</span>
              </Label>
              <Input
                id="name"
                placeholder="e.g., ubuntu-22.04-rootfs"
                value={formData.name}
                onChange={(e) => setFormData({ ...formData, name: e.target.value })}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="kind">
                Image Type <span className="text-destructive">*</span>
              </Label>
              <Select value={formData.kind} onValueChange={(value) => setFormData({ ...formData, kind: value })}>
                <SelectTrigger>
                  <SelectValue placeholder="Select type" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="kernel">Kernel</SelectItem>
                  <SelectItem value="rootfs">Root Filesystem</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>

          <div className="space-y-2">
            <Label htmlFor="project">Project</Label>
            <Select value={formData.project} onValueChange={(value) => setFormData({ ...formData, project: value })}>
              <SelectTrigger>
                <SelectValue placeholder="Select project (optional)" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="production">Production</SelectItem>
                <SelectItem value="staging">Staging</SelectItem>
                <SelectItem value="development">Development</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div className="space-y-2">
            <Label htmlFor="path">
              Storage Path <span className="text-destructive">*</span>
            </Label>
            <Input
              id="path"
              placeholder="/images/"
              value={formData.path}
              onChange={(e) => setFormData({ ...formData, path: e.target.value })}
            />
            <p className="text-sm text-muted-foreground">The file will be stored at this path on the host system</p>
          </div>
        </CardContent>
      </Card>

      <div className="flex justify-end gap-2">
        <Link href="/registry">
          <Button variant="outline">Cancel</Button>
        </Link>
        <Button onClick={formData.url ? handleImportFromUrl : handleUpload} disabled={isUploading}>
          {isUploading ? (
            <>Importing...</>
          ) : (
            <>
              {formData.url ? <LinkIcon className="mr-2 h-4 w-4" /> : <Upload className="mr-2 h-4 w-4" />}
              Import Image
            </>
          )}
        </Button>
      </div>
    </div>
  )
}
