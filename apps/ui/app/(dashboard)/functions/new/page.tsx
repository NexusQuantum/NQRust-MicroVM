"use client"

import { useRouter } from "next/navigation"
import { useEffect, useState } from "react"
import { useToast } from "@/components/ui/use-toast"
import { FunctionEditor } from "@/components/function/function-editor"

type Draft = { runtime?: "node" | "python"; code?: string; event?: string }

export default function NewFunctionPage() {
  const router = useRouter()
  const { toast } = useToast()
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
    // Navigate back to VMs page after successful creation
    toast({
      title: "Function created",
      description: `Function "${p?.name ?? "untitled"}" successfully created.`,
      variant: "success",
      duration: 2000,
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
