"use client";

import React from "react";
import dynamic from "next/dynamic";

// Load Monaco in the browser only
const Monaco = dynamic(() => import("@monaco-editor/react"), { ssr: false });

declare global {
  namespace JSX {
    interface IntrinsicElements {
      [elemName: string]: any;
    }
  }
}

interface FileNode {
  id: string;
  name: string;
  type: "file" | "folder";
  children?: FileNode[];
}

const SAMPLE_TREE: FileNode[] = [
  {
    id: "src",
    name: "src",
    type: "folder",
    children: [
      { id: "handler.js", name: "index.mjs", type: "file" },
      { id: "utils.js", name: "utils.js", type: "file" },
    ],
  },
  {
    id: "tests",
    name: "tests",
    type: "folder",
    children: [{ id: "event.json", name: "event.json", type: "file" }],
  },
];

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

  const result = a + b;
  return {
    statusCode: 200,
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ result })
  };
};`;

const DEFAULT_PAYLOAD = `{
  "key1": 10,
  "key2": 5
}`;

export default function LambdaConsoleLikeUI() {
  const [tree] = React.useState<FileNode[]>(SAMPLE_TREE);
  const [selectedFile, setSelectedFile] = React.useState<string>("index.mjs");
  const [openFiles, setOpenFiles] = React.useState<string[]>(["index.mjs"]);
  const [code, setCode] = React.useState<string>(DEFAULT_CODE);
  const [payload, setPayload] = React.useState<string>(DEFAULT_PAYLOAD);
  const [output, setOutput] = React.useState<string>("Ready. Click Run to execute.");
  const [status, setStatus] =
    React.useState<"idle" | "running" | "done" | "error">("idle");

  // === RUN: panggil API route dengan code + payload
  // === RUN: panggil API route dengan code + payload
  const run = React.useCallback(async () => {
    setStatus("running");
    setOutput("Invoking function...\n");

    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 5000); // 5s timeout

    try {
      const res = await fetch("/api/invoke", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          code,
          event: (() => {
            try { return JSON.parse(payload || "{}"); } catch { return payload; }
          })(),
        }),
        signal: controller.signal,
      });

      clearTimeout(timeout);

      // dapatkan text dulu (untuk debug jika bukan JSON)
      const text = await res.text();
      let json: any = null;
      try {
        json = text ? JSON.parse(text) : null;
      } catch {
        throw new Error(`Invalid JSON response from /api/invoke: ${text}`);
      }

      if (!res.ok || !json?.ok) {
        throw new Error(json?.error || res.statusText || "Invocation failed");
      }

      const response = json.response as {
        statusCode: number;
        headers: Record<string, string>;
        body: string;
      };

      setOutput(
        formatExecutionResult({
          status: "Succeeded",
          eventName: "event-tes",
          response,
        })
      );
      setStatus("done");
    } catch (err: any) {
      clearTimeout(timeout);
      const msg =
        err.name === "AbortError"
          ? "Request timed out (no response from /api/invoke)"
          : err?.message || String(err);
      const response = {
        statusCode: 500,
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ error: msg }),
      };
      setOutput(formatExecutionResult({ status: "Failed", eventName: "event-tes", response }));
      setStatus("error");
      // opsional: tampilkan juga di console untuk debugging
      // console.error("Invoke error:", err);
    }
  }, [code, payload]);


  const formatExecutionResult = ({
    status,
    eventName,
    response,
  }: {
    status: string;
    eventName: string;
    response: { statusCode: number; headers: Record<string, string>; body: string };
  }) => {
    const header = `Status: ${status}\nTest Event Name: ${eventName}\n`;
    const details = `\nResponse:\n${JSON.stringify(
      response,
      null,
      2
    )}\n\nParsed Body:\n${safePrettyJSON(response.body)}`;
    return header + details;
  };

  const safePrettyJSON = (raw: string) => {
    try {
      return JSON.stringify(JSON.parse(raw), null, 2);
    } catch {
      return raw;
    }
  };

  const onSelectFile = (name: string) => {
    setSelectedFile(name);
    setOpenFiles((prev: string[]) => (prev.includes(name) ? prev : [...prev, name]));
  };

  const onCloseTab = (name: string) => {
    setOpenFiles((prev: string[]) => prev.filter((f) => f !== name));
    if (selectedFile === name && openFiles.length > 1) {
      const idx = openFiles.findIndex((f) => f === name);
      const next = openFiles[idx === 0 ? 1 : idx - 1];
      setSelectedFile(next);
    }
  };

  return (
    <div className="h-screen w-full bg-neutral-950 text-neutral-100">
      <header className="h-12 border-b border-neutral-800 flex items-center px-4 gap-3">
        <div className="text-sm uppercase tracking-widest text-neutral-400">
          Custom Extension
        </div>
        <div className="ml-auto flex items-center gap-2">
          <button
            onClick={run}
            className="px-3 py-1.5 rounded-xl bg-neutral-100 text-neutral-900 hover:bg-neutral-200 transition disabled:opacity-50"
            disabled={status === "running"}
          >
            {status === "running" ? "Running..." : "Run"}
          </button>
        </div>
      </header>

      <div className="grid grid-cols-12 h-[calc(100vh-3rem)]">
        <aside className="col-span-5 border-r border-neutral-800 flex flex-col">
          <div className="flex gap-1 px-2 py-2 border-y border-neutral-800 overflow-x-auto">
            {openFiles.map((f) => (
              <div
                key={f}
                className={`flex items-center gap-2 px-3 py-1.5 rounded-xl text-xs cursor-pointer select-none ${f === selectedFile ? "bg-neutral-800" : "bg-neutral-900 hover:bg-neutral-800"
                  }`}
                onClick={() => setSelectedFile(f)}
              >
                <span>{f}</span>
                <submit
                  className="opacity-60 hover:opacity-100"
                  onClick={(e) => {
                    e.stopPropagation();
                    onCloseTab(f);
                  }}
                  aria-label={`Close ${f}`}
                >
                  ×
                </submit>
              </div>
            ))}
          </div>

          <div className="flex-1 min-h-0">
            <Monaco
              height="100%"
              language="javascript"
              theme="vs-dark"
              value={code}
              onChange={(v) => setCode(v || "")}
              options={{ fontSize: 13, minimap: { enabled: false }, wordWrap: "on" }}
            />
          </div>
        </aside>

        <section className="col-span-7 flex flex-col">
          <div className="flex items-center justify-between px-4 py-2 border-b border-neutral-800">
            <div>
              <div className="text-sm font-semibold">Edit test event</div>
              <div className="text-xs text-neutral-400">
                Event Name: <span className="font-mono">event-tes</span>
              </div>
            </div>
            <div className="flex items-center gap-2">
              <submit
                onClick={() => setPayload(DEFAULT_PAYLOAD)}
                className="px-3 py-1.5 rounded-xl bg-neutral-900 hover:bg-neutral-800 text-xs"
              >
                Reset
              </submit>
              <submit
                onClick={run}
                className="px-3 py-1.5 rounded-xl bg-neutral-100 text-neutral-900 hover:bg-white text-xs"
                disabled={status === "running"}
              >
                Invoke
              </submit>
            </div>
          </div>

          <div className="flex-1 min-h-0">
            <Monaco
              height="100%"
              language="json"
              theme="vs-dark"
              value={payload}
              onChange={(v) => setPayload(v || "")}
              options={{ fontSize: 13, minimap: { enabled: false }, wordWrap: "on" }}
            />
          </div>

          <div className="h-64 border-t border-neutral-800 bg-neutral-950/60">
            <div className="flex items-center justify-between px-4 py-2 border-b border-neutral-800">
              <div className="text-sm font-semibold">Execution Results</div>
              <div className="text-xs text-neutral-400">Layout: US</div>
            </div>
            <pre className="h-[calc(16rem-2.5rem)] overflow-auto p-4 text-xs leading-5 bg-neutral-950">
              {output}
            </pre>
          </div>
        </section>
      </div>
    </div>
  );
}

function TreeNode({ node, onOpen }: { node: FileNode; onOpen: (name: string) => void }) {
  const [open, setOpen] = React.useState(true);
  const isFolder = node.type === "folder";

  if (isFolder) {
    return (
      <div className="select-none">
        <div
          className="flex items-center gap-2 cursor-pointer text-neutral-300 hover:text-white"
          onClick={() => setOpen((o) => !o)}
        >
          <span className="w-4 text-center">{open ? "▾" : "▸"}</span>
          <span className="font-medium">{node.name}</span>
        </div>
        {open && (
          <div className="pl-6 mt-1 space-y-1">
            {node.children?.map((child) => (
              <TreeNode key={child.id} node={child} onOpen={onOpen} />
            ))}
          </div>
        )}
      </div>
    );
  }

  return (
    <div
      className="pl-6 cursor-pointer text-neutral-400 hover:text-white"
      onClick={() => onOpen(node.name)}
    >
      {node.name}
    </div>
  );
}
