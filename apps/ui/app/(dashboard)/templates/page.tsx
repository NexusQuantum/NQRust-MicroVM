"use client"

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Plus } from "lucide-react"
import { TemplateList } from "@/components/templates/template-list"
import Image from "next/image"
import { useTemplates } from "@/lib/queries"
import { Skeleton } from "@/components/ui/skeleton"
import { AlertCircle } from "lucide-react"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"

export default function TemplatesPage() {
  const { data: templates = [], isLoading, error } = useTemplates()

  return (
    <div className="space-y-6">
      <div className="relative overflow-hidden rounded-xl border border-border bg-gradient-to-br from-orange-50 to-orange-100/50 p-8">
        <div className="relative z-10 flex items-center justify-between">
          <div className="max-w-xl">
            <h1 className="text-3xl font-bold text-foreground">VM Templates</h1>
            <p className="mt-2 text-muted-foreground">
              Save and deploy VM configurations as templates. Quickly spin up new instances with pre-configured
              settings.
            </p>
            <Button className="mt-4">
              <Plus className="mr-2 h-4 w-4" />
              Create Template
            </Button>
          </div>
          <div className="hidden lg:block">
            <Image
              src="/cloud-server-template-illustration.jpg"
              alt="Templates illustration"
              width={300}
              height={200}
              className="rounded-lg"
            />
          </div>
        </div>
        <div className="absolute right-0 top-0 h-64 w-64 translate-x-32 -translate-y-32 rounded-full bg-gradient-to-br from-orange-400/30 to-orange-600/30 blur-3xl" />
      </div>

      <Card>
        <CardHeader>
          <CardTitle>All Templates</CardTitle>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="space-y-4">
              {[...Array(3)].map((_, i) => (
                <div key={i} className="p-6 border rounded-lg space-y-4">
                  <div className="flex items-start justify-between">
                    <div className="space-y-2">
                      <Skeleton className="h-6 w-48" />
                      <Skeleton className="h-4 w-64" />
                    </div>
                    <Skeleton className="h-8 w-20" />
                  </div>
                  <div className="grid grid-cols-2 gap-4">
                    <Skeleton className="h-4 w-32" />
                    <Skeleton className="h-4 w-32" />
                  </div>
                </div>
              ))}
            </div>
          ) : error ? (
            <Alert variant="destructive">
              <AlertCircle className="h-4 w-4" />
              <AlertTitle>Error</AlertTitle>
              <AlertDescription>
                Failed to load templates. Please try again later.
              </AlertDescription>
            </Alert>
          ) : (
            <TemplateList templates={templates} />
          )}
        </CardContent>
      </Card>
    </div>
  )
}
