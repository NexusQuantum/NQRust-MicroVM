"use client"

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { ArrowLeft, Download, Trash2, Copy, Check } from "lucide-react"
import Link from "next/link"
import { useState } from "react"
import { useToast } from "@/hooks/use-toast"
import { useRouter } from "next/navigation"
import { ConfirmDialog } from "@/components/shared/confirm-dialog"

// Mock data
const mockImage = {
  id: "img-1",
  name: "ubuntu-22.04-rootfs",
  kind: "rootfs" as const,
  size_bytes: 524288000,
  project: "production",
  path: "/images/ubuntu-22.04.ext4",
  created_at: new Date(Date.now() - 86400000 * 20).toISOString(),
  updated_at: new Date(Date.now() - 86400000 * 5).toISOString(),
  usage_count: 8,
  checksum: "sha256:a1b2c3d4e5f6...",
  format: "ext4",
}

export default function RegistryDetailPage({ params }: { params: { id: string } }) {
  const { toast } = useToast()
  const router = useRouter()
  const [copied, setCopied] = useState(false)
  const [showDeleteDialog, setShowDeleteDialog] = useState(false)

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)

    toast({
      title: "Copied to clipboard",
      description: "The text has been copied to your clipboard.",
    })
  }

  const handleDelete = async () => {
    await new Promise((resolve) => setTimeout(resolve, 1000))

    toast({
      title: "Image deleted",
      description: `${mockImage.name} has been deleted from the registry.`,
      variant: "destructive",
    })

    router.push("/registry")
  }

  const handleDownload = () => {
    toast({
      title: "Download started",
      description: `Downloading ${mockImage.name}...`,
    })
  }

  const formatBytes = (bytes: number) => {
    if (bytes === 0) return "0 Bytes"
    const k = 1024
    const sizes = ["Bytes", "KB", "MB", "GB"]
    const i = Math.floor(Math.log(bytes) / Math.log(k))
    return Math.round((bytes / Math.pow(k, i)) * 100) / 100 + " " + sizes[i]
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
            <div className="flex items-center gap-3">
              <h1 className="text-3xl font-bold text-foreground">{mockImage.name}</h1>
              <Badge variant="outline">{mockImage.kind}</Badge>
              <Badge variant="secondary">{mockImage.project}</Badge>
            </div>
            <p className="text-sm text-muted-foreground mt-1">
              {formatBytes(mockImage.size_bytes)} • Created {new Date(mockImage.created_at).toLocaleDateString()}
            </p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="outline" size="sm" onClick={handleDownload}>
            <Download className="mr-2 h-4 w-4" />
            Download
          </Button>
          <Button variant="destructive" size="sm" onClick={() => setShowDeleteDialog(true)}>
            <Trash2 className="mr-2 h-4 w-4" />
            Delete
          </Button>
        </div>
      </div>

      <div className="grid gap-6 md:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Image Details</CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            <div className="flex justify-between py-2 border-b">
              <span className="text-sm text-muted-foreground">Image ID</span>
              <span className="text-sm font-mono">{mockImage.id}</span>
            </div>
            <div className="flex justify-between py-2 border-b">
              <span className="text-sm text-muted-foreground">Type</span>
              <Badge variant="outline">{mockImage.kind}</Badge>
            </div>
            <div className="flex justify-between py-2 border-b">
              <span className="text-sm text-muted-foreground">Format</span>
              <span className="text-sm">{mockImage.format}</span>
            </div>
            <div className="flex justify-between py-2 border-b">
              <span className="text-sm text-muted-foreground">Size</span>
              <span className="text-sm">{formatBytes(mockImage.size_bytes)}</span>
            </div>
            <div className="flex justify-between py-2 border-b">
              <span className="text-sm text-muted-foreground">Project</span>
              <Badge variant="secondary">{mockImage.project}</Badge>
            </div>
            <div className="flex justify-between py-2">
              <span className="text-sm text-muted-foreground">Usage Count</span>
              <span className="text-sm font-semibold">{mockImage.usage_count} VMs</span>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Path & Checksum</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-2">
              <label className="text-sm text-muted-foreground">File Path</label>
              <div className="flex items-center gap-2">
                <code className="flex-1 rounded bg-muted px-3 py-2 text-sm">{mockImage.path}</code>
                <Button variant="outline" size="icon" onClick={() => copyToClipboard(mockImage.path)}>
                  {copied ? <Check className="h-4 w-4" /> : <Copy className="h-4 w-4" />}
                </Button>
              </div>
            </div>
            <div className="space-y-2">
              <label className="text-sm text-muted-foreground">Checksum</label>
              <div className="flex items-center gap-2">
                <code className="flex-1 rounded bg-muted px-3 py-2 text-sm break-all">{mockImage.checksum}</code>
                <Button variant="outline" size="icon" onClick={() => copyToClipboard(mockImage.checksum)}>
                  {copied ? <Check className="h-4 w-4" /> : <Copy className="h-4 w-4" />}
                </Button>
              </div>
            </div>
          </CardContent>
        </Card>

        <Card className="md:col-span-2">
          <CardHeader>
            <CardTitle>Usage History</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-2">
              {[
                { vm: "web-server-01", date: "2 days ago", status: "active" },
                { vm: "web-server-02", date: "5 days ago", status: "active" },
                { vm: "test-vm-03", date: "1 week ago", status: "stopped" },
                { vm: "prod-db-01", date: "2 weeks ago", status: "active" },
              ].map((usage, i) => (
                <div key={i} className="flex items-center justify-between py-2 border-b last:border-0">
                  <div className="flex items-center gap-3">
                    <div
                      className={`h-2 w-2 rounded-full ${usage.status === "active" ? "bg-green-500" : "bg-gray-400"}`}
                    />
                    <span className="text-sm font-medium">{usage.vm}</span>
                  </div>
                  <span className="text-sm text-muted-foreground">{usage.date}</span>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      </div>

      <ConfirmDialog
        open={showDeleteDialog}
        onOpenChange={setShowDeleteDialog}
        onConfirm={handleDelete}
        title="Delete Image"
        description={`Are you sure you want to delete "${mockImage.name}"? This will affect ${mockImage.usage_count} VMs using this image.`}
        confirmText="Delete"
        variant="destructive"
      />
    </div>
  )
}
