// app/ui/app/api/test/route.ts
import { NextRequest, NextResponse } from "next/server";

export const runtime = "nodejs";

import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { spawn } from "node:child_process";

const ENGINE_TAG = "invoke-v8-bun-esm";

type InvokeBody = {
  runtime: "python" | "javascript" | "typescript";
  code: string;
  event: unknown;
  handler?: string; // ← NEW: nama handler kustom (default: 'handler')
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
    return NextResponse.json(
      { ok: false, error: "Invalid JSON body", engineTag: ENGINE_TAG },
      { status: 400 }
    );
  }

  const { runtime, code, event, handler } = payload || {};
  if (!runtime || (runtime !== "python" && runtime !== "javascript" && runtime !== "typescript")) {
    return NextResponse.json(
      { ok: false, error: "Missing or invalid runtime (python|javascript|typescript)", engineTag: ENGINE_TAG },
      { status: 400 }
    );
  }
  if (typeof code !== "string" || !code.trim()) {
    return NextResponse.json(
      { ok: false, error: "Missing code", engineTag: ENGINE_TAG },
      { status: 400 }
    );
  }

  try {
    if (runtime === "javascript" || runtime === "typescript") {
      const result = await executeBun_ESM(code, event, handler ?? "handler", INVOKE_TIMEOUT_MS, runtime);
      return NextResponse.json({ ok: true, engineTag: ENGINE_TAG, strategy: "bun-esm", ...result });
    } else {
      const result = await executePython_TmpFiles(code, event, handler ?? "handler", INVOKE_TIMEOUT_MS);
      return NextResponse.json({ ok: true, engineTag: ENGINE_TAG, strategy: "python-tmp", ...result });
    }
  } catch (err: any) {
    return NextResponse.json(
      { ok: false, engineTag: ENGINE_TAG, error: err?.message || String(err) },
      { status: 500 }
    );
  }
}

