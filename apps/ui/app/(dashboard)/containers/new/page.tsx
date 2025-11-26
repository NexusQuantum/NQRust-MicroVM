"use client"

import { ContainerDeployForm } from "@/components/container/container-deploy-form"
import { Button } from "@/components/ui/button"
import { ArrowLeft } from "lucide-react"
import { useRouter } from "next/navigation"

export default function NewContainerPage() {
  const router = useRouter()

  return (
    <div className="space-y-6">
      <div className="flex items-start justify-between">
        <div className="flex-1">
          <div className="flex items-center gap-3 mb-2">
            <Button variant="ghost" size="icon" onClick={() => router.push("/containers")}>
              <ArrowLeft className="h-4 w-4" />
            </Button>
            <h1 className="text-3xl font-bold text-foreground">Deploy Container</h1>
          </div>
          <p className="text-muted-foreground ml-12">Configure and deploy a new container</p>
        </div>
      </div>
      <ContainerDeployForm />
    </div>
  )
}
