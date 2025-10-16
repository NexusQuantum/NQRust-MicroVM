"use client"

import dynamic from "next/dynamic";
import React from "react";
import { HiX, HiMenu } from "react-icons/hi";

const Monaco = dynamic(() => import("@monaco-editor/react"), { ssr: false });

declare global {
  namespace JSX {
    interface IntrinsicElements {
      [elemName: string]: any;
    }
  }
}

const DEFAULT_CODE = `// index.mjs  (Node.js 20.x / ESM)
export const handler = async (event) => {
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

const DEFAULT_PAYLOAD = `{
  \"key1\": 10,
  \"key2\": 5
}`;

const QUICK_FILES = [
  { name: "index.mjs", content: DEFAULT_CODE },
];

export default function Lambda() {
  const [code, setCode] = React.useState<string>(DEFAULT_CODE);
  const [payload, setPayload] = React.useState<string>(DEFAULT_PAYLOAD);
  const [output, setOutput] = React.useState<string>("Ready. Press Run or ⌘/Ctrl+Enter.");
  const [logs, setLogs] = React.useState<string[]>([]);
  const [status, setStatus] = React.useState<"idle" | "running" | "done" | "error">("idle");
  const [latencyMs, setLatencyMs] = React.useState<number | null>(null);

  // Layout sizes (resizable)
  const [leftW, setLeftW] = React.useState(240); // px (Quick Files)
  const [middleW, setMiddleW] = React.useState(720); // px (code editor)
  const [bottomH, setBottomH] = React.useState(220); // px (output)

  const [isSidebarOpen, setIsSidebarOpen] = React.useState(true); // For toggling Quick Files sidebar

  const dragXRef = React.useRef<{ startX: number; startW: number } | null>(null);
  const dragMiddleRef = React.useRef<{ startX: number; startW: number } | null>(null);
  const dragYRef = React.useRef<{ startY: number; startH: number } | null>(null);

  React.useEffect(() => {
    const onMove = (e: MouseEvent) => {
      if (dragXRef.current) {
        const dx = e.clientX - dragXRef.current.startX;
        const next = Math.min(Math.max(160, dragXRef.current.startW + dx), 600);
        setLeftW(next);
      }
      if (dragMiddleRef.current) {
        const dx = e.clientX - dragMiddleRef.current.startX;
        const next = Math.min(Math.max(360, dragMiddleRef.current.startW + dx), 1200);
        setMiddleW(next);
      }
      if (dragYRef.current) {
        const dy = dragYRef.current.startY - e.clientY;
        const next = Math.min(Math.max(140, dragYRef.current.startH + dy), 560);
        setBottomH(next);
      }
    };

    const onUp = () => {
      dragXRef.current = null;
      dragMiddleRef.current = null;
      dragYRef.current = null;
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };

    const startDragX = (e: React.MouseEvent) => {
      dragXRef.current = { startX: e.clientX, startW: leftW };
      document.body.style.cursor = "col-resize";
      document.body.style.userSelect = "none";
    };

    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
    return () => {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
    };
  }, []);

  const startDragMiddle = (e: React.MouseEvent) => {
    dragMiddleRef.current = { startX: e.clientX, startW: middleW };
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
  };

  const startDragY = (e: React.MouseEvent) => {
    dragYRef.current = { startY: e.clientY, startH: bottomH };
    document.body.style.cursor = "row-resize";
    document.body.style.userSelect = "none";
  };

  const copy = async (text: string) => {
    try { await navigator.clipboard.writeText(text); } catch { }
  };

  // run call app/api/invoke
  const run = React.useCallback(async () => {
    setStatus("running");
    setLogs([]);
    setLatencyMs(null);
    setOutput("Invoking function...");
    const t0 = performance.now();

    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 8000);

    try {
      const res = await fetch("/api/invoke", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          code,
          event: (() => { try { return JSON.parse(payload || "{}"); } catch { return payload; } })(),
        }),
        signal: controller.signal,
      });

      clearTimeout(timeout);
      const text = await res.text();
      const json = text ? JSON.parse(text) : {};
      const t1 = performance.now();
      setLatencyMs(Math.round(t1 - t0));

      if (!res.ok || !json?.ok) {
        const errMsg = json?.error || res.statusText || "Invocation failed";
        const response = { statusCode: 500, headers: { "content-type": "application/json" }, body: JSON.stringify({ error: errMsg }) };
        setOutput(formatExecutionResult({ status: "Failed", eventName: "event-tes", response }));
        setLogs(json?.logs || []);
        setStatus("error");
        return;
      }

      const response = json.response as { statusCode: number; headers: Record<string, string>; body: string };
      setOutput(formatExecutionResult({ status: "Succeeded", eventName: "event-tes", response }));
      setLogs(json?.logs || []);
      setStatus("done");
    } catch (err: any) {
      clearTimeout(timeout);
      const response = { statusCode: 500, headers: { "content-type": "application/json" }, body: JSON.stringify({ error: err?.name === "AbortError" ? "Request timed out" : (err?.message || String(err)) }) };
      setOutput(formatExecutionResult({ status: "Failed", eventName: "event-tes", response }));
      setStatus("error");
    }
  }, [code, payload]);


  React.useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === "Enter") {
        e.preventDefault();
        run();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [run]);

  const formatExecutionResult = ({ status, eventName, response }: { status: string; eventName: string; response: { statusCode: number; headers: Record<string, string>; body: string } }) => {
    const header = `Status: ${status}
