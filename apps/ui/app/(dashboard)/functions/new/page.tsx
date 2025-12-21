"use client"

import { useRouter } from "next/navigation"
import { useEffect, useState } from "react"
import { toast } from "sonner"
import { FunctionEditor } from "@/components/function/function-editor"

type Draft = { runtime?: "python" | "javascript" | "typescript"; code?: string; event?: string }

export default function NewFunctionPage() {
  const router = useRouter()
  const [initial, setInitial] = useState<Draft | null>(null)

  useEffect(() => {
    try {
      const raw = sessionStorage.getItem("playground:draft")
      if (raw) {
        const parsed = JSON.parse(raw) as Draft
        setInitial(parsed)
      }
    } catch { }
    // sekali pakai: selalu bersihkan
    try { sessionStorage.removeItem("playground:draft") } catch { }
  }, [])


  const handleComplete = (p?: { name?: string }) => {
    // Show success toast
    toast.success("Function Created Successfully", {
      description: `Function "${p?.name ?? "untitled"}" has been created and is ready to use`
    })
    router.push("/functions")
  }

  const handleCancel = () => {
    // Navigate back to VMs page on cancel
    router.push("/functions")
  }
  return <FunctionEditor mode="create"
    onComplete={handleComplete}
    onCancel={handleCancel}
    initialRuntime={initial?.runtime}
    initialCode={initial?.code}
    initialEvent={initial?.event} />
}
