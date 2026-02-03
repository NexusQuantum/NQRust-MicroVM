"use client"

import React, { useState, useRef, useMemo, useEffect, useRef as useRef2 } from "react"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Slider } from "@/components/ui/slider"
import { Save, Play, Loader2, Import, ArrowLeft } from "lucide-react"
import dynamic from "next/dynamic"
import { useRouter } from "next/navigation"
import { z } from "zod"
import { useForm, Controller } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import type { CreateFunction, UpdateFunction, Function as FnType } from "@/lib/types"
import { useCreateFunction, useUpdateFunction, useInvokeFunction } from "@/lib/queries"
import { useTheme } from "next-themes"

const Editor = dynamic(() => import("@monaco-editor/react"), { ssr: false })

interface FunctionEditorProps {
  onComplete?: (payload: { id?: string, name?: string }) => void
  onCancel?: () => void
  functionData?: FnType
  mode?: "create" | "update"
  functionId?: string
  // New
  initialRuntime?: "python" | "javascript" | "typescript"
  initialCode?: string
  initialEvent?: string
}

const fnCreationSchema = z.object({
  name: z.string().min(1, "Function Name is required").max(50, "Name too long"),
  runtime: z.enum(['python', 'javascript', 'typescript']),
  handler: z.string().min(1, "Handler Name is required").max(50, "Name too long"),
  code: z.string().min(1, "Code is required"),
  vcpu: z.number().min(1, "Minimum 1 vCPU").max(32, "Maximum 32 vCPU"),
  memory_mb: z.number().min(128, "Minimum 128 MB").max(3072, "Maximum 3072 MB"),
  timeout_seconds: z.number().min(1, "Minimum 1s").max(300, "Maximum 300s")
})

type FnCreationForm = z.infer<typeof fnCreationSchema>

const DEFAULT_CODE_PY = `# index.py — Python 3.11 Function
#
# How it works:
#   1. Your function receives an "event" dict from the request body.
#   2. Return a dict with: statusCode, headers, and body.
#   3. "body" must be a JSON string (use json.dumps).
#
# Invoke example (curl):
#   curl -X POST http://<manager>/v1/functions/<id>/invoke \\
#        -H "Content-Type: application/json" \\
#        -d '{"event": {"name": "World"}}'
#
# Tips:
#   - Use event.get("key", default) to safely read fields.
#   - Any print() output appears in the function logs.
#   - The handler name must match the "Handler" field in the config panel.

import json

def handler(event):
    """Main entry point — receives the JSON event payload as a dict."""

    # Read input from the event (sent via the "Test Event" panel or API)
    name = event.get("name", "World")

    # You can log to stdout; output shows up in the Logs tab
    print(f"Received request with name={name}")

    # Return a response — must include statusCode, headers, and body
    return {
        "statusCode": 200,
        "headers": {"content-type": "application/json"},
        "body": json.dumps({
            "message": f"Hello, {name}!",
        }),
    }`;

const DEFAULT_CODE_JS = `// index.js — JavaScript Function (runs on Bun)
//
// How it works:
//   1. Export an async "handler" function (must match the Handler config).
//   2. It receives an "event" object from the request body.
//   3. Return an object with: statusCode, headers, and body.
//   4. "body" must be a JSON string (use JSON.stringify).
//
// Invoke example (curl):
//   curl -X POST http://<manager>/v1/functions/<id>/invoke \\
//        -H "Content-Type: application/json" \\
//        -d '{"event": {"name": "World"}}'
//
// Tips:
//   - Use optional chaining (event?.key) to safely read fields.
//   - console.log() output appears in the function logs.
//   - You can use top-level await and ES module imports.

export async function handler(event) {
  // Read input from the event (sent via the "Test Event" panel or API)
  const name = event?.name ?? "World";

  // You can log to stdout; output shows up in the Logs tab
  console.log("Received request with name:", name);

  // Return a response — must include statusCode, headers, and body
  return {
    statusCode: 200,
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      message: \`Hello, \${name}!\`,
    }),
  };
}`;

