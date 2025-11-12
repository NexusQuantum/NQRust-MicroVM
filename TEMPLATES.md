# VM Templates Feature Documentation

## Overview

The VM Templates feature allows users to save VM configurations as reusable templates. Users can create templates from scratch or from existing VMs, then deploy new VMs instantly with pre-configured settings (CPU, memory, kernel, rootfs).

**Current Status**: ✅ Backend fully implemented | 🚧 Frontend UI needs implementation

---

## Backend API

### Base URL
```
http://localhost:18080/v1
```

### Authentication
Currently no authentication required (will be added later).

---

## API Endpoints

### 1. List Templates

**GET** `/v1/templates`

**Response**: `200 OK`
```json
{
  "items": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "name": "Ubuntu 22.04 Base",
      "spec": {
        "vcpu": 2,
        "mem_mib": 2048,
        "kernel_image_id": null,
        "rootfs_image_id": null,
        "kernel_path": "/srv/images/vmlinux-5.10.fc.bin",
        "rootfs_path": "/srv/images/ubuntu-22.04.ext4"
      },
      "created_at": "2025-11-06T12:00:00Z",
      "updated_at": "2025-11-06T12:00:00Z"
    }
  ]
}
```

**Use Case**: Display list of available templates on `/templates` page.

---

### 2. Create Template

**POST** `/v1/templates`

**Request Body**:
```json
{
  "name": "Ubuntu 22.04 Base",
  "spec": {
    "vcpu": 2,
    "mem_mib": 2048,
    "kernel_path": "/srv/images/vmlinux-5.10.fc.bin",
    "rootfs_path": "/srv/images/ubuntu-22.04.ext4"
  }
}
```

**Response**: `200 OK`
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000"
}
```

**Template Spec Fields**:
- `vcpu` (required): Number of virtual CPUs (1-32)
- `mem_mib` (required): Memory in MiB (128-16384)
- `kernel_image_id` (optional): Reference to image in registry
- `rootfs_image_id` (optional): Reference to image in registry
- `kernel_path` (optional): Direct file path to kernel
- `rootfs_path` (optional): Direct file path to rootfs

**Note**: Must provide either `*_image_id` OR `*_path` for kernel and rootfs.

**Use Case**: Create template from "Create Template" dialog or from existing VM.

---

### 3. Get Template Details

**GET** `/v1/templates/{id}`

**Response**: `200 OK`
```json
{
  "item": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "name": "Ubuntu 22.04 Base",
    "spec": {
      "vcpu": 2,
      "mem_mib": 2048,
      "kernel_path": "/srv/images/vmlinux-5.10.fc.bin",
      "rootfs_path": "/srv/images/ubuntu-22.04.ext4"
    },
    "created_at": "2025-11-06T12:00:00Z",
    "updated_at": "2025-11-06T12:00:00Z"
  }
}
```

**Use Case**: Show template details in a modal or detail page.

---

### 4. Update Template

**PUT** `/v1/templates/{id}`

**Request Body**:
```json
{
  "name": "Ubuntu 22.04 Base Updated",
  "spec": {
    "vcpu": 4,
    "mem_mib": 4096,
    "kernel_path": "/srv/images/vmlinux-5.10.fc.bin",
    "rootfs_path": "/srv/images/ubuntu-22.04.ext4"
  }
}
```

**Response**: `200 OK`
```json
{
  "item": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "name": "Ubuntu 22.04 Base Updated",
    "spec": {
      "vcpu": 4,
      "mem_mib": 4096,
      "kernel_image_id": null,
      "rootfs_image_id": null,
      "kernel_path": "/srv/images/vmlinux-5.10.fc.bin",
      "rootfs_path": "/srv/images/ubuntu-22.04.ext4"
    },
    "created_at": "2025-11-06T12:00:00Z",
    "updated_at": "2025-11-06T12:30:00Z"
  }
}
```

**Error Responses**:
- `404 Not Found`: Template with the given ID does not exist
- `500 Internal Server Error`: Failed to update template

**Template Spec Fields**:
- `name` (required): Template name
- `spec` (required): Template specification
  - `vcpu` (required): Number of virtual CPUs (1-32)
  - `mem_mib` (required): Memory in MiB (128-16384)
  - `kernel_image_id` (optional): Reference to image in registry
  - `rootfs_image_id` (optional): Reference to image in registry
  - `kernel_path` (optional): Direct file path to kernel
  - `rootfs_path` (optional): Direct file path to rootfs

**Note**: Must provide either `*_image_id` OR `*_path` for kernel and rootfs. The `updated_at` timestamp is automatically updated.

**Use Case**: Edit template from template detail page or edit dialog.

---

### 5. Delete Template

**DELETE** `/v1/templates/{id}`

**Response**: `200 OK`
```json
{
  "ok": true
}
```

**Error Responses**:
- `404 Not Found`: Template with the given ID does not exist
- `500 Internal Server Error`: Failed to delete template

**What Happens**:
1. Backend deletes the template from the database
2. Any VMs that were created from this template will have their `template_id` set to `NULL` (due to `ON DELETE SET NULL` foreign key constraint)
3. The template is permanently removed

**Use Case**: Delete button on template detail page or template list → Confirmation dialog → Call this endpoint → Remove from list.

---

### 6. Instantiate Template (Deploy VM)

**POST** `/v1/templates/{id}/instantiate`

**Request Body**:
```json
{
  "name": "my-ubuntu-vm-01"
}
```

**Response**: `200 OK`
```json
{
  "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890"
}
```

**What Happens**:
1. Backend reads the template
2. Creates a new VM with template's configuration
3. Automatically starts the VM
4. Returns the new VM's ID

**Use Case**: Deploy button on template card → Opens dialog → User enters VM name → Call this endpoint → Redirect to VM detail page.

---

## Frontend Implementation Guide

### Current State

**File**: `apps/ui/app/(dashboard)/templates/page.tsx`
- ✅ Page layout exists
- ✅ Uses `useTemplates()` hook
- ✅ Shows loading/error states
- 🚧 `TemplateList` component needs implementation

---

### Step-by-Step Implementation

#### Step 1: Create TemplateList Component

**File**: `apps/ui/components/templates/template-list.tsx`

```tsx
"use client"

