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
  initialRuntime?: "node" | "python" | "deno" | "bun"
  initialCode?: string
  initialEvent?: string
}

const fnCreationSchema = z.object({
  name: z.string().min(1, "Function Name is required").max(50, "Name too long"),
  runtime: z.enum(['node', 'python', 'deno', 'bun']),
  handler: z.string().min(1, "Handler Name is required").max(50, "Name too long"),
  code: z.string().min(1, "Code is required"),
  vcpu: z.number().min(1, "Minimum 1 vCPU").max(32, "Maximum 32 vCPU"),
  memory_mb: z.number().min(128, "Minimum 128 MB").max(3072, "Maximum 3072 MB"),
  timeout_seconds: z.number().min(1, "Minimum 1s").max(300, "Maximum 300s")
})

type FnCreationForm = z.infer<typeof fnCreationSchema>

// Default code (biarkan module.exports.handler agar familiar di UI)
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

const DEFAULT_CODE_DENO = `// index.ts (Deno / TypeScript)
interface Event {
  key1?: number | string;
  key2?: number | string;
}

export async function handler(event: Event) {
  const a = Number(event?.key1);
  const b = Number(event?.key2);
  
  if (!Number.isFinite(a) || !Number.isFinite(b)) {
    return {
      statusCode: 400,
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ error: "key1 and key2 must be numbers" }),
    };
  }
  
  return {
    statusCode: 200,
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ result: a + b }),
  };
}`;

const DEFAULT_CODE_BUN = `// index.ts (Bun / TypeScript)
interface Event {
  key1?: number | string;
  key2?: number | string;
}

export async function handler(event: Event) {
  const a = Number(event?.key1);
  const b = Number(event?.key2);
  
  if (!Number.isFinite(a) || !Number.isFinite(b)) {
    return {
      statusCode: 400,
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ error: "key1 and key2 must be numbers" }),
    };
  }
  
  return {
    statusCode: 200,
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ result: a + b }),
  };
}`;

const DEFAULT_PAYLOAD = `{
  // Test event payload
}`

function getDefaultCodeForRuntime(runtime: string): string {
  switch (runtime) {
    case 'python':
      return DEFAULT_CODE_PY
    case 'deno':
      return DEFAULT_CODE_DENO
    case 'bun':
      return DEFAULT_CODE_BUN
    default:
      return DEFAULT_CODE_NODE
  }
}

/* ---------------------------------------------
 * NORMALIZER #1 (BACKEND Create/Update):
 * Paksa format: const <handlerName> = async (...) => {...}
 * For Deno/Bun, code is passed as-is (ES modules).
 * --------------------------------------------- */
type RuntimeType = 'node' | 'python' | 'deno' | 'bun';

