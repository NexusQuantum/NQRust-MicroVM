"use client"

import React, { useState, useRef, useMemo, useEffect } from "react"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Slider } from "@/components/ui/slider"
import { Save, Play, Loader2, Import } from "lucide-react"
import dynamic from "next/dynamic"
import { z } from "zod"
import { useForm, Controller } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import type { CreateFunction, UpdateFunction, Function as FnType } from "@/lib/types"
import { useCreateFunction, useUpdateFunction, useInvokeFunction, useTestFunction } from "@/lib/queries"
import { useMutation } from "@tanstack/react-query";

const Editor = dynamic(() => import("@monaco-editor/react"), { ssr: false })

interface FunctionEditorProps {
  onComplete?: (payload: { id?: string, name?: string }) => void
  onCancel?: () => void
  functionData?: FnType
  mode?: "create" | "update"
  functionId?: string
}

const fnCreationSchema = z.object({
  name: z.string().min(1, "Function Name is required").max(50, "Name too long"),
  runtime: z.enum(['node', 'python']),
  handler: z.string().min(1, "Handler Name is required").max(50, "Name too long"),
  code: z.string().min(1, "Code is required"),
  vcpu: z.number().min(1, "Minimum 1 vCPU").max(32, "Maximum 32 vCPU"),
  memory_mb: z.number().min(128, "Minimum 128 MB").max(3072, "Maximum 3072 MB"),
  timeout_seconds: z.number().min(1, "Minimum 1s").max(300, "Maximum 300s")
})

type FnCreationForm = z.infer<typeof fnCreationSchema>

const DEFAULT_CODE_NODE = `// index.js (Node.js 20.x / CommonJS)
module.exports.handler = async (event) => {
  const a = Number(event?.key1);  
  const b = Number(event?.key2);
  if (!Number.isFinite(a) || !Number.isFinite(b)) {
    return {
      statusCode: 400,
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ error: 'key1 and key2 must be numbers' })
    };
  }
  return {
    statusCode: 200,
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ result: a + b })
  };
};`;

const DEFAULT_CODE_PY = `# index.py  (Python 3.11)
def handler(event):
    try:
        a = float(event.get("key1"))
        b = float(event.get("key2"))
    except Exception:
        return {
            "statusCode": 400,
            "headers": {"content-type": "application/json"},
            "body": '{"error":"key1 and key2 must be numbers"}',
        }

    return {
        "statusCode": 200,
        "headers": {"content-type": "application/json"},
        "body": '{"result": %s}' % (a + b),
    }`;

const DEFAULT_PAYLOAD = `{
  "key1": 10,
  "key2": 5
}`;