const DEFAULT_CODE_TS = `// index.ts — TypeScript Function (runs on Bun)
//
// How it works:
//   1. Export an async "handler" function (must match the Handler config).
//   2. It receives a typed "event" object from the request body.
//   3. Return an object with: statusCode, headers, and body.
//   4. "body" must be a JSON string (use JSON.stringify).
//
// Invoke example (curl):
//   curl -X POST http://<manager>/v1/functions/<id>/invoke \\
//        -H "Content-Type: application/json" \\
//        -d '{"event": {"name": "World"}}'
//
// Tips:
//   - Define an interface for your event to get type safety.
//   - console.log() output appears in the function logs.
//   - You can use top-level await and ES module imports.

// Define the shape of your event payload for type safety
interface FnEvent {
  name?: string;
}

// Response shape returned by the handler
interface FnResponse {
  statusCode: number;
  headers: Record<string, string>;
  body: string;
}

export async function handler(event: FnEvent): Promise<FnResponse> {
  // Read input from the event (sent via the "Test Event" panel or API)
  const name = event?.name ?? "World";

  // You can log to stdout; output shows up in the Logs tab
  console.log("Received request with name:", name);

  // Return a response — must include statusCode, headers, and body
  return {
    statusCode: 200,
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      message: \`Hello, \${name}!\`,
    }),
  };
}`;

const DEFAULT_PAYLOAD = `{
  "name": "World"
}`

function getDefaultCodeForRuntime(runtime: string): string {
  switch (runtime) {
    case 'python':
      return DEFAULT_CODE_PY
    case 'javascript':
      return DEFAULT_CODE_JS
    case 'typescript':
      return DEFAULT_CODE_TS
    default:
      return DEFAULT_CODE_TS
  }
}

/* ---------------------------------------------
 * NORMALIZER #1 (BACKEND Create/Update):
 * For JavaScript/TypeScript (Bun), code is passed as-is (ES modules).
 * --------------------------------------------- */
type RuntimeType = 'python' | 'javascript' | 'typescript';

function normalizeToConstHandlerForBackend(
  runtime: RuntimeType,
  rawCode: string,
  handlerName: string
) {
  // JavaScript and TypeScript use ES modules natively via Bun, no transformation needed
  // Python also passed as-is
  return rawCode;
}

/* ---------------------------------------------
 * NORMALIZER #2 (RUN TEST /api/test):
 * For JavaScript/TypeScript (Bun), code is passed as-is (ES modules).
 * --------------------------------------------- */
function normalizeToModuleExportsForRunTest(
  runtime: RuntimeType,
  rawCode: string,
  handlerName: string
) {
  // JavaScript and TypeScript use ES modules natively via Bun, no transformation needed
  // Python also passed as-is
  return rawCode;
}

