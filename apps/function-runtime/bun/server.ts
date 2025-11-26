#!/usr/bin/env bun

/**
 * Bun/TypeScript Function Runtime Server
 *
 * This HTTP server runs inside a Firecracker MicroVM and executes
 * serverless functions on demand. Each function gets its own VM.
 *
 * Bun provides:
 *   - Native TypeScript support (no transpilation needed)
 *   - Fast startup time (~25ms)
 *   - Built-in HTTP server
 *   - Web-standard APIs (fetch, Request, Response)
 *
 * Endpoints:
 *   GET  /health       - Health check
 *   POST /invoke       - Execute the function
 *   POST /write-code   - Write and reload function code
 *   POST /reload       - Hot-reload function code
 */

const PORT = parseInt(Bun.env.PORT || "3000");
const FUNCTION_CODE_PATH = Bun.env.FUNCTION_CODE_PATH || "/function/code.ts";
const FUNCTION_HANDLER = Bun.env.FUNCTION_HANDLER || "handler";

type HandlerFunction = (event: unknown, context?: ExecutionContext) => unknown | Promise<unknown>;

interface ExecutionContext {
  functionName: string;
  requestId: string;
  invokedAt: string;
  memoryLimitMB: number;
  timeoutMs: number;
}

interface HealthResponse {
  status: "healthy" | "unhealthy";
  runtime: string;
  version: string;
  handler: string;
  codeLoaded: boolean;
  error: string | null;
  uptime: number;
}

interface InvokeResponse {
  status: "success" | "error";
  duration_ms: number;
  response?: unknown;
  error?: string;
  stack?: string;
  logs: string[];
}

let handler: HandlerFunction | null = null;
let loadError: string | null = null;
const startTime = Date.now();

/**
 * Load (or reload) the function code
 */
async function loadFunction(): Promise<boolean> {
  try {
    // Bun's module cache - we need to bust it for hot reload
    // Using a timestamp query param trick
    const cacheBuster = `?t=${Date.now()}`;
    const modulePath = FUNCTION_CODE_PATH + cacheBuster;

    const file = Bun.file(FUNCTION_CODE_PATH);
    if (!(await file.exists())) {
      loadError = `Function code not found at ${FUNCTION_CODE_PATH}`;
      console.error(`[Runtime] ${loadError}`);
      return false;
    }

    // Dynamic import with cache busting
    const module = await import(modulePath);
    handler = module[FUNCTION_HANDLER] || module.default;

    if (typeof handler !== "function") {
      throw new Error(`Handler "${FUNCTION_HANDLER}" is not a function`);
    }

    loadError = null;
    console.log(`[Runtime] Loaded function handler: ${FUNCTION_HANDLER}`);
    return true;
  } catch (error) {
    loadError = error instanceof Error ? error.message : String(error);
    console.error(`[Runtime] Failed to load function: ${loadError}`);
    return false;
  }
}

/**
 * Generate a unique request ID
 */
function generateRequestId(): string {
  return `req_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 9)}`;
}

/**
 * Create execution context for the function
 */
function createContext(requestId: string): ExecutionContext {
  return {
    functionName: FUNCTION_HANDLER,
    requestId,
    invokedAt: new Date().toISOString(),
    memoryLimitMB: 128, // Default, can be configured
    timeoutMs: 30000,   // 30 second default timeout
  };
}

/**
 * Bun HTTP server
 */
