"use client"

import { useState } from "react"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Slider } from "@/components/ui/slider"
import { Plus, X } from "lucide-react"

export function ContainerDeployForm() {
  const [name, setName] = useState("")
  const [image, setImage] = useState("")
  const [cpuLimit, setCpuLimit] = useState(1)
  const [memoryLimit, setMemoryLimit] = useState(512)
  const [ports, setPorts] = useState<Array<{ host: string; container: string; protocol: string }>>([])
  const [envVars, setEnvVars] = useState<Array<{ key: string; value: string }>>([])
  const [volumes, setVolumes] = useState<Array<{ host: string; container: string }>>([])

  const addPort = () => {
    setPorts([...ports, { host: "", container: "", protocol: "tcp" }])
  }

  const removePort = (index: number) => {
    setPorts(ports.filter((_, i) => i !== index))
  }

  const addEnvVar = () => {
    setEnvVars([...envVars, { key: "", value: "" }])
  }

  const removeEnvVar = (index: number) => {
    setEnvVars(envVars.filter((_, i) => i !== index))
  }

  const addVolume = () => {
    setVolumes([...volumes, { host: "", container: "" }])
  }

  const removeVolume = (index: number) => {
    setVolumes(volumes.filter((_, i) => i !== index))
  }

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle>Basic Configuration</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="name">Container Name</Label>
            <Input id="name" value={name} onChange={(e) => setName(e.target.value)} placeholder="my-container" />
          </div>

          <div className="space-y-2">
            <Label htmlFor="image">Image</Label>
            <Input
              id="image"
              value={image}
              onChange={(e) => setImage(e.target.value)}
              placeholder="postgres:15, nginx:latest, redis:7-alpine"
            />
            <p className="text-xs text-muted-foreground">Popular: postgres, nginx, redis, mongo, mysql</p>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Resources</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <Label>CPU Limit: {cpuLimit} cores</Label>
            <Slider value={[cpuLimit]} onValueChange={(v) => setCpuLimit(v[0])} min={0.1} max={16} step={0.1} />
          </div>

          <div className="space-y-2">
            <Label>Memory Limit: {memoryLimit} MB</Label>
            <Slider value={[memoryLimit]} onValueChange={(v) => setMemoryLimit(v[0])} min={64} max={32768} step={64} />
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <CardTitle>Port Mappings</CardTitle>
          <Button onClick={addPort} size="sm">
            <Plus className="mr-2 h-4 w-4" />
            Add Port
          </Button>
        </CardHeader>
        <CardContent className="space-y-3">
          {ports.length === 0 ? (
            <p className="text-sm text-muted-foreground">No port mappings configured</p>
          ) : (
            ports.map((port, i) => (
              <div key={i} className="flex items-center gap-2">
                <Input
                  placeholder="Host port"
                  value={port.host}
                  onChange={(e) => {
                    const newPorts = [...ports]
                    newPorts[i].host = e.target.value
                    setPorts(newPorts)
                  }}
                />
                <span>→</span>
                <Input
                  placeholder="Container port"
                  value={port.container}
                  onChange={(e) => {
                    const newPorts = [...ports]
                    newPorts[i].container = e.target.value
                    setPorts(newPorts)
                  }}
                />
                <Select
                  value={port.protocol}
                  onValueChange={(value) => {
                    const newPorts = [...ports]
                    newPorts[i].protocol = value
                    setPorts(newPorts)
                  }}
                >
                  <SelectTrigger className="w-24">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="tcp">TCP</SelectItem>
                    <SelectItem value="udp">UDP</SelectItem>
                  </SelectContent>
                </Select>
                <Button variant="ghost" size="icon" onClick={() => removePort(i)}>
                  <X className="h-4 w-4" />
                </Button>
              </div>
            ))
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <CardTitle>Environment Variables</CardTitle>
          <Button onClick={addEnvVar} size="sm">
            <Plus className="mr-2 h-4 w-4" />
            Add Variable
          </Button>
        </CardHeader>
        <CardContent className="space-y-3">
          {envVars.length === 0 ? (
            <p className="text-sm text-muted-foreground">No environment variables configured</p>
          ) : (
            envVars.map((envVar, i) => (
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
            ))
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <CardTitle>Volumes</CardTitle>
          <Button onClick={addVolume} size="sm">
            <Plus className="mr-2 h-4 w-4" />
            Add Volume
          </Button>
        </CardHeader>
        <CardContent className="space-y-3">
          {volumes.length === 0 ? (
            <p className="text-sm text-muted-foreground">No volumes configured</p>
          ) : (
            volumes.map((volume, i) => (
              <div key={i} className="flex items-center gap-2">
                <Input
                  placeholder="Host path"
                  value={volume.host}
                  onChange={(e) => {
                    const newVolumes = [...volumes]
                    newVolumes[i].host = e.target.value
                    setVolumes(newVolumes)
                  }}
                />
                <span>→</span>
                <Input
                  placeholder="Container path"
                  value={volume.container}
                  onChange={(e) => {
                    const newVolumes = [...volumes]
                    newVolumes[i].container = e.target.value
                    setVolumes(newVolumes)
                  }}
                />
                <Button variant="ghost" size="icon" onClick={() => removeVolume(i)}>
                  <X className="h-4 w-4" />
                </Button>
              </div>
            ))
          )}
        </CardContent>
      </Card>

      <div className="flex justify-end gap-2">
        <Button variant="outline">Cancel</Button>
        <Button>Deploy Container</Button>
      </div>
    </div>
  )
}