export function FunctionEditor({
  initialRuntime,
  initialCode,
  initialEvent,
  functionId,
  mode = 'create',
  functionData,
  onComplete = () => { },
}: FunctionEditorProps) {
  const { theme } = useTheme()
  const router = useRouter()
  const [testEvent, setTestEvent] = useState(initialEvent ?? DEFAULT_PAYLOAD)
  const editorRef = useRef<any>(null)
  // console.log('initial Event: ', initialEvent)
  // console.log('initial Code: ', initialCode)

  // Test states
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
    reset,
    getValues,
    formState: { errors, isSubmitting },
  } = useForm<FnCreationForm>({
    resolver: zodResolver(fnCreationSchema) as any,
    mode: 'onChange',
    defaultValues: {
      name: functionData?.name ?? "my-function",
      runtime: (initialRuntime ?? functionData?.runtime ?? "typescript") as RuntimeType,
      handler: functionData?.handler ?? "handler",
      code:
        initialCode ??
        functionData?.code ??
        getDefaultCodeForRuntime(initialRuntime ?? functionData?.runtime ?? "typescript"),
      vcpu: functionData?.vcpu ?? 1,
      memory_mb: functionData?.memory_mb ?? 512,
      timeout_seconds: functionData?.timeout_seconds ?? 30
    }
  })

  const runtime = watch('runtime')

  // ✅ 1) Terapkan initial* yang datang setelah mount
  useEffect(() => {
    // kalau tidak ada apa-apa yang berubah dari playground, skip
    if (initialRuntime == null && initialCode == null && initialEvent == null) return

    // reset form agar nilai code/runtime sinkron (tanpa error/dirty lama)
    reset({
      // pertahankan field lain (atau isi default aman)
      name: getValues('name') ?? "my-function",
      handler: getValues('handler') ?? "handler",
      vcpu: getValues('vcpu') ?? 1,
      memory_mb: getValues('memory_mb') ?? 512,
      timeout_seconds: getValues('timeout_seconds') ?? 30,

      // ambil dari initial* jika ada, kalau tidak pakai nilai saat ini
      runtime: (initialRuntime ?? getValues('runtime') ?? "typescript") as RuntimeType,
      code: initialCode ?? getValues('code') ?? DEFAULT_CODE_TS,
    })

    // set event editor juga
    if (initialEvent != null) setTestEvent(initialEvent)
  }, [initialRuntime, initialCode, initialEvent, reset, getValues])

  useEffect(() => {
    if (isUpdate) return
    if (initialCode != null) return
    setValue('code', getDefaultCodeForRuntime(runtime))
  }, [runtime, setValue, isUpdate, initialCode])

  const getLanguage = () => {
    switch (runtime) {
      case 'python':
        return "python"
      case 'typescript':
        return "typescript"
      case 'javascript':
        return "javascript"
      default:
        return "javascript"
    }
  }

  // Monaco uses the path extension for diagnostics (e.g. .ts enables TS-only syntax)
  const getEditorPath = () => {
    switch (runtime) {
      case 'python':
        return "index.py"
      case 'typescript':
        return "index.ts"
      case 'javascript':
        return "index.js"
      default:
        return "index.js"
    }
  }

  const createFunction = useCreateFunction()
  const updateFunction = useUpdateFunction()
  const invokeMutation = useInvokeFunction()

  // Create/Update → kirim SELALU versi const handler (khusus Node)
  const onSubmit = async (data: FnCreationForm) => {
    try {
      const codeForBackend =
        (data.runtime === 'javascript' || data.runtime === 'typescript')
          ? normalizeToConstHandlerForBackend(data.runtime, data.code, data.handler)
          : data.code

      if (isUpdate) {
        const id = functionId ?? functionData?.id
        if (!id) throw new Error("Missing function id for update")
        const payload: UpdateFunction = {
          name: data.name,
          code: codeForBackend,
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
          code: codeForBackend,
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

  // Invoke function yang sudah dideploy (tidak kirim code)
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

  // RUN TEST → selalu jalankan exports.handler (tanpa bergantung module)
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
      const currentRuntime = watch('runtime');           // RuntimeType
      const currentCode = watch('code');
      const currentHandler = watch('handler') || 'handler';

      // Khusus RUN TEST: normalisasi ke exports.handler (+ shim aman)
      const codeForTest = normalizeToModuleExportsForRunTest(
        currentRuntime as RuntimeType,
        currentCode,
        currentHandler
      );

      const res = await fetch('/api/test', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({
          runtime: currentRuntime,
          code: codeForTest,   // penting: kirim versi exports.handler
          event: eventObj,
          handler: currentHandler, // opsional bagi runner
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
        <div className="flex gap-2 items-center justify-between w-full">
          <div className="flex gap-2 items-center">
            <Button variant="ghost" size="icon" onClick={() => router.push("/functions")}>
              <ArrowLeft className="h-4 w-4" />
            </Button>
            <p className="text-lg font-semibold">Create Function</p>
          </div>
          <div className="flex gap-4 items-center">
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
                      path={getEditorPath()}
                      value={field.value}
                      onChange={(value, ev) => {
                        field.onChange(value ?? "")
                        if ((ev as any)?.isFlush) {
                          setTimeout(() => editorRef.current?.getAction('editor.action.formatDocument')?.run(), 100)
                        }
                      }}
                      theme={theme === "dark" ? "vs-dark" : "light"}
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
                      onMount={(editor, monaco) => {
                        editorRef.current = editor

                        // Configure TypeScript compiler options
                        monaco.languages.typescript.typescriptDefaults.setCompilerOptions({
                          target: monaco.languages.typescript.ScriptTarget.ES2020,
                          allowNonTsExtensions: true,
                          moduleResolution: monaco.languages.typescript.ModuleResolutionKind.NodeJs,
                          module: monaco.languages.typescript.ModuleKind.ESNext,
                          noEmit: true,
                          esModuleInterop: true,
                          allowJs: true,
                          typeRoots: ["node_modules/@types"],
                        })

                        // Configure JavaScript compiler options
                        monaco.languages.typescript.javascriptDefaults.setCompilerOptions({
                          target: monaco.languages.typescript.ScriptTarget.ES2020,
                          allowNonTsExtensions: true,
                          moduleResolution: monaco.languages.typescript.ModuleResolutionKind.NodeJs,
                          module: monaco.languages.typescript.ModuleKind.ESNext,
                          noEmit: true,
                          allowJs: true,
                        })

                        // Disable validation for certain errors
                        monaco.languages.typescript.typescriptDefaults.setDiagnosticsOptions({
                          noSemanticValidation: false,
                          noSyntaxValidation: false,
                        })

                        monaco.languages.typescript.javascriptDefaults.setDiagnosticsOptions({
                          noSemanticValidation: false,
                          noSyntaxValidation: false,
                        })

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
                  <pre className="bg-muted p-3 rounded text-xs overflow-auto max-h-[200px] font-mono">{testOutput}</pre>
                ) : testOutput.error ? (
                  <div>
                    <div className="text-sm font-medium mb-1 text-destructive">Error</div>
                    <div className="bg-destructive/10 text-destructive p-3 rounded text-xs font-mono whitespace-pre-wrap">{testOutput.error}</div>
                  </div>
                ) : (
                  <>
                    <div className="flex items-center gap-6">
                      <div>
                        <div className="text-xs text-muted-foreground uppercase tracking-wide">Status</div>
                        <div className={`text-sm font-semibold capitalize ${testOutput.status === 'success' ? 'text-green-500' : 'text-red-500'}`}>
                          {testOutput.status}
                        </div>
                      </div>
                      {testOutput.duration_ms != null && (
                        <div>
                          <div className="text-xs text-muted-foreground uppercase tracking-wide">Duration</div>
                          <div className="text-sm font-semibold">{testOutput.duration_ms}ms</div>
                        </div>
                      )}
                      {testOutput.response?.statusCode != null && (
                        <div>
                          <div className="text-xs text-muted-foreground uppercase tracking-wide">HTTP Status</div>
                          <div className={`text-sm font-semibold ${testOutput.response.statusCode >= 200 && testOutput.response.statusCode < 300 ? 'text-green-500' : 'text-yellow-500'}`}>
                            {testOutput.response.statusCode}
                          </div>
                        </div>
                      )}
                    </div>

                    {testOutput.response && (() => {
                      // Try to parse body as JSON for pretty display
                      const resp = testOutput.response;
                      let bodyParsed: unknown = null;
                      if (typeof resp.body === 'string') {
                        try { bodyParsed = JSON.parse(resp.body); } catch { /* not JSON */ }
                      }

                      // Build a clean display object: show parsed body if possible
                      const display = bodyParsed != null
                        ? { statusCode: resp.statusCode, headers: resp.headers, body: bodyParsed }
                        : resp;

                      return (
                        <div>
                          <div className="text-sm font-medium mb-2">Response</div>
                          <pre className="bg-muted p-4 rounded-lg text-xs overflow-auto max-h-[250px] font-mono leading-relaxed">
                            {JSON.stringify(display, null, 2)}
                          </pre>
                        </div>
                      );
                    })()}

                    {testLogs.length > 0 && (
                    <div>
                      <div className="text-sm font-medium mb-2">Logs</div>
                      <div className="bg-black text-green-400 p-4 rounded-lg text-xs space-y-0.5 font-mono max-h-[200px] overflow-auto leading-relaxed">
                        {testLogs.map((log: string, i: number) => (
                          <div key={i}>{log}</div>
                        ))}
                      </div>
                    </div>
                    )}
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
                <Label htmlFor="runtime">Language</Label>
                <Controller
                  name='runtime'
                  control={control}
                  render={({ field }) => (
                    <Select value={field.value} onValueChange={field.onChange} disabled={isUpdate}>
                      <SelectTrigger id="runtime">
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="python">Python</SelectItem>
                        <SelectItem value="javascript">JavaScript</SelectItem>
                        <SelectItem value="typescript">TypeScript</SelectItem>
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
              <div className="border rounded-lg overflow-hidden w/full">
                <Editor
                  height="150px"
                  language="json"
                  value={testEvent}
                  onChange={(value) => setTestEvent(value || "")}
                  theme={theme === "dark" ? "vs-dark" : "light"}
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
