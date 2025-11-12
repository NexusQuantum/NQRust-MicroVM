"use client"

import { useEffect, useMemo, useState } from "react"
import Link from "next/link"
import { useParams, useRouter } from "next/navigation"
import { ArrowLeft, Save, Trash2, Rocket } from "lucide-react"

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Textarea } from "@/components/ui/textarea"
import { useToast } from "@/hooks/use-toast"
import { ConfirmDialog } from "@/components/shared/confirm-dialog"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { useTemplate, useDeleteTemplate, useUpdateTemplate, useInstantiateTemplate, useImage } from "@/lib/queries"
import { useDateFormat } from "@/lib/hooks/use-date-format"

// Bentuk form yang disederhanakan (flatten) dari backend
type TemplateForm = {
  id: string
  name: string
  description?: string // kalau backend punya field ini, tinggal diisi; kalau tidak, biarkan kosong
  vcpu: number
  mem_mib: number
  kernel_image_id: string
  rootfs_image_id: string
  created_at?: string
  updated_at?: string
}

export default function TemplateDetailPage() {
  const { id } = useParams<{ id: string }>()
  const router = useRouter()
  const { toast } = useToast()

  const { data: template, isLoading, error } = useTemplate(id)

  // Fetch kernel and rootfs image details
  const { data: kernelImage, isLoading: isLoadingKernelImage } = useImage(
    template?.spec.kernel_image_id || ""
  )
  const { data: rootfsImage, isLoading: isLoadingRootfsImage } = useImage(
    template?.spec.rootfs_image_id || ""
  )

  const deleteTemplate = useDeleteTemplate()
  const updateTemplate = useUpdateTemplate()
  const instantiateTemplate = useInstantiateTemplate()

  const [formData, setFormData] = useState<TemplateForm | null>(null)
  const [isEditing, setIsEditing] = useState(false)
  const [showDeleteDialog, setShowDeleteDialog] = useState(false)
  const [showDeployDialog, setShowDeployDialog] = useState(false)
  const [vmName, setVmName] = useState("")

  // Seed form dari data backend
  useEffect(() => {
    if (!template) return
    setFormData({
      id: template.id,
      name: template.name ?? "",
      description: (template as any).description ?? "", // optional, aman jika backend belum punya
      vcpu: Number(template.spec?.vcpu ?? 0),
      mem_mib: Number(template.spec?.mem_mib ?? 0),
      kernel_image_id: template.spec?.kernel_image_id ?? "",
      rootfs_image_id: template.spec?.rootfs_image_id ?? "",
      created_at: template.created_at,
      updated_at: template.updated_at,
    })
  }, [template])

  const dateFormat = useDateFormat()
  
  const createdText = useMemo(() => {
    return template?.created_at ? dateFormat.formatDateTime(template.created_at) : "-"
  }, [template?.created_at, dateFormat])

  const updatedText = useMemo(() => {
    return template?.updated_at ? dateFormat.formatDateTime(template.updated_at) : "-"
  }, [template?.updated_at, dateFormat])

  const handleSave = async () => {
    if (!formData) return

    try {
      await updateTemplate.mutateAsync({
        id,
        data: {
          name: formData.name,
          description: formData.description,
          spec: {
            vcpu: formData.vcpu,
            mem_mib: formData.mem_mib,
          },
        },
      })
      router.push("/templates?action=updated")
    } catch {
      // Error is handled by the mutation hook
    }
  }

  const handleDelete = async () => {
    try {
      await deleteTemplate.mutateAsync(id)
      setShowDeleteDialog(false)
      router.push("/templates?action=deleted")
    } catch {
      // Error is already handled by the mutation hook with toast
      // Just keep dialog open so user can retry
    }
  }

  const handleDeploy = () => {
    // Generate default VM name from template name
    const defaultName = formData?.name ? `${formData.name}-${Date.now()}` : `vm-${Date.now()}`
    setVmName(defaultName)
    setShowDeployDialog(true)
  }

  const handleConfirmDeploy = async () => {
    if (!vmName.trim()) {
      toast({
        title: "VM name required",
        description: "Please enter a name for the VM",
        variant: "error",
        duration: 2000,
      })
      return
    }

    try {
      await instantiateTemplate.mutateAsync({ id, name: vmName.trim() })
      setShowDeployDialog(false)
      setVmName("")
      toast({
        title: "VM deployed successfully",
        description: `VM "${vmName.trim()}" has been created from template "${formData?.name}"`,
        variant: "success",
        duration: 2000,
      })
      router.push("/templates")
    } catch {
      // Error is handled by the mutation hook
    }
  }

  if (isLoading) {
    return (
      <div className="container mx-auto py-6">
        <div className="animate-pulse space-y-4">
          <div className="h-8 bg-muted rounded w-1/4" />
          <div className="grid gap-4">
            {[...Array(6)].map((_, i) => (
              <div key={i} className="h-24 bg-muted rounded-lg" />
            ))}
          </div>
        </div>
      </div>
    )
  }

  if (error) {
    return (
      <div className="container mx-auto py-6 text-center space-y-4">
        <h1 className="text-2xl font-bold text-destructive">Failed to load Template</h1>
        <p className="text-muted-foreground">Unable to fetch template detail. Please try again.</p>
        <Button variant="outline" onClick={() => location.reload()}>
          Try again
        </Button>
      </div>
    )
  }

  if (!template || !formData) {
    return (
      <div className="container mx-auto py-6 text-center space-y-4">
        <h1 className="text-2xl font-bold">Template not found</h1>
        <Link href="/templates">
          <Button variant="outline">Back to Templates</Button>
        </Link>
      </div>
    )
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <Link href="/templates">
            <Button variant="ghost" size="icon" aria-label="Back">
              <ArrowLeft className="h-4 w-4" />
            </Button>
          </Link>
          <div>
            <h1 className="text-3xl font-bold text-foreground">{formData.name}</h1>
            <p className="text-sm text-muted-foreground mt-1">
              Created {createdText} • Updated {updatedText}
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
              <Button variant="outline" size="sm" onClick={() => setIsEditing(false)} disabled={updateTemplate.isPending}>
                Cancel
              </Button>
              <Button variant="default" size="sm" onClick={handleSave} disabled={updateTemplate.isPending}>
                <Save className="mr-2 h-4 w-4" />
                {updateTemplate.isPending ? "Saving..." : "Save Changes"}
              </Button>
            </>
          ) : (
            <Button variant="outline" size="sm" onClick={() => setIsEditing(true)}>
              Edit
            </Button>
          )}
          <Button
            variant="destructive"
            size="sm"
            onClick={() => setShowDeleteDialog(true)}
            disabled={deleteTemplate.isPending || isEditing}
          >
            <Trash2 className="mr-2 h-4 w-4" />
            {deleteTemplate.isPending ? "Deleting..." : "Delete"}
          </Button>
        </div>
      </div>

      {/* Content */}
      <div className="grid gap-6 md:grid-cols-2">
        {/* Info */}
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
                value={formData.description ?? ""}
                onChange={(e) => setFormData({ ...formData, description: e.target.value })}
                rows={4}
                disabled={!isEditing}
              />
            </div>
          </CardContent>
        </Card>

        {/* Spec */}
        <Card>
          <CardHeader>
            <CardTitle>Resource Configuration</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="vcpu">vCPU</Label>
                <Input
                  id="vcpu"
                  type="number"
                  value={Number.isFinite(formData.vcpu) ? formData.vcpu : 0}
                  onChange={(e) =>
                    setFormData({
                      ...formData,
                      vcpu: Number.isNaN(parseInt(e.target.value)) ? 0 : parseInt(e.target.value, 10),
                    })
                  }
                  disabled={!isEditing}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="mem_mib">Memory (MiB)</Label>
                <Input
                  id="mem_mib"
                  type="number"
                  min="128"
                  step="128"
                  value={Number.isFinite(formData.mem_mib) ? formData.mem_mib : 0}
                  onChange={(e) =>
                    setFormData({
                      ...formData,
                      mem_mib: Number.isNaN(parseInt(e.target.value)) ? 0 : parseInt(e.target.value, 10),
                    })
                  }
                  disabled={!isEditing}
                />
              </div>
            </div>
            <div className="rounded-lg bg-muted p-4 space-y-2">
              <p className="text-sm font-medium">Resource Summary</p>
              <div className="grid grid-cols-2 gap-2 text-sm">
                <div>
                  <span className="text-muted-foreground">CPU:</span> {formData.vcpu} cores
                </div>
                <div>
                  <span className="text-muted-foreground">RAM:</span> {(formData.mem_mib / 1024).toFixed(1)} GB
                </div>
              </div>
            </div>
          </CardContent>
        </Card>

        {/* Images */}
        <Card className="md:col-span-2">
          <CardHeader>
            <CardTitle>Image References</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="kernel_image_id">Kernel Image</Label>
              <div className="space-y-1">
                {isLoadingKernelImage ? (
                  <div className="text-sm text-muted-foreground">Loading image info...</div>
                ) : kernelImage ? (
                  <div className="text-sm text-muted-foreground mb-1">
                    <span className="font-medium text-foreground">{kernelImage.name}</span>
                    {kernelImage.project && <span className="ml-2 text-xs">({kernelImage.project})</span>}
                  </div>
                ) : formData.kernel_image_id ? (
                  <div className="text-sm text-muted-foreground mb-1">Image not found</div>
                ) : null}
                <Input
                  id="kernel_image_id"
                  value={formData.kernel_image_id}
                  onChange={(e) => setFormData({ ...formData, kernel_image_id: e.target.value })}
                  disabled={true}
                  placeholder="Kernel Image ID"
                />
              </div>
            </div>
            <div className="space-y-2">
              <Label htmlFor="rootfs_image_id">RootFS Image</Label>
              <div className="space-y-1">
                {isLoadingRootfsImage ? (
                  <div className="text-sm text-muted-foreground">Loading image info...</div>
                ) : rootfsImage ? (
                  <div className="text-sm text-muted-foreground mb-1">
                    <span className="font-medium text-foreground">{rootfsImage.name}</span>
                    {rootfsImage.project && <span className="ml-2 text-xs">({rootfsImage.project})</span>}
                  </div>
                ) : formData.rootfs_image_id ? (
                  <div className="text-sm text-muted-foreground mb-1">Image not found</div>
                ) : null}
                <Input
                  id="rootfs_image_id"
                  value={formData.rootfs_image_id}
                  onChange={(e) => setFormData({ ...formData, rootfs_image_id: e.target.value })}
                  disabled={true}
                  placeholder="RootFS Image ID"
                />
              </div>
            </div>
          </CardContent>
        </Card>

        {/* Metadata */}
        <Card className="md:col-span-2">
          <CardHeader>
            <CardTitle>Metadata</CardTitle>
          </CardHeader>
          <CardContent className="grid grid-cols-2 gap-4">
            <div>
              <p className="text-sm text-muted-foreground">Created at</p>
              <p className="text-lg font-medium">{createdText}</p>
            </div>
            <div>
              <p className="text-sm text-muted-foreground">Updated at</p>
              <p className="text-lg font-medium">{updatedText}</p>
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
        isLoading={deleteTemplate.isPending}
      />

      <Dialog open={showDeployDialog} onOpenChange={setShowDeployDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Deploy VM from Template</DialogTitle>
            <DialogDescription>
              Enter a name for the new VM. This will create a VM instance based on the template &quot;{formData?.name}&quot;.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="vm-name">VM Name</Label>
              <Input
                id="vm-name"
                value={vmName}
                onChange={(e) => setVmName(e.target.value)}
                placeholder="Enter VM name"
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    handleConfirmDeploy()
                  }
                }}
              />
            </div>

            {formData && (
              <div className="rounded-lg border p-4 space-y-2 text-sm">
                <h4 className="font-medium">Template Configuration</h4>
                <div className="grid grid-cols-2 gap-2 text-muted-foreground">
                  <div>
                    vCPU: <span className="text-foreground font-mono">{formData.vcpu}</span>
                  </div>
                  <div>
                    RAM: <span className="text-foreground font-mono">{formData.mem_mib} MiB</span>
                  </div>
                </div>
              </div>
            )}
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => {
                setShowDeployDialog(false)
                setVmName("")
              }}
            >
              Cancel
            </Button>
            <Button
              onClick={handleConfirmDeploy}
              disabled={instantiateTemplate.isPending || !vmName.trim()}
            >
              {instantiateTemplate.isPending ? "Creating..." : "Create VM"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}
