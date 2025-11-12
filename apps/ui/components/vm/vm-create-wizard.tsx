"use client"

import { useState, useEffect, useMemo } from "react"
import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import { usePreferences } from "@/lib/queries"
import { z } from "zod"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Slider } from "@/components/ui/slider"
import { Checkbox } from "@/components/ui/checkbox"
import { Textarea } from "@/components/ui/textarea"
import { Separator } from "@/components/ui/separator"
import { ChevronLeft, ChevronRight } from "lucide-react"
import { useCreateVM } from "@/lib/queries"
import type { CreateVmReq } from "@/lib/types"

const steps = ["Basic Info", "Credentials", "Machine Config", "Boot Source", "Network", "Review"]

// Validation schema
const vmCreationSchema = z.object({
  name: z.string().min(1, "VM Name is required").max(50, "Name too long"),
  description: z.string().max(200, "Description too long").optional(),
  environment: z.enum(["development", "staging", "production"]),
  owner: z.string().min(1, "Owner is required").default("developer"),

  // Credentials
  username: z.string().min(1, "Username is required").max(32, "Username too long").default("root"),
  password: z.string().min(1, "Password is required").max(128, "Password too long"),

  // Machine config
  vcpu: z.number().min(1, "Minimum 1 vCPU").max(32, "Maximum 32 vCPUs"),
  memory: z
    .number()
    .min(128, "Minimum 128 MiB")
    .max(32768, "Maximum 32768 MiB")
    .refine((n) => n % 128 === 0, {
      message: "Memory must be a multiple of 128 MiB",
    }),
  smtEnabled: z.boolean().default(false),
  trackDirtyPages: z.boolean().default(false),

  // Boot source
  kernelPath: z.string().min(1, "Kernel image is required"),
  rootfsPath: z.string().min(1, "Rootfs image is required"),
  initrdPath: z.string().optional(),
  bootArgs: z.string().optional(),

  // Network (optional)
  enableNetwork: z.boolean().default(true),
  hostDevice: z.string().optional(),
  guestMac: z.string().optional(),
})

type VMCreationForm = z.infer<typeof vmCreationSchema>

interface VMCreateWizardProps {
  onComplete?: () => void
  onCancel?: () => void
}