Test Event Name: ${eventName}
`;
    const details = `
Response:
${JSON.stringify(response, null, 2)}

Parsed Body:
${safePrettyJSON(response.body)}`;
    return header + details;
  };

  const safePrettyJSON = (raw: string) => { try { return JSON.stringify(JSON.parse(raw), null, 2); } catch { return raw; } };

  const onSelectFile = (fileName: string) => {
    const file = QUICK_FILES.find((f) => f.name === fileName);
    if (file) {
      setCode(file.content); // update editor content
    }
  };

  return (
    <div className="h-screen w-full bg-neutral-950 text-neutral-100 dark:bg-black">
      {/* Header */}
      <header className="h-12 border-b border-neutral-800 flex items-center px-4 gap-3">
        <h1 className="text-sm font-semibold">Lambda Playground</h1>
        <span className={`text-2xs px-2 py-0.5 rounded-full border ml-2 ${status === "running" ? "border-amber-400 text-amber-300" : status === "done" ? "border-emerald-400 text-emerald-300" : status === "error" ? "border-rose-400 text-rose-300" : "border-neutral-700 text-neutral-400"}`}>{status.toUpperCase()}</span>
        <div className="ml-auto flex items-center gap-2">
          {latencyMs !== null && <div className="text-xs text-neutral-400 tabular-nums">{latencyMs} ms</div>}
          <button onClick={() => copy(code)} className="px-2.5 py-1 rounded-lg bg-neutral-900 hover:bg-neutral-800 text-xs cursor-pointer" title="Copy code">Copy code</button>
          <button onClick={run} className="px-3 py-1.5 rounded-xl bg-neutral-100 text-neutral-900 hover:bg-neutral-300 transition disabled:opacity-50 cursor-pointer" disabled={status === "running"}>{status === "running" ? "Running..." : "Run (Ctrl/⌘+Enter)"}</button>
        </div>
      </header>

      {/* Main Content */}
      <div className="flex h-[calc(100vh-3rem)]">
        {/* Left: Quick Files */}
        <aside className={`w-${isSidebarOpen ? '100' : '40'} border-r border-neutral-800 transition-all duration-200 overflow-hidden flex flex-col`}>
          <div className="flex items-center justify-between px-3 py-2">
            <h1>
              {isSidebarOpen ? "Quick Files" : "Files"}
            </h1>
            <button
              onClick={() => setIsSidebarOpen(!isSidebarOpen)}
              className="px-3 py-2 text-neutral-400 text-lg"
            >
              {isSidebarOpen ? <HiX /> : <HiMenu />}
            </button>
          </div>
          <div className="px-3 py-2 space-y-1 overflow-auto">
            {QUICK_FILES.map((file) => (
              <div
                key={file.name}
                className={`cursor-pointer text-neutral-400 hover:bg-neutral-800 hover:text-neutral-100 p-2 rounded-lg ${code === file.content ? "bg-neutral-800 text-neutral-100" : ""
                  }`}
                onClick={() => onSelectFile(file.name)}
              >
                {file.name}
              </div>
            ))}
          </div>
        </aside>

        {/* Vertical resizer */}
        {/* <div className="w-1.5 cursor-col-resize bg-transparent hover:bg-neutral-800/40" onMouseDown={startDragX} aria-label="Resize Quick Files / Code Editor" /> */}
      </div>
    </div>
  )
}