function normalizeToConstHandlerForBackend(
  runtime: RuntimeType,
  rawCode: string,
  handlerName: string
) {
  // Deno and Bun use ES modules natively, no transformation needed
  if (runtime === 'deno' || runtime === 'bun' || runtime === 'python') return rawCode;

  if (runtime !== 'node') return rawCode;

  let code = rawCode;

  // module.exports.handler = <func>  →  const <handler> =
  code = code.replace(/module\.exports\.handler\s*=\s*/g, `const ${handlerName} = `);

  // exports.handler = <func> → const <handler> =
  code = code.replace(/exports\.handler\s*=\s*/g, `const ${handlerName} = `);

  // module.exports = { handler: <func> }  → ambil value after handler:
  if (/module\.exports\s*=\s*{[\s\S]*?handler\s*:/m.test(code)) {
    code = code.replace(
      /module\.exports\s*=\s*{[\s\S]*?handler\s*:\s*/m,
      `const ${handlerName} = `
    );
    // copot "}" terakhir
    code = code.replace(/}\s*;?\s*$/, "");
  }

  // Bersihkan sisa ekspor lain
  code = code.replace(/module\.exports\s*=\s*[^\n;]+;?/g, "");
  code = code.replace(/exports\.[a-zA-Z0-9_$]+\s*=\s*[^\n;]+;?/g, "");

  return code.trim();
}

/* ---------------------------------------------
 * NORMALIZER #2 (RUN TEST /api/test):
 * Pastikan selalu ada exports.handler (tanpa mengandalkan module),
 * lalu mirror ke module.exports.handler jika module tersedia.
 * For Deno/Bun, code is passed as-is (ES modules).
 * --------------------------------------------- */
function normalizeToModuleExportsForRunTest(
  runtime: RuntimeType,
  rawCode: string,
  handlerName: string
) {
  // Deno and Bun use ES modules natively, no transformation needed
  if (runtime === 'deno' || runtime === 'bun' || runtime === 'python') return rawCode;

  if (runtime !== 'node') return rawCode;

  let code = rawCode;

  // 1) module.exports.handler = ... → exports.handler = ...
  code = code.replace(/module\.exports\.handler\s*=\s*/g, "exports.handler = ");

  // 2) module.exports = { handler: ... } → exports.handler = ...
  if (/module\.exports\s*=\s*{[\s\S]*?handler\s*:/m.test(code)) {
    code = code
      .replace(/module\.exports\s*=\s*{[\s\S]*?handler\s*:\s*/m, "exports.handler = ")
      .replace(/}\s*;?\s*$/, "");
  }

  // 3) Biarkan exports.handler = ... kalau sudah ada.

  // 4) Jika belum ada ekspor, tapi ada deklarasi handlerName/handler → ekspor
  const hasExportsHandler = /exports\.handler\s*=/.test(code);
  const hasNamedHandlerDecl =
    new RegExp(`\\b(const|let|var)\\s+${handlerName}\\s*=`).test(code) ||
    new RegExp(`\\b(async\\s+)?function\\s+${handlerName}\\s*\\(`).test(code);
  const hasDefaultHandlerDecl =
    /\b(const|let|var)\s+handler\s*=/.test(code) ||
    /\b(async\s+)?function\s+handler\s*\(/.test(code);

  if (!hasExportsHandler) {
    if (hasNamedHandlerDecl) {
      code += `\n\n// Auto-export for test runner (named)\nexports.handler = ${handlerName};`;
    } else if (hasDefaultHandlerDecl) {
      code += `\n\n// Auto-export for test runner (default)\nexports.handler = handler;`;
    } else {
      code += `\n\n// Auto-export for test runner (fallback)\nexports.handler = async function(){ throw new Error("Handler '${handlerName}' is not defined."); };`;
    }
  }

  // 5) Mirror aman ke module.exports.handler jika module tersedia
  code += `

/* Guarded mirror to module.exports.handler when module exists */
try {
  if (typeof module !== "undefined" && module && module.exports && !module.exports.handler && typeof exports !== "undefined" && exports && exports.handler) {
    module.exports.handler = exports.handler;
  }
} catch {}
`;

  return code.trim();
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
      runtime: (initialRuntime ?? functionData?.runtime ?? "node") as RuntimeType,
      handler: functionData?.handler ?? "handler",
      code:
        initialCode ??
        functionData?.code ??
        getDefaultCodeForRuntime(initialRuntime ?? functionData?.runtime ?? "node"),
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
      runtime: (initialRuntime ?? getValues('runtime') ?? "node") as RuntimeType,
      code: initialCode ?? getValues('code') ?? DEFAULT_CODE_NODE,
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
      case 'deno':
      case 'bun':
        return "typescript"
      default:
        return "javascript"
    }
  }

  const createFunction = useCreateFunction()
  const updateFunction = useUpdateFunction()
  const invokeMutation = useInvokeFunction()

  // Create/Update → kirim SELALU versi const handler (khusus Node)
  const onSubmit = async (data: FnCreationForm) => {
    try {
      const codeForBackend =
        data.runtime === 'node'
          ? normalizeToConstHandlerForBackend('node', data.code, data.handler)
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
      const currentRuntime = watch('runtime');           // 'node' | 'python'
      const currentCode = watch('code');
      const currentHandler = watch('handler') || 'handler';

      // Khusus RUN TEST: normalisasi ke exports.handler (+ shim aman)
      const codeForTest = normalizeToModuleExportsForRunTest(
        currentRuntime as 'node' | 'python',
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
                        <div className={`font-medium capitalize ${testOutput.status === 'success' ? 'text-green-600' : 'text-red-600'}`} >
                          {testOutput.status}
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
                    <Select value={field.value} onValueChange={field.onChange} disabled={isUpdate}>
                      <SelectTrigger id="runtime">
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="node">Node.js</SelectItem>
                        <SelectItem value="python">Python</SelectItem>
                        <SelectItem value="deno">Deno (TypeScript)</SelectItem>
                        <SelectItem value="bun">Bun (TypeScript)</SelectItem>
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
