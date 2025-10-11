"use client"

import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { RegistryBrowser } from "@/components/registry-browser"
import { useRouter } from "next/navigation"

export default function RegistryPage() {
  const router = useRouter()

  const handleSelect = (type: 'kernel' | 'rootfs') => (id: string) => {
    // Store the selected image ID for VM creation
    if (type === 'kernel') {
      try { localStorage.setItem('NR_PREFILL_KERNEL_ID', id) } catch {}
    } else {
      try { localStorage.setItem('NR_PREFILL_ROOTFS_ID', id) } catch {}
    }
    router.push('/vms/create')
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold tracking-tight">Registry</h1>
        <p className="text-muted-foreground">Manage kernel images and root filesystems. Upload, import, rename, delete, and use them when creating VMs.</p>
      </div>

      <Tabs defaultValue="kernel" className="w-full">
        <TabsList>
          <TabsTrigger value="kernel">Kernels</TabsTrigger>
          <TabsTrigger value="rootfs">RootFS</TabsTrigger>
        </TabsList>
        <TabsContent value="kernel" className="mt-4">
          <RegistryBrowser type="kernel" onSelect={handleSelect('kernel')} onCancel={() => {}} />
        </TabsContent>
        <TabsContent value="rootfs" className="mt-4">
          <RegistryBrowser type="rootfs" onSelect={handleSelect('rootfs')} onCancel={() => {}} />
        </TabsContent>
      </Tabs>
    </div>
  )
}
