"use client"

import { FunctionEditor } from "@/components/function/function-editor"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { ArrowLeft, Play, Trash2, FileText } from "lucide-react"
import Link from "next/link"
import { use } from "react"
import { useFunction } from "@/lib/queries"


const getStatusColor = (state: string) => {
  switch (state) {
    case "active":
      return "bg-green-500/10 text-green-700 border-green-200"
    case "inactive":
      return "bg-gray-500/10 text-gray-700 border-gray-200"
    case "error":
      return "bg-red-500/10 text-red-700 border-red-200"
    default:
      return "bg-blue-500/10 text-blue-700 border-blue-200"
  }
}

export default function FunctionEditorPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = use(params)
  const { data: functions, isLoading, error } = useFunction(id)

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
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <Link href="/functions">
            <Button variant="ghost" size="icon">
              <ArrowLeft className="h-4 w-4" />
            </Button>
          </Link>
          <div>
            <div className="flex items-center gap-3">
              <h1 className="text-3xl font-bold text-foreground">{functions?.name}</h1>
              <Badge className={getStatusColor(String((functions as any)?.state ?? "unknown"))}>{(functions as any)?.state ?? "unknown"}</Badge> {/* ready */}
            </div>
            <p className="text-sm text-muted-foreground mt-1">
              {functions?.runtime} • {functions?.memory_mb}MB • {functions?.timeout_seconds}s timeout
            </p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <Link href={`/functions/${id}/logs`}>
            <Button variant="outline" size="sm">
              <FileText className="mr-2 h-4 w-4" />
              View Logs
            </Button>
          </Link>
          <Button variant="outline" size="sm">
            <Play className="mr-2 h-4 w-4" />
            Test Function
          </Button>
          <Button variant="destructive" size="sm">
            <Trash2 className="mr-2 h-4 w-4" />
            Delete
          </Button>
        </div>
      </div>

      <FunctionEditor functionData={functions} />
    </div>
  )
}
