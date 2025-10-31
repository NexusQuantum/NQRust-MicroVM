"use client"

import { useMemo, useState } from "react"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { ImageRegistry } from "@/components/registry/image-registry"
import { DockerHubBrowser } from "@/components/registry/dockerhub-browser"
import { UploadImageDialog } from "@/components/registry/upload-image-dialog"
import Image from "next/image"
import { useRegistryImages } from "@/lib/queries"
import { Skeleton } from "@/components/ui/skeleton"
import { AlertCircle, Box, Container as ContainerIcon, Upload, RefreshCw } from "lucide-react"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { useQueryClient } from "@tanstack/react-query"
import { queryKeys } from "@/lib/queries"

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
      <div className="relative overflow-hidden rounded-xl border border-border bg-gradient-to-br from-green-50 to-green-100/50 p-8">
        <div className="relative z-10 flex items-center justify-between">
          <div className="max-w-xl">
            <h1 className="text-3xl font-bold text-foreground">Image Registry</h1>
            <p className="mt-2 text-muted-foreground">
              Manage VM images and Docker container images with integrated Docker Hub marketplace
            </p>
          </div>
          <div className="hidden lg:block">
            <Image
              src="/image-registry-storage-database-illustration.jpg"
              alt="Image Registry"
              width={300}
              height={200}
              className="rounded-lg"
            />
          </div>
        </div>
        <div className="absolute right-0 top-0 h-64 w-64 translate-x-32 -translate-y-32 rounded-full bg-gradient-to-br from-green-400/30 to-green-600/30 blur-3xl" />
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
