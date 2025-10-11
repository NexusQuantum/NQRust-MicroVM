"use client"

import { useState } from "react"
import type { VM } from "@/types/firecracker"
import type { Vm } from "@/types/nexus"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Switch } from "@/components/ui/switch"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Textarea } from "@/components/ui/textarea"
import { Separator } from "@/components/ui/separator"
import { AlertBanner } from "@/components/alert-banner"
import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import { machineConfigSchema, bootSourceSchema } from "@/lib/validators"
// Configuration updates not supported in new backend
import { Settings, Play, Save, RotateCcw } from "lucide-react"
import type { z } from "zod"

interface VMConfigTabProps {
  vm: VM | Vm // Support both old and new VM types
}

type MachineConfigForm = z.infer<typeof machineConfigSchema>
type BootSourceForm = z.infer<typeof bootSourceSchema>

export function VMConfigTab({ vm }: VMConfigTabProps) {
  const [activeSection, setActiveSection] = useState<"machine" | "boot">("machine")

  // Configuration updates not supported in new backend

  const canEdit = vm.state === "stopped"

  // Adapt to new backend structure
  const newVm = vm as Vm
  const oldVm = vm as VM
  
  const machine = {
    vcpu_count: newVm.vcpu || oldVm.config?.machine?.vcpu_count,
    mem_size_mib: newVm.mem_mib || oldVm.config?.machine?.mem_size_mib,
    smt: false, // Not configurable in new backend
    cpu_template: "None", // Not configurable in new backend
  }
  
  const boot = {
    kernel_image_path: newVm.kernel_path || oldVm.config?.boot?.kernel_image_path,
    initrd_path: undefined, // Not available in new backend
    boot_args: undefined, // Not available in new backend
  }

  // Machine config form
  const machineForm = useForm<MachineConfigForm>({
    resolver: zodResolver(machineConfigSchema),
    defaultValues: {
      vcpu_count: machine?.vcpu_count ?? 1,
      mem_size_mib: machine?.mem_size_mib ?? 128,
      smt: !!machine?.smt,
      cpu_template: machine?.cpu_template ?? "None",
    },
  })

  // Boot source form
  const bootForm = useForm<BootSourceForm>({
    resolver: zodResolver(bootSourceSchema),
    defaultValues: {
      kernel_image_path: boot?.kernel_image_path ?? "",
      initrd_path: boot?.initrd_path ?? "",
      boot_args: boot?.boot_args ?? "",
    },
  })

  const onMachineSubmit = (_data: MachineConfigForm) => {}

  const onBootSubmit = (_data: BootSourceForm) => {}

  const resetMachineForm = () => {
    machineForm.reset()
  }

  const resetBootForm = () => {
    bootForm.reset()
  }

  return (
    <div className="space-y-6">
      {/* Guardrail Alert */}
      {!canEdit && (
        <AlertBanner
          type="warning"
          title="Configuration Locked"
          message="VM must be stopped to modify configuration. Stop the VM to enable editing."
        />
      )}

      {/* Section Selector */}
      <div className="flex space-x-1 bg-muted p-1 rounded-lg w-fit">
        <Button
          variant={activeSection === "machine" ? "default" : "ghost"}
          size="sm"
          onClick={() => setActiveSection("machine")}
        >
          <Settings className="h-4 w-4 mr-2" />
          Machine Config
        </Button>
        <Button
          variant={activeSection === "boot" ? "default" : "ghost"}
          size="sm"
          onClick={() => setActiveSection("boot")}
        >
          <Play className="h-4 w-4 mr-2" />
          Boot Source
        </Button>
      </div>

      {/* Machine Configuration */}
      {activeSection === "machine" && (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Settings className="h-5 w-5" />
              Machine Configuration
            </CardTitle>
          </CardHeader>
          <CardContent>
            <form onSubmit={machineForm.handleSubmit(onMachineSubmit)} className="space-y-6">
              <div className="grid gap-6 md:grid-cols-2">
                {/* vCPU Count */}
                <div className="space-y-2">
                  <Label htmlFor="vcpu_count">vCPU Count</Label>
                  <Input
                    id="vcpu_count"
                    type="number"
                    min="1"
                    max="32"
                    disabled={!canEdit}
                    {...machineForm.register("vcpu_count", { valueAsNumber: true })}
                  />
                  {machineForm.formState.errors.vcpu_count && (
                    <p className="text-sm text-destructive">{machineForm.formState.errors.vcpu_count.message}</p>
                  )}
                  <p className="text-xs text-muted-foreground">Number of virtual CPUs (1-32)</p>
                </div>

                {/* Memory Size */}
                <div className="space-y-2">
                  <Label htmlFor="mem_size_mib">Memory (MiB)</Label>
                  <Input
                    id="mem_size_mib"
                    type="number"
                    min="128"
                    disabled={!canEdit}
                    {...machineForm.register("mem_size_mib", { valueAsNumber: true })}
                  />
                  {machineForm.formState.errors.mem_size_mib && (
                    <p className="text-sm text-destructive">{machineForm.formState.errors.mem_size_mib.message}</p>
                  )}
                  <p className="text-xs text-muted-foreground">Memory size in MiB (minimum 128)</p>
                </div>

                {/* CPU Template */}
                <div className="space-y-2">
                  <Label htmlFor="cpu_template">CPU Template</Label>
                  <Select
                    disabled={!canEdit}
                    value={machineForm.watch("cpu_template")}
                    onValueChange={(value) => machineForm.setValue("cpu_template", value)}
                  >
                    <SelectTrigger>
                      <SelectValue placeholder="Select CPU template" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="None">None</SelectItem>
                      <SelectItem value="C3">C3</SelectItem>
                      <SelectItem value="T2">T2</SelectItem>
                    </SelectContent>
                  </Select>
                  <p className="text-xs text-muted-foreground">CPU template for guest compatibility</p>
                </div>

                {/* SMT */}
                <div className="space-y-2">
                  <div className="flex items-center justify-between">
                    <Label htmlFor="smt">Simultaneous Multithreading (SMT)</Label>
                    <Switch
                      id="smt"
                      disabled={!canEdit}
                      checked={machineForm.watch("smt")}
                      onCheckedChange={(checked) => machineForm.setValue("smt", checked)}
                    />
                  </div>
                  <p className="text-xs text-muted-foreground">Enable SMT for better CPU utilization</p>
                </div>
              </div>

              <Separator />

              {/* Form Actions */}
              <div className="flex justify-between">
                <Button type="button" variant="outline" onClick={resetMachineForm} disabled={!canEdit}>
                  <RotateCcw className="h-4 w-4 mr-2" />
                  Reset
                </Button>

                <Button
                  type="submit"
                  disabled={!canEdit || updateMachineConfig.isPending || !machineForm.formState.isDirty}
                >
                  <Save className="h-4 w-4 mr-2" />
                  {updateMachineConfig.isPending ? "Saving..." : "Save Changes"}
                </Button>
              </div>
            </form>
          </CardContent>
        </Card>
      )}

      {/* Boot Source Configuration */}
      {activeSection === "boot" && (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Play className="h-5 w-5" />
              Boot Source Configuration
            </CardTitle>
          </CardHeader>
          <CardContent>
            <form onSubmit={bootForm.handleSubmit(onBootSubmit)} className="space-y-6">
              <div className="space-y-6">
                {/* Kernel Image Path */}
                <div className="space-y-2">
                  <Label htmlFor="kernel_image_path">Kernel Image Path</Label>
                  <Input id="kernel_image_path" disabled={!canEdit} {...bootForm.register("kernel_image_path")} />
                  {bootForm.formState.errors.kernel_image_path && (
                    <p className="text-sm text-destructive">{bootForm.formState.errors.kernel_image_path.message}</p>
                  )}
                  <p className="text-xs text-muted-foreground">Path to the kernel image file</p>
                </div>

                {/* Initial RAM Disk Path */}
                <div className="space-y-2">
                  <Label htmlFor="initrd_path">Initial RAM Disk Path (Optional)</Label>
                  <Input id="initrd_path" disabled={!canEdit} {...bootForm.register("initrd_path")} />
                  {bootForm.formState.errors.initrd_path && (
                    <p className="text-sm text-destructive">{bootForm.formState.errors.initrd_path.message}</p>
                  )}
                  <p className="text-xs text-muted-foreground">Path to the initial RAM disk file</p>
                </div>

                {/* Boot Arguments */}
                <div className="space-y-2">
                  <Label htmlFor="boot_args">Boot Arguments (Optional)</Label>
                  <Textarea id="boot_args" disabled={!canEdit} rows={3} {...bootForm.register("boot_args")} />
                  {bootForm.formState.errors.boot_args && (
                    <p className="text-sm text-destructive">{bootForm.formState.errors.boot_args.message}</p>
                  )}
                  <p className="text-xs text-muted-foreground">Kernel command line arguments</p>
                </div>
              </div>

              <Separator />

              {/* Form Actions */}
              <div className="flex justify-between">
                <Button type="button" variant="outline" onClick={resetBootForm} disabled={!canEdit}>
                  <RotateCcw className="h-4 w-4 mr-2" />
                  Reset
                </Button>

                <Button type="submit" disabled={!canEdit || updateBootSource.isPending || !bootForm.formState.isDirty}>
                  <Save className="h-4 w-4 mr-2" />
                  {updateBootSource.isPending ? "Saving..." : "Save Changes"}
                </Button>
              </div>
            </form>
          </CardContent>
        </Card>
      )}
    </div>
  )
}
