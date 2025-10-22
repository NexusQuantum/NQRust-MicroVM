"use client"

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Textarea } from "@/components/ui/textarea"
import { ArrowLeft, Save, Trash2, Rocket } from "lucide-react"
import Link from "next/link"
import { useState } from "react"
import { useToast } from "@/hooks/use-toast"
import { useRouter } from "next/navigation"
import { ConfirmDialog } from "@/components/shared/confirm-dialog"

// Mock data
const mockTemplate = {
  id: "tpl-1",
  name: "Standard Web Server",
  description: "Ubuntu 22.04 with 4 vCPU and 8GB RAM, optimized for web applications",
  vcpu: 4,
  mem_mib: 8192,
  kernel_path: "/images/vmlinux-5.10",
  rootfs_path: "/images/ubuntu-22.04.ext4",
  created_at: new Date(Date.now() - 86400000 * 15).toISOString(),
  updated_at: new Date(Date.now() - 86400000 * 2).toISOString(),
  usage_count: 8,
}

export default function TemplateDetailPage({ params }: { params: { id: string } }) {
  const { toast } = useToast()
  const router = useRouter()
  const [isEditing, setIsEditing] = useState(false)
  const [isSaving, setIsSaving] = useState(false)
  const [showDeleteDialog, setShowDeleteDialog] = useState(false)
  const [formData, setFormData] = useState(mockTemplate)

  const handleSave = async () => {
    setIsSaving(true)
    await new Promise((resolve) => setTimeout(resolve, 1000))

    toast({
      title: "Template updated",
      description: "Your changes have been saved successfully.",
    })

    setIsSaving(false)
    setIsEditing(false)
  }

  const handleDelete = async () => {
    await new Promise((resolve) => setTimeout(resolve, 1000))

    toast({
      title: "Template deleted",
      description: `${formData.name} has been deleted.`,
      variant: "destructive",
    })

    router.push("/templates")
  }

  const handleDeploy = () => {
    router.push(`/vms/create?template=${params.id}`)
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <Link href="/templates">
            <Button variant="ghost" size="icon">
              <ArrowLeft className="h-4 w-4" />
            </Button>
          </Link>
          <div>
            <h1 className="text-3xl font-bold text-foreground">{formData.name}</h1>
            <p className="text-sm text-muted-foreground mt-1">
              Used {formData.usage_count} times • Created {new Date(formData.created_at).toLocaleDateString()}
            </p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="default" size="sm" onClick={handleDeploy}>
            <Rocket className="mr-2 h-4 w-4" />
            Deploy VM
          </Button>
          {isEditing ? (
            <>
              <Button variant="outline" size="sm" onClick={() => setIsEditing(false)}>
                Cancel
              </Button>
              <Button variant="default" size="sm" onClick={handleSave} disabled={isSaving}>
                <Save className="mr-2 h-4 w-4" />
                {isSaving ? "Saving..." : "Save Changes"}
              </Button>
            </>
          ) : (
            <Button variant="outline" size="sm" onClick={() => setIsEditing(true)}>
              Edit
            </Button>
          )}
          <Button variant="destructive" size="sm" onClick={() => setShowDeleteDialog(true)}>
            <Trash2 className="mr-2 h-4 w-4" />
            Delete
          </Button>
        </div>
      </div>

      <div className="grid gap-6 md:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Template Information</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="name">Template Name</Label>
              <Input
                id="name"
                value={formData.name}
                onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                disabled={!isEditing}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="description">Description</Label>
              <Textarea
                id="description"
                value={formData.description}
                onChange={(e) => setFormData({ ...formData, description: e.target.value })}
                rows={4}
                disabled={!isEditing}
              />
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Resource Configuration</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="vcpu">vCPU Cores</Label>
                <Input
                  id="vcpu"
                  type="number"
                  value={formData.vcpu}
                  onChange={(e) => setFormData({ ...formData, vcpu: Number.parseInt(e.target.value) })}
                  disabled={!isEditing}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="memory">Memory (MB)</Label>
                <Input
                  id="memory"
                  type="number"
                  value={formData.mem_mib}
                  onChange={(e) => setFormData({ ...formData, mem_mib: Number.parseInt(e.target.value) })}
                  disabled={!isEditing}
                />
              </div>
            </div>
          </CardContent>
        </Card>

        <Card className="md:col-span-2">
          <CardHeader>
            <CardTitle>Image Configuration</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="kernel">Kernel Image Path</Label>
              <Input
                id="kernel"
                value={formData.kernel_path}
                onChange={(e) => setFormData({ ...formData, kernel_path: e.target.value })}
                disabled={!isEditing}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="rootfs">Root Filesystem Path</Label>
              <Input
                id="rootfs"
                value={formData.rootfs_path}
                onChange={(e) => setFormData({ ...formData, rootfs_path: e.target.value })}
                disabled={!isEditing}
              />
            </div>
          </CardContent>
        </Card>

        <Card className="md:col-span-2">
          <CardHeader>
            <CardTitle>Usage Statistics</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-3 gap-4">
              <div className="space-y-1">
                <p className="text-sm text-muted-foreground">Total Deployments</p>
                <p className="text-2xl font-bold">{formData.usage_count}</p>
              </div>
              <div className="space-y-1">
                <p className="text-sm text-muted-foreground">Last Used</p>
                <p className="text-2xl font-bold">2d ago</p>
              </div>
              <div className="space-y-1">
                <p className="text-sm text-muted-foreground">Active VMs</p>
                <p className="text-2xl font-bold">3</p>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>

      <ConfirmDialog
        open={showDeleteDialog}
        onOpenChange={setShowDeleteDialog}
        onConfirm={handleDelete}
        title="Delete Template"
        description={`Are you sure you want to delete "${formData.name}"? This action cannot be undone.`}
        confirmText="Delete"
        variant="destructive"
      />
    </div>
  )
}
