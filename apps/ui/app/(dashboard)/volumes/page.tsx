"use client"

import { useState } from "react"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Loader2, HardDrive, RotateCw, Plus } from "lucide-react"
import { useVolumes, useCreateVolume, useHosts } from "@/lib/queries"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { VolumeTable } from "@/components/volume/volume-table"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Label } from "@/components/ui/label"
import { Input } from "@/components/ui/input"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { toast } from "sonner"

export default function VolumesPage() {
  const { data: volumes = [], isLoading, error, refetch, isFetching } = useVolumes()
  const { data: hosts = [] } = useHosts()
  const createVolume = useCreateVolume()

  const [showCreateDialog, setShowCreateDialog] = useState(false)
  const [formData, setFormData] = useState({
    name: "",
    description: "",
    size_gb: "",
    type: "ext4" as "raw" | "qcow2" | "ext4",
    host_id: "",
  })

  const resetForm = () => {
    setFormData({ name: "", description: "", size_gb: "", type: "ext4", host_id: "" })
  }

  const handleCreate = () => {
    createVolume.mutate(
      {
        name: formData.name,
        description: formData.description || undefined,
        size_gb: parseInt(formData.size_gb, 10),
        type: formData.type,
        host_id: formData.host_id,
      },
      {
        onSuccess: () => {
          toast.success("Volume created", { description: `${formData.name} has been created` })
          setShowCreateDialog(false)
          resetForm()
        },
        onError: (error) => {
          toast.error("Failed to create volume", {
            description: error instanceof Error ? error.message : "An unexpected error occurred",
          })
        },
      }
    )
  }

  return (
    <div className="space-y-6">
      <div className="relative overflow-hidden rounded-xl border border-border bg-gradient-to-br from-indigo-50 to-indigo-100/50 dark:from-indigo-950/30 dark:to-indigo-900/20 p-8">
        <div className="relative z-10 flex items-center justify-between">
          <div className="max-w-xl">
            <h1 className="text-3xl font-bold text-foreground">Volumes</h1>
            <p className="mt-2 text-muted-foreground">
              Manage persistent block storage volumes. Create volumes independently and attach them to VMs as needed.
            </p>
          </div>
          <div className="hidden lg:flex items-center justify-center h-32 w-32 rounded-full bg-gradient-to-br from-indigo-500/20 to-indigo-600/20 dark:from-indigo-700/30 dark:to-indigo-800/20">
            <HardDrive className="h-16 w-16 text-indigo-600 dark:text-indigo-400" />
          </div>
        </div>
        <div className="absolute right-0 top-0 h-64 w-64 translate-x-32 -translate-y-32 rounded-full bg-gradient-to-br from-indigo-400/30 to-indigo-600/30 dark:from-indigo-500/20 dark:to-indigo-600/10 blur-3xl" />
      </div>

      <Card>
        <CardHeader className="flex items-center justify-between">
          <CardTitle>All Volumes</CardTitle>
          <div className="flex items-center gap-2">
            <Button
              variant="outline"
              onClick={() => refetch()}
              disabled={isFetching}
              title="Refresh volume list"
            >
              {isFetching ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  Refreshing...
                </>
              ) : (
                <>
                  <RotateCw className="mr-2 h-4 w-4" />
                  Refresh
                </>
              )}
            </Button>
            <Button onClick={() => { resetForm(); setShowCreateDialog(true) }}>
              <Plus className="mr-2 h-4 w-4" />
              Create Volume
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
            </div>
          ) : error ? (
            <Alert variant="destructive">
              <AlertDescription>
                Failed to load volumes. Please try again.
              </AlertDescription>
            </Alert>
          ) : volumes.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              <p>No volumes found. Create your first volume to get started.</p>
            </div>
          ) : (
            <VolumeTable volumes={volumes} />
          )}
        </CardContent>
      </Card>

      <Dialog open={showCreateDialog} onOpenChange={setShowCreateDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Create Volume</DialogTitle>
            <DialogDescription>
              Create a new persistent block storage volume. The volume file will be allocated on the selected host.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="vol-name">Name *</Label>
              <Input
                id="vol-name"
                placeholder="e.g., postgres-data"
                value={formData.name}
                onChange={(e) => setFormData({ ...formData, name: e.target.value })}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="vol-desc">Description</Label>
              <Input
                id="vol-desc"
                placeholder="Optional description"
                value={formData.description}
                onChange={(e) => setFormData({ ...formData, description: e.target.value })}
              />
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="vol-size">Size (GB) *</Label>
                <Input
                  id="vol-size"
                  type="number"
                  min={1}
                  placeholder="10"
                  value={formData.size_gb}
                  onChange={(e) => setFormData({ ...formData, size_gb: e.target.value })}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="vol-type">Type *</Label>
                <Select
                  value={formData.type}
                  onValueChange={(value) => setFormData({ ...formData, type: value as "raw" | "qcow2" | "ext4" })}
                >
                  <SelectTrigger id="vol-type">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="ext4">EXT4</SelectItem>
                    <SelectItem value="raw">RAW</SelectItem>
                    <SelectItem value="qcow2">QCOW2</SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </div>
            <div className="space-y-2">
              <Label htmlFor="vol-host">Host *</Label>
              <Select
                value={formData.host_id}
                onValueChange={(value) => setFormData({ ...formData, host_id: value })}
              >
                <SelectTrigger id="vol-host">
                  <SelectValue placeholder="Select host" />
                </SelectTrigger>
                <SelectContent>
                  {hosts.map((host) => (
                    <SelectItem key={host.id} value={host.id}>
                      {host.name} ({host.addr})
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowCreateDialog(false)}>
              Cancel
            </Button>
            <Button
              onClick={handleCreate}
              disabled={!formData.name || !formData.size_gb || parseInt(formData.size_gb, 10) < 1 || !formData.host_id || createVolume.isPending}
            >
              {createVolume.isPending ? "Creating..." : "Create Volume"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}
