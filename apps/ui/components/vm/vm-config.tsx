import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import type { VM } from "@/lib/types"

interface VMConfigProps {
  vm: VM
}

function guestOsLabel(os?: string): string {
  switch (os) {
    case "windows":
      return "Windows"
    case "linux_disk":
      return "Linux"
    case "linux_kernel":
      return "Linux (kernel)"
    case "other":
      return "Other"
    default:
      return "—"
  }
}

export function VMConfig({ vm }: VMConfigProps) {
  const isQemu = vm.vmm_kind === "qemu"

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle>Machine Configuration</CardTitle>
        </CardHeader>
        <CardContent>
          <dl className="grid grid-cols-2 gap-4">
            <div>
              <dt className="text-sm font-medium text-muted-foreground">Backend</dt>
              <dd className="mt-1 text-sm">{isQemu ? "QEMU (full VM)" : "Firecracker (microVM)"}</dd>
            </div>
            <div>
              <dt className="text-sm font-medium text-muted-foreground">vCPU Count</dt>
              <dd className="mt-1 text-sm">{vm.vcpu}</dd>
            </div>
            <div>
              <dt className="text-sm font-medium text-muted-foreground">Memory</dt>
              <dd className="mt-1 text-sm">{vm.mem_mib} MiB</dd>
            </div>
            {isQemu ? (
              <>
                <div>
                  <dt className="text-sm font-medium text-muted-foreground">Guest OS</dt>
                  <dd className="mt-1 text-sm">{guestOsLabel(vm.guest_os)}</dd>
                </div>
                <div>
                  <dt className="text-sm font-medium text-muted-foreground">CPU Model</dt>
                  <dd className="mt-1 text-sm font-mono">{vm.cpu_type || "host"}</dd>
                </div>
                <div>
                  <dt className="text-sm font-medium text-muted-foreground">Firmware</dt>
                  <dd className="mt-1 text-sm">UEFI (OVMF)</dd>
                </div>
                <div>
                  <dt className="text-sm font-medium text-muted-foreground">Machine Type</dt>
                  <dd className="mt-1 text-sm">q35</dd>
                </div>
                <div>
                  <dt className="text-sm font-medium text-muted-foreground">Console</dt>
                  <dd className="mt-1 text-sm">
                    {vm.console_kind === "vnc" ? "Graphical (VNC)" : "Serial"}
                  </dd>
                </div>
              </>
            ) : (
              <>
                <div>
                  <dt className="text-sm font-medium text-muted-foreground">CPU Template</dt>
                  <dd className="mt-1 text-sm">Default</dd>
                </div>
                <div>
                  <dt className="text-sm font-medium text-muted-foreground">SMT Enabled</dt>
                  <dd className="mt-1 text-sm">No</dd>
                </div>
              </>
            )}
          </dl>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Boot Source</CardTitle>
        </CardHeader>
        <CardContent>
          {isQemu ? (
            <dl className="space-y-4">
              <div>
                <dt className="text-sm font-medium text-muted-foreground">Boot Disk</dt>
                <dd className="mt-1 text-sm font-mono bg-muted px-2 py-1 rounded break-all">
                  {vm.rootfs_path || "—"}
                </dd>
              </div>
              <div>
                <dt className="text-sm font-medium text-muted-foreground">Boot Order</dt>
                <dd className="mt-1 text-sm text-muted-foreground">
                  UEFI — disk first, then any attached installer ISO/CD-ROM.
                </dd>
              </div>
            </dl>
          ) : (
            <dl className="space-y-4">
              <div>
                <dt className="text-sm font-medium text-muted-foreground">Kernel Path</dt>
                <dd className="mt-1 text-sm font-mono bg-muted px-2 py-1 rounded">{vm.kernel_path}</dd>
              </div>
              <div>
                <dt className="text-sm font-medium text-muted-foreground">Rootfs Path</dt>
                <dd className="mt-1 text-sm font-mono bg-muted px-2 py-1 rounded">{vm.rootfs_path}</dd>
              </div>
              <div>
                <dt className="text-sm font-medium text-muted-foreground">Initrd Path</dt>
                <dd className="mt-1 text-sm text-muted-foreground">Not configured</dd>
              </div>
              <div>
                <dt className="text-sm font-medium text-muted-foreground">Boot Args</dt>
                <dd className="mt-1 text-sm text-muted-foreground">Default</dd>
              </div>
            </dl>
          )}
        </CardContent>
      </Card>
    </div>
  )
}
