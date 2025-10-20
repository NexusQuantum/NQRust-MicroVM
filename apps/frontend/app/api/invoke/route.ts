// app/api/invoke/route.ts
import { NextRequest, NextResponse } from "next/server";

export const runtime = "nodejs"; // perlu child_process untuk Node & Python

import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { spawn } from "node:child_process";
import { pathToFileURL } from "node:url";

const ENGINE_TAG = "invoke-v6-node-child";

type InvokeBody = {
  runtime: "node" | "python";
  code: string;
  event: unknown;
};

type LambdaResponse = {
  statusCode: number;
  headers?: Record<string, string>;
  body?: string;
};

const INVOKE_TIMEOUT_MS = 7000;

export async function POST(req: NextRequest) {
  let payload: InvokeBody;
  try {
    payload = await req.json();
  } catch {
    return NextResponse.json({ ok: false, error: "Invalid JSON body", engineTag: ENGINE_TAG }, { status: 400 });
  }

  const { runtime, code, event } = payload || {};
  if (!runtime || (runtime !== "node" && runtime !== "python")) {
    return NextResponse.json({ ok: false, error: "Missing or invalid runtime (node|python)", engineTag: ENGINE_TAG }, { status: 400 });
  }
  if (typeof code !== "string" || !code.trim()) {
    return NextResponse.json({ ok: false, error: "Missing code", engineTag: ENGINE_TAG }, { status: 400 });
  }

  try {
    if (runtime === "node") {
      const result = await executeNode_ChildProcess(code, event, INVOKE_TIMEOUT_MS);
      return NextResponse.json({ ok: true, engineTag: ENGINE_TAG, strategy: "node-child", ...result });
    } else {
      const result = await executePython_TmpFiles(code, event, INVOKE_TIMEOUT_MS);
      return NextResponse.json({ ok: true, engineTag: ENGINE_TAG, strategy: "python-tmp", ...result });
    }
  } catch (err: any) {
    return NextResponse.json(
      { ok: false, engineTag: ENGINE_TAG, error: err?.message || String(err) },
      { status: 500 }
    );
  }
}