/** ===================== Bun (ES Modules) via child process ===================== */
async function executeBun_ESM(code: string, event: unknown, handlerName: string, timeoutMs: number, runtime: "javascript" | "typescript") {
  const tmpDir = await fs.mkdtemp(path.join(os.tmpdir(), "invk-bun-esm-"));
  const ext = runtime === "typescript" ? "ts" : "js";
  const userFile = path.join(tmpDir, `index.${ext}`);
  const runnerFile = path.join(tmpDir, "runner.js");
  const eventFile = path.join(tmpDir, "event.json");

  await fs.writeFile(userFile, code, "utf8");
  await fs.writeFile(eventFile, JSON.stringify(event ?? {}, null, 2), "utf8");

  const runnerSrc = `
// runner.js (ES Modules for Bun)
import { readFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const RESULT_START = "___RESULT_START___";
const RESULT_END   = "___RESULT_END___";

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

function readJSON(p) {
  try { return JSON.parse(readFileSync(p, "utf8") || "{}"); } catch { return {}; }
}

function emit(obj) {
  process.stdout.write(RESULT_START + "\\n");
  try { process.stdout.write(JSON.stringify(obj) + "\\n"); }
  catch {
    process.stdout.write(JSON.stringify({statusCode:500,headers:{"content-type":"application/json"},body: JSON.stringify({error:"Non-serializable response"})}) + "\\n");
  }
  process.stdout.write(RESULT_END + "\\n");
}

function normalize(r) {
  const statusCode = Number(r && r.statusCode != null ? r.statusCode : 200);
  const headers = r && typeof r === "object" && r.headers && typeof r.headers === "object" ? r.headers : {};
  let body = r && r.body;
  if (typeof body !== "string") {
    try { body = JSON.stringify(body ?? null); } catch { body = String(body); }
  }
  return { statusCode, headers, body };
}

async function main() {
  const event = readJSON(join(__dirname, "event.json"));
  let mod;

  try {
    mod = await import(join(__dirname, "index.${ext}"));
  } catch (e) {
    emit({ statusCode:500, headers:{"content-type":"application/json"},
           body: JSON.stringify({ error: "Failed to import user module: " + (e && e.message ? e.message : String(e)) }) });
    return;
  }

  const handlerName = process.env.HANDLER_NAME || "handler";
  const fn = mod && typeof mod[handlerName] === "function" ? mod[handlerName]
          : (mod && mod.default && typeof mod.default[handlerName] === "function" ? mod.default[handlerName] : null);

  if (typeof fn !== "function") {
    emit({ statusCode:500, headers:{"content-type":"application/json"},
           body: JSON.stringify({ error: "Missing ES module export: export function " + handlerName + "()" }) });
    return;
  }

  try {
    const resp = await fn(event);
    emit(normalize(resp));
  } catch (e) {
    emit({ statusCode:500, headers:{"content-type":"application/json"},
           body: JSON.stringify({ error: e && e.message ? e.message : String(e) }) });
  }
}

main();
`;
  await fs.writeFile(runnerFile, runnerSrc, "utf8");

  const logs: string[] = [];
  let capturedJson = "";

  try {
    // Try bun first, fall back to node
    const bunCmd = process.env.BUN_BIN || "bun";
    const nodeCmd = process.execPath;
    
    let cmd = bunCmd;
    let args = ["run", runnerFile];
    
    // Check if bun is available, otherwise use node
    try {
      const { execSync } = await import("node:child_process");
      execSync("which bun", { stdio: "ignore" });
    } catch {
      // Bun not found, use node
      cmd = nodeCmd;
      args = [runnerFile];
    }

    const child = spawn(cmd, args, {
      cwd: tmpDir,
      stdio: ["ignore", "pipe", "pipe"],
      env: { ...process.env, NODE_NO_WARNINGS: "1", HANDLER_NAME: handlerName },
    });

    let inResult = false;

    child.stdout.setEncoding("utf8");
    child.stdout.on("data", (chunk: string) => {
      for (const line of chunk.split(/\\r?\\n/)) {
        if (line === "___RESULT_START___") {
          inResult = true;
          capturedJson = "";
        } else if (line === "___RESULT_END___") {
          inResult = false;
        } else if (inResult) {
          capturedJson += line + "\\n";
        } else if (line.trim()) {
          logs.push(line.trim());
        }
      }
    });

    child.stderr.setEncoding("utf8");
    child.stderr.on("data", (chunk: string) => {
      for (const line of String(chunk).split(/\\r?\\n/)) {
        if (line.trim()) logs.push(line.trim());
      }
    });

    const exitCode: number = await withTimeout(
      new Promise((resolve, reject) => {
        child.on("error", reject);
        child.on("close", resolve as any);
      }),
      timeoutMs,
      "Bun/Node execution timed out"
    );

    if (exitCode !== 0) {
      logs.push(`runner exited with code ${exitCode}`);
    }

    let response: LambdaResponse;
    try {
      response = JSON.parse(capturedJson || "{}");
    } catch {
      response = {
        statusCode: 500,
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ error: "Invalid JSON from runner" }),
      };
    }

    return { response: normalizeResponse(response), logs };
  } finally {
    try { await fs.rm(tmpDir, { recursive: true, force: true }); } catch {}
  }
}

/** ===================== Python via tmp files (mendukung handlerName via ENV) ===================== */
async function executePython_TmpFiles(code: string, event: unknown, handlerName: string, timeoutMs: number) {
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
    base = os.path.dirname(__file__)
    event_path = os.path.join(base, "event.json")
    with open(event_path, "r", encoding="utf-8") as f:
        event = json.load(f)

    mod = load_module_from_path("user_code", os.path.join(base, "index.py"))

    handler_name = os.environ.get("HANDLER_NAME", "handler")
    fn = getattr(mod, handler_name, None)

    if not callable(fn):
        print("handler is missing or not callable", file=sys.stderr)
        print(RESULT_START)
        print(json.dumps({"statusCode":500,"headers":{"content-type":"application/json"},"body":json.dumps({"error":"Missing handler: "+handler_name})}))
        print(RESULT_END)
        return

    try:
        resp = fn(event)
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
    const pythonCmd = process.env.PYTHON_BIN || (process.platform === "win32" ? "python" : "python3");
    const child = spawn(pythonCmd, [runnerFile], {
      cwd: tmpDir,
      stdio: ["ignore", "pipe", "pipe"],
      env: { ...process.env, PYTHONUNBUFFERED: "1", HANDLER_NAME: handlerName },
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