export function FunctionEditor({ functionId, mode = 'create', functionData, onComplete = () => { } }: FunctionEditorProps) {
  const [testEvent, setTestEvent] = useState(DEFAULT_PAYLOAD)
  const editorRef = useRef<any>(null)

  // Manual state for test results, inspired by Ide.tsx
  const [testOutput, setTestOutput] = useState<any | null>(null);
  const [testLogs, setTestLogs] = useState<string[]>([]);
  const [testStatus, setTestStatus] = useState<"idle" | "running" | "done" | "error">("idle");

  const isUpdate = useMemo(
    () => mode === 'update' || !!functionId || !!functionData?.id,
    [mode, functionId, functionData?.id]
  )

  const {
    register,
    handleSubmit,
    watch,
    control,
    setValue,
    formState: { errors, isSubmitting },
  } = useForm<FnCreationForm>({
    resolver: zodResolver(fnCreationSchema) as any,
    mode: 'onChange',
    defaultValues: {
      name: functionData?.name ?? "my-function",
      runtime: functionData?.runtime ?? "node",
      handler: functionData?.handler ?? "handler",
      code: functionData?.code ?? (functionData?.runtime === 'python' ? DEFAULT_CODE_PY : DEFAULT_CODE_NODE),
      vcpu: functionData?.vcpu ?? 1,
      memory_mb: functionData?.memory_mb ?? 512,
      timeout_seconds: functionData?.timeout_seconds ?? 30
    }
  })

  const runtime = watch('runtime')

  useEffect(() => {
    if (isUpdate) return
    if (runtime === 'python') {
      setValue('code', DEFAULT_CODE_PY)
    } else {
      setValue('code', DEFAULT_CODE_NODE)
    }
  }, [runtime, setValue, isUpdate])


  const getLanguage = () => (runtime === 'python' ? "python" : "javascript")


  const createFunction = useCreateFunction()
  const updateFunction = useUpdateFunction()
  const invokeMutation = useInvokeFunction()

  const onSubmit = async (data: FnCreationForm) => {
    try {
      if (isUpdate) {
        const id = functionId ?? functionData?.id
        if (!id) throw new Error("Missing function id for update")
        const payload: UpdateFunction = {
          name: data.name,
          code: data.code,
          handler: data.handler,
          timeout_seconds: data.timeout_seconds,
          memory_mb: data.memory_mb,
        }
        await updateFunction.mutateAsync({ fnId: id, data: payload })
        onComplete?.({ id, name: data.name })
      } else {
        const fnReq: CreateFunction = {
          name: data.name,
          runtime: data.runtime,
          handler: data.handler,
          code: data.code,
          vcpu: data.vcpu,
          memory_mb: data.memory_mb
        }
        const created = await createFunction.mutateAsync(fnReq)
        onComplete?.({ id: (created as any)?.id, name: data.name })
      }
    } catch (err) {
      console.log("Error: ", err)
    }
  }

  const handleInvoke = async () => {
    const id = functionId ?? functionData?.id
    if (!id) return
    try {
      setTestStatus('idle');
      setTestOutput(null);
      setTestLogs([]);
      const result = await invokeMutation.mutateAsync({
        fnId: id,
        payload: { event: JSON.parse(testEvent) },
      })
      setTestOutput(result);
      setTestStatus('done');
    } catch (error) {
      console.error("Invalid JSON for test event:", error)
      setTestOutput({ error: (error as Error).message });
      setTestStatus('error');
    }
  }

  const handleTestRun = React.useCallback(async () => {
    setTestStatus('running');
    setTestOutput(null);
    setTestLogs([]);

    let eventObj: unknown;
    try {
      eventObj = JSON.parse(testEvent || "{}");
    } catch {
      setTestStatus('error');
      setTestOutput({ error: "Test Event harus berupa JSON valid" });
      return;
    }

    try {
      const currentRuntime = watch('runtime');  // 'node' | 'python'
      const currentCode = watch('code');
      const currentHandler = watch('handler') || 'handler'; // ← kirim ke API

      const res = await fetch('/api/test', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({
          runtime: currentRuntime,
          code: currentCode,
          event: eventObj,
          handler: currentHandler,             // ← NEW
        }),
      });

      const json = await res.json();
      setTestLogs(Array.isArray(json?.logs) ? json.logs : []);

      if (!res.ok || json?.ok === false) {
        setTestStatus('error');
        setTestOutput({ error: json?.error || `HTTP ${res.status}` });
        return;
      }

      setTestOutput(json);
      setTestStatus('done');
    } catch (e: any) {
      setTestStatus('error');
      setTestOutput({ error: e?.message || String(e) });
    }
  }, [testEvent, watch]);



  return (
    <div className="space-y-6">
      <div className="flex items-center justify-end">
        <div className="flex gap-2 items-center">
          <Button type="submit" variant="outline" onClick={handleInvoke} disabled={invokeMutation.isPending || !isUpdate}>
            {invokeMutation.isPending ? (
              <>
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                Invoking...
              </>
            ) : (
              <>
                <Import className="mr-2 h-4 w-4" />
                Invoke
              </>
            )}
          </Button>
          <form onSubmit={handleSubmit(onSubmit)}>
            <Button type="submit" disabled={isSubmitting}>
              {!isSubmitting ?
                <Save className="mr-2 h-4 w-4" /> : <Loader2 className="h-4 w-4 animate-spin" />}
              {isUpdate ? "Update" : "Create"}
            </Button>
          </form>
        </div>
      </div>

      <div className="grid gap-6 lg:grid-cols-3">
        <div className="lg:col-span-2 space-y-6">
          <Card>
            <CardHeader>
              <CardTitle>Code Editor</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="border rounded-lg overflow-hidden w-full">
                <Controller
                  name="code"
                  control={control}
                  render={({ field }) => (
                    <Editor
                      height="500px"
                      language={getLanguage()}
                      value={field.value}
                      onChange={(value, ev) => {
                        field.onChange(value ?? "")
                        if (ev.isFlush) {
                          setTimeout(() => editorRef.current?.getAction('editor.action.formatDocument')?.run(), 100)
                        }
                      }}
                      theme="light"
                      options={{
                        minimap: { enabled: false },
                        fontSize: 14,
                        lineNumbers: "on",
                        scrollBeyondLastLine: false,
                        automaticLayout: true,
                        tabSize: 2,
                        wordWrap: "on",
                        formatOnPaste: true,
                        formatOnType: true,
                      }}
                      onMount={(editor) => {
                        editorRef.current = editor
                        setTimeout(() => editor.getAction('editor.action.formatDocument')?.run(), 100)
                      }}
                    />
                  )}
                />
              </div>
            </CardContent>
          </Card>

          {(testStatus !== 'idle' && testOutput) && (
            <Card>
              <CardHeader>
                <CardTitle>Test Results</CardTitle>
                <CardDescription>
                  Result of the last invocation.
                </CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                {typeof testOutput === 'string' ? (
                  <pre className="bg-muted p-3 rounded text-xs overflow-auto max-h-[200px]">{testOutput}</pre>
                ) : testOutput.error ? (
                  <div>
                    <div className="text-sm font-medium mb-1 text-destructive">Error</div>
                    <div className="bg-destructive/10 text-destructive p-3 rounded text-xs">{testOutput.error}</div>
                  </div>
                ) : (
                  <>
                    <div className="flex items-center gap-4">
                      <div>
                        <div className="text-sm text-muted-foreground">Status</div>
                        <div className={`font-medium capitalize ${testStatus === 'done' && testOutput.ok ? 'text-emerald-600' : 'text-red-600'}`}>
                          {testStatus === 'running' ? 'Running' : (testOutput.ok ? 'Success' : 'Error')}
                        </div>
                      </div>
                      <div>
                        <div className="text-sm text-muted-foreground">Duration</div>
                        <div className="font-medium">{testOutput.duration_ms ?? 'N/A'}ms</div>
                      </div>
                    </div>

                    <div>
                      <div className="text-sm font-medium mb-2">Response</div>
                      <pre className="bg-muted p-3 rounded text-xs overflow-auto max-h-[200px]">
                        {JSON.stringify(testOutput.response, null, 2)}
                      </pre>
                    </div>

                    <div>
                      <div className="text-sm font-medium mb-2">Logs</div>
                      <div className="bg-black text-green-400 p-3 rounded text-xs space-y-1 font-mono max-h-[200px] overflow-auto">
                        {testLogs.map((log: string, i: number) => (
                          <div key={i}>{log}</div>
                        ))}
                      </div>
                    </div>
                  </>
                )}
              </CardContent>
            </Card>
          )}
        </div>

        <div className="space-y-6">
          <Card>
            <CardHeader>
              <CardTitle>Configuration</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="name">Function Name</Label>
                <Input
                  autoComplete="off"
                  id="name"
                  {...register("name")}
                  aria-invalid={!!errors.name}
                  aria-required="true"
                  placeholder="my-function" />
                {errors.name && <p className="text-sm text-red-600">{errors.name.message}</p>}
              </div>

              <div className="space-y-2">
                <Label htmlFor="runtime">Runtime</Label>
                <Controller
                  name='runtime'
                  control={control}
                  render={({ field }) => (
                    <Select value={field.value} onValueChange={field.onChange}>
                      <SelectTrigger id="runtime">
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="node">Node.js</SelectItem>
                        <SelectItem value="python">Python</SelectItem>
                      </SelectContent>
                    </Select>
                  )}
                />
              </div>

              <div className="space-y-2">
                <Label htmlFor="handler">Handler</Label>
                <Input id="handler" {...register("handler")} />
                {errors.handler && <p className="text-sm text-red-600">{errors.handler.message}</p>}
              </div>

              <div className="space-y-2">
                <Label>vCPU: {watch('vcpu')}</Label>
                <Controller
                  name="vcpu"
                  control={control}
                  render={({ field }) => (
                    <Slider
                      disabled={isUpdate}
                      onBlur={field.onBlur}
                      value={[field.value ?? 1]}
                      onValueChange={(val) => field.onChange(val[0])}
                      min={1}
                      max={32}
                      step={1} />
                  )}
                />
                {errors.vcpu && <p className="text-sm text-red-600">{errors.vcpu.message}</p>}
              </div>

              <div className="space-y-2">
                <Label>Timeout (seconds): {watch('timeout_seconds')}</Label>
                <Controller
                  name="timeout_seconds"
                  control={control}
                  render={({ field }) => (
                    <Slider
                      onBlur={field.onBlur}
                      value={[field.value ?? 30]}
                      onValueChange={(val) => field.onChange(val[0])}
                      min={1}
                      max={300}
                      step={1} />
                  )}
                />
                {errors.timeout_seconds && <p className="text-sm text-red-600">{errors.timeout_seconds.message}</p>}
              </div>

              <div className="space-y-2">
                <Label>Memory: {watch('memory_mb')} MB</Label>
                <Controller
                  name="memory_mb"
                  control={control}
                  render={({ field }) => (
                    <Slider
                      onBlur={field.onBlur}
                      value={[field.value ?? 512]}
                      onValueChange={(val) => field.onChange(val[0])}
                      min={128}
                      max={3072}
                      step={128} />
                  )}
                />
                {errors.memory_mb && <p className="text-sm text-red-600">{errors.memory_mb.message}</p>}
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>Test Event</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="border rounded-lg overflow-hidden w-full">
                <Editor
                  height="150px"
                  language="json"
                  value={testEvent}
                  onChange={(value) => setTestEvent(value || "")}
                  theme="light"
                  options={{
                    minimap: { enabled: false },
                    fontSize: 12,
                    lineNumbers: "on",
                    scrollBeyondLastLine: false,
                    automaticLayout: true,
                    wordWrap: "on",
                    wordWrapColumn: 100,
                    wrappingIndent: "same",
                    scrollBeyondLastColumn: 0
                  }}
                />
              </div>
              <Button className="w-full" onClick={handleTestRun} disabled={testStatus === 'running'}>
                {testStatus === 'running' ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    Running Test...
                  </>
                ) : (
                  <>
                    <Play className="mr-2 h-4 w-4" />
                    Run Test
                  </>
                )}
              </Button>
            </CardContent>
          </Card>
        </div>
      </div>
    </div >
  )
}
