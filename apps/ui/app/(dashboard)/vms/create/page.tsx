import { VMCreateWizard } from "@/components/vm/vm-create-wizard"

export default function CreateVMPage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold text-foreground">Create Virtual Machine</h1>
        <p className="text-muted-foreground">Configure and deploy a new VM</p>
      </div>

      <VMCreateWizard />
    </div>
  )
}
