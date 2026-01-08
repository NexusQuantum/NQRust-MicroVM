"use client"

import type React from "react"

import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog"
import { ArrowLeft, Save, Loader2, Cpu, HardDrive, FileCode, Folder } from "lucide-react"
import Link from "next/link"
import { useState, useEffect } from "react"
import { useRouter } from "next/navigation"
import { useRegistryImages, useImage } from "@/lib/queries"
import { useMutation, useQueryClient } from "@tanstack/react-query"
import { facadeApi } from "@/lib/api/facade"
import { toast } from "sonner"
import { Skeleton } from "@/components/ui/skeleton"
import { ConfirmDialog } from "@/components/shared/confirm-dialog"

export default function NewTemplatePage() {
  const router = useRouter()
  const queryClient = useQueryClient()
  const { data: images, isLoading: imagesLoading } = useRegistryImages()

  const [formData, setFormData] = useState({
    name: "",
    vcpu: 2,
    mem_mib: 2048,
    kernel_image_id: "",
    rootfs_image_id: "",
    kernel_path: "",
    rootfs_path: "",
  })

  const [useImageIds, setUseImageIds] = useState(true)
  const [showCancelDialog, setShowCancelDialog] = useState(false)
  const [showReviewDialog, setShowReviewDialog] = useState(false)

  // Filter images by type
  const kernelImages = images?.filter(img => img.kind === "kernel") || []
  const rootfsImages = images?.filter(img => img.kind === "rootfs") || []

  // Fetch image details to get host_path
  const { data: kernelImage } = useImage(formData.kernel_image_id || "")
  const { data: rootfsImage } = useImage(formData.rootfs_image_id || "")

  // Update paths when image IDs change and we're using registry
  useEffect(() => {
    if (useImageIds && kernelImage?.host_path) {
      setFormData(prev => ({ ...prev, kernel_path: kernelImage.host_path }))
    }
  }, [kernelImage, useImageIds])

  useEffect(() => {
    if (useImageIds && rootfsImage?.host_path) {
      setFormData(prev => ({ ...prev, rootfs_path: rootfsImage.host_path }))
    }
  }, [rootfsImage, useImageIds])

  const createMutation = useMutation({
    mutationFn: () => {
      // Prepare spec with both image IDs and paths
      const spec: any = {
        vcpu: formData.vcpu,
        mem_mib: formData.mem_mib,
      }

      // Always include both image_id and path if available
      if (formData.kernel_image_id) {
        spec.kernel_image_id = formData.kernel_image_id
      }
      if (formData.kernel_path) {
        spec.kernel_path = formData.kernel_path
      }

      if (formData.rootfs_image_id) {
        spec.rootfs_image_id = formData.rootfs_image_id
      }
      if (formData.rootfs_path) {
        spec.rootfs_path = formData.rootfs_path
      }

      return facadeApi.createTemplate({
        name: formData.name,
        spec,
      })
    },
    onSuccess: () => {
      toast.success("Template created successfully", {
        description: `"${formData.name}" is now available for deployment`,
      })
      queryClient.invalidateQueries({ queryKey: ["templates"] })
      router.push("/templates")
    },
    onError: (error: Error) => {
      try {
        const e = JSON.parse(error.message)
        toast.error(e.error || "Failed to create template", {
          description: e.suggestion || e.fault_message,
        })
      } catch {
        toast.error("Failed to create template", {
          description: error.message,
        })
      }
    },
  })

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()

    // Validation
    if (!formData.name.trim()) {
      toast.error("Validation Error", {
        description: "Template name is required",
      })
      return
    }

    if (useImageIds) {
      if (!formData.kernel_image_id && !formData.rootfs_image_id) {
        toast.error("Validation Error", {
          description: "Please select at least kernel or rootfs image",
        })
        return
      }

      // Check if paths are populated (should be auto-populated from image details)
      if (formData.kernel_image_id && !formData.kernel_path) {
        toast.error("Loading Error", {
          description: "Kernel image path is still loading, please wait",
        })
        return
      }

      if (formData.rootfs_image_id && !formData.rootfs_path) {
        toast.error("Loading Error", {
          description: "Rootfs image path is still loading, please wait",
        })
        return
      }
    } else {
      if (!formData.kernel_path && !formData.rootfs_path) {
        toast.error("Validation Error", {
          description: "Please provide at least kernel or rootfs path",
        })
        return
      }
    }

    // Show review dialog instead of creating immediately
    setShowReviewDialog(true)
  }

  const handleConfirmCreate = () => {
    setShowReviewDialog(false)
    createMutation.mutate()
  }

  if (imagesLoading) {
    return (
      <div className="space-y-6">
        <Skeleton className="h-24 w-full" />
        <div className="grid gap-6 md:grid-cols-2">
          <Skeleton className="h-96 w-full" />
          <Skeleton className="h-96 w-full" />
        </div>
      </div>
    )
  }

  return (
    <form onSubmit={handleSubmit}>
      <div className="space-y-6">
        {/* Header */}
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-4">
            <Button type="button" variant="ghost" size="icon" onClick={() => setShowCancelDialog(true)}>
              <ArrowLeft className="h-4 w-4" />
            </Button>
            <div>
              <h1 className="text-3xl font-bold text-foreground">Create New Template</h1>
              <p className="text-sm text-muted-foreground mt-1">Save a VM configuration as a reusable template</p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <Button type="button" variant="outline" onClick={() => setShowCancelDialog(true)}>
              Cancel
            </Button>
            <Button type="submit" disabled={createMutation.isPending}>
              {createMutation.isPending ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  Creating...
                </>
              ) : (
                <>
                  <Save className="mr-2 h-4 w-4" />
                  Create Template
                </>
              )}
            </Button>
          </div>
        </div>

        <div className="grid gap-6 md:grid-cols-2">
          {/* Template Information */}
          <Card>
            <CardHeader>
              <CardTitle>Template Information</CardTitle>
              <CardDescription>Basic details for this template</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="name">
                  Template Name <span className="text-destructive">*</span>
                </Label>
                <Input
                  autoComplete="off"
                  id="name"
                  placeholder="e.g., Ubuntu 22.04 Base"
                  value={formData.name}
                  onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                  required
                />
                <p className="text-xs text-muted-foreground">
                  A descriptive name to identify this template
                </p>
              </div>
            </CardContent>
          </Card>

          {/* Resource Configuration */}
          <Card>
            <CardHeader>
              <CardTitle>Resource Configuration</CardTitle>
              <CardDescription>CPU and memory allocation</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-2">
                  <Label htmlFor="vcpu">
                    vCPU Cores <span className="text-destructive">*</span>
                  </Label>
                  <Input
                    id="vcpu"
                    type="number"
                    min="1"
                    max="32"
                    value={formData.vcpu}
                    onChange={(e) => setFormData({ ...formData, vcpu: Number.parseInt(e.target.value) })}
                    required
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="memory">
                    Memory (MiB) <span className="text-destructive">*</span>
                  </Label>
                  <Input
                    id="memory"
                    type="number"
                    min="128"
                    step="128"
                    value={formData.mem_mib}
                    onChange={(e) => setFormData({ ...formData, mem_mib: Number.parseInt(e.target.value) })}
                    required
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

          {/* Image Configuration */}
          <Card className="md:col-span-2">
            <CardHeader>
              <CardTitle>Image Configuration</CardTitle>
              <CardDescription>
                Select kernel and rootfs images from registry or specify custom paths
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              {/* Toggle between Image IDs and Paths */}
              <div className="flex items-center gap-4 p-3 bg-muted rounded-lg">
                <Label htmlFor="use-registry" className="flex-1">
                  Use Image Registry
                </Label>
                <Button
                  type="button"
                  variant={useImageIds ? "default" : "outline"}
                  size="sm"
                  onClick={() => setUseImageIds(!useImageIds)}
                >
                  {useImageIds ? "Using Registry" : "Using Paths"}
                </Button>
              </div>

              {useImageIds ? (
                <>
                  {/* Kernel Image from Registry */}
                  <div className="space-y-2">
                    <Label htmlFor="kernel_image_id">
                      Kernel Image {kernelImages.length === 0 && <span className="text-xs text-muted-foreground">(optional)</span>}
                    </Label>
                    <Select
                      value={formData.kernel_image_id}
                      onValueChange={(value) => setFormData({ ...formData, kernel_image_id: value })}
                    >
                      <SelectTrigger>
                        <SelectValue placeholder={kernelImages.length > 0 ? "Select kernel image" : "No kernel images in registry"} />
                      </SelectTrigger>
                      <SelectContent>
                        {kernelImages.map((img) => (
                          <SelectItem key={img.id} value={img.id}>
                            {img.name} {img.project && `(${img.project})`}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                    {kernelImages.length === 0 && (
                      <p className="text-xs text-muted-foreground">
                        No kernel images found. Upload one in the{" "}
                        <Link href="/registry" className="underline">
                          registry
                        </Link>
                        .
                      </p>
                    )}
                  </div>

                  {/* Rootfs Image from Registry */}
                  <div className="space-y-2">
                    <Label htmlFor="rootfs_image_id">
                      Rootfs Image {rootfsImages.length === 0 && <span className="text-xs text-muted-foreground">(optional)</span>}
                    </Label>
                    <Select
                      value={formData.rootfs_image_id}
                      onValueChange={(value) => setFormData({ ...formData, rootfs_image_id: value })}
                    >
                      <SelectTrigger>
                        <SelectValue placeholder={rootfsImages.length > 0 ? "Select rootfs image" : "No rootfs images in registry"} />
                      </SelectTrigger>
                      <SelectContent>
                        {rootfsImages.map((img) => (
                          <SelectItem key={img.id} value={img.id}>
                            {img.name} {img.project && `(${img.project})`}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                    {rootfsImages.length === 0 && (
                      <p className="text-xs text-muted-foreground">
                        No rootfs images found. Upload one in the{" "}
                        <Link href="/registry" className="underline">
                          registry
                        </Link>
                        .
                      </p>
                    )}
                  </div>
                </>
              ) : (
                <>
                  {/* Kernel Path */}
                  <div className="space-y-2">
                    <Label htmlFor="kernel_path">
                      Kernel Path <span className="text-xs text-muted-foreground">(optional)</span>
                    </Label>
                    <Input
                      id="kernel_path"
                      placeholder="/srv/images/vmlinux-5.10.fc.bin"
                      value={formData.kernel_path}
                      onChange={(e) => setFormData({ ...formData, kernel_path: e.target.value })}
                    />
                    <p className="text-xs text-muted-foreground">
                      Absolute path to kernel file on the host
                    </p>
                  </div>

                  {/* Rootfs Path */}
                  <div className="space-y-2">
                    <Label htmlFor="rootfs_path">
                      Rootfs Path <span className="text-xs text-muted-foreground">(optional)</span>
                    </Label>
                    <Input
                      id="rootfs_path"
                      placeholder="/srv/images/ubuntu-22.04.ext4"
                      value={formData.rootfs_path}
                      onChange={(e) => setFormData({ ...formData, rootfs_path: e.target.value })}
                    />
                    <p className="text-xs text-muted-foreground">
                      Absolute path to rootfs file on the host
                    </p>
                  </div>
                </>
              )}
            </CardContent>
          </Card>
        </div>
      </div>

      {/* Cancel Confirmation Dialog */}
      <ConfirmDialog
        open={showCancelDialog}
        onOpenChange={setShowCancelDialog}
        onConfirm={() => {
          setShowCancelDialog(false)
          router.push("/templates")
        }}
        title="Cancel Template Creation"
        description="Are you sure you want to cancel? Any unsaved changes will be lost."
        confirmText="Yes, Cancel"
        cancelText="Continue Editing"
        variant="default"
      />

      {/* Review Template Dialog */}
      <Dialog open={showReviewDialog} onOpenChange={setShowReviewDialog}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>Review Template Configuration</DialogTitle>
            <DialogDescription>
              Please review the template configuration before creating. Make sure all settings are correct.
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-6 py-4">
            {/* Template Name */}
            <div className="space-y-2">
              <h3 className="text-sm font-semibold text-muted-foreground uppercase tracking-wide">Template Name</h3>
              <p className="text-lg font-medium">{formData.name}</p>
            </div>

            {/* Resource Configuration */}
            <div className="space-y-3">
              <h3 className="text-sm font-semibold text-muted-foreground uppercase tracking-wide">Resource Configuration</h3>
              <div className="grid grid-cols-2 gap-4">
                <div className="flex items-center gap-3 rounded-lg border p-4">
                  <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
                    <Cpu className="h-5 w-5 text-primary" />
                  </div>
                  <div>
                    <p className="text-sm text-muted-foreground">vCPU Cores</p>
                    <p className="text-xl font-semibold">{formData.vcpu}</p>
                  </div>
                </div>
                <div className="flex items-center gap-3 rounded-lg border p-4">
                  <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
                    <HardDrive className="h-5 w-5 text-primary" />
                  </div>
                  <div>
                    <p className="text-sm text-muted-foreground">Memory</p>
                    <p className="text-xl font-semibold">{formData.mem_mib} MiB</p>
                    <p className="text-xs text-muted-foreground">({(formData.mem_mib / 1024).toFixed(1)} GB)</p>
                  </div>
                </div>
              </div>
            </div>

            {/* Image Configuration */}
            <div className="space-y-3">
              <h3 className="text-sm font-semibold text-muted-foreground uppercase tracking-wide">Boot Images</h3>
              <div className="space-y-3">
                {/* Kernel */}
                <div className="rounded-lg border p-4 space-y-2">
                  <div className="flex items-center gap-2">
                    <FileCode className="h-4 w-4 text-muted-foreground" />
                    <span className="text-sm font-medium">Kernel Image</span>
                  </div>
                  {formData.kernel_image_id ? (
                    <div className="space-y-1 pl-6">
                      <p className="text-sm">
                        <span className="text-muted-foreground">Registry:</span>{" "}
                        <span className="font-mono text-xs">{kernelImage?.name || formData.kernel_image_id}</span>
                      </p>
                      <p className="text-sm">
                        <span className="text-muted-foreground">Path:</span>{" "}
                        <span className="font-mono text-xs">{formData.kernel_path || "Loading..."}</span>
                      </p>
                    </div>
                  ) : formData.kernel_path ? (
                    <p className="text-sm font-mono text-xs pl-6">{formData.kernel_path}</p>
                  ) : (
                    <p className="text-sm text-muted-foreground pl-6">Not configured</p>
                  )}
                </div>

                {/* Rootfs */}
                <div className="rounded-lg border p-4 space-y-2">
                  <div className="flex items-center gap-2">
                    <Folder className="h-4 w-4 text-muted-foreground" />
                    <span className="text-sm font-medium">Rootfs Image</span>
                  </div>
                  {formData.rootfs_image_id ? (
                    <div className="space-y-1 pl-6">
                      <p className="text-sm">
                        <span className="text-muted-foreground">Registry:</span>{" "}
                        <span className="font-mono text-xs">{rootfsImage?.name || formData.rootfs_image_id}</span>
                      </p>
                      <p className="text-sm">
                        <span className="text-muted-foreground">Path:</span>{" "}
                        <span className="font-mono text-xs">{formData.rootfs_path || "Loading..."}</span>
                      </p>
                    </div>
                  ) : formData.rootfs_path ? (
                    <p className="text-sm font-mono text-xs pl-6">{formData.rootfs_path}</p>
                  ) : (
                    <p className="text-sm text-muted-foreground pl-6">Not configured</p>
                  )}
                </div>
              </div>
            </div>

            {/* Warning if incomplete */}
            {(!formData.kernel_path && !formData.rootfs_path) && (
              <div className="rounded-lg bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-900 p-4">
                <p className="text-sm text-yellow-800 dark:text-yellow-200">
                  <strong>Warning:</strong> Neither kernel nor rootfs is configured. VMs created from this template may not boot properly.
                </p>
              </div>
            )}
          </div>

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => setShowReviewDialog(false)}
              disabled={createMutation.isPending}
            >
              Back to Edit
            </Button>
            <Button
              type="button"
              onClick={handleConfirmCreate}
              disabled={createMutation.isPending}
            >
              {createMutation.isPending ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  Creating...
                </>
              ) : (
                <>
                  <Save className="mr-2 h-4 w-4" />
                  Confirm & Create
                </>
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </form>
  )
}