const server = Bun.serve({
  port: PORT,
  hostname: "0.0.0.0",

  async fetch(req: Request): Promise<Response> {
    const url = new URL(req.url);
    const path = url.pathname;
    const method = req.method;

    // CORS headers for development
    const corsHeaders = {
      "Access-Control-Allow-Origin": "*",
      "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
      "Access-Control-Allow-Headers": "Content-Type",
    };

    // Handle preflight
    if (method === "OPTIONS") {
      return new Response(null, { status: 204, headers: corsHeaders });
    }

    // Health check endpoint
    if (path === "/health" && method === "GET") {
      const response: HealthResponse = {
        status: handler !== null ? "healthy" : "unhealthy",
        runtime: "bun",
        version: Bun.version,
        handler: FUNCTION_HANDLER,
        codeLoaded: handler !== null,
        error: loadError,
        uptime: Math.floor((Date.now() - startTime) / 1000),
      };

      return Response.json(response, {
        headers: { ...corsHeaders, "Content-Type": "application/json" },
      });
    }

    // Write code endpoint
    if (path === "/write-code" && method === "POST") {
      try {
        const body = await req.json() as { code?: string; handler?: string };

        if (!body.code) {
          return Response.json(
            { error: "Missing code field" },
            { status: 400, headers: corsHeaders }
          );
        }

        const handlerName = body.handler || FUNCTION_HANDLER;

        // Write TypeScript code with export
        // Support both named export and default export
        let codeContent = body.code;
        
        // Check if code already has exports
        if (!codeContent.includes("export ")) {
          // Add named export
          codeContent = `${codeContent}\n\nexport { ${handlerName} };`;
        }

        await Bun.write(FUNCTION_CODE_PATH, codeContent);

        // Reload the function
        const success = await loadFunction();

        return Response.json(
          { success, error: loadError },
          { status: success ? 200 : 500, headers: corsHeaders }
        );
      } catch (e) {
        return Response.json(
          { error: e instanceof Error ? e.message : String(e) },
          { status: 500, headers: corsHeaders }
        );
      }
    }

    // Reload endpoint
    if (path === "/reload" && method === "POST") {
      const success = await loadFunction();
      return Response.json(
        { success, error: loadError },
        { status: success ? 200 : 500, headers: corsHeaders }
      );
    }

    // Invoke endpoint
    if (path === "/invoke" && method === "POST") {
      if (!handler) {
        return Response.json(
          {
            status: "error",
            error: loadError || "Function not loaded",
            duration_ms: 0,
            logs: [],
          } as InvokeResponse,
          { status: 500, headers: corsHeaders }
        );
      }

      const startTime = performance.now();
      const logs: string[] = [];
      const requestId = generateRequestId();

      // Capture console output
      const originalLog = console.log;
      const originalError = console.error;
      const originalWarn = console.warn;
      const originalInfo = console.info;
      const originalDebug = console.debug;

      const captureLog = (level: string) => (...args: unknown[]) => {
        const msg = args.map((a) => (typeof a === "object" ? JSON.stringify(a) : String(a))).join(" ");
        logs.push(`[${level}] ${msg}`);
        originalLog(`[Function:${level}]`, ...args);
      };

      console.log = captureLog("LOG");
      console.error = captureLog("ERROR");
      console.warn = captureLog("WARN");
      console.info = captureLog("INFO");
      console.debug = captureLog("DEBUG");

      try {
        // Parse event from request
        let event: unknown = {};
        const contentType = req.headers.get("content-type") || "";
        
        if (contentType.includes("application/json")) {
          const body = await req.json() as { event?: unknown };
          event = body.event ?? body;
        }

        console.info(`Invoking with requestId: ${requestId}`);

        // Create execution context
        const context = createContext(requestId);

        // Invoke the handler with timeout
        const timeoutMs = context.timeoutMs;
        const result = await Promise.race([
          Promise.resolve(handler(event, context)),
          new Promise((_, reject) =>
            setTimeout(() => reject(new Error(`Function timed out after ${timeoutMs}ms`)), timeoutMs)
          ),
        ]);

        // Restore console
        console.log = originalLog;
        console.error = originalError;
        console.warn = originalWarn;
        console.info = originalInfo;
        console.debug = originalDebug;

        const duration = performance.now() - startTime;

        const response: InvokeResponse = {
          status: "success",
          duration_ms: Math.round(duration * 100) / 100,
          response: result,
          logs,
        };

        console.log(`[Runtime] Function completed in ${duration.toFixed(2)}ms`);

        return Response.json(response, { headers: corsHeaders });
      } catch (error) {
        // Restore console
        console.log = originalLog;
        console.error = originalError;
        console.warn = originalWarn;
        console.info = originalInfo;
        console.debug = originalDebug;

        const duration = performance.now() - startTime;
        const errorMessage = error instanceof Error ? error.message : String(error);
        const errorStack = error instanceof Error ? error.stack : undefined;

        console.error(`[Runtime] Function failed: ${errorMessage}`);

        const response: InvokeResponse = {
          status: "error",
          duration_ms: Math.round(duration * 100) / 100,
          error: errorMessage,
          stack: errorStack,
          logs,
        };

        return Response.json(response, { status: 500, headers: corsHeaders });
      }
    }

    // 404 for all other routes
    return Response.json(
      {
        error: "Not found",
        endpoints: {
          "GET /health": "Health check",
          "POST /invoke": "Execute function",
          "POST /write-code": "Write and reload function code",
          "POST /reload": "Reload function code",
        },
      },
      { status: 404, headers: corsHeaders }
    );
  },

  error(error: Error) {
    console.error(`[Runtime] Server error: ${error.message}`);
    return Response.json(
      { error: "Internal server error", message: error.message },
      { status: 500 }
    );
  },
});

console.log(`[Runtime] Bun/TypeScript function runtime v${Bun.version} listening on port ${PORT}`);
console.log(`[Runtime] Function handler: ${FUNCTION_HANDLER}`);
console.log(`[Runtime] Code path: ${FUNCTION_CODE_PATH}`);

// Try to load function code on startup
await loadFunction();

// Graceful shutdown
process.on("SIGTERM", () => {
  console.log("[Runtime] Received SIGTERM, shutting down gracefully...");
  server.stop();
  process.exit(0);
});

process.on("SIGINT", () => {
  console.log("[Runtime] Received SIGINT, shutting down gracefully...");
  server.stop();
  process.exit(0);
});
