// app/api/invoke/route.ts
import { NextRequest, NextResponse } from "next/server";
import vm from "node:vm";

export const runtime = "nodejs"; // WAJIB: jangan Edge

type LambdaResponse = {
  statusCode: number;
  headers?: Record<string, string>;
  body?: string;
};

// -- ESM -> CJS transform sederhana untuk kasus umum
function transformToCJS(src: string): string {
  let code = src;

  // 1) export default async function handler(...) { ... }
  //    -> async function handler(...) { ... }; exports.__default = handler;
  code = code.replace(
    /export\s+default\s+async\s+function\s+([A-Za-z_$][0-9A-Za-z_$]*)\s*\(/g,
    (_m, name) => `async function ${name}(`,
  ) + `\n;exports.__default = (typeof handler !== "undefined" ? handler : exports.__default);\n`;

  // 2) export default (async function|function|class|const|let|var) ...
  //    -> simpan ke exports.__default
  if (/export\s+default\s+/.test(code)) {
    code = code.replace(
      /export\s+default\s+/g,
      "exports.__default = ",
    );
  }

  // 3) export const handler = ...
  //    export let/var handler = ...
  //    -> const/let/var handler = ...
  code = code.replace(
    /^\s*export\s+(const|let|var)\s+/gm,
    "$1 ",
  );

  // 4) export async function handler(...) / export function handler(...)
  //    -> async function handler(...)/function handler(...)
  code = code.replace(
    /^\s*export\s+(async\s+)?function\s+/gm,
    "$1function ",
  );

  // 5) export class Foo -> class Foo
  code = code.replace(
    /^\s*export\s+class\s+/gm,
    "class ",
  );

  // 6) Named export (mis. "export { handler }" atau "export { a as handler }")
  //    Hapus saja; kita akhiri dengan module.exports.handler di bawah.
  code = code.replace(/^\s*export\s*\{[\s\S]*?\}\s*;?\s*$/gm, "");

  // 7) Akhiri dengan mengekspor handler:
  //    - jika ada variabel/fungsi bernama handler, ambil itu
  //    - jika tidak ada handler tapi ada __default (dari export default handler), gunakan itu
  code += `
;module.exports = {
  handler: (typeof handler !== "undefined"
              ? handler
              : (typeof exports !== "undefined" && exports.__default ? exports.__default : undefined))
};
`;
  return code;
}

function normalizeResult(result: any): LambdaResponse {
  if (result && typeof result === "object" && "statusCode" in result) {
    return result as LambdaResponse;
  }
  return {
    statusCode: 200,
    headers: { "content-type": "application/json" },
    body: JSON.stringify(result ?? null),
  };
}

export async function POST(req: NextRequest) {
  try {
    const { code, event } = (await req.json()) as { code: string; event: any };

    if (!code || typeof code !== "string") {
      return NextResponse.json({ ok: false, error: "Field `code` wajib string." }, { status: 400 });
    }

    const transformed = transformToCJS(code);

    const logs: string[] = [];
    const safeConsole = {
      log: (...a: any[]) => logs.push(a.map(String).join(" ")),
      error: (...a: any[]) => logs.push("[error] " + a.map(String).join(" ")),
      warn: (...a: any[]) => logs.push("[warn] " + a.map(String).join(" ")),
      info: (...a: any[]) => logs.push("[info] " + a.map(String).join(" ")),
    };

    const sandbox = {
      module: { exports: {} as any },
      exports: {} as any,
      console: safeConsole,
      require: undefined,
      process: undefined,
      Buffer: undefined,
      __dirname: undefined,
      __filename: undefined,
      globalThis: {} as any,
    };

    const context = vm.createContext(sandbox, { name: "lambda-sandbox" });
    const script = new vm.Script(transformed, {
      filename: "index.mjs",
      displayErrors: true,
      timeout: 1000,
    });

    script.runInContext(context, { timeout: 1000 });

    const handler =
      (sandbox.module?.exports as any)?.handler ??
      (sandbox.exports as any)?.handler;

    if (typeof handler !== "function") {
      return NextResponse.json(
        {
          ok: false,
          error:
            "Handler tidak ditemukan. Pastikan ada `export const handler = ...` atau `export default async function handler(...) {}`.",
          logs,
        },
        { status: 400 },
      );
    }

    const result = await Promise.race([
      Promise.resolve(handler(event)),
      new Promise((_r, rej) => setTimeout(() => rej(new Error("Timeout")), 2000)),
    ]);

    const response = normalizeResult(result);
    return NextResponse.json({ ok: true, response, logs }, { status: 200 });
  } catch (err: any) {
    return NextResponse.json(
      { ok: false, error: err?.message || "Invocation error" },
      { status: 500 },
    );
  }
}