import { useState } from "react"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { Template } from "@/lib/types"
import { Play, Calendar, Cpu, HardDrive } from "lucide-react"
import { format } from "date-fns"
import { InstantiateTemplateDialog } from "./instantiate-template-dialog"

interface TemplateListProps {
  templates: Template[]
}

export function TemplateList({ templates }: TemplateListProps) {
  const [selectedTemplate, setSelectedTemplate] = useState<Template | null>(null)

  if (templates.length === 0) {
    return (
      <div className="text-center py-12">
        <p className="text-muted-foreground">No templates found. Create your first template to get started.</p>
      </div>
    )
  }

  return (
    <>
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
        {templates.map((template) => (
          <Card key={template.id}>
            <CardHeader>
              <CardTitle className="flex items-center justify-between">
                <span className="truncate">{template.name}</span>
                <Badge variant="secondary">Template</Badge>
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2 text-sm">
                <div className="flex items-center gap-2">
                  <Cpu className="h-4 w-4 text-muted-foreground" />
                  <span className="text-muted-foreground">CPU:</span>
                  <span className="font-mono">{template.spec.vcpu} vCPU</span>
                </div>
                <div className="flex items-center gap-2">
                  <HardDrive className="h-4 w-4 text-muted-foreground" />
                  <span className="text-muted-foreground">RAM:</span>
                  <span className="font-mono">{template.spec.mem_mib} MiB</span>
                </div>
                <div className="flex items-center gap-2">
                  <Calendar className="h-4 w-4 text-muted-foreground" />
                  <span className="text-muted-foreground">Created:</span>
                  <span className="text-xs">{format(new Date(template.created_at), "MMM d, yyyy")}</span>
                </div>
              </div>

              <Button
                onClick={() => setSelectedTemplate(template)}
                className="w-full"
              >
                <Play className="mr-2 h-4 w-4" />
                Deploy VM
              </Button>
            </CardContent>
          </Card>
        ))}
      </div>

      {selectedTemplate && (
        <InstantiateTemplateDialog
          template={selectedTemplate}
          open={!!selectedTemplate}
          onClose={() => setSelectedTemplate(null)}
        />
      )}
    </>
  )
}
```

---

#### Step 2: Create Instantiate Dialog

**File**: `apps/ui/components/templates/instantiate-template-dialog.tsx`

```tsx
"use client"

