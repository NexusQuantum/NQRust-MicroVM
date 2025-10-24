"use client"

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Plus } from "lucide-react"
import Link from "next/link"
import { FunctionTable } from "@/components/function/function-table"
import Image from "next/image"
import { useFunctions } from "@/lib/queries"
import { useState } from "react"


export default function FunctionsPage() {
  const { data: functions, isLoading, error } = useFunctions()
  const [searchTerm, setSearchTerm] = useState("")

  const filteredFunctions = functions?.filter(fn =>
    fn.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
    fn?.id.toLowerCase().includes(searchTerm.toLowerCase())
  ) || []


  if (isLoading) {
    return (
      <div className="container mx-auto py-6">
        <div className="animate-pulse space-y-4">
          <div className="h-8 bg-muted rounded w-1/4" />
          <div className="grid gap-4">
            {[...Array(6)].map((_, i) => <div key={i} className="h-24 bg-muted rounded-lg" />)}
          </div>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="container mx-auto py-6 text-center space-y-4">
        <h1 className="text-2xl font-bold text-destructive">Failed to load Functions</h1>
        <p className="text-muted-foreground">Unable to fetch function list. Please check your connection and try again.</p>
        <Button variant="outline" onClick={() => location.reload()}>Try again</Button>
      </div>
    );
  }


  return (
    <div className="space-y-6">
      <div className="relative overflow-hidden rounded-xl border border-border bg-linear-to-br from-yellow-50 to-yellow-100/50 p-8">
        <div className="relative z-10 flex items-center justify-between">
          <div className="max-w-xl">
            <h1 className="text-3xl font-bold text-foreground">Serverless Functions</h1>
            <p className="mt-2 text-muted-foreground">
              Deploy and manage Lambda-like functions with automatic scaling and pay-per-use pricing
            </p>
            <Button asChild className="mt-4">
              <Link href="/functions/new">
                <Plus className="mr-2 h-4 w-4" />
                New Function
              </Link>
            </Button>
          </div>
          <div className="hidden lg:block">
            <Image
              src="/serverless-functions-code-lightning-fast-illustrat.jpg"
              alt="Serverless Functions"
              width={300}
              height={200}
              className="rounded-lg"
            />
          </div>
        </div>
        <div className="absolute right-0 top-0 h-64 w-64 translate-x-32 -translate-y-32 rounded-full bg-linear-to-br from-yellow-400/30 to-yellow-600/30 blur-3xl" />
      </div>

      <Card>
        <CardHeader>
          <CardTitle>All Functions</CardTitle>
        </CardHeader>
        <CardContent>
          {/* <FunctionTable functions={mockFunctions} /> */}
          {filteredFunctions.length === 0 ? (
            <div className="text-center py-12">
              <h3 className="text-lg font-medium">No Functions found</h3>
              <p className="text-muted-foreground mt-2">
                {searchTerm
                  ? "No Functions match your search criteria."
                  : "Get started by creating your first Function."
                }
              </p>
              {!searchTerm && (
                <Button asChild className="mt-4">
                  <Link href="/functions/new">
                    <Plus className="mr-2 h-4 w-4" />
                    Create your first Function
                  </Link>
                </Button>
              )}
            </div>
          ) : (
            <FunctionTable functions={filteredFunctions} />
          )}
        </CardContent>
      </Card>
    </div>
  )
}
