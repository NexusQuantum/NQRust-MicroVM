"use client"

import { useMemo, useState } from "react"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { ImageRegistry } from "@/components/registry/image-registry"
import { DockerHubBrowser } from "@/components/registry/dockerhub-browser"
import { UploadImageDialog } from "@/components/registry/upload-image-dialog"
import { useRegistryImages } from "@/lib/queries"
import { Skeleton } from "@/components/ui/skeleton"
import { AlertCircle, Box, Container as ContainerIcon, Upload, RefreshCw } from "lucide-react"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { useQueryClient } from "@tanstack/react-query"
import { queryKeys } from "@/lib/queries"

const RegistryFlowDiagram = () => (
  <svg width="350" height="200" viewBox="0 0 350 200" fill="none" xmlns="http://www.w3.org/2000/svg" className="drop-shadow-lg">
    <defs>
      <linearGradient id="greenGradient" x1="0%" y1="0%" x2="100%" y2="0%">
        <stop offset="0%" stopColor="#16a34a" />
        <stop offset="100%" stopColor="#22c55e" />
      </linearGradient>
    </defs>

    {/* Docker Hub (Top Left) */}
    <rect x="0" y="30" width="80" height="50" rx="8" fill="#dcfce7" stroke="#16a34a" strokeWidth="2" />
    <text x="40" y="52" textAnchor="middle" fill="#15803d" fontWeight="700" fontSize="12">Docker Hub</text>
    <text x="40" y="70" textAnchor="middle" fill="#15803d" fontSize="8">Images</text>

    {/* Upload Source (Bottom Left) */}
    <rect x="0" y="145" width="80" height="50" rx="8" fill="#dcfce7" stroke="#16a34a" strokeWidth="2" />
    <text x="40" y="167" textAnchor="middle" fill="#15803d" fontWeight="700" fontSize="12">Upload</text>
    <text x="40" y="183" textAnchor="middle" fill="#15803d" fontSize="9">Local Files</text>

    {/* Central Registry */}
    <rect x="125" y="75" width="100" height="90" rx="10" fill="#f0fdf4" stroke="#16a34a" strokeWidth="3" />
    <text x="175" y="100" textAnchor="middle" fill="#15803d" fontWeight="700" fontSize="16">Registry</text>
    <line x1="140" y1="110" x2="210" y2="110" stroke="#22c55e" strokeWidth="1.5" opacity="0.4" />
    <text x="175" y="125" textAnchor="middle" fill="#15803d" fontSize="10.5">VM Images</text>
    <text x="175" y="140" textAnchor="middle" fill="#15803d" fontSize="10.5">Containers</text>
    <text x="175" y="155" textAnchor="middle" fill="#15803d" fontSize="10.5">Kernels</text>

    {/* Simple straight arrows to Registry */}
    <line x1="80" y1="65" x2="125" y2="100" stroke="#16a34a" strokeWidth="2" />
    <text x="110" y="84" textAnchor="middle" fill="#15803d" fontSize="10" fontWeight="400">Pull</text>

    <line x1="80" y1="155" x2="125" y2="140" stroke="#16a34a" strokeWidth="2" />
    <text x="100" y="141" textAnchor="middle" fill="#15803d" fontSize="10" fontWeight="400">Import</text>


    {/* Simple straight arrows from Registry to Resources */}
    <line x1="225" y1="95" x2="260" y2="60" stroke="#16a34a" strokeWidth="2" />
    <text x="242" y="95" textAnchor="middle" fill="#15803d" fontSize="8" fontWeight="400">Deploy</text>

    <line x1="225" y1="120" x2="260" y2="120" stroke="#16a34a" strokeWidth="2" />

    <line x1="225" y1="145" x2="260" y2="168" stroke="#16a34a" strokeWidth="2" />

    {/* Deployed Resources (Right) */}
    {/* VM */}
    <rect x="260" y="30" width="75" height="55" rx="8" fill="#dcfce7" stroke="#16a34a" strokeWidth="2" />
    <text x="297.5" y="56" textAnchor="middle" fill="#15803d" fontWeight="700" fontSize="12">VM</text>
    <text x="297.5" y="75" textAnchor="middle" fill="#16a34a" fontWeight="600" fontSize="8">● Running</text>

    {/* Container */}
    <rect x="260" y="95" width="75" height="55" rx="8" fill="#dcfce7" stroke="#16a34a" strokeWidth="2" />
    <text x="297.5" y="121" textAnchor="middle" fill="#15803d" fontWeight="700" fontSize="12">Container</text>
    <text x="297.5" y="140" textAnchor="middle" fill="#16a34a" fontWeight="600" fontSize="8">● Active</text>

    {/* Function */}
    <rect x="260" y="160" width="75" height="35" rx="8" fill="#dcfce7" stroke="#16a34a" strokeWidth="2" />
    <text x="297.5" y="182" textAnchor="middle" fill="#15803d" fontWeight="700" fontSize="12">Function</text>
  </svg>
)