/** ===================== Node runtime via child process ===================== */
async function executeNode_ChildProcess(code: string, event: unknown, timeoutMs: number) {
  const tmpDir = await fs.mkdtemp(path.join(os.tmpdir(), "invk-node-"));
  const userFile = path.join(tmpDir, "index.mjs");
  const runnerFile = path.join(tmpDir, "runner.mjs");
  const eventFile = path.join(tmpDir, "event.json");

  await fs.writeFile(userFile, code, "utf8");
  await fs.writeFile(eventFile, JSON.stringify(event ?? {}, null, 2), "utf8");

  // Small ESM runner that imports the user's module and calls handler(event)
  const runnerSrc = `
import fs from "node:fs/promises";
import { pathToFileURL } from "node:url";
import path from "node:path";
import process from "node:process";

const RESULT_START = "___RESULT_START___";
const RESULT_END   = "___RESULT_END___";

// Capture console from user code and forward to parent stdout (lines outside markers are treated as logs)
const original = { ...console };
["log","info","warn","error"].forEach(k=>{
  console[k] = (...args)=> {
    try {
      const line = args.map(a => {
        try { return typeof a === "string" ? a : JSON.stringify(a); } catch { return String(a); }
      }).join(" ");
      process.stdout.write(line + "\\n");
    } catch {}
    original[k](...args);
  };
});

async function main() {
  const base = path.dirname(new URL(import.meta.url).pathname);
  const eventPath = path.join(base, "event.json");
  let event = {};
  try {
    const txt = await fs.readFile(eventPath, "utf8");
    event = JSON.parse(txt || "{}");
  } catch {}

  const modUrl = pathToFileURL(path.join(base, "index.mjs")).href + "?v=" + Date.now();

  let mod;
  try {
    mod = await import(modUrl);
  } catch (e) {
    emit({ statusCode:500, headers:{"content-type":"application/json"},
           body: JSON.stringify({ error: "Failed to import user module: " + (e?.message ?? String(e)) }) });
    return;
  }

  if (typeof mod?.handler !== "function") {
    emit({ statusCode:500, headers:{"content-type":"application/json"},
           body: JSON.stringify({ error: "Expected \\"export const handler = (event) => {...}\\"" }) });
    return;
  }

  try {
    const resp = await Promise.resolve(mod.handler(event));
    emit(normalize(resp));
  } catch (e) {
    emit({ statusCode:500, headers:{"content-type":"application/json"},
           body: JSON.stringify({ error: e?.message ?? String(e) }) });
  }
}

function normalize(r) {
  const statusCode = Number(r?.statusCode ?? 200);
  const headers = r && typeof r === "object" && !Array.isArray(r) && r.headers && typeof r.headers === "object" ? r.headers : {};
  let body = r?.body;
  if (typeof body !== "string") {
    try { body = JSON.stringify(body ?? null); } catch { body = String(body); }
  }
  return { statusCode, headers, body };
}

function emit(obj) {
  process.stdout.write(RESULT_START + "\\n");
  try { process.stdout.write(JSON.stringify(obj) + "\\n"); }
  catch {
    process.stdout.write(JSON.stringify({statusCode:500,headers:{"content-type":"application/json"},body: JSON.stringify({error:"Non-serializable response"})}) + "\\n");
  }
  process.stdout.write(RESULT_END + "\\n");
}

await main();
`;
  await fs.writeFile(runnerFile, runnerSrc, "utf8");

  const logs: string[] = [];
  let capturedJson = "";

  try {
    const child = spawn(process.env.NODE_PATH || "node", [runnerFile], {
      cwd: tmpDir,
      stdio: ["ignore", "pipe", "pipe"],
      env: { ...process.env, NODE_NO_WARNINGS: "1" },
    });

    let inResult = false;

    child.stdout.setEncoding("utf8");
    child.stdout.on("data", (chunk: string) => {
      for (const line of chunk.split(/\r?\n/)) {
        if (line === "___RESULT_START___") {
          inResult = true;
          capturedJson = "";
        } else if (line === "___RESULT_END___") {
          inResult = false;
        } else if (inResult) {
          capturedJson += line + "\n";
        } else if (line.trim()) {
          logs.push(line.trim());
        }
      }
    });

    child.stderr.setEncoding("utf8");
    child.stderr.on("data", (chunk: string) => {
      for (const line of String(chunk).split(/\r?\n/)) {
        if (line.trim()) logs.push(line.trim());
      }
    });

    const exitCode: number = await withTimeout(
      new Promise((resolve, reject) => {
        child.on("error", reject);
        child.on("close", resolve as any);
      }),
      timeoutMs,
      "Node execution timed out"
    );

    if (exitCode !== 0) {
      logs.push(`node exited with code ${exitCode}`);
    }

    let response: LambdaResponse;
    try {
      response = JSON.parse(capturedJson || "{}");
    } catch {
      response = {
        statusCode: 500,
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ error: "Invalid JSON from node runner" }),
      };
    }

    return { response: normalizeResponse(response), logs };
  } finally {
    try { await fs.rm(tmpDir, { recursive: true, force: true }); } catch {}
  }
}

