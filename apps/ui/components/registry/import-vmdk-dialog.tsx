"use client"

import { useState } from "react"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { useImportVmdk } from "@/lib/queries"

interface ImportVmdkDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

/**
 * Import a VMware VMDK (or any qemu-img-readable disk) as a registered
 * UEFI disk image. With virt-v2v enabled, the guest's VMware paravirt
 * drivers (vmxnet3 / pvscsi) are converted to virtio so the imported VM
 * boots on QEMU. The source path must be readable by the manager host.
 */
export function ImportVmdkDialog({ open, onOpenChange }: ImportVmdkDialogProps) {
  const [sourcePath, setSourcePath] = useState("")
  const [name, setName] = useState("")
  const [runVirtV2v, setRunVirtV2v] = useState(true)
  const importVmdk = useImportVmdk()

  const reset = () => {
    setSourcePath("")
    setName("")
    setRunVirtV2v(true)
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>Import from VMware (VMDK)</DialogTitle>
          <DialogDescription>
            Convert a VMware disk to a QEMU-bootable image. Requires
            libguestfs-tools (virt-v2v) on the manager host.
          </DialogDescription>
        </DialogHeader>
        <div className="space-y-4 py-2">
          <div className="space-y-2">
            <Label htmlFor="vmdk-source">Source disk path</Label>
            <Input
              id="vmdk-source"
              value={sourcePath}
              onChange={(e) => setSourcePath(e.target.value)}
              placeholder="/srv/images/import/win2022.vmdk"
            />
            <p className="text-xs text-muted-foreground">
              Absolute path on the manager host. VMDK, qcow2, or raw.
            </p>
          </div>
          <div className="space-y-2">
            <Label htmlFor="vmdk-name">Image name (optional)</Label>
            <Input
              id="vmdk-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Defaults to source filename"
            />
          </div>
          <div className="flex items-center gap-2">
            <input
              type="checkbox"
              id="vmdk-virtv2v"
              checked={runVirtV2v}
              onChange={(e) => setRunVirtV2v(e.target.checked)}
              className="h-4 w-4"
            />
            <Label htmlFor="vmdk-virtv2v" className="cursor-pointer">
              Run virt-v2v (adapt drivers to virtio)
            </Label>
          </div>
          <p className="text-xs text-muted-foreground">
            Recommended for Windows / older Linux guests. Disable for a fast
            format-only convert (qemu-img) when the guest already uses virtio.
          </p>
          {importVmdk.isError && (
            <p className="text-sm text-red-600">
              Import failed. Check that virt-v2v is installed and the path is readable.
            </p>
          )}
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button
            disabled={!sourcePath.trim() || importVmdk.isPending}
            onClick={() =>
              importVmdk.mutate(
                {
                  sourcePath: sourcePath.trim(),
                  name: name.trim() || undefined,
                  runVirtV2v,
                },
                {
                  onSuccess: () => {
                    onOpenChange(false)
                    reset()
                  },
                }
              )
            }
          >
            {importVmdk.isPending ? "Importing…" : "Import"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
