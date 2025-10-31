"use client"

import { FunctionEditor, FunctionOverview, FunctionStats, FunctionEvent, FunctionLogs } from "@/components/function"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { ArrowLeft, Play, Trash2, FileText } from "lucide-react"
import Link from "next/link"
import { use, useState } from "react"
import { useDeleteFunction, useFunction } from "@/lib/queries"
import { ConfirmDialog } from "@/components/shared/confirm-dialog"


const getStatusColor = (state: string) => {
  switch (state) {
    case "ready":
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
  const [deleteDialog, setDeleteDialog] = useState(false)

  const deleteFunction = useDeleteFunction()
  const handleDelete = () => {
    deleteFunction.mutate(id, {
      onSuccess: () => {
        window.location.href = '/functions'
      }
    })
  }

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
          {functions?.vm_id && (
            <Link href={`/vms/${functions.vm_id}`}>
              <Button variant="outline" size="sm">
                <svg className="mr-2 h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z" />
                </svg>
                View VM
              </Button>
            </Link>
          )}
          <Link href={`/functions/${id}/logs`}>
            <Button variant="outline" size="sm">
              <FileText className="mr-2 h-4 w-4" />
              View Logs
            </Button>
          </Link>
          {/* <Button variant="outline" size="sm">
            <Play className="mr-2 h-4 w-4" />
            Test Function
          </Button> */}
          <Button variant="destructive" size="sm" onClick={() => setDeleteDialog(true)} className="cursor-pointer">
            <Trash2 className="mr-2 h-4 w-4" />
            Delete
          </Button>
        </div>
      </div>

      <Tabs defaultValue="editor" className="space-y-4">
        <TabsList className="bg-muted/50">
          <TabsTrigger value="editor">Editor</TabsTrigger>
          <TabsTrigger value="overview">Overview</TabsTrigger>
          <TabsTrigger value="stats">Stats</TabsTrigger>
          <TabsTrigger value="events">Events</TabsTrigger>
          <TabsTrigger value="logs">Logs</TabsTrigger>
        </TabsList>
        <TabsContent value="editor" className="space-y-4">
          <FunctionEditor
            functionData={functions}
            mode="update"
            functionId={id}
            onComplete={() => location.reload()}
          />
        </TabsContent>

        <TabsContent value="overview" className="space-y-4">
          <FunctionOverview functionData={functions} />
        </TabsContent>

        <TabsContent value="stats" className="space-y-4">
          <FunctionStats functionData={functions} />
        </TabsContent>

        <TabsContent value="events" className="space-y-4">
          <FunctionEvent functionData={functions} />
        </TabsContent>

        <TabsContent value="logs" className="space-y-4">
          <FunctionLogs functionId={id} />
        </TabsContent>
      </Tabs>

      <ConfirmDialog
        open={deleteDialog}
        onOpenChange={setDeleteDialog}
        title="Delete Function"
        description={`Are you sure you want to delete function? This action cannot be undone.`}
        onConfirm={handleDelete}
        isPending={deleteFunction.isPending}
      />
    </div >
  )
}
