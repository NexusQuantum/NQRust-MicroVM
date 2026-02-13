"use client"

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { useVMs, useTemplates, useInstantiateTemplate } from "@/lib/queries"
import { VMTable } from "@/components/vm/vm-table"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Plus, Search, Server, Cpu, HardDrive, Zap } from "lucide-react"
import Link from "next/link"
import { useState } from "react"
import { useAuthStore, canCreateResource } from "@/lib/auth/store"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Label } from "@/components/ui/label"
import type { Template } from "@/lib/types"
import { ScrollArea } from "@/components/ui/scroll-area"
import { toast } from "sonner"

const VMFlowDiagram = () => (
  <svg width="300" height="200" viewBox="0 0 300 200" fill="none" xmlns="http://www.w3.org/2000/svg" className="drop-shadow-lg">
    <style>{`
      .vm-text { fill: #92400e; }
      .dark .vm-text { fill: #f59e0b; }
      .vm-bg { fill: #fef3c7; }
      .dark .vm-bg { fill: rgba(146, 64, 14, 0.3); }
    `}</style>
    {/* Host Machine */}
    <rect x="10" y="120" width="280" height="70" rx="8" className="vm-bg" stroke="#f59e0b" strokeWidth="2" />
    <text x="150" y="150" textAnchor="middle" className="vm-text" fontWeight="600" fontSize="14">Host Machine (KVM)</text>
    <text x="150" y="165" textAnchor="middle" className="vm-text" fontSize="11">Firecracker Hypervisor</text>

    {/* VM 1 */}
    <rect x="20" y="30" width="70" height="70" rx="6" className="vm-bg" stroke="#f59e0b" strokeWidth="2" />
    <circle cx="55" cy="55" r="15" fill="#fbbf24" opacity="0.3" />
    <text x="55" y="58" textAnchor="middle" className="vm-text" fontWeight="600" fontSize="12">VM 1</text>
    <text x="55" y="85" textAnchor="middle" className="vm-text" fontSize="9">Guest OS</text>

    {/* VM 2 */}
    <rect x="115" y="30" width="70" height="70" rx="6" className="vm-bg" stroke="#f59e0b" strokeWidth="2" />
    <circle cx="150" cy="55" r="15" fill="#fbbf24" opacity="0.3" />
    <text x="150" y="58" textAnchor="middle" className="vm-text" fontWeight="600" fontSize="12">VM 2</text>
    <text x="150" y="85" textAnchor="middle" className="vm-text" fontSize="9">Container</text>

    {/* VM 3 */}
    <rect x="210" y="30" width="70" height="70" rx="6" className="vm-bg" stroke="#f59e0b" strokeWidth="2" />
    <circle cx="245" cy="55" r="15" fill="#fbbf24" opacity="0.3" />
    <text x="245" y="58" textAnchor="middle" className="vm-text" fontWeight="600" fontSize="12">VM 3</text>
    <text x="245" y="85" textAnchor="middle" className="vm-text" fontSize="9">Function</text>

    {/* Connection lines */}
    <line x1="55" y1="100" x2="55" y2="120" stroke="#f59e0b" strokeWidth="2" strokeDasharray="4" />
    <line x1="150" y1="100" x2="150" y2="120" stroke="#f59e0b" strokeWidth="2" strokeDasharray="4" />
    <line x1="245" y1="100" x2="245" y2="120" stroke="#f59e0b" strokeWidth="2" strokeDasharray="4" />

    {/* Arrows */}
    <polygon points="55,120 52,115 58,115" fill="#f59e0b" />
    <polygon points="150,120 147,115 153,115" fill="#f59e0b" />
    <polygon points="245,120 242,115 248,115" fill="#f59e0b" />
  </svg>
)

