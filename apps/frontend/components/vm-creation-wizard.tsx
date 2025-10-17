"use client"

import { useEffect, useMemo, useRef, useState } from "react"
import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import { z } from "zod"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Textarea } from "@/components/ui/textarea"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Badge } from "@/components/ui/badge"
import { Separator } from "@/components/ui/separator"
import { ArrowLeft, ArrowRight, Check, Server, HardDrive, Network, Settings } from "lucide-react"
import { useCreateVM } from "@/lib/queries"
import type { CreateVmReq } from "@/types/nexus"
import { RegistryBrowser } from "./registry-browser"

const vmCreationSchema = z.object({
  name: z.string().min(1, "VM Name is required").max(50, "Name too long"),
  description: z.string().max(200, "Description too long").optional(),
  environment: z.enum(["development", "staging", "production"]),
  owner: z.string().min(1, "Owner is required").default("developer"),

  // Credentials
  username: z.string().min(1, "Username is required").max(32, "Username too long").default("root"),
  password: z.string().min(1, "Password is required").max(128, "Password too long"),

  // Machine config
  vcpu_count: z.number().min(1, "Minimum 1 vCPU").max(32, "Maximum 32 vCPUs"),
  mem_size_mib: z
    .number()
    .min(128, "Minimum 128 MiB")
    .max(32768, "Maximum 32768 MiB")
    .refine((n) => n % 128 === 0, {
      message: "Memory must be a multiple of 128 MiB",
    }),
  cpu_template: z.enum(["C3", "T2", "None"]).optional(),
  smt: z.boolean().default(false),
  track_dirty_pages: z.boolean().default(false),

  // Boot source - support both image IDs and paths
  kernel_image_id: z.string().optional(),
  kernel_image_path: z.string().optional(),
  initrd_path: z.string().optional(),
  boot_args: z.string().optional(),

  // Root drive - support both image IDs and paths
  rootfs_image_id: z.string().optional(),
  root_drive_path: z.string().optional(),
  root_drive_readonly: z.boolean().default(false),

  // Network (optional)
  enable_network: z.boolean().default(false),
  host_dev_name: z.string().optional(),
  guest_mac: z.string().optional(),
}).refine(
  (data) => data.kernel_image_id || data.kernel_image_path,
  {
    message: "Either kernel image ID or path is required",
    path: ["kernel_image_path"],
  }
).refine(
  (data) => data.rootfs_image_id || data.root_drive_path,
  {
    message: "Either rootfs image ID or path is required", 
    path: ["root_drive_path"],
  }
)

type VMCreationForm = z.infer<typeof vmCreationSchema>

const steps = [
  { id: "basic", title: "Basic Info", icon: Settings },
  { id: "machine", title: "Machine Config", icon: Server },
  { id: "boot", title: "Boot Source", icon: HardDrive },
  { id: "network", title: "Network", icon: Network },
  { id: "review", title: "Review", icon: Check },
]

interface VMCreationWizardProps {
  onComplete: () => void
  onCancel: () => void
}

