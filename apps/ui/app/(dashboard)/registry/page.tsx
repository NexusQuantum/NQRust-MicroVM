"use client"

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Plus } from "lucide-react"
import { ImageRegistry } from "@/components/registry/image-registry"
import Image from "next/image"
import { useRegistryImages } from "@/lib/queries"
import { Skeleton } from "@/components/ui/skeleton"
import { AlertCircle } from "lucide-react"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"

export default function RegistryPage() {
  const { data: images = [], isLoading, error } = useRegistryImages()

  return (
    <div className="space-y-6">
      <div className="relative overflow-hidden rounded-xl border border-border bg-gradient-to-br from-green-50 to-green-100/50 p-8">
        <div className="relative z-10 flex items-center justify-between">
          <div className="max-w-xl">
            <h1 className="text-3xl font-bold text-foreground">Image Registry</h1>
            <p className="mt-2 text-muted-foreground">
              Manage kernel images, rootfs images, and volumes for your virtual machines
            </p>
            <Button className="mt-4">
              <Plus className="mr-2 h-4 w-4" />
              Import Image
            </Button>
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

      <Card className="shadow-none bg-card">
        <CardHeader>
          <CardTitle>All Images</CardTitle>
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
          ) : (
            <ImageRegistry images={images} />
          )}
        </CardContent>
      </Card>
    </div>
  )
}
