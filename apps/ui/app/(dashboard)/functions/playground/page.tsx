"use client"

import React, { useState, useRef, useEffect } from "react"
import { useRouter } from "next/navigation"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Label } from "@/components/ui/label"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Play, Loader2, ArrowLeft, Save } from "lucide-react"
import dynamic from "next/dynamic"
import Link from "next/link"
import { z } from "zod"
import { useForm, Controller } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import { useTheme } from "next-themes"

const Editor = dynamic(() => import("@monaco-editor/react"), { ssr: false })

/** --- Playground hanya butuh runtime & code --- */
const playgroundSchema = z.object({
  runtime: z.enum(["python", "javascript", "typescript"]),
  code: z.string().min(1, "Code is required"),
})
type PlaygroundForm = z.infer<typeof playgroundSchema>

/** --- Default code (JavaScript) --- */
const DEFAULT_CODE_JS = `// index.js (JavaScript)
export async function handler(event) {
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

/** --- Default code (TypeScript) --- */
const DEFAULT_CODE_TS = `// index.ts (TypeScript)
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

/** --- Default payload --- */
const DEFAULT_PAYLOAD = `{
  "key1": 10,
  "key2": 5
}`

/** ---------------------------------------------
 * NORMALIZER untuk RUN TEST:
 * For JavaScript/TypeScript (Bun), code is passed as-is since they use ES modules.
 * --------------------------------------------- */
type RuntimeType = "python" | "javascript" | "typescript";

function normalizeToModuleExportsForRunTest(
  runtime: RuntimeType,
  rawCode: string,
  handlerName: string = "handler",
) {
  // JavaScript and TypeScript use ES modules natively via Bun, no transformation needed
  // Python also passed as-is
  return rawCode;
}