export function VMCreateWizard({ onComplete, onCancel }: VMCreateWizardProps) {
  const [currentStep, setCurrentStep] = useState(0)
  const createVM = useCreateVM()

  // Load user preferences for VM defaults
  const { data: preferences } = usePreferences()

  // Load kernel and rootfs options from backend
  const [kernelOptions, setKernelOptions] = useState<{ name: string; path: string; id: string }[]>([])
  const [rootfsOptions, setRootfsOptions] = useState<{ name: string; path: string; id: string }[]>([])

  // Use react-hook-form with zod validation
  const {
    register,
    handleSubmit,
    watch,
    setValue,
    trigger,
    formState: { errors, isSubmitting },
  } = useForm<VMCreationForm>({
    resolver: zodResolver(vmCreationSchema) as any,
    mode: "onChange",
    defaultValues: {
      name: "",
      owner: "developer",
      environment: "development",
      description: "",
      username: "root",
      password: "",
      vcpu: preferences?.vm_defaults?.vcpu || 2,
      memory: preferences?.vm_defaults?.mem_mib || 2048,
      smtEnabled: false,
      trackDirtyPages: false,
      kernelPath: "",
      rootfsPath: "",
      initrdPath: "",
      bootArgs: "",
      enableNetwork: true,
      hostDevice: "tap0",
      guestMac: "",
    },
  })

  const formData = watch()

  // Update form defaults when preferences load
  useEffect(() => {
    if (preferences?.vm_defaults) {
      setValue('vcpu', preferences.vm_defaults.vcpu || 2, { shouldValidate: false })
      setValue('memory', preferences.vm_defaults.mem_mib || 2048, { shouldValidate: false })
    }
  }, [preferences, setValue])

  useEffect(() => {
    (async () => {
      try {
        const baseUrl = process.env.NEXT_PUBLIC_API_BASE_URL || "/api/proxy/v1"

        // Get all images and filter by kind
        const allImagesRes = await fetch(`${baseUrl}/images`)
        const allImagesData = await allImagesRes.json()
        console.log('All images response:', allImagesData)

        // Filter kernels (kind === "kernel")
        const kernels = (allImagesData.items || [])
          .filter((i: any) => i.kind === 'kernel')
          .map((i: any) => ({
            name: i.name,
            path: i.host_path,
            id: i.id,
            kind: i.kind
          }))
        console.log('Kernel options:', kernels)
        setKernelOptions(kernels)

        // Filter rootfs (kind === "rootfs")
        const rootfs = (allImagesData.items || [])
          .filter((i: any) => i.kind === 'rootfs')
          .map((i: any) => ({
            name: i.name,
            path: i.host_path,
            id: i.id,
            kind: i.kind
          }))
        console.log('Rootfs options:', rootfs)
        setRootfsOptions(rootfs)

        // Auto-select first available images for convenience (only on mount)
        if (kernels.length > 0) {
          setValue('kernelPath', kernels[0].path, { shouldValidate: false })
        }
        if (rootfs.length > 0) {
          setValue('rootfsPath', rootfs[0].path, { shouldValidate: false })
        }
      } catch (e) {
        console.error('Failed to fetch images:', e)
      }
    })()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []) // Only run once on mount

  // Per-step validation schemas
  const stepSchemaFields = {
    name: z.string().min(1, "VM Name is required").max(50, "Name too long"),
    description: z.string().max(200, "Description too long").optional(),
    environment: z.enum(["development", "staging", "production"]),
    owner: z.string().min(1, "Owner is required"),
    username: z.string().min(1, "Username is required").max(32, "Username too long"),
    password: z.string().min(1, "Password is required").max(128, "Password too long"),
    vcpu: z.number().min(1, "Minimum 1 vCPU").max(32, "Maximum 32 vCPUs"),
    memory: z.number().min(128, "Minimum 128 MiB").max(32768, "Maximum 32768 MiB"),
    smtEnabled: z.boolean().default(false),
    trackDirtyPages: z.boolean().default(false),
    kernelPath: z.string().min(1, "Kernel image is required"),
    rootfsPath: z.string().min(1, "Rootfs image is required"),
    initrdPath: z.string().optional(),
    bootArgs: z.string().optional(),
    enableNetwork: z.boolean().default(false),
    hostDevice: z.string().optional(),
    guestMac: z.string().optional(),
  }

  const stepSchemas = useMemo(
    () => [
      // Step 0: Basic Info
      z.object({
        name: stepSchemaFields.name,
        owner: stepSchemaFields.owner,
        environment: stepSchemaFields.environment,
        description: stepSchemaFields.description,
      }),
      // Step 1: Credentials
      z.object({
        username: stepSchemaFields.username,
        password: stepSchemaFields.password,
      }),
      // Step 2: Machine Config
      z.object({
        vcpu: stepSchemaFields.vcpu,
        memory: stepSchemaFields.memory,
        smtEnabled: stepSchemaFields.smtEnabled,
        trackDirtyPages: stepSchemaFields.trackDirtyPages,
      }),
      // Step 3: Boot Source
      z.object({
        kernelPath: stepSchemaFields.kernelPath,
        rootfsPath: stepSchemaFields.rootfsPath,
        initrdPath: stepSchemaFields.initrdPath,
        bootArgs: stepSchemaFields.bootArgs,
      }),
      // Step 4: Network (all optional)
      z.object({
        enableNetwork: stepSchemaFields.enableNetwork,
        hostDevice: stepSchemaFields.hostDevice,
        guestMac: stepSchemaFields.guestMac,
      }),
      z.any(), // Review step
    ],
    []
  )

  const canProceed = stepSchemas[currentStep]?.safeParse(formData).success ?? true
  const allValid = useMemo(() => vmCreationSchema.safeParse(formData).success, [formData])

  const nextStep = async () => {
    // Get the fields to validate for the current step
    const fieldsToValidate: (keyof VMCreationForm)[] = []

    if (currentStep === 0) {
      fieldsToValidate.push('name', 'owner', 'environment', 'description')
    } else if (currentStep === 1) {
      fieldsToValidate.push('username', 'password')
    } else if (currentStep === 2) {
      fieldsToValidate.push('vcpu', 'memory', 'smtEnabled', 'trackDirtyPages')
    } else if (currentStep === 3) {
      fieldsToValidate.push('kernelPath', 'rootfsPath', 'initrdPath', 'bootArgs')
    } else if (currentStep === 4) {
      fieldsToValidate.push('enableNetwork', 'hostDevice', 'guestMac')
    }

    // Trigger validation only for current step fields
    const isValid = await trigger(fieldsToValidate)

    // Only proceed if validation passes
    if (isValid && currentStep < steps.length - 1) {
      setCurrentStep(currentStep + 1)
    }
    // If validation fails, errors will be shown automatically
  }

  const prevStep = () => {
    if (currentStep > 0) {
      setCurrentStep(currentStep - 1)
    }
  }

  const generateMac = () => {
    const mac = Array.from({ length: 6 }, () =>
      Math.floor(Math.random() * 256)
        .toString(16)
        .padStart(2, "0"),
    ).join(":")
    setValue('guestMac', mac)
  }

  const onSubmit = async (data: VMCreationForm) => {
    // Only allow submission on the final review step
    if (currentStep !== 5) {
      console.log('Form submitted but not on final step, ignoring')
      return
    }

    try {
      const vmReq: CreateVmReq = {
        name: data.name,
        vcpu: data.vcpu,
        mem_mib: data.memory,
        kernel_path: data.kernelPath,
        rootfs_path: data.rootfsPath,
        username: data.username,
        password: data.password,
      }

      console.log('Creating VM with:', vmReq)
      await createVM.mutateAsync(vmReq)
      onComplete?.()
    } catch (error) {
      console.error('VM creation failed:', error)
    }
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        {steps.map((step, index) => (
          <div key={step} className="flex items-center">
            <div
              className={`flex h-10 w-10 items-center justify-center rounded-full border-2 ${
                index === currentStep
                  ? "border-primary bg-primary text-primary-foreground"
                  : index < currentStep
                    ? "border-primary bg-primary text-primary-foreground"
                    : "border-muted bg-muted text-muted-foreground"
              }`}
            >
              {index + 1}
            </div>
            <div className="ml-2 text-sm font-medium">{step}</div>
            {index < steps.length - 1 && <div className="mx-4 h-0.5 w-12 bg-muted" />}
          </div>
        ))}
      </div>

      <form onSubmit={handleSubmit(onSubmit)}>
        <Card>
          <CardHeader>
            <CardTitle>{steps[currentStep]}</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            {currentStep === 0 && (
              <>
                <div className="space-y-2">
                  <Label htmlFor="name">
                    Name <span className="text-destructive">*</span>
                  </Label>
                  <Input
                    id="name"
                    {...register("name")}
                    placeholder="e.g., my-vm-001"
                    aria-required="true"
                    aria-invalid={!!errors.name}
                  />
                  {errors.name && <p className="text-sm text-red-600">{errors.name.message}</p>}
                </div>
                <div className="space-y-2">
                  <Label htmlFor="owner">
                    Owner <span className="text-destructive">*</span>
                  </Label>
                  <Input
                    id="owner"
                    {...register("owner")}
                    aria-required="true"
                    aria-invalid={!!errors.owner}
                    className={formData.owner === "developer" ? "text-foreground" : ""}
                  />
                  <p className="text-xs text-muted-foreground">Default: developer</p>
                  {errors.owner && <p className="text-sm text-red-600">{errors.owner.message}</p>}
                </div>
                <div className="space-y-2">
                  <Label htmlFor="environment">
                    Environment <span className="text-destructive">*</span>
                  </Label>
                  <Select value={formData.environment} onValueChange={(value) => setValue("environment", value as any)}>
                    <SelectTrigger aria-required="true">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="development">Development</SelectItem>
                      <SelectItem value="staging">Staging</SelectItem>
                      <SelectItem value="production">Production</SelectItem>
                    </SelectContent>
                  </Select>
                  <p className="text-xs text-muted-foreground">Default: development</p>
                </div>
                <div className="space-y-2">
                  <Label htmlFor="description">Description</Label>
                  <Textarea
                    id="description"
                    {...register("description")}
                    placeholder="e.g., Development VM for testing features"
                  />
                  {errors.description && <p className="text-sm text-red-600">{errors.description.message}</p>}
                </div>
              </>
            )}

          {currentStep === 1 && (
            <>
              <div className="space-y-2">
                <Label htmlFor="username">
                  Username <span className="text-destructive">*</span>
                </Label>
                <Input
                  id="username"
                  {...register("username")}
                  aria-required="true"
                  aria-invalid={!!errors.username}
                />
                <p className="text-xs text-muted-foreground">Default: root</p>
                {errors.username && <p className="text-sm text-red-600">{errors.username.message}</p>}
              </div>
              <div className="space-y-2">
                <Label htmlFor="password">
                  Password <span className="text-destructive">*</span>
                </Label>
                <Input
                  id="password"
                  type="password"
                  {...register("password")}
                  placeholder="Enter secure password"
                  aria-required="true"
                  aria-invalid={!!errors.password}
                />
                {errors.password && <p className="text-sm text-red-600">{errors.password.message}</p>}
              </div>
            </>
          )}

          {currentStep === 2 && (
            <>
              <div className="space-y-2">
                <Label>vCPU Count: {formData.vcpu}</Label>
                <Slider
                  value={[formData.vcpu]}
                  onValueChange={(v) => setValue("vcpu", v[0])}
                  min={1}
                  max={32}
                  step={1}
                />
                {errors.vcpu && <p className="text-sm text-red-600">{errors.vcpu.message}</p>}
              </div>
              <div className="space-y-2">
                <Label>Memory: {formData.memory} MiB</Label>
                <Slider
                  value={[formData.memory]}
                  onValueChange={(v) => setValue("memory", v[0])}
                  min={128}
                  max={32768}
                  step={128}
                />
                <p className="text-xs text-muted-foreground">Must be a multiple of 128 MiB</p>
                {errors.memory && <p className="text-sm text-red-600">{errors.memory.message}</p>}
              </div>
              <div className="flex items-center space-x-2">
                <Checkbox
                  id="smt"
                  checked={formData.smtEnabled}
                  onCheckedChange={(checked) => setValue("smtEnabled", checked as boolean)}
                />
                <Label htmlFor="smt" className="text-sm font-normal">
                  Enable SMT (Simultaneous Multithreading)
                </Label>
              </div>
              <div className="flex items-center space-x-2">
                <Checkbox
                  id="dirty-pages"
                  checked={formData.trackDirtyPages}
                  onCheckedChange={(checked) => setValue("trackDirtyPages", checked as boolean)}
                />
                <Label htmlFor="dirty-pages" className="text-sm font-normal">
                  Track dirty pages
                </Label>
              </div>
            </>
          )}

          {currentStep === 3 && (
            <>
              <div className="space-y-2">
                <Label htmlFor="kernel">
                  Kernel Image <span className="text-destructive">*</span>
                </Label>
                <Select
                  value={formData.kernelPath}
                  onValueChange={(value) => setValue("kernelPath", value)}
                >
                  <SelectTrigger id="kernel" aria-required="true" aria-invalid={!!errors.kernelPath}>
                    <SelectValue placeholder={kernelOptions.length > 0 ? "Select kernel image" : "No kernel images available"} />
                  </SelectTrigger>
                  <SelectContent>
                    {kernelOptions.length > 0 ? (
                      kernelOptions.map((kernel) => (
                        <SelectItem key={kernel.id} value={kernel.path}>
                          {kernel.name} <span className="text-muted-foreground text-xs">({kernel.kind})</span>
                        </SelectItem>
                      ))
                    ) : (
                      <div className="px-2 py-1.5 text-sm text-muted-foreground">No kernel images found</div>
                    )}
                  </SelectContent>
                </Select>
                <p className="text-xs text-muted-foreground">
                  {kernelOptions.length > 0 ? `${kernelOptions.length} kernel(s) available` : 'Upload kernel images first'}
                </p>
                {errors.kernelPath && <p className="text-sm text-red-600">{errors.kernelPath.message}</p>}
              </div>
              <div className="space-y-2">
                <Label htmlFor="rootfs">
                  Rootfs Image <span className="text-destructive">*</span>
                </Label>
                <Select
                  value={formData.rootfsPath}
                  onValueChange={(value) => setValue("rootfsPath", value)}
                >
                  <SelectTrigger id="rootfs" aria-required="true" aria-invalid={!!errors.rootfsPath}>
                    <SelectValue placeholder={rootfsOptions.length > 0 ? "Select rootfs image" : "No rootfs images available"} />
                  </SelectTrigger>
                  <SelectContent>
                    {rootfsOptions.length > 0 ? (
                      rootfsOptions.map((rootfs) => (
                        <SelectItem key={rootfs.id} value={rootfs.path}>
                          {rootfs.name} <span className="text-muted-foreground text-xs">({rootfs.kind})</span>
                        </SelectItem>
                      ))
                    ) : (
                      <div className="px-2 py-1.5 text-sm text-muted-foreground">No rootfs images found</div>
                    )}
                  </SelectContent>
                </Select>
                <p className="text-xs text-muted-foreground">
                  {rootfsOptions.length > 0 ? `${rootfsOptions.length} rootfs image(s) available` : 'Upload rootfs images first'}
                </p>
                {errors.rootfsPath && <p className="text-sm text-red-600">{errors.rootfsPath.message}</p>}
              </div>
              <div className="space-y-2">
                <Label htmlFor="initrd">Initrd Path (Optional)</Label>
                <Input id="initrd" {...register("initrdPath")} />
              </div>
              <div className="space-y-2">
                <Label htmlFor="boot-args">Boot Arguments (Optional)</Label>
                <Input id="boot-args" {...register("bootArgs")} />
              </div>
            </>
          )}

          {currentStep === 4 && (
            <>
              <div className="flex items-center space-x-2">
                <Checkbox
                  id="enable-network"
                  checked={formData.enableNetwork}
                  onCheckedChange={(checked) => setValue("enableNetwork", checked as boolean)}
                />
                <Label htmlFor="enable-network" className="text-sm font-normal">
                  Enable networking
                </Label>
              </div>
              <p className="text-xs text-muted-foreground">Default: enabled</p>
              {formData.enableNetwork && (
                <>
                  <div className="space-y-2">
                    <Label htmlFor="host-device">Host Device Name</Label>
                    <Input id="host-device" {...register("hostDevice")} placeholder="e.g., tap0" />
                    <p className="text-xs text-muted-foreground">Default: tap0</p>
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="guest-mac">Guest MAC Address</Label>
                    <div className="flex gap-2">
                      <Input id="guest-mac" {...register("guestMac")} placeholder="e.g., AA:FC:00:00:00:01" />
                      <Button type="button" variant="outline" onClick={generateMac}>
                        Generate
                      </Button>
                    </div>
                    <p className="text-xs text-muted-foreground">Leave empty for auto-generation</p>
                  </div>
                </>
              )}
            </>
          )}

          {currentStep === 5 && (
            <div className="space-y-4">
              <div className="rounded-lg border border-border p-4 space-y-3">
                <h3 className="font-medium">Basic Information</h3>
                <dl className="grid grid-cols-2 gap-2 text-sm">
                  <dt className="text-muted-foreground">Name:</dt>
                  <dd>{formData.name || "—"}</dd>
                  <dt className="text-muted-foreground">Owner:</dt>
                  <dd>{formData.owner || "—"}</dd>
                  <dt className="text-muted-foreground">Environment:</dt>
                  <dd>{formData.environment || "—"}</dd>
                  <dt className="text-muted-foreground">Description:</dt>
                  <dd>{formData.description || "—"}</dd>
                </dl>
              </div>

              <div className="rounded-lg border border-border p-4 space-y-3">
                <h3 className="font-medium">Machine Configuration</h3>
                <dl className="grid grid-cols-2 gap-2 text-sm">
                  <dt className="text-muted-foreground">vCPU:</dt>
                  <dd>{formData.vcpu}</dd>
                  <dt className="text-muted-foreground">Memory:</dt>
                  <dd>{formData.memory} MiB</dd>
                  <dt className="text-muted-foreground">SMT:</dt>
                  <dd>{formData.smtEnabled ? "Enabled" : "Disabled"}</dd>
                  <dt className="text-muted-foreground">Track Dirty Pages:</dt>
                  <dd>{formData.trackDirtyPages ? "Yes" : "No"}</dd>
                </dl>
              </div>

              <div className="rounded-lg border border-border p-4 space-y-3">
                <h3 className="font-medium">Boot Source</h3>
                <dl className="grid grid-cols-2 gap-2 text-sm">
                  <dt className="text-muted-foreground">Kernel:</dt>
                  <dd className="font-mono text-xs">{formData.kernelPath || "—"}</dd>
                  <dt className="text-muted-foreground">Rootfs:</dt>
                  <dd className="font-mono text-xs">{formData.rootfsPath || "—"}</dd>
                </dl>
              </div>

              <div className="rounded-lg border border-border p-4 space-y-3">
                <h3 className="font-medium">Network</h3>
                <dl className="grid grid-cols-2 gap-2 text-sm">
                  <dt className="text-muted-foreground">Enabled:</dt>
                  <dd>{formData.enableNetwork ? "Yes" : "No"}</dd>
                  {formData.enableNetwork && (
                    <>
                      <dt className="text-muted-foreground">Host Device:</dt>
                      <dd>{formData.hostDevice || "tap0"}</dd>
                      <dt className="text-muted-foreground">Guest MAC:</dt>
                      <dd className="font-mono text-xs">{formData.guestMac || "—"}</dd>
                    </>
                  )}
                </dl>
              </div>
            </div>
          )}
        </CardContent>
      </Card>

      <div className="flex justify-between mt-6">
        <div className="flex gap-2">
          {onCancel && (
            <Button type="button" variant="outline" onClick={onCancel}>
              Cancel
            </Button>
          )}
          <Button type="button" variant="outline" onClick={prevStep} disabled={currentStep === 0}>
            <ChevronLeft className="mr-2 h-4 w-4" />
            Previous
          </Button>
        </div>
        {currentStep < steps.length - 1 ? (
          <Button
            type="button"
            onClick={nextStep}
          >
            Next
            <ChevronRight className="ml-2 h-4 w-4" />
          </Button>
        ) : (
          <Button
            type="submit"
            disabled={isSubmitting}
          >
            {isSubmitting ? "Creating VM..." : "Create VM"}
          </Button>
        )}
      </div>
      </form>
    </div>
  )
}