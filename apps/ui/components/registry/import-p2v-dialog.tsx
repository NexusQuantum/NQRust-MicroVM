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
import { useImportP2v } from "@/lib/queries"

interface ImportP2vDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

type AuthMode = "password" | "key"

/**
 * Agentless P2V / B2V (baremetal-to-VM). The manager SSHes into a reachable
 * physical machine, streams the chosen block device, and registers it as a
 * QEMU-bootable image. virt-v2v adapts the guest's drivers to virtio. For a
 * consistent image the source should be quiesced or live-USB-booted.
 */
export function ImportP2vDialog({ open, onOpenChange }: ImportP2vDialogProps) {
  const [sshHost, setSshHost] = useState("")
  const [sshPort, setSshPort] = useState("22")
  const [sshUser, setSshUser] = useState("root")
  const [authMode, setAuthMode] = useState<AuthMode>("password")
  const [sshPassword, setSshPassword] = useState("")
  const [sshKeyPath, setSshKeyPath] = useState("")
  const [sourceDisk, setSourceDisk] = useState("/dev/sda")
  const [name, setName] = useState("")
  const [runVirtV2v, setRunVirtV2v] = useState(true)
  const importP2v = useImportP2v()

  const reset = () => {
    setSshHost("")
    setSshPort("22")
    setSshUser("root")
    setAuthMode("password")
    setSshPassword("")
    setSshKeyPath("")
    setSourceDisk("/dev/sda")
    setName("")
    setRunVirtV2v(true)
  }

  const canSubmit =
    sshHost.trim() &&
    sshUser.trim() &&
    sourceDisk.trim() &&
    name.trim() &&
    (authMode === "password" ? sshPassword : sshKeyPath.trim()) &&
    !importP2v.isPending

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>Import from Physical Machine (P2V)</DialogTitle>
          <DialogDescription>
            Stream a baremetal disk over SSH and convert it to a QEMU-bootable
            image. The source should be quiesced or live-USB-booted for a clean
            image. Requires virt-v2v + sshpass on the manager host.
          </DialogDescription>
        </DialogHeader>
        <div className="space-y-4 py-2">
          <div className="grid grid-cols-3 gap-2">
            <div className="col-span-2 space-y-2">
              <Label htmlFor="p2v-host">SSH host</Label>
              <Input
                id="p2v-host"
                value={sshHost}
                onChange={(e) => setSshHost(e.target.value)}
                placeholder="10.0.0.50"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="p2v-port">Port</Label>
              <Input
                id="p2v-port"
                value={sshPort}
                onChange={(e) => setSshPort(e.target.value)}
                placeholder="22"
              />
            </div>
          </div>
          <div className="space-y-2">
            <Label htmlFor="p2v-user">SSH user</Label>
            <Input
              id="p2v-user"
              value={sshUser}
              onChange={(e) => setSshUser(e.target.value)}
              placeholder="root"
            />
            <p className="text-xs text-muted-foreground">
              Must be root or have passwordless sudo to read the block device.
            </p>
          </div>
          <div className="space-y-2">
            <Label>Authentication</Label>
            <div className="flex gap-4 text-sm">
              <label className="flex items-center gap-1.5 cursor-pointer">
                <input
                  type="radio"
                  checked={authMode === "password"}
                  onChange={() => setAuthMode("password")}
                />
                Password
              </label>
              <label className="flex items-center gap-1.5 cursor-pointer">
                <input
                  type="radio"
                  checked={authMode === "key"}
                  onChange={() => setAuthMode("key")}
                />
                SSH key
              </label>
            </div>
            {authMode === "password" ? (
              <Input
                type="password"
                value={sshPassword}
                onChange={(e) => setSshPassword(e.target.value)}
                placeholder="SSH password"
              />
            ) : (
              <Input
                value={sshKeyPath}
                onChange={(e) => setSshKeyPath(e.target.value)}
                placeholder="/path/to/private_key (on manager host)"
              />
            )}
          </div>
          <div className="space-y-2">
            <Label htmlFor="p2v-disk">Source disk</Label>
            <Input
              id="p2v-disk"
              value={sourceDisk}
              onChange={(e) => setSourceDisk(e.target.value)}
              placeholder="/dev/sda"
            />
            <p className="text-xs text-muted-foreground">
              Whole disk (e.g. /dev/sda, /dev/nvme0n1), not a partition.
            </p>
          </div>
          <div className="space-y-2">
            <Label htmlFor="p2v-name">Image name</Label>
            <Input
              id="p2v-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="old-server-01"
            />
          </div>
          <div className="flex items-center gap-2">
            <input
              type="checkbox"
              id="p2v-virtv2v"
              checked={runVirtV2v}
              onChange={(e) => setRunVirtV2v(e.target.checked)}
              className="h-4 w-4"
            />
            <Label htmlFor="p2v-virtv2v" className="cursor-pointer">
              Run virt-v2v (adapt drivers to virtio)
            </Label>
          </div>
          <p className="text-xs text-muted-foreground">
            Strongly recommended — physical machines use vendor/AHCI/NVMe
            drivers a VM doesn&apos;t have, so an unconverted disk may not boot.
          </p>
          {importP2v.isError && (
            <p className="text-sm text-red-600">
              Import failed. Check SSH reachability, credentials, and that the
              user can read the disk (root / passwordless sudo).
            </p>
          )}
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button
            disabled={!canSubmit}
            onClick={() =>
              importP2v.mutate(
                {
                  sshHost: sshHost.trim(),
                  sshPort: parseInt(sshPort, 10) || 22,
                  sshUser: sshUser.trim(),
                  sshPassword: authMode === "password" ? sshPassword : undefined,
                  sshKeyPath: authMode === "key" ? sshKeyPath.trim() : undefined,
                  sourceDisk: sourceDisk.trim(),
                  name: name.trim(),
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
            {importP2v.isPending ? "Importing…" : "Import"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
