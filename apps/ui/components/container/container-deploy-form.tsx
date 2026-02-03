"use client"

import { useState, useMemo } from "react"
import { useRouter } from "next/navigation"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Slider } from "@/components/ui/slider"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Badge } from "@/components/ui/badge"
import { Checkbox } from "@/components/ui/checkbox"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Plus, X, Loader2, Archive, Upload as UploadIcon, HardDrive, Trash2 } from "lucide-react"
import { useCreateContainer, useRegistryImages, useUploadImage, useVolumes } from "@/lib/queries"
import { parseFacadeError } from "@/lib/api/http"
import { toast } from "sonner"
import type { CreateContainerReq } from "@/lib/types"

export function ContainerDeployForm() {
  const router = useRouter()
  const createContainer = useCreateContainer()
  const uploadImage = useUploadImage()
  const { data: registryImages = [] } = useRegistryImages()

  const [name, setName] = useState("")
  const [image, setImage] = useState("")
  const [imageSource, setImageSource] = useState<"registry" | "dockerhub" | "upload">("registry")
  const [selectedFile, setSelectedFile] = useState<File | null>(null)
  const [cpuLimit, setCpuLimit] = useState(1)
  const [memoryLimit, setMemoryLimit] = useState(512)
  const [ports, setPorts] = useState<Array<{ host: string; container: string; protocol: string }>>([])
  const [envVars, setEnvVars] = useState<Array<{ key: string; value: string }>>([])
  // Volume with extended fields for better UX
  interface ContainerVolume {
    id: string
    name: string
    hostPath: string
    containerPath: string
    sizeMb: number
    readOnly: boolean
    source: "new" | "existing"
  }

  const [volumes, setVolumes] = useState<ContainerVolume[]>([])
  const [showVolumeDialog, setShowVolumeDialog] = useState(false)
  const [volumeFormData, setVolumeFormData] = useState({
    name: "",
    hostPath: "",
    containerPath: "",
    sizeMb: "1024",
    readOnly: false,
    source: "new" as "new" | "existing",
  })

  // Registry auth fields
  const [usePrivateRegistry, setUsePrivateRegistry] = useState(false)
  const [registryUsername, setRegistryUsername] = useState("")
  const [registryPassword, setRegistryPassword] = useState("")
  const [registryServer, setRegistryServer] = useState("")

  // Filter docker images from registry
  const dockerImages = useMemo(() =>
    registryImages.filter((img) => img.kind === "docker"),
    [registryImages]
  )

  // Fetch available volumes for picker
  const { data: availableVolumes = [] } = useVolumes()

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
    setVolumeFormData({
      name: "",
      hostPath: "",
      containerPath: "",
      sizeMb: "1024",
      readOnly: false,
      source: "new",
    })
    setShowVolumeDialog(true)
  }

  const handleAddVolume = () => {
    const newVolume: ContainerVolume = {
      id: crypto.randomUUID(),
      name: volumeFormData.name || `volume-${volumes.length + 1}`,
      hostPath: volumeFormData.source === "existing"
        ? volumeFormData.hostPath
        : `/srv/container-data/${volumeFormData.name || `volume-${volumes.length + 1}`}`,
      containerPath: volumeFormData.containerPath,
      sizeMb: parseInt(volumeFormData.sizeMb) || 1024,
      readOnly: volumeFormData.readOnly,
      source: volumeFormData.source,
    }
    setVolumes([...volumes, newVolume])
    setShowVolumeDialog(false)
  }

  const removeVolume = (id: string) => {
    setVolumes(volumes.filter(v => v.id !== id))
  }

  const handleSubmit = async () => {
    if (!name.trim()) {
      return
    }

    // If upload mode, upload first
    if (imageSource === "upload" && selectedFile) {
      uploadImage.mutate(
        {
          file: selectedFile,
          kind: "docker",
          name: image.trim() || selectedFile.name,
        },
        {
          onSuccess: (uploadResp) => {
            // After upload, create container with the uploaded image
            proceedWithContainerCreation(image.trim() || selectedFile.name)
          },
        }
      )
      return
    }

    // Otherwise proceed normally
    if (!image.trim()) {
      return
    }
    proceedWithContainerCreation(image.trim())
  }

  const proceedWithContainerCreation = (imageName: string) => {
    const portMappings = ports
      .filter((p) => p.host && p.container)
      .map((p) => ({
        host: parseInt(p.host),
        container: parseInt(p.container),
        protocol: p.protocol as "tcp" | "udp",
      }))

    const envVarsObj = envVars
      .filter((e) => e.key && e.value)
      .reduce((acc, e) => ({ ...acc, [e.key]: e.value }), {})

    const volumeMounts = volumes
      .filter((v) => v.hostPath && v.containerPath)
      .map((v) => ({
        host: v.hostPath,
        container: v.containerPath,
        read_only: v.readOnly,
      }))

    const registryAuth = usePrivateRegistry && registryUsername && registryPassword
      ? {
        username: registryUsername,
        password: registryPassword,
        server_address: registryServer || undefined,
      }
      : undefined

    const params: CreateContainerReq = {
      name: name.trim(),
      image: imageName,
      cpu_limit: cpuLimit,
      memory_limit_mb: memoryLimit,
      port_mappings: portMappings.length > 0 ? portMappings : undefined,
      env_vars: Object.keys(envVarsObj).length > 0 ? envVarsObj : undefined,
      volumes: volumeMounts.length > 0 ? volumeMounts : undefined,
      restart_policy: "no",
      registry_auth: registryAuth,
    }

    createContainer.mutate(params, {
      onSuccess: (data) => {
        router.push(`/containers/${data.id}`)
      },
      onError: (error: any) => {
        // Parse the facade error to get a user-friendly message
        const facadeError = parseFacadeError(error)
        let errorMessage = "Failed to create container"

        if (facadeError) {
          // Use the error message from the backend
          errorMessage = facadeError.error || facadeError.fault_message || errorMessage
        } else if (error?.message) {
          // Try to extract message directly
          errorMessage = error.message
        }

        toast.error("Container Creation Failed", {
          description: errorMessage,
          duration: 5000,
        })
      },
    })
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
            <Label>Image Source</Label>
            <Tabs value={imageSource} onValueChange={(v) => setImageSource(v as "registry" | "dockerhub" | "upload")}>
              <TabsList className="grid w-full grid-cols-3">
                <TabsTrigger value="registry">
                  <Archive className="mr-2 h-4 w-4" />
                  Registry ({dockerImages.length})
                </TabsTrigger>
                <TabsTrigger value="dockerhub">Docker Hub</TabsTrigger>
                <TabsTrigger value="upload">
                  <UploadIcon className="mr-2 h-4 w-4" />
                  Upload
                </TabsTrigger>
              </TabsList>

              <TabsContent value="registry" className="space-y-2 mt-4">
                {dockerImages.length === 0 ? (
                  <div className="text-sm text-muted-foreground p-4 border border-dashed rounded-lg text-center">
                    No cached images. Visit the Registry page to download images from Docker Hub.
                  </div>
                ) : (
                  <>
                    <Label htmlFor="registry-image">Select Cached Image</Label>
                    <Select value={image} onValueChange={setImage}>
                      <SelectTrigger id="registry-image">
                        <SelectValue placeholder="Select an image from registry" />
                      </SelectTrigger>
                      <SelectContent>
                        {dockerImages.map((img) => (
                          <SelectItem key={img.id} value={img.name}>
                            <div className="flex items-center gap-2">
                              <span className="font-mono text-sm">{img.name}</span>
                              <span className="text-xs text-muted-foreground">
                                ({(img.size / 1024 / 1024).toFixed(0)} MB)
                              </span>
                            </div>
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </>
                )}
              </TabsContent>

              <TabsContent value="dockerhub" className="space-y-2 mt-4">
                <Label htmlFor="dockerhub-image">Image Name</Label>
                <Input
                  id="dockerhub-image"
                  value={image}
                  onChange={(e) => setImage(e.target.value)}
                  placeholder="nginx:latest, postgres:15, redis:7-alpine"
                />
                <p className="text-xs text-muted-foreground">
                  Enter any Docker Hub image. Popular: nginx, postgres, redis, mongo, mysql
                </p>
              </TabsContent>

              <TabsContent value="upload" className="space-y-2 mt-4">
                <Label htmlFor="upload-file">Docker Image Tarball</Label>
                <div className="flex items-center gap-2">
                  <Input
                    id="upload-file"
                    type="file"
                    accept=".tar,.tar.gz"
                    onChange={(e) => {
                      const file = e.target.files?.[0]
                      if (file) {
                        setSelectedFile(file)
                        if (!image) {
                          setImage(file.name.replace(/\.(tar|tar\.gz)$/, ""))
                        }
                      }
                    }}
                  />
                </div>
                {selectedFile && (
                  <p className="text-xs text-muted-foreground">
                    Selected: {selectedFile.name} ({(selectedFile.size / 1024 / 1024).toFixed(1)} MB)
                  </p>
                )}
                <div className="rounded-lg bg-blue-50 dark:bg-blue-950 p-3 text-sm text-blue-900 dark:text-blue-100">
                  <p className="font-medium mb-1">Export Docker images using:</p>
                  <code className="text-xs bg-blue-100 dark:bg-blue-900 px-2 py-1 rounded block">
                    docker save -o image.tar image:tag
                  </code>
                </div>
              </TabsContent>
            </Tabs>
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
            <Slider value={[memoryLimit]} onValueChange={(v) => setMemoryLimit(v[0])} min={512} max={32768} step={64} />
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
          <div className="flex items-center gap-2">
            <HardDrive className="h-5 w-5" />
            <CardTitle>Volume Mounts</CardTitle>
          </div>
          <Button onClick={addVolume} size="sm">
            <Plus className="mr-2 h-4 w-4" />
            Add Volume
          </Button>
        </CardHeader>
        <CardContent>
          {volumes.length === 0 ? (
            <p className="text-sm text-muted-foreground">No volumes configured. Click "Add Volume" to mount storage into the container.</p>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Name</TableHead>
                  <TableHead>Host Path</TableHead>
                  <TableHead>Container Path</TableHead>
                  <TableHead>Size</TableHead>
                  <TableHead>Read Only</TableHead>
                  <TableHead className="text-right">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {volumes.map((volume) => (
                  <TableRow key={volume.id}>
                    <TableCell className="font-medium">
                      {volume.name}
                      {volume.source === "new" && (
                        <Badge variant="outline" className="ml-2 bg-green-100 text-green-700 border-green-200">
                          New
                        </Badge>
                      )}
                    </TableCell>
                    <TableCell className="font-mono text-sm">{volume.hostPath}</TableCell>
                    <TableCell className="font-mono text-sm">{volume.containerPath}</TableCell>
                    <TableCell>{volume.sizeMb} MB</TableCell>
                    <TableCell>
                      {volume.readOnly ? (
                        <Badge variant="outline" className="bg-yellow-100 text-yellow-700 border-yellow-200">
                          Yes
                        </Badge>
                      ) : (
                        <span className="text-muted-foreground">No</span>
                      )}
                    </TableCell>
                    <TableCell className="text-right">
                      <Button
                        variant="ghost"
                        size="icon"
                        onClick={() => removeVolume(volume.id)}
                      >
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      {/* Add Volume Dialog */}
      <Dialog open={showVolumeDialog} onOpenChange={setShowVolumeDialog}>
        <DialogContent className="max-w-2xl max-h-[85vh] overflow-hidden flex flex-col">
          <DialogHeader>
            <DialogTitle>Add New Volume</DialogTitle>
            <DialogDescription>
              Mount a volume into the container. You can create a new volume or use an existing one.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4 overflow-y-auto flex-1">
            <div className="space-y-2">
              <Label>Volume Source</Label>
              <Select
                value={volumeFormData.source}
                onValueChange={(value: "new" | "existing") =>
                  setVolumeFormData({ ...volumeFormData, source: value })
                }
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="new">Create New Volume</SelectItem>
                  <SelectItem value="existing">Use Existing Volume</SelectItem>
                </SelectContent>
              </Select>
            </div>

            {volumeFormData.source === "new" ? (
              <>
                <div className="space-y-2">
                  <Label htmlFor="volume_name">Volume Name *</Label>
                  <Input
                    id="volume_name"
                    placeholder="e.g., data, logs, config"
                    value={volumeFormData.name}
                    onChange={(e) => setVolumeFormData({ ...volumeFormData, name: e.target.value })}
                  />
                  <p className="text-xs text-muted-foreground">Unique identifier for this volume</p>
                </div>

                <div className="space-y-2">
                  <Label htmlFor="volume_size">Size (MB) *</Label>
                  <Input
                    id="volume_size"
                    type="number"
                    placeholder="1024"
                    value={volumeFormData.sizeMb}
                    onChange={(e) => setVolumeFormData({ ...volumeFormData, sizeMb: e.target.value })}
                  />
                  <p className="text-xs text-muted-foreground">Size for the volume in megabytes</p>
                </div>
              </>
            ) : (
              <div className="space-y-2">
                <Label>Select Existing Volume</Label>
                <Select
                  value={volumeFormData.hostPath}
                  onValueChange={(value) =>
                    setVolumeFormData({
                      ...volumeFormData,
                      hostPath: value,
                      name: availableVolumes.find(v => v.path === value)?.name || "existing-volume"
                    })
                  }
                >
                  <SelectTrigger>
                    <SelectValue placeholder="Select a volume" />
                  </SelectTrigger>
                  <SelectContent className="max-h-[200px]" position="popper" sideOffset={4}>
                    {availableVolumes.length === 0 ? (
                      <SelectItem value="" disabled>No volumes available</SelectItem>
                    ) : (
                      availableVolumes.map((vol) => (
                        <SelectItem key={vol.id} value={vol.path}>
                          <div className="flex flex-col">
                            <span className="font-medium">{vol.name}</span>
                            <span className="text-xs text-muted-foreground">
                              {vol.path} • {Math.round(vol.size_bytes / 1024 / 1024)} MB
                            </span>
                          </div>
                        </SelectItem>
                      ))
                    )}
                  </SelectContent>
                </Select>
              </div>
            )}

            <div className="space-y-2">
              <Label htmlFor="container_path">Container Path *</Label>
              <Input
                id="container_path"
                placeholder="e.g., /data, /var/log, /app/config"
                value={volumeFormData.containerPath}
                onChange={(e) => setVolumeFormData({ ...volumeFormData, containerPath: e.target.value })}
              />
              <p className="text-xs text-muted-foreground">Path where the volume will be mounted inside the container</p>
            </div>

            <div className="flex items-center space-x-2">
              <Checkbox
                id="read_only"
                checked={volumeFormData.readOnly}
                onCheckedChange={(checked) =>
                  setVolumeFormData({ ...volumeFormData, readOnly: checked as boolean })
                }
              />
              <Label htmlFor="read_only" className="cursor-pointer">Read-only</Label>
            </div>
          </div>
          <DialogFooter className="mt-auto pt-4 border-t">
            <Button variant="outline" onClick={() => setShowVolumeDialog(false)}>
              Cancel
            </Button>
            <Button
              onClick={handleAddVolume}
              disabled={!volumeFormData.containerPath || (volumeFormData.source === "new" && !volumeFormData.name)}
            >
              Add Volume
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Card>
        <CardHeader>
          <CardTitle>Private Registry Authentication (Optional)</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center space-x-2">
            <input
              type="checkbox"
              id="use-private-registry"
              checked={usePrivateRegistry}
              onChange={(e) => setUsePrivateRegistry(e.target.checked)}
              className="h-4 w-4"
            />
            <Label htmlFor="use-private-registry">Use private registry authentication</Label>
          </div>

          {usePrivateRegistry && (
            <>
              <div className="space-y-2">
                <Label htmlFor="registry-username">Registry Username</Label>
                <Input
                  id="registry-username"
                  value={registryUsername}
                  onChange={(e) => setRegistryUsername(e.target.value)}
                  placeholder="username"
                />
              </div>

              <div className="space-y-2">
                <Label htmlFor="registry-password">Registry Password</Label>
                <Input
                  id="registry-password"
                  type="password"
                  value={registryPassword}
                  onChange={(e) => setRegistryPassword(e.target.value)}
                  placeholder="password or access token"
                />
              </div>

              <div className="space-y-2">
                <Label htmlFor="registry-server">Registry Server (Optional)</Label>
                <Input
                  id="registry-server"
                  value={registryServer}
                  onChange={(e) => setRegistryServer(e.target.value)}
                  placeholder="registry.example.com (leave empty for Docker Hub)"
                />
                <p className="text-xs text-muted-foreground">
                  Leave empty for Docker Hub. For other registries, enter the server address (e.g., ghcr.io, registry.gitlab.com)
                </p>
              </div>
            </>
          )}
        </CardContent>
      </Card>

      <div className="flex justify-end gap-2">
        <Button variant="outline" onClick={() => router.back()} disabled={createContainer.isPending}>
          Cancel
        </Button>
        <Button
          onClick={handleSubmit}
          disabled={
            !name.trim() ||
            (imageSource === "upload" ? !selectedFile : !image.trim()) ||
            createContainer.isPending ||
            uploadImage.isPending
          }
        >
          {(createContainer.isPending || uploadImage.isPending) && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
          {uploadImage.isPending ? "Uploading..." : "Deploy Container"}
        </Button>
      </div>
    </div>
  )
}
