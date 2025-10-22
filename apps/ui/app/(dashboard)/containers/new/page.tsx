import { ContainerDeployForm } from "@/components/container/container-deploy-form"

export default function NewContainerPage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold text-foreground">Deploy Container</h1>
        <p className="text-muted-foreground">Configure and deploy a new container</p>
      </div>

      <ContainerDeployForm />
    </div>
  )
}
