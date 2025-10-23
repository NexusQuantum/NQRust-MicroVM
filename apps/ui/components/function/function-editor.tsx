"use client"

import { useState, useRef } from "react"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Slider } from "@/components/ui/slider"
import { Save, Play, Loader2, Check } from "lucide-react"
import type { Function } from "@/lib/types"
import dynamic from "next/dynamic"

const Editor = dynamic(() => import("@monaco-editor/react"), { ssr: false })

interface FunctionEditorProps {
  functionData?: Function
}

export function FunctionEditor({ functionData }: FunctionEditorProps) {
  const [name, setName] = useState(functionData?.name || "")
  const [runtime, setRuntime] = useState(functionData?.runtime || "node")
  const [code, setCode] = useState(
    functionData?.code ||
      `export const handler = async (event) => {
  // Your code here
  console.log('Event:', event);
  
  return { 
    statusCode: 200,
    body: JSON.stringify({ message: 'Hello World' })
  };
};`,
  )
  const [handler, setHandler] = useState(functionData?.handler || "index.handler")
  const [memory, setMemory] = useState(functionData?.memory_mb || 512)
  const [timeout, setTimeout] = useState(functionData?.timeout_seconds || 30)
  const [testEvent, setTestEvent] = useState('{\n  "key": "value",\n  "userId": "123"\n}')
  const [testResult, setTestResult] = useState<any>(null)
  const [isRunning, setIsRunning] = useState(false)
  const [isSaved, setIsSaved] = useState(false)
  const editorRef = useRef<any>(null)

  const getLanguage = () => {
    switch (runtime) {
      case "python":
        return "python"
      case "go":
        return "go"
      case "rust":
        return "rust"
      default:
        return "javascript"
    }
  }

  const handleTest = async () => {
    setIsRunning(true)
    await new Promise((resolve) => setTimeout(resolve, 1500))
    setTestResult({
      status: "success",
      duration: Math.floor(Math.random() * 500) + 100,
      memory: Math.floor(Math.random() * 200) + 50,
      response: {
        statusCode: 200,
        body: JSON.stringify({ message: "Test successful", timestamp: new Date().toISOString() }),
      },
      logs: [
        `[${new Date().toISOString()}] START RequestId: ${Math.random().toString(36).substring(7)}`,
        `[${new Date().toISOString()}] Event: ${testEvent.substring(0, 50)}...`,
        `[${new Date().toISOString()}] Processing request`,
        `[${new Date().toISOString()}] END RequestId: ${Math.random().toString(36).substring(7)}`,
      ],
    })
    setIsRunning(false)
  }

  

  const handleSave = async () => {
    setIsSaved(true)
    setTimeout(() => setIsSaved(false), 2000)
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-foreground">
            {functionData ? `Edit ${functionData.name}` : "New Function"}
          </h1>
          <p className="text-muted-foreground">Configure and test your serverless function</p>
        </div>
        <div className="flex gap-2">
          <Button variant="outline" onClick={handleSave}>
            {isSaved ? (
              <>
                <Check className="mr-2 h-4 w-4" />
                Saved
              </>
            ) : (
              <>
                <Save className="mr-2 h-4 w-4" />
                Save Draft
              </>
            )}
          </Button>
          <Button onClick={handleSave}>
            <Save className="mr-2 h-4 w-4" />
            Deploy
          </Button>
        </div>
      </div>

      <div className="grid gap-6 lg:grid-cols-3">
        <div className="lg:col-span-2 space-y-6">
          <Card>
            <CardHeader>
              <CardTitle>Code Editor</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="border rounded-lg overflow-hidden">
                <Editor
                  height="500px"
                  language={getLanguage()}
                  value={code}
                  onChange={(value) => setCode(value || "")}
                  theme="vs-dark"
                  options={{
                    minimap: { enabled: false },
                    fontSize: 14,
                    lineNumbers: "on",
                    scrollBeyondLastLine: false,
                    automaticLayout: true,
                    tabSize: 2,
                  }}
                  onMount={(editor) => {
                    editorRef.current = editor
                  }}
                />
              </div>
            </CardContent>
          </Card>

          {testResult && (
            <Card>
              <CardHeader>
                <CardTitle>Test Results</CardTitle>
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="flex items-center gap-4">
                  <div>
                    <div className="text-sm text-muted-foreground">Status</div>
                    <div className="font-medium capitalize text-emerald-600">{testResult.status}</div>
                  </div>
                  <div>
                    <div className="text-sm text-muted-foreground">Duration</div>
                    <div className="font-medium">{testResult.duration}ms</div>
                  </div>
                  <div>
                    <div className="text-sm text-muted-foreground">Memory</div>
                    <div className="font-medium">{testResult.memory}MB</div>
                  </div>
                </div>

                <div>
                  <div className="text-sm font-medium mb-2">Response</div>
                  <pre className="bg-muted p-3 rounded text-xs overflow-auto max-h-[200px]">
                    {JSON.stringify(testResult.response, null, 2)}
                  </pre>
                </div>

                <div>
                  <div className="text-sm font-medium mb-2">Logs</div>
                  <div className="bg-black text-green-400 p-3 rounded text-xs space-y-1 font-mono max-h-[200px] overflow-auto">
                    {testResult.logs.map((log: string, i: number) => (
                      <div key={i}>{log}</div>
                    ))}
                  </div>
                </div>
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
                <Input id="name" value={name} onChange={(e) => setName(e.target.value)} placeholder="my-function" />
              </div>

              <div className="space-y-2">
                <Label htmlFor="runtime">Runtime</Label>
                <Select value={runtime} onValueChange={(value: any) => setRuntime(value)}>
                  <SelectTrigger id="runtime">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="node">Node.js 20.x</SelectItem>
                    <SelectItem value="python">Python 3.11</SelectItem>
                    <SelectItem value="go">Go 1.21</SelectItem>
                    <SelectItem value="rust">Rust 1.75</SelectItem>
                  </SelectContent>
                </Select>
              </div>

              <div className="space-y-2">
                <Label htmlFor="handler">Handler</Label>
                <Input id="handler" value={handler} onChange={(e) => setHandler(e.target.value)} />
              </div>

              <div className="space-y-2">
                <Label>Memory: {memory} MB</Label>
                <Slider value={[memory]} onValueChange={(v) => setMemory(v[0])} min={128} max={3072} step={128} />
              </div>

              <div className="space-y-2">
                <Label>Timeout: {timeout}s</Label>
                <Slider value={[timeout]} onValueChange={(v) => setTimeout(v[0])} min={1} max={900} step={1} />
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>Test Event</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="border rounded-lg overflow-hidden">
                <Editor
                  height="150px"
                  language="json"
                  value={testEvent}
                  onChange={(value) => setTestEvent(value || "")}
                  theme="vs-dark"
                  options={{
                    minimap: { enabled: false },
                    fontSize: 12,
                    lineNumbers: "off",
                    scrollBeyondLastLine: false,
                    automaticLayout: true,
                  }}
                />
              </div>
              <Button onClick={handleTest} disabled={isRunning} className="w-full">
                {isRunning ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    Running...
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