/** ===================== Python runtime via tmp files (unchanged) ===================== */
async function executePython_TmpFiles(code: string, event: unknown, timeoutMs: number) {
  const tmpDir = await fs.mkdtemp(path.join(os.tmpdir(), "invk-py-"));
  const userFile = path.join(tmpDir, "index.py");
  const runnerFile = path.join(tmpDir, "runner.py");
  const eventFile = path.join(tmpDir, "event.json");

  await fs.writeFile(userFile, code, "utf8");
  await fs.writeFile(eventFile, JSON.stringify(event ?? {}, null, 2), "utf8");

  const runnerSource = `
import json, sys, traceback, importlib.util, os

RESULT_START = "___RESULT_START___"
RESULT_END   = "___RESULT_END___"

def load_module_from_path(module_name, file_path):
    spec = importlib.util.spec_from_file_location(module_name, file_path)
    if spec is None:
        raise ImportError("Cannot create spec for " + file_path)
    mod = importlib.util.module_from_spec(spec)
    loader = spec.loader
    if loader is None:
        raise ImportError("No loader for " + file_path)
    loader.exec_module(mod)
    return mod

def main():
    event_path = os.path.join(os.path.dirname(__file__), "event.json")
    with open(event_path, "r", encoding="utf-8") as f:
        event = json.load(f)

    mod = load_module_from_path("user_code", os.path.join(os.path.dirname(__file__), "index.py"))

    if not hasattr(mod, "handler") or not callable(mod.handler):
        print("handler is missing or not callable", file=sys.stderr)
        print(RESULT_START)
        print(json.dumps({"statusCode":500,"headers":{"content-type":"application/json"},"body":"{\\"error\\": \\"Missing handler\\"}"}))
        print(RESULT_END)
        return

    try:
        resp = mod.handler(event)
    except Exception as e:
        traceback.print_exc()
        resp = {"statusCode": 500, "headers": {"content-type":"application/json"}, "body": json.dumps({"error": str(e)})}

    print(RESULT_START)
    try:
        print(json.dumps(resp))
    except Exception:
        print(json.dumps({"statusCode":500,"headers":{"content-type":"application/json"},"body": json.dumps({"error":"Non-serializable response"})}))
    print(RESULT_END)

if __name__ == "__main__":
    main()
`;
  await fs.writeFile(runnerFile, runnerSource, "utf8");

  const logs: string[] = [];
  let capturedJson = "";

  try {
    const child = spawn(process.env.PYTHON_PATH || "python", [runnerFile], {
      cwd: tmpDir,
      stdio: ["ignore", "pipe", "pipe"],
      env: { ...process.env, PYTHONUNBUFFERED: "1" },
    });

    let inResult = false;

    child.stdout.setEncoding("utf8");
    child.stdout.on("data", (chunk: string) => {
      for (const line of chunk.split(/\r?\n/)) {
        if (line === "___RESULT_START___") {
          inResult = true;
          capturedJson = "";
        } else if (line === "___RESULT_END___") {
          inResult = false;
        } else if (inResult) {
          capturedJson += line + "\n";
        } else if (line.trim()) {
          logs.push(line.trim());
        }
      }
    });

    child.stderr.setEncoding("utf8");
    child.stderr.on("data", (chunk: string) => {
      for (const line of String(chunk).split(/\r?\n/)) {
        if (line.trim()) logs.push(line.trim());
      }
    });

    const exitCode: number = await withTimeout(
      new Promise((resolve, reject) => {
        child.on("error", reject);
        child.on("close", resolve as any);
      }),
      timeoutMs,
      "Python execution timed out"
    );

    if (exitCode !== 0) {
      logs.push(`python exited with code ${exitCode}`);
    }

    let response: LambdaResponse;
    try {
      response = JSON.parse(capturedJson || "{}");
    } catch {
      response = {
        statusCode: 500,
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ error: "Invalid JSON from python runner" }),
      };
    }

    return { response: normalizeResponse(response), logs };
  } finally {
    try { await fs.rm(tmpDir, { recursive: true, force: true }); } catch {}
  }
}

/** ===================== Util ===================== */
function normalizeResponse(resp: any): LambdaResponse {
  const statusCode = Number(resp?.statusCode ?? 200);
  const headers = isPlainObject(resp?.headers) ? resp.headers : {};
  let body = resp?.body;
  if (typeof body !== "string") {
    try { body = JSON.stringify(body ?? null); } catch { body = String(body); }
  }
  return { statusCode, headers, body };
}

function isPlainObject(v: any) {
  return v && typeof v === "object" && !Array.isArray(v);
}

function withTimeout<T>(p: Promise<T>, ms: number, msg: string): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    const to = setTimeout(() => reject(new Error(msg)), ms);
    p.then(
      (v) => { clearTimeout(to); resolve(v); },
      (e) => { clearTimeout(to); reject(e); }
    );
  });
}