export default function RegistryPage() {
  const { data: images = [], isLoading, error, refetch } = useRegistryImages()
  const queryClient = useQueryClient()

  const handleRefresh = () => {
    queryClient.invalidateQueries({ queryKey: queryKeys.registryImages })
    refetch()
  }
  const [uploadDialogOpen, setUploadDialogOpen] = useState(false)
  const [uploadKind, setUploadKind] = useState<"docker" | "kernel" | "rootfs">("docker")

  // Split images by type
  // Note: Internal system images (container-runtime, node-runtime, python-runtime, etc.)
  // are already filtered out globally in useRegistryImages hook
  const vmImages = useMemo(() => images.filter((img) => img.kind !== "docker"), [images])
  const dockerImages = useMemo(() => images.filter((img) => img.kind === "docker"), [images])

  const openUploadDialog = (kind: "docker" | "kernel" | "rootfs") => {
    setUploadKind(kind)
    setUploadDialogOpen(true)
  }

  return (
    <div className="space-y-6">
      <div className="relative overflow-hidden rounded-xl border border-border bg-gradient-to-br from-green-50 to-green-100/50 dark:from-green-950/30 dark:to-green-900/20 p-8">
        <div className="relative z-10 flex items-center justify-between">
          <div className="max-w-xl">
            <h1 className="text-3xl font-bold text-foreground">Image Registry</h1>
            <p className="mt-2 text-muted-foreground">
              Manage VM images and Docker container images with integrated Docker Hub marketplace
            </p>
          </div>
          <div className="hidden lg:block">
            <RegistryFlowDiagram />
          </div>
        </div>
        <div className="absolute right-0 top-0 h-64 w-64 translate-x-32 -translate-y-32 rounded-full bg-gradient-to-br from-green-400/30 to-green-600/30 dark:from-green-500/20 dark:to-green-600/10 blur-3xl" />
      </div>

      <Tabs defaultValue="docker" className="space-y-4">
        <TabsList className="bg-muted/50">
          <TabsTrigger value="docker" className="flex items-center gap-2">
            <ContainerIcon className="h-4 w-4" />
            Docker Images ({dockerImages.length})
          </TabsTrigger>
          <TabsTrigger value="vm" className="flex items-center gap-2">
            <Box className="h-4 w-4" />
            VM Images ({vmImages.length})
          </TabsTrigger>
          <TabsTrigger value="marketplace">
            Docker Hub Marketplace
          </TabsTrigger>
        </TabsList>

        <TabsContent value="docker" className="space-y-4">
          <Card className="shadow-none bg-card">
            <CardHeader className="flex flex-row items-center justify-between">
              <CardTitle>Cached Docker Images</CardTitle>
              <div className="flex gap-2">
                <Button onClick={handleRefresh} size="sm" variant="outline" disabled={isLoading}>
                  <RefreshCw className={`mr-2 h-4 w-4 ${isLoading ? 'animate-spin' : ''}`} />
                  Refresh
                </Button>
                <Button onClick={() => openUploadDialog("docker")} size="sm">
                  <Upload className="mr-2 h-4 w-4" />
                  Upload Docker Image
                </Button>
              </div>
            </CardHeader>
            <CardContent>
              {isLoading ? (
                <div className="space-y-4">
                  {[...Array(3)].map((_, i) => (
                    <div key={i} className="flex items-center space-x-4">
                      <Skeleton className="h-12 w-12 rounded-lg" />
                      <div className="space-y-2">
                        <Skeleton className="h-4 w-[250px]" />
                        <Skeleton className="h-4 w-[200px]" />
                      </div>
                    </div>
                  ))}
                </div>
              ) : error ? (
                <Alert variant="destructive">
                  <AlertCircle className="h-4 w-4" />
                  <AlertTitle>Error</AlertTitle>
                  <AlertDescription>
                    Failed to load registry images. Please try again later.
                  </AlertDescription>
                </Alert>
              ) : dockerImages.length === 0 ? (
                <div className="text-center py-12">
                  <ContainerIcon className="h-12 w-12 mx-auto text-muted-foreground mb-4" />
                  <p className="text-muted-foreground">
                    No Docker images cached yet. Visit the Docker Hub Marketplace to download images.
                  </p>
                </div>
              ) : (
                <ImageRegistry images={dockerImages} />
              )}
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="vm" className="space-y-4">
          <Card className="shadow-none bg-card">
            <CardHeader className="flex flex-row items-center justify-between">
              <CardTitle>VM Images (Kernel & Rootfs)</CardTitle>
              <div className="flex gap-2">
                <Button onClick={handleRefresh} size="sm" variant="outline" disabled={isLoading}>
                  <RefreshCw className={`mr-2 h-4 w-4 ${isLoading ? 'animate-spin' : ''}`} />
                  Refresh
                </Button>
                <Button onClick={() => openUploadDialog("kernel")} size="sm" variant="outline">
                  <Upload className="mr-2 h-4 w-4" />
                  Upload Kernel
                </Button>
                <Button onClick={() => openUploadDialog("rootfs")} size="sm" variant="outline">
                  <Upload className="mr-2 h-4 w-4" />
                  Upload Rootfs
                </Button>
              </div>
            </CardHeader>
            <CardContent>
              {isLoading ? (
                <div className="space-y-4">
                  {[...Array(3)].map((_, i) => (
                    <div key={i} className="flex items-center space-x-4">
                      <Skeleton className="h-12 w-12 rounded-lg" />
                      <div className="space-y-2">
                        <Skeleton className="h-4 w-[250px]" />
                        <Skeleton className="h-4 w-[200px]" />
                      </div>
                    </div>
                  ))}
                </div>
              ) : error ? (
                <Alert variant="destructive">
                  <AlertCircle className="h-4 w-4" />
                  <AlertTitle>Error</AlertTitle>
                  <AlertDescription>
                    Failed to load registry images. Please try again later.
                  </AlertDescription>
                </Alert>
              ) : vmImages.length === 0 ? (
                <div className="text-center py-12">
                  <Box className="h-12 w-12 mx-auto text-muted-foreground mb-4" />
                  <p className="text-muted-foreground">
                    No VM images found. Import kernel and rootfs images to get started.
                  </p>
                </div>
              ) : (
                <ImageRegistry images={vmImages} />
              )}
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="marketplace">
          <DockerHubBrowser />
        </TabsContent>
      </Tabs>

      <UploadImageDialog
        open={uploadDialogOpen}
        onOpenChange={setUploadDialogOpen}
        defaultKind={uploadKind}
      />
    </div>
  )
}