import { useState } from "react"
import { useRouter } from "next/navigation"
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { useToast } from "@/hooks/use-toast"
import { Template } from "@/lib/types"
import { facadeApi } from "@/lib/api/facade"
import { useMutation, useQueryClient } from "@tanstack/react-query"

interface InstantiateTemplateDialogProps {
  template: Template
  open: boolean
  onClose: () => void
}

export function InstantiateTemplateDialog({ template, open, onClose }: InstantiateTemplateDialogProps) {
  const [vmName, setVmName] = useState(`${template.name}-${Date.now().toString().slice(-4)}`)
  const router = useRouter()
  const { toast } = useToast()
  const queryClient = useQueryClient()

  const instantiateMutation = useMutation({
    mutationFn: (name: string) => facadeApi.instantiateTemplate(template.id, { name }),
    onSuccess: (data) => {
      toast({
        title: "VM Deployed",
        description: `VM "${vmName}" is being created from template.`,
      })
      queryClient.invalidateQueries({ queryKey: ["vms"] })
      onClose()
      router.push(`/vms/${data.id}`)
    },
    onError: (error: Error) => {
      toast({
        title: "Deployment Failed",
        description: error.message,
        variant: "destructive",
      })
    },
  })

  const handleDeploy = () => {
    if (!vmName.trim()) {
      toast({
        title: "Invalid Name",
        description: "Please enter a VM name.",
        variant: "destructive",
      })
      return
    }
    instantiateMutation.mutate(vmName)
  }

  return (
    <Dialog open={open} onOpenChange={onClose}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Deploy VM from Template</DialogTitle>
          <DialogDescription>
            Create a new VM using the "{template.name}" template configuration.
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

          <div className="rounded-lg border p-4 space-y-2 text-sm">
            <h4 className="font-medium">Template Configuration</h4>
            <div className="grid grid-cols-2 gap-2 text-muted-foreground">
              <div>vCPU: <span className="text-foreground font-mono">{template.spec.vcpu}</span></div>
              <div>RAM: <span className="text-foreground font-mono">{template.spec.mem_mib} MiB</span></div>
            </div>
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={onClose} disabled={instantiateMutation.isPending}>
            Cancel
          </Button>
          <Button onClick={handleDeploy} disabled={instantiateMutation.isPending}>
            {instantiateMutation.isPending ? "Deploying..." : "Deploy VM"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
```

---

#### Step 3: Create Template Dialog (Optional)

**File**: `apps/ui/components/templates/create-template-dialog.tsx`

```tsx
"use client"

import { useState } from "react"
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { useToast } from "@/hooks/use-toast"
import { facadeApi } from "@/lib/api/facade"
import { useMutation, useQueryClient } from "@tanstack/react-query"

interface CreateTemplateDialogProps {
  open: boolean
  onClose: () => void
}

export function CreateTemplateDialog({ open, onClose }: CreateTemplateDialogProps) {
  const [formData, setFormData] = useState({
    name: "",
    vcpu: 1,
    mem_mib: 512,
    kernel_path: "/srv/images/vmlinux-5.10.fc.bin",
    rootfs_path: "/srv/images/alpine-3.18.ext4",
  })

  const { toast } = useToast()
  const queryClient = useQueryClient()

  const createMutation = useMutation({
    mutationFn: () => facadeApi.createTemplate({
      name: formData.name,
      spec: {
        vcpu: formData.vcpu,
        mem_mib: formData.mem_mib,
        kernel_path: formData.kernel_path,
        rootfs_path: formData.rootfs_path,
      },
    }),
    onSuccess: () => {
      toast({
        title: "Template Created",
        description: `Template "${formData.name}" created successfully.`,
      })
      queryClient.invalidateQueries({ queryKey: ["templates"] })
      onClose()
    },
    onError: (error: Error) => {
      toast({
        title: "Creation Failed",
        description: error.message,
        variant: "destructive",
      })
    },
  })

  const handleCreate = () => {
    if (!formData.name.trim()) {
      toast({
        title: "Invalid Name",
        description: "Please enter a template name.",
        variant: "destructive",
      })
      return
    }
    createMutation.mutate()
  }

  return (
    <Dialog open={open} onOpenChange={onClose}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>Create Template</DialogTitle>
          <DialogDescription>
            Define a reusable VM configuration template.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4">
          <div className="space-y-2">
            <Label htmlFor="name">Template Name</Label>
            <Input
              id="name"
              value={formData.name}
              onChange={(e) => setFormData({ ...formData, name: e.target.value })}
              placeholder="e.g., Ubuntu 22.04 Base"
            />
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label htmlFor="vcpu">vCPU</Label>
              <Input
                id="vcpu"
                type="number"
                min={1}
                max={32}
                value={formData.vcpu}
                onChange={(e) => setFormData({ ...formData, vcpu: parseInt(e.target.value) })}
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="mem">RAM (MiB)</Label>
              <Input
                id="mem"
                type="number"
                min={128}
                max={16384}
                value={formData.mem_mib}
                onChange={(e) => setFormData({ ...formData, mem_mib: parseInt(e.target.value) })}
              />
            </div>
          </div>

          <div className="space-y-2">
            <Label htmlFor="kernel">Kernel Path</Label>
            <Input
              id="kernel"
              value={formData.kernel_path}
              onChange={(e) => setFormData({ ...formData, kernel_path: e.target.value })}
              placeholder="/srv/images/vmlinux-5.10.fc.bin"
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="rootfs">Rootfs Path</Label>
            <Input
              id="rootfs"
              value={formData.rootfs_path}
              onChange={(e) => setFormData({ ...formData, rootfs_path: e.target.value })}
              placeholder="/srv/images/alpine-3.18.ext4"
            />
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={onClose} disabled={createMutation.isPending}>
            Cancel
          </Button>
          <Button onClick={handleCreate} disabled={createMutation.isPending}>
            {createMutation.isPending ? "Creating..." : "Create Template"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
```

---

#### Step 4: Update Templates Page

**File**: `apps/ui/app/(dashboard)/templates/page.tsx`

```tsx
"use client"

import { useState } from "react"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Plus, AlertCircle } from "lucide-react"
import { TemplateList } from "@/components/templates/template-list"
import { CreateTemplateDialog } from "@/components/templates/create-template-dialog"
import { useTemplates } from "@/lib/queries"
import { Skeleton } from "@/components/ui/skeleton"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"

export default function TemplatesPage() {
  const { data: templates = [], isLoading, error } = useTemplates()
  const [createDialogOpen, setCreateDialogOpen] = useState(false)

  return (
    <div className="space-y-6">
      <div className="relative overflow-hidden rounded-xl border border-border bg-gradient-to-br from-orange-50 to-orange-100/50 p-8">
        <div className="relative z-10 flex items-center justify-between">
          <div className="max-w-xl">
            <h1 className="text-3xl font-bold text-foreground">VM Templates</h1>
            <p className="mt-2 text-muted-foreground">
              Save and deploy VM configurations as templates. Quickly spin up new instances with pre-configured
              settings.
            </p>
            <Button className="mt-4" onClick={() => setCreateDialogOpen(true)}>
              <Plus className="mr-2 h-4 w-4" />
              Create Template
            </Button>
          </div>
        </div>
        <div className="absolute right-0 top-0 h-64 w-64 translate-x-32 -translate-y-32 rounded-full bg-gradient-to-br from-orange-400/30 to-orange-600/30 blur-3xl" />
      </div>

      <Card>
        <CardHeader>
          <CardTitle>All Templates ({templates.length})</CardTitle>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
              {[...Array(3)].map((_, i) => (
                <div key={i} className="p-6 border rounded-lg space-y-4">
                  <Skeleton className="h-6 w-48" />
                  <Skeleton className="h-4 w-32" />
                  <Skeleton className="h-10 w-full" />
                </div>
              ))}
            </div>
          ) : error ? (
            <Alert variant="destructive">
              <AlertCircle className="h-4 w-4" />
              <AlertTitle>Error</AlertTitle>
              <AlertDescription>
                Failed to load templates. Please try again later.
              </AlertDescription>
            </Alert>
          ) : (
            <TemplateList templates={templates} />
          )}
        </CardContent>
      </Card>

      <CreateTemplateDialog
        open={createDialogOpen}
        onClose={() => setCreateDialogOpen(false)}
      />
    </div>
  )
}
```

---

#### Step 5: Export Components

**File**: `apps/ui/components/templates/index.ts`

```ts
export { TemplateList } from "./template-list"
export { CreateTemplateDialog } from "./create-template-dialog"
export { InstantiateTemplateDialog } from "./instantiate-template-dialog"
```

---

## Testing Checklist

### Backend Testing (with curl)

```bash
# 1. Create a template
curl -X POST http://localhost:18080/v1/templates \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Test Template",
    "spec": {
      "vcpu": 1,
      "mem_mib": 512,
      "kernel_path": "/srv/images/vmlinux-5.10.fc.bin",
      "rootfs_path": "/srv/images/alpine-3.18.ext4"
    }
  }'

# 2. List templates
curl http://localhost:18080/v1/templates

# 3. Get template details
curl http://localhost:18080/v1/templates/{template-id}

# 4. Update template
curl -X PUT http://localhost:18080/v1/templates/{template-id} \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Test Template Updated",
    "spec": {
      "vcpu": 2,
      "mem_mib": 1024,
      "kernel_path": "/srv/images/vmlinux-5.10.fc.bin",
      "rootfs_path": "/srv/images/alpine-3.18.ext4"
    }
  }'

# 5. Delete template
curl -X DELETE http://localhost:18080/v1/templates/{template-id}

# 6. Instantiate template (deploy VM)
curl -X POST http://localhost:18080/v1/templates/{template-id}/instantiate \
  -H "Content-Type: application/json" \
  -d '{"name": "my-test-vm"}'

# 7. Verify VM was created
curl http://localhost:18080/v1/vms/{vm-id}
```

### Frontend Testing

1. ✅ Templates page loads without errors
2. ✅ Template list displays correctly
3. ✅ Create template dialog opens and validates input
4. ✅ Create template successfully adds to list
5. ✅ Deploy button opens instantiate dialog
6. ✅ Instantiate creates VM and redirects to VM detail page
7. ✅ Loading states show during API calls
8. ✅ Error messages display when API fails

---

## Database Schema

```sql
CREATE TABLE template (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    spec_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- VMs track which template they were created from
ALTER TABLE vm ADD COLUMN template_id UUID;
ALTER TABLE vm ADD CONSTRAINT fk_vm_template
    FOREIGN KEY (template_id) REFERENCES template(id) ON DELETE SET NULL;
```

---

## Future Enhancements

1. **Template from VM** - Button on VM detail page to create template from existing VM
2. **Template Tags** - Add tags/categories for organization
3. **Template Sharing** - Export/import templates as JSON files
4. **Network Templates** - Include network configuration in templates
5. **Storage Templates** - Include additional drives in templates

---

## Common Issues & Solutions

### Issue: "Failed to instantiate template"
**Cause**: Kernel or rootfs path doesn't exist on host
**Solution**: Verify paths exist in `/srv/images/` or use image registry IDs

### Issue: Templates not showing in UI
**Cause**: API call failing or CORS issue
**Solution**: Check browser console, verify manager is running on port 18080

### Issue: VM doesn't start after instantiation
**Cause**: Invalid template spec (missing kernel/rootfs)
**Solution**: Add validation in create template dialog

---

## Contact & Support

For questions or issues:
- Check manager logs: `RUST_LOG=info ./target/debug/manager`
- Review OpenAPI docs: `http://localhost:18080/api-docs/openapi.yaml`
- Ask in team chat

---

**Last Updated**: 2025-11-06
**Backend Version**: Fully implemented (including update and delete endpoints)
**Frontend Version**: Components ready to build
