"use client"

import { useState } from "react"
import Link from "next/link"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Play, Trash2, Search, Server, Cpu, HardDrive } from "lucide-react"
import type { Template } from "@/lib/types"
import { useDateFormat } from "@/lib/hooks/use-date-format"
import { ConfirmDialog } from "@/components/shared/confirm-dialog"
import { useDeleteTemplate } from "@/lib/queries"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Label } from "@/components/ui/label"
import { useMutation, useQueryClient } from "@tanstack/react-query"
import { facadeApi } from "@/lib/api/facade"
import { toast } from "sonner"

interface TemplateListProps {
  templates: Template[]
}

export function TemplateList({ templates }: TemplateListProps) {
  const dateFormat = useDateFormat()
  const queryClient = useQueryClient()
  const [searchQuery, setSearchQuery] = useState("")

  // console.log('templates: ', templates)

  // Delete dialog state
  const deleteMutation = useDeleteTemplate()
  const [deleteDialog, setDeleteDialog] = useState<{
    open: boolean
    templateId: string
    templateName: string
  }>({
    open: false,
    templateId: "",
    templateName: "",
  })

  // Deploy/Instantiate dialog state
  const [deployDialog, setDeployDialog] = useState<{
    open: boolean
    template: Template | null
  }>({
    open: false,
    template: null,
  })
  const [vmName, setVmName] = useState("")

  const instantiateMutation = useMutation({
    mutationFn: ({ templateId, name }: { templateId: string; name: string }) =>
      facadeApi.instantiateTemplate(templateId, { name }),
    onSuccess: () => {
      const templateName = deployDialog.template?.name || "template"
      toast.success("VM deployed successfully", {
        description: `VM "${vmName}" has been created from template "${templateName}"`,
      })
      queryClient.invalidateQueries({ queryKey: ["vms"] })
      setDeployDialog({ open: false, template: null })
      setVmName("")
    },
    onError: (error: Error) => {
      try {
        const e = JSON.parse(error.message)
        toast.error(e.error || "Failed to deploy VM", {
          description: e.suggestion || e.fault_message,
        })
      } catch {
        toast.error("Failed to deploy VM", {
          description: error.message,
        })
      }
    },
  })

  const filteredTemplates = templates.filter((template) => {
    return template.name.toLowerCase().includes(searchQuery.toLowerCase())
  })

  const handleDelete = () => {
    if (deleteDialog.templateId && deleteDialog.templateName) {
      deleteMutation.mutate(deleteDialog.templateId, {
        onSuccess: () => {
          toast.success("Template Deleted", {
            description: `${deleteDialog.templateName} has been deleted successfully`,
          })
          setDeleteDialog({ open: false, templateId: "", templateName: "" })
        },
        onError: (error: Error) => {
          toast.error("Delete Failed", {
            description: `Failed to delete ${deleteDialog.templateName}: ${error.message}`,
          })
          setDeleteDialog({ open: false, templateId: "", templateName: "" })
        },
      })
    }
  }

  const handleDeploy = () => {
    if (!vmName.trim()) {
      toast.error("Validation Error", {
        description: "Please enter a VM name",
      })
      return
    }
    if (deployDialog.template) {
      instantiateMutation.mutate({
        templateId: deployDialog.template.id,
        name: vmName,
      })
    }
  }

  const openDeployDialog = (template: Template) => {
    const defaultName = `${template.name}-${Date.now().toString().slice(-4)}`
    setVmName(defaultName)
    setDeployDialog({ open: true, template })
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-4">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            placeholder="Search templates..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-9"
          />
        </div>
      </div>

      {filteredTemplates.length === 0 && searchQuery === "" ? (
        <div className="flex flex-col items-center justify-center rounded-lg border-2 border-dashed border-border bg-muted/20 py-16">
          <div className="mb-4 rounded-full bg-muted p-4">
            <Server className="h-8 w-8 text-muted-foreground" />
          </div>
          <h3 className="mb-2 text-lg font-semibold">No templates yet</h3>
          <p className="mb-4 text-sm text-muted-foreground">Create your first VM template to get started</p>
          <Button asChild>
            <Link href="/templates/new">
              <Play className="mr-2 h-4 w-4" />
              Create Template
            </Link>
          </Button>
        </div>
      ) : (
        <div className="rounded-lg border border-border">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Name</TableHead>
                <TableHead>vCPU</TableHead>
                <TableHead>Memory</TableHead>
                <TableHead>Created</TableHead>
                <TableHead className="text-right">Actions</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {filteredTemplates.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={5} className="text-center py-8 text-muted-foreground">
                    No templates found
                  </TableCell>
                </TableRow>
              ) : (
                filteredTemplates.map((template) => (
                  <TableRow key={template.id}>
                    <TableCell>
                      <Link href={`/templates/${template.id}`} className="block hover:opacity-80 transition-opacity">
                        <div className="flex items-center gap-3">
                          <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-gradient-to-br from-orange-500/10 to-orange-600/10">
                            <Server className="h-5 w-5 text-orange-600 dark:text-orange-600" />
                          </div>
                          <span className="font-medium hover:underline">{template.name}</span>
                        </div>
                      </Link>
                    </TableCell>
                    <TableCell className="text-sm">
                      <div className="flex items-center gap-1">
                        <Cpu className="h-3 w-3 text-muted-foreground" />
                        {template.spec.vcpu}
                      </div>
                    </TableCell>
                    <TableCell className="text-sm">
                      <div className="flex items-center gap-1">
                        <HardDrive className="h-3 w-3 text-muted-foreground" />
                        {template.spec.mem_mib} MiB
                      </div>
                    </TableCell>
                    <TableCell className="text-sm text-muted-foreground">
                      {dateFormat.formatRelative(template.created_at)}
                    </TableCell>
                    <TableCell className="text-right">
                      <div className="flex justify-end gap-1">
                        <Button
                          variant="default"
                          size="sm"
                          onClick={() => openDeployDialog(template)}
                        >
                          <Play className="mr-2 h-4 w-4" />
                          Deploy
                        </Button>
                        <Button
                          variant="ghost"
                          size="icon"
                          title="Delete"
                          onClick={() =>
                            setDeleteDialog({
                              open: true,
                              templateId: template.id,
                              templateName: template.name,
                            })
                          }
                        >
                          <Trash2 className="h-4 w-4" />
                        </Button>
                      </div>
                    </TableCell>
                  </TableRow>
                ))
              )}
            </TableBody>
          </Table>
        </div>
      )}

      {/* Delete Confirmation Dialog */}
      <ConfirmDialog
        open={deleteDialog.open}
        onOpenChange={(open) =>
          setDeleteDialog({ open, templateId: "", templateName: "" })
        }
        title="Delete Template"
        description={`Are you sure you want to delete "${deleteDialog.templateName}"? This action cannot be undone.`}
        confirmText="Delete"
        cancelText="Cancel"
        onConfirm={handleDelete}
        variant="destructive"
      />

      {/* Deploy/Instantiate Dialog */}
      <Dialog
        open={deployDialog.open}
        onOpenChange={(open) =>
          setDeployDialog({ open, template: open ? deployDialog.template : null })
        }
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Deploy VM from Template</DialogTitle>
            <DialogDescription>
              Create a new VM using the "{deployDialog.template?.name}" template configuration.
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="vm-name">VM Name</Label>
              <Input
                id="vm-name"
                value={vmName}
                onChange={(e) => setVmName(e.target.value)}
                placeholder="Enter VM name"
              />
            </div>

            {deployDialog.template && (
              <div className="rounded-lg border p-4 space-y-2 text-sm">
                <h4 className="font-medium">Template Configuration</h4>
                <div className="grid grid-cols-2 gap-2 text-muted-foreground">
                  <div>
                    vCPU: <span className="text-foreground font-mono">{deployDialog.template.spec.vcpu}</span>
                  </div>
                  <div>
                    RAM: <span className="text-foreground font-mono">{deployDialog.template.spec.mem_mib} MiB</span>
                  </div>
                </div>
              </div>
            )}
          </div>

          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setDeployDialog({ open: false, template: null })}
              disabled={instantiateMutation.isPending}
            >
              Cancel
            </Button>
            <Button onClick={handleDeploy} disabled={instantiateMutation.isPending}>
              {instantiateMutation.isPending ? "Deploying..." : "Deploy VM"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}
