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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { useHosts, useMigrateVM, useRescheduleVM, useBackupVM } from "@/lib/queries"
import type { Vm } from "@/lib/types"

/**
 * Day-2 operations for QEMU VMs: live migrate, reschedule (HA), backup.
 * Rendered as a small action cluster on the VM detail page. Only shown for
 * `vmm_kind === "qemu"`.
 */
export function VmActions({ vm }: { vm: Vm }) {
  const [migrateOpen, setMigrateOpen] = useState(false)
  const [rescheduleOpen, setRescheduleOpen] = useState(false)
  const [backupOpen, setBackupOpen] = useState(false)

  return (
    <>
      <Button variant="outline" size="sm" onClick={() => setMigrateOpen(true)}>
        Migrate
      </Button>
      <Button variant="outline" size="sm" onClick={() => setRescheduleOpen(true)}>
        Reschedule
      </Button>
      <Button variant="outline" size="sm" onClick={() => setBackupOpen(true)}>
        Backup
      </Button>

      <MigrateDialog vm={vm} open={migrateOpen} onOpenChange={setMigrateOpen} />
      <RescheduleDialog vm={vm} open={rescheduleOpen} onOpenChange={setRescheduleOpen} />
      <BackupDialog vm={vm} open={backupOpen} onOpenChange={setBackupOpen} />
    </>
  )
}

function HostPicker({
  vm,
  value,
  onChange,
}: {
  vm: Vm
  value: string | undefined
  onChange: (v: string) => void
}) {
  const { data: hosts = [] } = useHosts()
  // Exclude the VM's current host; only healthy hosts are valid targets.
  const candidates = hosts.filter((h) => h.id !== vm.host_id && h.status === "healthy")
  return (
    <div className="space-y-2">
      <Label>Target host</Label>
      <Select value={value} onValueChange={onChange}>
        <SelectTrigger>
          <SelectValue
            placeholder={candidates.length ? "Select target host" : "No other healthy host available"}
          />
        </SelectTrigger>
        <SelectContent>
          {candidates.map((h) => (
            <SelectItem key={h.id} value={h.id}>
              {h.name} ({h.vm_count} VMs)
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      <p className="text-xs text-muted-foreground">
        Requires the disk on shared storage (iSCSI / NFS / SPDK / TrueNAS).
      </p>
    </div>
  )
}

function MigrateDialog({ vm, open, onOpenChange }: { vm: Vm; open: boolean; onOpenChange: (o: boolean) => void }) {
  const [target, setTarget] = useState<string>()
  const [port, setPort] = useState("54321")
  const migrate = useMigrateVM()
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>Live migrate VM</DialogTitle>
          <DialogDescription>
            Transfer this running VM to another host with zero downtime via QEMU live migration.
          </DialogDescription>
        </DialogHeader>
        <div className="space-y-4 py-2">
          <HostPicker vm={vm} value={target} onChange={setTarget} />
          <div className="space-y-2">
            <Label htmlFor="migrate-port">Migration port</Label>
            <Input id="migrate-port" value={port} onChange={(e) => setPort(e.target.value)} />
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>Cancel</Button>
          <Button
            disabled={!target || migrate.isPending}
            onClick={() =>
              migrate.mutate(
                { id: vm.id, targetHostId: target!, targetPort: Number(port) || 54321 },
                { onSuccess: () => onOpenChange(false) }
              )
            }
          >
            {migrate.isPending ? "Migrating…" : "Migrate"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

function RescheduleDialog({ vm, open, onOpenChange }: { vm: Vm; open: boolean; onOpenChange: (o: boolean) => void }) {
  const [target, setTarget] = useState<string>()
  const reschedule = useRescheduleVM()
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>Reschedule VM</DialogTitle>
          <DialogDescription>
            Rebuild this VM on another host (HA recovery). The source is assumed
            gone; the disk must live on shared storage.
          </DialogDescription>
        </DialogHeader>
        <div className="space-y-4 py-2">
          <HostPicker vm={vm} value={target} onChange={setTarget} />
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>Cancel</Button>
          <Button
            disabled={!target || reschedule.isPending}
            onClick={() =>
              reschedule.mutate(
                { id: vm.id, targetHostId: target! },
                { onSuccess: () => onOpenChange(false) }
              )
            }
          >
            {reschedule.isPending ? "Rescheduling…" : "Reschedule"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

function BackupDialog({ vm, open, onOpenChange }: { vm: Vm; open: boolean; onOpenChange: (o: boolean) => void }) {
  const [destination, setDestination] = useState(`/srv/backups/${vm.name}-${Date.now()}.qcow2`)
  const [compress, setCompress] = useState(true)
  const backup = useBackupVM()
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>Back up VM</DialogTitle>
          <DialogDescription>
            Snapshot the VM disk to a backup file on the agent host (network
            share recommended). Volume-backed VMs use the chunked backup
            pipeline automatically.
          </DialogDescription>
        </DialogHeader>
        <div className="space-y-4 py-2">
          <div className="space-y-2">
            <Label htmlFor="backup-dest">Destination path (overlay VMs)</Label>
            <Input id="backup-dest" value={destination} onChange={(e) => setDestination(e.target.value)} />
          </div>
          <div className="flex items-center gap-2">
            <input
              type="checkbox"
              id="backup-compress"
              checked={compress}
              onChange={(e) => setCompress(e.target.checked)}
              className="h-4 w-4"
            />
            <Label htmlFor="backup-compress" className="cursor-pointer">Compress (qemu-img -c)</Label>
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>Cancel</Button>
          <Button
            disabled={backup.isPending}
            onClick={() =>
              backup.mutate(
                { id: vm.id, destinationPath: destination, compress },
                { onSuccess: () => onOpenChange(false) }
              )
            }
          >
            {backup.isPending ? "Backing up…" : "Back up"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
