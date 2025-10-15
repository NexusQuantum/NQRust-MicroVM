// app/api/invoke/route.ts
import { NextRequest, NextResponse } from "next/server";
import vm from "node:vm";

export const runtime = "nodejs"; // penting: gunakan Node runtime, bukan edge

type LambdaResponse = {
  statusCode: number;
  headers?: Record<string, string>;
  body?: string;
};

function normalizeResult(result: any): LambdaResponse {
  // Jika handler sudah mengembalikan objek LambdaResponse, kirim apa adanya
  if (result && typeof result === "object" && "statusCode" in result) {
    return result as LambdaResponse;
  }
  // Jika bukan, bungkus sebagai 200 JSON
  return {
    statusCode: 200,
    headers: { "content-type": "application/json" },
    body: JSON.stringify(result ?? null),
  };
}

export async function POST(req: NextRequest) {
  try {
    const { code, event } = (await req.json()) as { code: string; event: any };

    if (typeof code !== "string" || !code.trim()) {
      return NextResponse.json(
        { ok: false, error: "Code kosong. Kirim source code di field `code`." },
        { status: 400 }
      );
    }

    // Transform sederhana: ambil handler dari ESM ke CommonJS export
    // Cukup untuk pola: `export const handler = ...`
    const transformed = `${code}
;module.exports = { handler: (typeof handler !== "undefined" ? handler : (typeof exports !== "undefined" ? exports.handler : undefined)) };`;

    // (Opsional) kumpulkan console.log agar bisa ditampilkan ke UI
    const logs: string[] = [];
    const safeConsole = {
      log: (...args: any[]) => logs.push(args.map(String).join(" ")),
      error: (...args: any[]) => logs.push("[error] " + args.map(String).join(" ")),
      warn: (...args: any[]) => logs.push("[warn] " + args.map(String).join(" ")),
      info: (...args: any[]) => logs.push("[info] " + args.map(String).join(" ")),
    };

    // Sandbox minim: tanpa akses require/process/dll
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
      timeout: 1000, // cegah loop tak hingga saat compile
    });

    // Jalankan script untuk mengisi module.exports.handler
    script.runInContext(context, { timeout: 1000 });

    const handler =
      (sandbox.module?.exports as any)?.handler ??
      (sandbox.exports as any)?.handler;

    if (typeof handler !== "function") {
      return NextResponse.json(
        {
          ok: false,
          error:
            "Handler tidak ditemukan. Pastikan ada `export const handler = async (event) => { ... }`.",
          logs,
        },
        { status: 400 }
      );
    }

    // Panggil handler dengan timeout eksekusi
    const result = await Promise.race([
      Promise.resolve(handler(event)),
      new Promise((_res, rej) => setTimeout(() => rej(new Error("Timeout")), 1500)),
    ]);

    const response = normalizeResult(result);

    return NextResponse.json(
      { ok: true, response, logs },
      { status: 200 }
    );
  } catch (err: any) {
    return NextResponse.json(
      {
        ok: false,
        error: err?.message || "Invocation error",
      },
      { status: 500 }
    );
  }
}
