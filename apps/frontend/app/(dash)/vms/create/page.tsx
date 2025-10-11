"use client"

import { VMCreationWizard } from "@/components/vm-creation-wizard"
import { useRouter } from "next/navigation"

export default function CreateVMPage() {
  const router = useRouter()

  const handleComplete = () => {
    router.push("/vms")
  }

  const handleCancel = () => {
    router.push("/vms")
  }

  return (
    <div className="container mx-auto py-8">
      <VMCreationWizard onComplete={handleComplete} onCancel={handleCancel} />
    </div>
  )
}
