"use client"

import type React from "react"

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Textarea } from "@/components/ui/textarea"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { ArrowLeft, Save } from "lucide-react"
import Link from "next/link"
import { useState } from "react"
import { useToast } from "@/hooks/use-toast"
import { useRouter } from "next/navigation"

export default function NewTemplatePage() {
  const { toast } = useToast()
  const router = useRouter()
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [formData, setFormData] = useState({
    name: "",
    description: "",
    vcpu: 4,
    memory: 8192,
    kernel: "",
    rootfs: "",
  })

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setIsSubmitting(true)

    // Simulate API call
    await new Promise((resolve) => setTimeout(resolve, 1000))

    toast({
      title: "Template created",
      description: `${formData.name} has been created successfully.`,
    })

    setIsSubmitting(false)
    router.push("/templates")
  }

  return (
    <form onSubmit={handleSubmit}>
      <div className="space-y-6">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-4">
            <Link href="/templates">
              <Button type="button" variant="ghost" size="icon">
                <ArrowLeft className="h-4 w-4" />
              </Button>
            </Link>
            <div>
              <h1 className="text-3xl font-bold text-foreground">Create New Template</h1>
              <p className="text-sm text-muted-foreground mt-1">Save a VM configuration as a reusable template</p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <Link href="/templates">
              <Button type="button" variant="outline">
                Cancel
              </Button>
            </Link>
            <Button type="submit" disabled={isSubmitting}>
              <Save className="mr-2 h-4 w-4" />
              {isSubmitting ? "Creating..." : "Create Template"}
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
                <Label htmlFor="name">
                  Template Name <span className="text-destructive">*</span>
                </Label>
                <Input
                  id="name"
                  placeholder="e.g., Production Web Server"
                  value={formData.name}
                  onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                  required
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="description">Description</Label>
                <Textarea
                  id="description"
                  placeholder="Describe what this template is used for..."
                  rows={4}
                  value={formData.description}
                  onChange={(e) => setFormData({ ...formData, description: e.target.value })}
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
                    Memory (MB) <span className="text-destructive">*</span>
                  </Label>
                  <Input
                    id="memory"
                    type="number"
                    min="512"
                    step="512"
                    value={formData.memory}
                    onChange={(e) => setFormData({ ...formData, memory: Number.parseInt(e.target.value) })}
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
                    <span className="text-muted-foreground">RAM:</span> {(formData.memory / 1024).toFixed(1)} GB
                  </div>
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
                <Label htmlFor="kernel">
                  Kernel Image <span className="text-destructive">*</span>
                </Label>
                <Select
                  value={formData.kernel}
                  onValueChange={(value) => setFormData({ ...formData, kernel: value })}
                  required
                >
                  <SelectTrigger>
                    <SelectValue placeholder="Select kernel image" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="vmlinux-5.10">vmlinux-5.10 (Stable)</SelectItem>
                    <SelectItem value="vmlinux-5.15">vmlinux-5.15 (LTS)</SelectItem>
                    <SelectItem value="vmlinux-6.1">vmlinux-6.1 (Latest)</SelectItem>
                  </SelectContent>
                </Select>
              </div>
              <div className="space-y-2">
                <Label htmlFor="rootfs">
                  Root Filesystem <span className="text-destructive">*</span>
                </Label>
                <Select
                  value={formData.rootfs}
                  onValueChange={(value) => setFormData({ ...formData, rootfs: value })}
                  required
                >
                  <SelectTrigger>
                    <SelectValue placeholder="Select rootfs image" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="ubuntu-22.04">Ubuntu 22.04 LTS (500 MB)</SelectItem>
                    <SelectItem value="ubuntu-20.04">Ubuntu 20.04 LTS (480 MB)</SelectItem>
                    <SelectItem value="alpine">Alpine Linux (120 MB)</SelectItem>
                    <SelectItem value="debian">Debian 11 (450 MB)</SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </CardContent>
          </Card>
        </div>
      </div>
    </form>
  )
}
