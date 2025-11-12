"use client"

import { useState } from "react"
import { useMutation } from "@tanstack/react-query"
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { Label } from "@/components/ui/label"
import { Textarea } from "@/components/ui/textarea"
import { useToast } from "@/hooks/use-toast"
import type { Function } from "@/lib/types"
import { api } from "@/lib/api"
import { Loader2 } from "lucide-react"

// This hook should ideally be in /apps/frontend/lib/queries.ts
// Defining it here as file modification outside of the current file is not possible.
const useInvokeFunction = () => {
  return useMutation({
    mutationFn: ({ id, payload }: { id: string; payload: string }) => {
      let body = {}
      try {
        body = JSON.parse(payload)
      } catch (e) {
        // Let the server handle invalid JSON
      }
      return api.post(`/api/functions/${id}/invoke`, body)
    },
  })
}

interface FunctionInvokeDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  fn: Function
}

export function FunctionInvokeDialog({
  open,
  onOpenChange,
  fn,
}: FunctionInvokeDialogProps) {
  const [payload, setPayload] = useState("{}")
  const [result, setResult] = useState<string | null>(null)
  const { toast } = useToast()
  const invokeMutation = useInvokeFunction()

  const handleInvoke = () => {
    let parsedPayload = "{}"
    try {
      JSON.parse(payload)
      parsedPayload = payload
    } catch (error) {
      toast({
        title: "Invalid JSON",
        description: "The payload is not valid JSON.",
        variant: "error",
        duration: 2000,
      })
      return
    }

    invokeMutation.mutate(
      { id: fn.id, payload: parsedPayload },
      {
        onSuccess: (data: any) => {
          const resultString = JSON.stringify(data, null, 2)
          setResult(resultString)
          toast({
            title: "Invocation Succeeded",
            description: `Function ${fn.name} invoked successfully.`,
            variant: "success",
            duration: 2000,
          })
        },
        onError: (error: any) => {
          const errorMessage = error.response?.data?.message || error.message
          setResult(`Error: ${errorMessage}`)
          toast({
            title: "Invocation Failed",
            description: `Failed to invoke ${fn.name}: ${errorMessage}`,
            variant: "error",
            duration: 2000,
          })
        },
      }
    )
  }

  const handleClose = () => {
    onOpenChange(false)
    setTimeout(() => {
      setResult(null)
      setPayload("{}")
      invokeMutation.reset()
    }, 300)
  }

  return (
    <Dialog open={open} onOpenChange={handleClose}>
      <DialogContent className="sm:max-w-[625px]">
        <DialogHeader>
          <DialogTitle>Invoke Function: {fn.name}</DialogTitle>
          <DialogDescription>
            Enter the JSON payload to send to the function.
          </DialogDescription>
        </DialogHeader>
        <div className="grid gap-4 py-4">
          <div className="grid gap-2">
            <Label htmlFor="payload">Request Payload (JSON)</Label>
            <Textarea
              id="payload"
              value={payload}
              onChange={(e) => setPayload(e.target.value)}
              className="min-h-[150px] font-mono bg-background"
              placeholder="key,"
            />
          </div>
          {invokeMutation.isPending && (
            <div className="flex items-center gap-2 text-muted-foreground">
              <Loader2 className="h-4 w-4 animate-spin" />
              <span>Invoking...</span>
            </div>
          )}
          {result && (
            <div className="grid gap-2">
              <Label>Result</Label>
              <pre className="p-4 bg-muted rounded-md text-sm overflow-auto max-h-[200px]">
                <code>{result}</code>
              </pre>
            </div>
          )}
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={handleClose}>
            Close
          </Button>
          <Button onClick={handleInvoke} disabled={invokeMutation.isPending}>
            {invokeMutation.isPending ? "Invoking..." : "Invoke"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}