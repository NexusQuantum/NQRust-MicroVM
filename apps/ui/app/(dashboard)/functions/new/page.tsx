"use client"

import { useRouter } from "next/navigation"
import { useToast } from "@/components/ui/use-toast"
import { FunctionEditor } from "@/components/function/function-editor"

export default function NewFunctionPage() {
  const router = useRouter()
  const { toast } = useToast()

  const handleComplete = (p?: { name?: string }) => {
    // Navigate back to VMs page after successful creation
    toast({
      title: "Function created",
      description: `Function "${p?.name ?? "untitled"}" successfully created.`,
    })
    router.push("/functions")
  }

  const handleCancel = () => {
    // Navigate back to VMs page on cancel
    router.push("/functions")
  }
  return <FunctionEditor mode="create" onComplete={handleComplete} onCancel={handleCancel} />
}