export default function FunctionPlayground() {
  const router = useRouter()
  const { theme } = useTheme()
  const {
    watch,
    control,
    setValue,
    formState: { errors },
  } = useForm<PlaygroundForm>({
    resolver: zodResolver(playgroundSchema) as any,
    mode: "onChange",
    defaultValues: { runtime: "typescript", code: DEFAULT_CODE_TS },
  })

  const runtime = watch("runtime")
  const editorRef = useRef<any>(null)

  const [testEvent, setTestEvent] = useState(DEFAULT_PAYLOAD)
  const [testOutput, setTestOutput] = useState<any | null>(null)
  const [testLogs, setTestLogs] = useState<string[]>([])
  const [running, setRunning] = useState(false)

  // Ganti template code saat runtime berubah
  useEffect(() => {
    switch (runtime) {
      case "python":
        setValue("code", DEFAULT_CODE_PY)
        break
      case "javascript":
        setValue("code", DEFAULT_CODE_JS)
        break
      case "typescript":
        setValue("code", DEFAULT_CODE_TS)
        break
    }
  }, [runtime, setValue])

  const getLanguage = () => {
    switch (runtime) {
      case "python":
        return "python"
      case "typescript":
        return "typescript"
      default:
        return "javascript"
    }
  }

  const handleGoCreate = () => {
    const currentRuntime = watch("runtime")
    const currentCode = watch("code") as string
    const currentEvent = testEvent

    const draft = { runtime: currentRuntime, code: currentCode, event: currentEvent }
    // console.log("Draft: ", draft)
    try {
      sessionStorage.setItem("playground:draft", JSON.stringify(draft))
    } catch { }


    // opsional tambahkan query flag agar jelas asalnya dari playground
    router.push("/functions/new")

  }

  const handleTestRun = React.useCallback(async () => {
    setRunning(true)
    setTestOutput(null)
    setTestLogs([])

    // Parse event JSON
    let eventObj: unknown
    try {
      eventObj = JSON.parse(testEvent || "{}")
    } catch {
      setRunning(false)
      setTestOutput({ error: "Test Event harus berupa JSON valid" })
      return
    }

    try {
      const currentRuntime = watch("runtime") as RuntimeType
      const currentCode = watch("code") as string
      const handlerName = "handler" // playground fix: hanya pakai 'handler'

      // Normalisasi code Node ke exports.handler (CommonJS friendly)
      // Deno/Bun use ES modules, no transformation needed
      const codeForTest = normalizeToModuleExportsForRunTest(
        currentRuntime,
        currentCode,
        handlerName
      )

      const res = await fetch("/api/test", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          runtime: currentRuntime,
          code: codeForTest,
          event: eventObj,
          handler: handlerName, // opsional, runner kita dukung
        }),
      })

      const json = await res.json()
      setTestLogs(Array.isArray(json?.logs) ? json.logs : [])

      if (!res.ok || json?.ok === false) {
        setTestOutput({ error: json?.error || `HTTP ${res.status}` })
      } else {
        setTestOutput(json)
      }
    } catch (e: any) {
      setTestOutput({ error: e?.message || String(e) })
    } finally {
      setRunning(false)
    }
  }, [testEvent, watch])

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <Link href="/functions">
            <Button variant="ghost" size="icon">
              <ArrowLeft className="h-4 w-4" />
            </Button>
          </Link>
          <h1 className="text-3xl font-bold text-foreground">Function Playground</h1>
        </div>
        <div className="flex gap-2 items-center">
          {/* <Link href="/functions/new"> */}
          <Button onClick={handleGoCreate} disabled={running} >
            <Save className="mr-2 h-4 w-4" />
            Create
          </Button>
          {/* </Link> */}
        </div>
      </div>

      {/* Main Content */}
      <div className="grid gap-6 lg:grid-cols-3">
        {/* Code Editor */}
        <div className="lg:col-span-2 space-y-6">
          <Card>
            <CardHeader>
              <div className="flex items-center justify-between">
                <CardTitle>Code Editor</CardTitle>
                <div className="space-y-2 flex flex-row items-center justify-center gap-3">
                  <Label htmlFor="runtime" className="my-auto font-semibold">
                    Language
                  </Label>
                  <Controller
                    name="runtime"
                    control={control}
                    render={({ field }) => (
                      <Select value={field.value} onValueChange={field.onChange}>
                        <SelectTrigger id="runtime">
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="python">Python</SelectItem>
                          <SelectItem value="javascript">JavaScript (Bun)</SelectItem>
                          <SelectItem value="typescript">TypeScript (Bun)</SelectItem>
                        </SelectContent>
                      </Select>
                    )}
                  />
                </div>
              </div>
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
                          setTimeout(() => editorRef.current?.getAction("editor.action.formatDocument")?.run(), 100)
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
                        setTimeout(() => editor.getAction("editor.action.formatDocument")?.run(), 100)
                      }}
                    />
                  )}
                />
              </div>
            </CardContent>
          </Card>

          {/* Hasil Test */}
          {testOutput && (
            <Card>
              <CardHeader>
                <CardTitle>Test Results</CardTitle>
                <CardDescription>Result of the last run.</CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                {typeof testOutput === "string" ? (
                  <pre className="bg-muted p-3 rounded text-xs overflow-auto max-h-[240px]">
                    {testOutput}
                  </pre>
                ) : testOutput.error ? (
                  <div>
                    <div className="text-sm font-medium mb-1 text-destructive">Error</div>
                    <div className="bg-destructive/10 text-destructive p-3 rounded text-xs">
                      {testOutput.error}
                    </div>
                  </div>
                ) : (
                  <>
                    <div className="flex items-center gap-6">
                      <div>
                        <div className="text-sm text-muted-foreground">Status</div>
                        <div className={`font-medium ${testOutput.ok ? "text-emerald-600" : "text-red-600"}`}>
                          {testOutput.ok ? "Success" : "Error"}
                        </div>
                      </div>
                      <div>
                        <div className="text-sm text-muted-foreground">Duration</div>
                        <div className="font-medium">{testOutput.duration_ms ?? "N/A"}ms</div>
                      </div>
                    </div>

                    <div>
                      <div className="text-sm font-medium mb-2">Response</div>
                      <pre className="bg-muted p-3 rounded text-xs overflow-auto max-h-[240px]">
                        {JSON.stringify(testOutput.response, null, 2)}
                      </pre>
                    </div>

                    <div>
                      <div className="text-sm font-medium mb-2">Logs</div>
                      <div className="bg-black text-green-400 p-3 rounded text-xs space-y-1 font-mono max-h-[240px] overflow-auto">
                        {(testLogs || []).map((log, i) => (
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

        {/* Event Editor + Run */}
        <div className="space-y-6">
          <Card>
            <CardHeader>
              <CardTitle>Test Event</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="border rounded-lg overflow-hidden w-full">
                <Editor
                  height="475px"
                  language="json"
                  value={testEvent}
                  onChange={(value) => setTestEvent(value || "")}
                  theme={theme === "dark" ? "vs-dark" : "light"}
                  options={{
                    minimap: { enabled: false },
                    fontSize: 14,
                    lineNumbers: "on",
                    scrollBeyondLastLine: false,
                    automaticLayout: true,
                    wordWrap: "on",
                    wordWrapColumn: 100,
                    wrappingIndent: "same",
                    scrollBeyondLastColumn: 0,
                  }}
                />
              </div>
              <Button className="w-full" onClick={handleTestRun} disabled={running}>
                {running ? (
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
    </div>
  )
}