export default function VMsPage() {
  const { data: vms, isLoading, error } = useVMs(false, 5000)
  const { data: templates, isLoading: templatesLoading } = useTemplates()
  const [searchTerm, setSearchTerm] = useState("")
  const { user } = useAuthStore()

  // Quick Create Dialog state
  const [quickCreateOpen, setQuickCreateOpen] = useState(false)
  const [selectedTemplate, setSelectedTemplate] = useState<Template | null>(null)
  const [vmName, setVmName] = useState("")
  const instantiateMutation = useInstantiateTemplate()

  const filteredVMs = vms?.filter(vm =>
    vm.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
    vm.id.toLowerCase().includes(searchTerm.toLowerCase())
  ) || []

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
    // console.log("Deploying VM with template: --")
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

    console.log("Id:", selectedTemplate.id)
    console.log("Name:", vmName)

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
        },
        onError: () => {
          // Error toast is already shown by useInstantiateTemplate hook
        }
      }
    )
  }

  if (isLoading) {
    return (
      <div className="container mx-auto py-6">
        <div className="animate-pulse space-y-4">
          <div className="h-8 bg-muted rounded w-1/4"></div>
          <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
            {[...Array(6)].map((_, i) => (
              <div key={i} className="h-48 bg-muted rounded-lg"></div>
            ))}
          </div>
        </div>
      </div>
    )
  }

  if (error) {
    return (
      <div className="container mx-auto py-6">
        <div className="text-center space-y-4">
          <h1 className="text-2xl font-bold text-destructive">Failed to load VMs</h1>
          <p className="text-muted-foreground">
            Unable to fetch VM list. Please check your connection and try again.
          </p>
        </div>
      </div>
    )
  }

  return (
    <div className="space-y-6">
      <div className="relative overflow-hidden rounded-xl border border-border bg-gradient-to-br from-yellow-50 to-yellow-100/50 dark:from-yellow-950/30 dark:to-yellow-900/20 p-8">
        <div className="relative z-10 flex items-center justify-between">
          <div className="max-w-xl">
            <h1 className="text-3xl font-bold text-foreground dark:text-foreground">Virtual Machines</h1>
            <p className="mt-2 text-muted-foreground font-medium">
              Manage your microVMs
            </p>
            {canCreateResource(user) && (
              <div className="">
                <Button asChild className="mt-4">
                  <Link href="/vms/create">
                    <Plus className="mr-2 h-4 w-4" />
                    Create VM
                  </Link>
                </Button>
                <Button variant="outline" className="mt-4 ml-4" onClick={handleQuickCreate}>
                  <Zap className="mr-2 h-4 w-4" />
                  Quick create
                </Button>
              </div>
            )}
          </div>
          <div className="hidden lg:block">
            <VMFlowDiagram />
          </div>
        </div>
        <div className="absolute right-0 top-0 h-64 w-64 translate-x-32 -translate-y-32 rounded-full bg-gradient-to-br from-yellow-400/30 to-yellow-600/30 dark:from-yellow-500/20 dark:to-yellow-600/10 blur-3xl" />
      </div>

      <Card>
        <CardHeader>
          <CardTitle>All Virtual Machine</CardTitle>
        </CardHeader>
        <CardContent>
          {filteredVMs.length === 0 ? (
            <div className="text-center py-12">
              <h3 className="text-lg font-medium">No VMs found</h3>
              <p className="text-muted-foreground mt-2">
                {searchTerm
                  ? "No VMs match your search criteria."
                  : "Get started by creating your first VM."
                }
              </p>
              {!searchTerm && canCreateResource(user) && (
                <Button asChild className="mt-4">
                  <Link href="/vms/create">
                    <Plus className="mr-2 h-4 w-4" />
                    Create your first VM
                  </Link>
                </Button>
              )}
            </div>
          ) : (
            <VMTable vms={filteredVMs} />
          )}
        </CardContent>
      </Card>

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
              <Button asChild variant="outline">
                <Link href="/templates/new">Create Template</Link>
              </Button>
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
                      className={`w-full text-left p-4 rounded-lg border-2 transition-all hover:border-primary/50 ${selectedTemplate?.id === template.id
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
