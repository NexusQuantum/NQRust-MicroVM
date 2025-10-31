"use client"

import { useState, useEffect } from "react"
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Slider } from "@/components/ui/slider"
import { Plus, X, Loader2 } from "lucide-react"
import { useUpdateContainer } from "@/lib/queries"
import type { Container } from "@/lib/types"

interface EditContainerDialogProps {
  container: Container
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function EditContainerDialog({ container, open, onOpenChange }: EditContainerDialogProps) {
  const updateContainer = useUpdateContainer()

  const [name, setName] = useState(container.name)
  const [cpuLimit, setCpuLimit] = useState(container.cpu_limit || 1)
  const [memoryLimit, setMemoryLimit] = useState(container.memory_limit_mb || 512)
  const [restartPolicy, setRestartPolicy] = useState(container.restart_policy || "no")
  const [envVars, setEnvVars] = useState<Array<{ key: string; value: string }>>(
    Object.entries(container.env_vars || {}).map(([key, value]) => ({ key, value }))
  )

  // Reset form when container changes
  useEffect(() => {
    setName(container.name)
    setCpuLimit(container.cpu_limit || 1)
    setMemoryLimit(container.memory_limit_mb || 512)
    setRestartPolicy(container.restart_policy || "no")
    setEnvVars(Object.entries(container.env_vars || {}).map(([key, value]) => ({ key, value })))
  }, [container])

  const addEnvVar = () => {
    setEnvVars([...envVars, { key: "", value: "" }])
  }

  const removeEnvVar = (index: number) => {
    setEnvVars(envVars.filter((_, i) => i !== index))
  }

  const handleSubmit = async () => {
    if (!name.trim()) {
      return
    }

    const envVarsObj = envVars
      .filter((e) => e.key && e.value)
      .reduce((acc, e) => ({ ...acc, [e.key]: e.value }), {})

    updateContainer.mutate(
      {
        id: container.id,
        params: {
          name: name.trim(),
          env_vars: Object.keys(envVarsObj).length > 0 ? envVarsObj : undefined,
          cpu_limit: cpuLimit,
          memory_limit_mb: memoryLimit,
          restart_policy: restartPolicy,
        },
      },
      {
        onSuccess: () => {
          onOpenChange(false)
        },
      }
    )
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl max-h-[80vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>Edit Container</DialogTitle>
        </DialogHeader>

        <div className="space-y-6 py-4">
          <div className="space-y-2">
            <Label htmlFor="edit-name">Container Name</Label>
            <Input id="edit-name" value={name} onChange={(e) => setName(e.target.value)} />
          </div>

          <div className="space-y-4">
            <h3 className="text-sm font-semibold">Resources</h3>

            <div className="space-y-2">
              <Label>CPU Limit: {cpuLimit} cores</Label>
              <Slider value={[cpuLimit]} onValueChange={(v) => setCpuLimit(v[0])} min={0.1} max={16} step={0.1} />
            </div>

            <div className="space-y-2">
              <Label>Memory Limit: {memoryLimit} MB</Label>
              <Slider value={[memoryLimit]} onValueChange={(v) => setMemoryLimit(v[0])} min={64} max={32768} step={64} />
            </div>
          </div>

          <div className="space-y-2">
            <Label htmlFor="edit-restart-policy">Restart Policy</Label>
            <Select value={restartPolicy} onValueChange={setRestartPolicy}>
              <SelectTrigger id="edit-restart-policy">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="no">No</SelectItem>
                <SelectItem value="always">Always</SelectItem>
                <SelectItem value="on-failure">On Failure</SelectItem>
                <SelectItem value="unless-stopped">Unless Stopped</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <Label>Environment Variables</Label>
              <Button onClick={addEnvVar} size="sm" variant="outline">
                <Plus className="mr-2 h-4 w-4" />
                Add Variable
              </Button>
            </div>

            {envVars.length === 0 ? (
              <p className="text-sm text-muted-foreground">No environment variables configured</p>
            ) : (
              <div className="space-y-2">
                {envVars.map((envVar, i) => (
                  <div key={i} className="flex items-center gap-2">
                    <Input
                      placeholder="KEY"
                      value={envVar.key}
                      onChange={(e) => {
                        const newEnvVars = [...envVars]
                        newEnvVars[i].key = e.target.value
                        setEnvVars(newEnvVars)
                      }}
                    />
                    <Input
                      placeholder="value"
                      value={envVar.value}
                      onChange={(e) => {
                        const newEnvVars = [...envVars]
                        newEnvVars[i].value = e.target.value
                        setEnvVars(newEnvVars)
                      }}
                    />
                    <Button variant="ghost" size="icon" onClick={() => removeEnvVar(i)}>
                      <X className="h-4 w-4" />
                    </Button>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={updateContainer.isPending}>
            Cancel
          </Button>
          <Button onClick={handleSubmit} disabled={!name.trim() || updateContainer.isPending}>
            {updateContainer.isPending && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
            Save Changes
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
