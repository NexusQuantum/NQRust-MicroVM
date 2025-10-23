"use client"

import { useRouter } from "next/navigation"
import { VMCreateWizard } from "@/components/vm/vm-create-wizard"

export default function CreateVMPage() {
  const router = useRouter()

  const handleComplete = () => {
    // Navigate back to VMs page after successful creation
    router.push("/vms")
  }

  const handleCancel = () => {
    // Navigate back to VMs page on cancel
    router.push("/vms")
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold text-foreground">Create Virtual Machine</h1>
        <p className="text-muted-foreground">Configure and deploy a new VM</p>
      </div>

      <VMCreateWizard onComplete={handleComplete} onCancel={handleCancel} />
    </div>
  )
}
