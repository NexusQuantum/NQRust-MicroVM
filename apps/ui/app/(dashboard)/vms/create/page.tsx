"use client"

import { useRouter } from "next/navigation"
import { useState } from "react"
import { VMCreateWizard } from "@/components/vm/vm-create-wizard"
import { useTemplates, useInstantiateTemplate } from "@/lib/queries"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Server, Cpu, HardDrive, Zap } from "lucide-react"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { ScrollArea } from "@/components/ui/scroll-area"
import { toast } from "sonner"
import type { Template } from "@/lib/types"

export default function CreateVMPage() {
  const router = useRouter()
  const { data: templates, isLoading: templatesLoading } = useTemplates()
  const instantiateMutation = useInstantiateTemplate()

  // Quick Create Dialog state
  const [quickCreateOpen, setQuickCreateOpen] = useState(false)
  const [selectedTemplate, setSelectedTemplate] = useState<Template | null>(null)
  const [vmName, setVmName] = useState("")

  const handleComplete = () => {
    // Show success toast
    toast.success("VM Created Successfully", {
      description: "Your virtual machine has been created and is ready to use"
    })
    // Navigate back to VMs page after successful creation
    router.push("/vms")
  }

  const handleCancel = () => {
    // Navigate back to VMs page on cancel
    router.push("/vms")
  }

  const handleQuickCreate = () => {
    if (!templates || templates.length === 0) {
      toast.error("No templates available", {
        description: "Please create a template first before using Quick Create"
      })
      return
    }
    setQuickCreateOpen(true)
  }

  const handleDialogClose = (open: boolean) => {
    setQuickCreateOpen(open)
    if (!open) {
      // Reset state when dialog is closed
      setSelectedTemplate(null)
      setVmName("")
    }
  }

  const handleSelectTemplate = (template: Template) => {
    setSelectedTemplate(template)
    const defaultName = `${template.name}-${Date.now().toString().slice(-4)}`
    setVmName(defaultName)
  }

  const handleDeploy = () => {
    if (!vmName.trim()) {
      toast.error("Validation Error", {
        description: "Please enter a VM name"
      })
      return
    }
    if (!selectedTemplate) {
      toast.error("No template selected", {
        description: "Please select a template first"
      })
      return
    }

    console.log("Creating VM from template:", selectedTemplate.id, "Name:", vmName)

    instantiateMutation.mutate(
      { id: selectedTemplate.id, name: vmName },
      {
        onSuccess: () => {
          // Show success toast
          toast.success("VM Created Successfully", {
            description: `VM "${vmName}" has been created from template "${selectedTemplate.name}"`
          })
          setQuickCreateOpen(false)
          setSelectedTemplate(null)
          setVmName("")
          // Navigate to VMs page after successful creation
          router.push("/vms")
        },
        onError: () => {
          // Error toast is already shown by useInstantiateTemplate hook
        }
      }
    )
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-foreground">Create Virtual Machine</h1>
          <p className="text-muted-foreground">Configure and deploy a new VM</p>
        </div>
        <Button variant="outline" onClick={handleQuickCreate}>
          <Zap className="mr-2 h-4 w-4" />
          Quick Create from Template
        </Button>
      </div>

      <VMCreateWizard onComplete={handleComplete} onCancel={handleCancel} />

      {/* Quick Create Dialog */}
      <Dialog open={quickCreateOpen} onOpenChange={handleDialogClose}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>Quick Create VM from Template</DialogTitle>
            <DialogDescription>
              Select a template to quickly create a new VM with pre-configured settings
            </DialogDescription>
          </DialogHeader>

          {templatesLoading ? (
            <div className="py-8 text-center">
              <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary mx-auto"></div>
              <p className="mt-2 text-sm text-muted-foreground">Loading templates...</p>
            </div>
          ) : !templates || templates.length === 0 ? (
            <div className="py-8 text-center space-y-4">
              <div className="mx-auto w-12 h-12 rounded-full bg-muted flex items-center justify-center">
                <Server className="h-6 w-6 text-muted-foreground" />
              </div>
              <div>
                <h3 className="font-medium">No templates available</h3>
                <p className="text-sm text-muted-foreground mt-1">
                  Create a template first to use Quick Create
                </p>
              </div>
            </div>
          ) : (
            <div className="space-y-4">
              {/* Template Selection */}
              <ScrollArea className="h-[300px] pr-4">
                <div className="space-y-2">
                  {templates.map((template) => (
                    <button
                      key={template.id}
                      onClick={() => handleSelectTemplate(template)}
                      className={`w-full text-left p-4 rounded-lg border-2 transition-all hover:border-primary/50 ${
                        selectedTemplate?.id === template.id
                          ? "border-primary bg-primary/5"
                          : "border-border"
                      }`}
                    >
                      <div className="flex items-start gap-3">
                        <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-gradient-to-br from-orange-500/10 to-orange-600/10 flex-shrink-0">
                          <Server className="h-5 w-5 text-orange-600" />
                        </div>
                        <div className="flex-1 min-w-0">
                          <h4 className="font-medium truncate">{template.name}</h4>
                          {template.description && (
                            <p className="text-sm text-muted-foreground mt-1 line-clamp-2">
                              {template.description}
                            </p>
                          )}
                          <div className="flex gap-4 mt-2 text-xs text-muted-foreground">
                            <span className="flex items-center gap-1">
                              <Cpu className="h-3 w-3" />
                              {template.spec.vcpu} vCPU
                            </span>
                            <span className="flex items-center gap-1">
                              <HardDrive className="h-3 w-3" />
                              {template.spec.mem_mib} MiB
                            </span>
                          </div>
                        </div>
                        {selectedTemplate?.id === template.id && (
                          <div className="h-5 w-5 rounded-full bg-primary flex items-center justify-center flex-shrink-0">
                            <svg
                              className="h-3 w-3 text-primary-foreground"
                              fill="none"
                              strokeLinecap="round"
                              strokeLinejoin="round"
                              strokeWidth="2"
                              viewBox="0 0 24 24"
                              stroke="currentColor"
                            >
                              <path d="M5 13l4 4L19 7" />
                            </svg>
                          </div>
                        )}
                      </div>
                    </button>
                  ))}
                </div>
              </ScrollArea>

              {/* VM Name Input - shown when template is selected */}
              {selectedTemplate && (
                <div className="space-y-2 pt-2 border-t">
                  <Label htmlFor="quick-vm-name">VM Name</Label>
                  <Input
                    id="quick-vm-name"
                    value={vmName}
                    onChange={(e) => setVmName(e.target.value)}
                    placeholder="Enter VM name"
                    className="w-full"
                  />
                </div>
              )}
            </div>
          )}

          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => {
                setQuickCreateOpen(false)
                setSelectedTemplate(null)
                setVmName("")
              }}
              disabled={instantiateMutation.isPending}
            >
              Cancel
            </Button>
            <Button
              onClick={handleDeploy}
              disabled={!selectedTemplate || instantiateMutation.isPending}
            >
              {instantiateMutation.isPending ? "Creating..." : "Create VM"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}