export function VMCreationWizard({ onComplete, onCancel }: VMCreationWizardProps) {
  const [currentStep, setCurrentStep] = useState(0)
  const [showRegistryBrowser, setShowRegistryBrowser] = useState<"kernel" | "rootfs" | null>(null)
  const [kernelOptions, setKernelOptions] = useState<{ name: string; path: string; id: string; kind: string }[]>([])
  const [rootfsOptions, setRootfsOptions] = useState<{ name: string; path: string; id: string; kind: string }[]>([])
  const createVM = useCreateVM()
  const containerRef = useRef<HTMLDivElement | null>(null)

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
      name: "my-vm-" + Date.now().toString().slice(-6),
      owner: "developer",
      environment: "development",
      description: "Test VM created from frontend",
      username: "root",
      password: "changeme",
      vcpu_count: 1,
      mem_size_mib: 512,
      smt: false,
      cpu_template: "None",
      track_dirty_pages: false,
      root_drive_readonly: false,
      enable_network: false,
      // Auto-select first available images
      kernel_image_id: "",
      rootfs_image_id: "",
    },
  })

  const formData = watch()
  
  // Load prefilled values from localStorage (when coming from registry)
  useEffect(() => {
    try {
      const kernelId = localStorage.getItem('NR_PREFILL_KERNEL_ID')
      const rootfsId = localStorage.getItem('NR_PREFILL_ROOTFS_ID')
      
      if (kernelId) {
        setValue('kernel_image_id', kernelId)
        setValue('kernel_image_path', '')
        localStorage.removeItem('NR_PREFILL_KERNEL_ID')
      }
      
      if (rootfsId) {
        setValue('rootfs_image_id', rootfsId)
        setValue('root_drive_path', '')
        localStorage.removeItem('NR_PREFILL_ROOTFS_ID')
      }
    } catch (e) {
      // Ignore localStorage errors
    }
  }, [setValue])
  
  // Load registry items for kernel and rootfs from new backend
  useEffect(() => {
    (async () => {
      try {
        const baseUrl = process.env.NEXT_PUBLIC_API_BASE_URL || "/api/proxy/v1"
        
        // Get kernel images
        const kernelRes = await fetch(`${baseUrl}/images?kind=kernel`)
        const kernelData = await kernelRes.json()
        const kernels = (kernelData.items || []).map((i: any) => ({ 
          name: i.name, 
          path: i.host_path, 
          id: i.id,
          kind: i.kind 
        }))
        setKernelOptions(kernels)

        // Get rootfs images
        const rootfsRes = await fetch(`${baseUrl}/images?kind=rootfs`)
        const rootfsData = await rootfsRes.json()
        const rootfs = (rootfsData.items || []).map((i: any) => ({
          name: i.name,
          path: i.host_path,
          id: i.id,
          kind: i.kind
        }))
        setRootfsOptions(rootfs)

        // Auto-select first available images for convenience
        if (kernels.length > 0 && !formData.kernel_image_id) {
          setValue('kernel_image_id', kernels[0].id)
        }
        if (rootfs.length > 0 && !formData.rootfs_image_id) {
          setValue('rootfs_image_id', rootfs[0].id)
        }
      } catch (e) {
        // ignore fetch failures; user can type paths manually
      }
    })()
  }, [setValue, formData.kernel_image_id, formData.rootfs_image_id])


  // Step validation schemas (per-step gating)
  // Create individual field schemas to avoid the refined schema shape issue
  const stepSchemaFields = {
    name: z.string().min(1, "VM Name is required").max(50, "Name too long"),
    description: z.string().max(200, "Description too long").optional(),
    environment: z.enum(["development", "staging", "production"]),
    owner: z.string().min(1, "Owner is required"),
    username: z.string().min(1, "Username is required").max(32, "Username too long"),
    password: z.string().min(1, "Password is required").max(128, "Password too long"),
    vcpu_count: z.number().min(1, "Minimum 1 vCPU").max(32, "Maximum 32 vCPUs"),
    mem_size_mib: z.number().min(128, "Minimum 128 MiB").max(32768, "Maximum 32768 MiB"),
    cpu_template: z.enum(["C3", "T2", "None"]).optional(),
    smt: z.boolean().default(false),
    track_dirty_pages: z.boolean().default(false),
    kernel_image_id: z.string().optional(),
    kernel_image_path: z.string().optional(),
    rootfs_image_id: z.string().optional(),
    root_drive_path: z.string().optional(),
    initrd_path: z.string().optional(),
    boot_args: z.string().optional(),
    root_drive_readonly: z.boolean().default(false),
    enable_network: z.boolean().default(false),
    host_dev_name: z.string().optional(),
    guest_mac: z.string().optional(),
  }
  const stepSchemas = useMemo(
    () => [
      z.object({
        name: stepSchemaFields.name,
        owner: stepSchemaFields.owner,
        environment: stepSchemaFields.environment,
        description: stepSchemaFields.description,
        username: stepSchemaFields.username,
        password: stepSchemaFields.password,
      }),
      z.object({
        vcpu_count: stepSchemaFields.vcpu_count,
        mem_size_mib: stepSchemaFields.mem_size_mib,
        cpu_template: stepSchemaFields.cpu_template,
        smt: stepSchemaFields.smt,
        track_dirty_pages: stepSchemaFields.track_dirty_pages,
      }),
      z.object({
        kernel_image_id: stepSchemaFields.kernel_image_id,
        kernel_image_path: stepSchemaFields.kernel_image_path,
        rootfs_image_id: stepSchemaFields.rootfs_image_id,
        root_drive_path: stepSchemaFields.root_drive_path,
        initrd_path: stepSchemaFields.initrd_path,
        boot_args: stepSchemaFields.boot_args,
        root_drive_readonly: stepSchemaFields.root_drive_readonly,
      }).refine(
        (data) => data.kernel_image_id || data.kernel_image_path,
        { message: "Kernel image is required", path: ["kernel_image_path"] }
      ).refine(
        (data) => data.rootfs_image_id || data.root_drive_path,
        { message: "Root filesystem is required", path: ["root_drive_path"] }
      ),
      // Network step has no required fields; optional inputs always valid
      z.object({
        enable_network: stepSchemaFields.enable_network,
        host_dev_name: stepSchemaFields.host_dev_name,
        guest_mac: stepSchemaFields.guest_mac,
      }),
      z.any(), // Review step
    ],
    []
  )

  const canProceed = stepSchemas[currentStep]?.safeParse(formData).success ?? true
  const allValid = useMemo(() => vmCreationSchema.safeParse(formData).success, [formData])

  const onSubmit = async (data: VMCreationForm) => {
    try {
      // Convert form data to new backend format
      const vmReq: CreateVmReq = {
        name: data.name,
        vcpu: data.vcpu_count,
        mem_mib: data.mem_size_mib,
        // Prefer image IDs over paths for the new unified system
        kernel_image_id: data.kernel_image_id || undefined,
        rootfs_image_id: data.rootfs_image_id || undefined,
        // Fall back to paths if no image IDs are selected
        kernel_path: data.kernel_image_id ? undefined : data.kernel_image_path,
        rootfs_path: data.rootfs_image_id ? undefined : data.root_drive_path,
        // Add credentials
        username: data.username,
        password: data.password,
      }

      console.log('Creating VM with:', vmReq) // Debug log
      await createVM.mutateAsync(vmReq)
      onComplete()
    } catch (error) {
      // Error handling is done in the mutation, but let's also log for debugging
      console.error('VM creation failed:', error)
    }
  }

  const nextStep = () => {
    if (currentStep < steps.length - 1) {
      setCurrentStep(currentStep + 1)
    }
  }

  const prevStep = () => {
    if (currentStep > 0) {
      setCurrentStep(currentStep - 1)
    }
  }

  const handleRegistrySelect = (type: "kernel" | "rootfs", path: string) => {
    if (type === "kernel") {
      setValue("kernel_image_path", path)
    } else {
      setValue("root_drive_path", path)
    }
    setShowRegistryBrowser(null)
  }

  // Focus the container on mount for better a11y and set up keyboard shortcuts
  useEffect(() => {
    containerRef.current?.focus()
  }, [])

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") {
      e.preventDefault()
      onCancel()
      return
    }
    if (e.key === "Enter") {
      const tag = (e.target as HTMLElement)?.tagName
      if (tag === "TEXTAREA") return // allow new lines
      e.preventDefault()
      if (currentStep < steps.length - 1) {
        if (canProceed) nextStep()
      }
      // submit is handled by form submit button on last step
    }
    if (e.key === "Tab" && containerRef.current) {
      // focus trap
      const focusable = containerRef.current.querySelectorAll<HTMLElement>(
        'a[href], button:not([disabled]), textarea, input, select, [tabindex]:not([tabindex="-1"])'
      )
      const items = Array.from(focusable).filter((el) => el.offsetParent !== null)
      if (items.length === 0) return
      const first = items[0]
      const last = items[items.length - 1]
      const active = document.activeElement as HTMLElement | null
      if (!e.shiftKey && active === last) {
        e.preventDefault()
        first.focus()
      } else if (e.shiftKey && active === first) {
        e.preventDefault()
        last.focus()
      }
    }
  }

  if (showRegistryBrowser) {
    return (
      <RegistryBrowser
        type={showRegistryBrowser}
        onSelect={(path) => handleRegistrySelect(showRegistryBrowser, path)}
        onCancel={() => setShowRegistryBrowser(null)}
      />
    )
  }

  return (
    <div
      ref={containerRef}
      tabIndex={-1}
      role="region"
      aria-label="Create VM Wizard"
      className="max-w-4xl mx-auto space-y-6"
      onKeyDown={onKeyDown}
    >
      {/* Progress Steps */}
      <nav aria-label="Progress">
        <ol className="flex items-center justify-between">
          {steps.map((step, index) => {
            const Icon = step.icon
            const isActive = index === currentStep
            const isCompleted = index < currentStep

            return (
              <li key={step.id} className="flex items-center">
                <div
                  aria-current={isActive ? "step" : undefined}
                  className={`flex items-center justify-center w-10 h-10 rounded-full border-2 transition-colors ${
                    isActive
                      ? "border-primary bg-primary text-primary-foreground"
                      : isCompleted
                        ? "border-success/60 bg-success/10 text-success"
                        : "border-muted-foreground/40 text-muted-foreground"
                  }`}
                  title={step.title}
                >
                  {isCompleted ? <Check className="h-5 w-5" /> : <Icon className="h-5 w-5" />}
                </div>
                <div className="ml-3">
                  <p className={`text-sm font-medium ${isActive ? "text-foreground" : "text-muted-foreground"}`}>
                    {step.title}
                  </p>
                </div>
                {index < steps.length - 1 && (
                  <div className={`w-16 h-0.5 mx-4 ${index < currentStep ? "bg-success" : "bg-muted"}`} />)
                }
              </li>
            )
          })}
        </ol>
      </nav>

      {/* Quick Create Button */}
      <div className="text-center">
        <Button
          type="button"
          onClick={handleSubmit(onSubmit)}
          variant="outline"
          className="mb-4"
          disabled={isSubmitting || !formData.name || !formData.kernel_image_id || !formData.rootfs_image_id}
        >
          🚀 Quick Create VM with Defaults
        </Button>
        <p className="text-xs text-muted-foreground">
          Creates VM with current values (name: {formData.name || 'unnamed'})
        </p>
      </div>

      <form onSubmit={handleSubmit(onSubmit)}>
        <Card className="rounded-xl shadow-sm">
          <CardHeader>
            <CardTitle>{steps[currentStep].title}</CardTitle>
          </CardHeader>
          <CardContent className="space-y-6">
            {/* Step Content */}
            {currentStep === 0 && (
              <div className="grid gap-4 md:grid-cols-2">
                <div>
                  <Label htmlFor="name" className="mb-1 inline-block">
                    VM Name<span className="text-red-600 ml-0.5" aria-hidden>*</span>
                  </Label>
                  <Input id="name" aria-required="true" aria-invalid={!!errors.name} {...register("name")} placeholder="my-vm" className="bg-muted/40 focus:bg-background" />
                  <p className="text-xs text-muted-foreground mt-1">Short, unique name (max 50 chars)</p>
                  {errors.name && <p className="text-sm text-red-600 mt-1">{errors.name.message}</p>}
                </div>

                <div>
                  <Label htmlFor="owner" className="mb-1 inline-block">
                    Owner<span className="text-red-600 ml-0.5" aria-hidden>*</span>
                  </Label>
                  <Input id="owner" aria-required="true" aria-invalid={!!errors.owner} {...register("owner")} placeholder="username" className="bg-muted/40 focus:bg-background" />
                  {errors.owner && <p className="text-sm text-red-600 mt-1">{errors.owner.message}</p>}
                </div>

                <div>
                  <Label htmlFor="environment" className="mb-1 inline-block">
                    Environment<span className="text-red-600 ml-0.5" aria-hidden>*</span>
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
                </div>

                <div className="md:col-span-2">
                  <Label htmlFor="description" className="mb-1 inline-block text-gray-500">Description (Optional)</Label>
                  <Textarea id="description" {...register("description")} placeholder="Describe your VM..." rows={3} className="bg-muted/40 focus:bg-background" />
                  {errors.description && <p className="text-sm text-red-600 mt-1">{errors.description.message}</p>}
                </div>

                <div className="md:col-span-2">
                  <Separator className="my-4" />
                  <h4 className="text-sm font-medium mb-3">VM Credentials</h4>
                </div>

                <div>
                  <Label htmlFor="username" className="mb-1 inline-block">
                    Username<span className="text-red-600 ml-0.5" aria-hidden>*</span>
                  </Label>
                  <Input id="username" aria-required="true" aria-invalid={!!errors.username} {...register("username")} placeholder="root" className="bg-muted/40 focus:bg-background" />
                  <p className="text-xs text-muted-foreground mt-1">Username for SSH/console access</p>
                  {errors.username && <p className="text-sm text-red-600 mt-1">{errors.username.message}</p>}
                </div>

                <div>
                  <Label htmlFor="password" className="mb-1 inline-block">
                    Password<span className="text-red-600 ml-0.5" aria-hidden>*</span>
                  </Label>
                  <Input id="password" type="password" aria-required="true" aria-invalid={!!errors.password} {...register("password")} placeholder="Enter password" className="bg-muted/40 focus:bg-background" />
                  <p className="text-xs text-muted-foreground mt-1">Password will be injected into VM</p>
                  {errors.password && <p className="text-sm text-red-600 mt-1">{errors.password.message}</p>}
                </div>
              </div>
            )}

            {currentStep === 1 && (
              <div className="grid gap-4 md:grid-cols-2">
                <div>
                  <Label htmlFor="vcpu_count" className="mb-1 inline-block">
                    vCPU Count<span className="text-red-600 ml-0.5" aria-hidden>*</span>
                  </Label>
                  <Input
                    id="vcpu_count"
                    type="number"
                    min="1"
                    max="32"
                    aria-required="true"
                    aria-invalid={!!errors.vcpu_count}
                    {...register("vcpu_count", { valueAsNumber: true })}
                    className="bg-muted/40 focus:bg-background"
                  />
                  <p className="text-xs text-muted-foreground mt-1">1 - 32 vCPUs</p>
                  {errors.vcpu_count && <p className="text-sm text-red-600 mt-1">{errors.vcpu_count.message}</p>}
                </div>

                <div>
                  <Label htmlFor="mem_size_mib" className="mb-1 inline-block">
                    Memory (MiB)<span className="text-red-600 ml-0.5" aria-hidden>*</span>
                  </Label>
                  <Input
                    id="mem_size_mib"
                    type="number"
                    min="128"
                    max="32768"
                    step="128"
                    aria-required="true"
                    aria-invalid={!!errors.mem_size_mib}
                    {...register("mem_size_mib", { valueAsNumber: true })}
                    className="bg-muted/40 focus:bg-background"
                  />
                  <p className="text-xs text-muted-foreground mt-1">Must be a multiple of 128 MiB</p>
                  {errors.mem_size_mib && (
                    <p className="text-sm text-red-600 mt-1">{errors.mem_size_mib.message}</p>
                  )}
                </div>

                <details className="md:col-span-2 group border-t border-muted pt-2">
                  <summary className="cursor-pointer text-sm text-muted-foreground hover:text-foreground select-none">
                    Advanced settings
                  </summary>
                  <div className="mt-3 grid gap-4 md:grid-cols-2 pl-4 border-l-2 border-muted">
                    <div>
                      <Label htmlFor="cpu_template" className="mb-1 inline-block text-gray-500">CPU Template (Optional)</Label>
                      <Select
                        value={formData.cpu_template || "None"}
                        onValueChange={(value) => setValue("cpu_template", value as any)}
                      >
                        <SelectTrigger>
                          <SelectValue placeholder="Select template" />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="C3">C3</SelectItem>
                          <SelectItem value="T2">T2</SelectItem>
                          <SelectItem value="None">None</SelectItem>
                        </SelectContent>
                      </Select>
                    </div>

                    <div className="space-y-4">
                      <div className="flex items-center space-x-2">
                        <input
                          type="checkbox"
                          id="smt"
                          {...register("smt")}
                          className="rounded border-gray-300 focus-visible:ring-2 focus-visible:ring-ring"
                        />
                        <Label htmlFor="smt">Enable SMT</Label>
                      </div>

                      <div className="flex items-center space-x-2">
                        <input
                          type="checkbox"
                          id="track_dirty_pages"
                          {...register("track_dirty_pages")}
                          className="rounded border-gray-300 focus-visible:ring-2 focus-visible:ring-ring"
                        />
                        <Label htmlFor="track_dirty_pages">Track Dirty Pages</Label>
                      </div>
                    </div>
                  </div>
                </details>
              </div>
            )}

            {currentStep === 2 && (
              <div className="space-y-4">
                <div>
                  <Label htmlFor="kernel_selection" className="mb-1 inline-block">
                    Kernel Image<span className="text-red-600 ml-0.5" aria-hidden>*</span>
                  </Label>
                  <div className="space-y-2">
                    <Select
                      value={formData.kernel_image_id || ''}
                      onValueChange={(value) => {
                        if (value) {
                          setValue('kernel_image_id', value)
                          setValue('kernel_image_path', '')
                        } else {
                          setValue('kernel_image_id', '')
                        }
                      }}
                    >
                      <SelectTrigger aria-label="Select kernel">
                        <SelectValue placeholder="Select kernel from registry…" />
                      </SelectTrigger>
                      <SelectContent>
                        {kernelOptions.map(k => (
                          <SelectItem key={k.id} value={k.id}>{k.name} ({k.kind})</SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                    <div className="flex items-center gap-2">
                      <span className="text-xs text-muted-foreground whitespace-nowrap">or enter path:</span>
                      <Input 
                        {...register("kernel_image_path")} 
                        placeholder="/path/to/kernel" 
                        className="bg-muted/40 focus:bg-background"
                        disabled={!!formData.kernel_image_id}
                        onChange={(e) => {
                          if (e.target.value) {
                            setValue('kernel_image_id', '')
                          }
                        }}
                      />
                      <Button type="button" variant="outline" onClick={() => setShowRegistryBrowser("kernel")}>
                        Browse
                      </Button>
                    </div>
                  </div>
                  {errors.kernel_image_path && (
                    <p className="text-sm text-red-600 mt-1">{errors.kernel_image_path.message}</p>
                  )}
                </div>

                <div>
                  <Label htmlFor="rootfs_selection" className="mb-1 inline-block">
                    RootFS Image<span className="text-red-600 ml-0.5" aria-hidden>*</span>
                  </Label>
                  <div className="space-y-2">
                    <Select
                      value={formData.rootfs_image_id || ''}
                      onValueChange={(value) => {
                        if (value) {
                          setValue('rootfs_image_id', value)
                          setValue('root_drive_path', '')
                        } else {
                          setValue('rootfs_image_id', '')
                        }
                      }}
                    >
                      <SelectTrigger aria-label="Select rootfs">
                        <SelectValue placeholder="Select rootfs from registry…" />
                      </SelectTrigger>
                      <SelectContent>
                        {rootfsOptions.map(r => (
                          <SelectItem key={r.id} value={r.id}>{r.name} ({r.kind})</SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                    <div className="flex items-center gap-2">
                      <span className="text-xs text-muted-foreground whitespace-nowrap">or enter path:</span>
                      <Input 
                        {...register("root_drive_path")} 
                        placeholder="/path/to/rootfs" 
                        className="bg-muted/40 focus:bg-background"
                        disabled={!!formData.rootfs_image_id}
                        onChange={(e) => {
                          if (e.target.value) {
                            setValue('rootfs_image_id', '')
                          }
                        }}
                      />
                      <Button type="button" variant="outline" onClick={() => setShowRegistryBrowser("rootfs")}>
                        Browse
                      </Button>
                    </div>
                  </div>
                  {errors.root_drive_path && (
                    <p className="text-sm text-red-600 mt-1">{errors.root_drive_path.message}</p>
                  )}
                </div>

                <div>
                  <Label htmlFor="initrd_path" className="mb-1 inline-block text-gray-500">Initrd Path (Optional)</Label>
                  <Input id="initrd_path" {...register("initrd_path")} placeholder="/path/to/initrd" className="bg-muted/40 focus:bg-background" />
                </div>

                <div>
                  <Label htmlFor="boot_args" className="mb-1 inline-block text-gray-500">Boot Arguments (Optional)</Label>
                  <Input id="boot_args" {...register("boot_args")} placeholder="console=ttyS0 reboot=k panic=1" className="bg-muted/40 focus:bg-background" />
                </div>

                <div className="flex items-center space-x-2">
                  <input
                    type="checkbox"
                    id="root_drive_readonly"
                    {...register("root_drive_readonly")}
                    className="rounded border-gray-300 focus-visible:ring-2 focus-visible:ring-ring"
                  />
                  <Label htmlFor="root_drive_readonly">Root Drive Read-Only</Label>
                </div>
              </div>
            )}

            {currentStep === 3 && (
              <div className="space-y-4">
                <div className="flex items-center space-x-2">
                  <input
                    type="checkbox"
                    id="enable_network"
                    {...register("enable_network")}
                    className="rounded border-gray-300 focus-visible:ring-2 focus-visible:ring-ring"
                  />
                  <Label htmlFor="enable_network">Enable Network Interface</Label>
                </div>

                {formData.enable_network && (
                  <div className="grid gap-4 md:grid-cols-2 pl-6 border-l-2 border-muted">
                    <div>
                      <Label htmlFor="host_dev_name" className="mb-1 inline-block text-gray-500">Host Device Name (Optional)</Label>
                      <Input id="host_dev_name" {...register("host_dev_name")} placeholder="tap0" className="bg-muted/40 focus:bg-background" />
                    </div>

                    <div>
                      <Label htmlFor="guest_mac" className="mb-1 inline-block text-gray-500">Guest MAC Address (Optional)</Label>
                      <Input id="guest_mac" {...register("guest_mac")} placeholder="AA:FC:00:00:00:01" className="bg-muted/40 focus:bg-background" />
                    </div>
                  </div>
                )}
              </div>
            )}

            {currentStep === 4 && (
              <div className="space-y-6">
                <div>
                  <h3 className="text-lg font-semibold mb-4">Review Configuration</h3>
                  <div className="grid gap-6 md:grid-cols-2">
                    <section aria-labelledby="review-basic">
                      <h4 id="review-basic" className="text-sm font-medium text-muted-foreground mb-2 flex items-center gap-2">
                        <Settings className="h-4 w-4" /> Basic Info
                      </h4>
                      <dl className="divide-y divide-muted rounded-lg border border-muted-foreground/10">
                        <div className="px-3 py-2 grid grid-cols-3 items-center">
                          <dt className="text-xs text-muted-foreground">Name</dt>
                          <dd className="col-span-2 text-sm">{formData.name || <span className="text-muted-foreground">Not set</span>}</dd>
                        </div>
                        <div className="px-3 py-2 grid grid-cols-3 items-center">
                          <dt className="text-xs text-muted-foreground">Owner</dt>
                          <dd className="col-span-2 text-sm">{formData.owner || <span className="text-muted-foreground">Not set</span>}</dd>
                        </div>
                        <div className="px-3 py-2 grid grid-cols-3 items-center">
                          <dt className="text-xs text-muted-foreground">Environment</dt>
                          <dd className="col-span-2">
                            <Badge variant="outline" className="capitalize">{formData.environment}</Badge>
                          </dd>
                        </div>
                      </dl>
                    </section>

                    <section aria-labelledby="review-machine">
                      <h4 id="review-machine" className="text-sm font-medium text-muted-foreground mb-2 flex items-center gap-2">
                        <Server className="h-4 w-4" /> Machine Config
                      </h4>
                      <dl className="divide-y divide-muted rounded-lg border border-muted-foreground/10">
                        <div className="px-3 py-2 grid grid-cols-3 items-center">
                          <dt className="text-xs text-muted-foreground">vCPUs</dt>
                          <dd className="col-span-2 text-sm">{formData.vcpu_count}</dd>
                        </div>
                        <div className="px-3 py-2 grid grid-cols-3 items-center">
                          <dt className="text-xs text-muted-foreground">Memory</dt>
                          <dd className="col-span-2 text-sm">{formData.mem_size_mib} MiB</dd>
                        </div>
                        <div className="px-3 py-2 grid grid-cols-3 items-center">
                          <dt className="text-xs text-muted-foreground">CPU Template</dt>
                          <dd className="col-span-2 text-sm">{formData.cpu_template || <span className="text-muted-foreground">Not set</span>}</dd>
                        </div>
                      </dl>
                    </section>

                    <section aria-labelledby="review-boot">
                      <h4 id="review-boot" className="text-sm font-medium text-muted-foreground mb-2 flex items-center gap-2">
                        <HardDrive className="h-4 w-4" /> Boot Source
                      </h4>
                      <dl className="divide-y divide-muted rounded-lg border border-muted-foreground/10">
                        <div className="px-3 py-2 grid grid-cols-3 items-center">
                          <dt className="text-xs text-muted-foreground">Kernel</dt>
                          <dd className="col-span-2 text-xs font-mono truncate max-w-64">{formData.kernel_image_path}</dd>
                        </div>
                        <div className="px-3 py-2 grid grid-cols-3 items-center">
                          <dt className="text-xs text-muted-foreground">Root Drive</dt>
                          <dd className="col-span-2 text-xs font-mono truncate max-w-64">{formData.root_drive_path}</dd>
                        </div>
                      </dl>
                    </section>

                    <section aria-labelledby="review-network">
                      <h4 id="review-network" className="text-sm font-medium text-muted-foreground mb-2 flex items-center gap-2">
                        <Network className="h-4 w-4" /> Network
                      </h4>
                      <dl className="divide-y divide-muted rounded-lg border border-muted-foreground/10">
                        {formData.enable_network ? (
                          <>
                            <div className="px-3 py-2 grid grid-cols-3 items-center">
                              <dt className="text-xs text-muted-foreground">Host Device</dt>
                              <dd className="col-span-2 text-sm">{formData.host_dev_name || "tap0"}</dd>
                            </div>
                            <div className="px-3 py-2 grid grid-cols-3 items-center">
                              <dt className="text-xs text-muted-foreground">MAC Address</dt>
                              <dd className="col-span-2 text-xs font-mono">{formData.guest_mac || <span className="text-muted-foreground">Not set</span>}</dd>
                            </div>
                          </>
                        ) : (
                          <div className="px-3 py-2">
                            <dd className="text-sm text-muted-foreground">No network interface</dd>
                          </div>
                        )}
                      </dl>
                    </section>
                  </div>
                </div>
              </div>
            )}

            {/* Navigation */}
            {/* Required legend */}
            {currentStep !== 4 && (
              <p className="text-xs text-muted-foreground">Fields marked with <span className="text-red-600">*</span> are required</p>
            )}
            <Separator />
            <div className="flex justify-between">
              <div className="flex gap-2">
                <Button type="button" variant="ghost" className="border border-muted-foreground/30" onClick={onCancel}>
                  Cancel
                </Button>
                {currentStep > 0 && (
                  <Button type="button" variant="outline" onClick={prevStep}>
                    <ArrowLeft className="h-4 w-4 mr-2" />
                    Previous
                  </Button>
                )}
              </div>

              <div>
                {currentStep < steps.length - 1 ? (
                  <Button
                    type="button"
                    className="bg-primary text-primary-foreground hover:bg-success disabled:opacity-50"
                    onClick={async () => {
                      await trigger()
                      if (canProceed) nextStep()
                    }}
                    disabled={!canProceed}
                  >
                    Next
                    <ArrowRight className="h-4 w-4 ml-2" />
                  </Button>
                ) : (
                  <Button type="submit" variant="success" className="w-full" disabled={isSubmitting || !allValid}>
                    {isSubmitting ? "Creating VM..." : "Create VM"}
                  </Button>
                )}
              </div>
            </div>
          </CardContent>
        </Card>
      </form>
    </div>
  )
}
