import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import type { VM } from "@/lib/types"

interface VMConfigProps {
  vm: VM
}

export function VMConfig({ vm }: VMConfigProps) {
  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle>Machine Configuration</CardTitle>
        </CardHeader>
        <CardContent>
          <dl className="grid grid-cols-2 gap-4">
            <div>
              <dt className="text-sm font-medium text-muted-foreground">vCPU Count</dt>
              <dd className="mt-1 text-sm">{vm.vcpu}</dd>
            </div>
            <div>
              <dt className="text-sm font-medium text-muted-foreground">Memory</dt>
              <dd className="mt-1 text-sm">{vm.mem_mib} MiB</dd>
            </div>
            <div>
              <dt className="text-sm font-medium text-muted-foreground">CPU Template</dt>
              <dd className="mt-1 text-sm">Default</dd>
            </div>
            <div>
              <dt className="text-sm font-medium text-muted-foreground">SMT Enabled</dt>
              <dd className="mt-1 text-sm">No</dd>
            </div>
          </dl>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Boot Source</CardTitle>
        </CardHeader>
        <CardContent>
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
        </CardContent>
      </Card>
    </div>
  )
}
