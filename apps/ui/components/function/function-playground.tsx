import React, { useState, useRef, useMemo, useEffect } from "react"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Slider } from "@/components/ui/slider"
import { Save, Play, Loader2, Import } from "lucide-react"
import dynamic from "next/dynamic"
import { z } from "zod"
import { useForm, Controller } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import type { CreateFunction, UpdateFunction, Function as FnType } from "@/lib/types"
import { useCreateFunction, useUpdateFunction, useInvokeFunction } from "@/lib/queries"

const Editor = dynamic(() => import("@monaco-editor/react"), { ssr: false })

interface FunctionEditorProps {
  onComplete?: (payload: { id?: string, name?: string }) => void
  onCancel?: () => void
  functionData?: FnType
  mode?: "create" | "update"
  functionId?: string
}

const fnCreationSchema = z.object({
  runtime: z.enum(['node', 'python']),
})

type FnCreationForm = z.infer<typeof fnCreationSchema>
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

const DEFAULT_PAYLOAD = `{
  "key1": 10,
  "key2": 5
}`

function normalizeToConstHandlerForBackend(
  runtime: 'node' | 'python',
  rawCode: string,
  handlerName: string
) {
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

function normalizeToModuleExportsForRunTest(
  runtime: 'node' | 'python',
  rawCode: string,
  handlerName: string
) {
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

export function FunctionPlayground({ functionId, mode = 'create', functionData, onComplete = () => { } }: FunctionEditorProps) {
  const [testEvent, setTestEvent] = useState(DEFAULT_PAYLOAD)
  const editorRef = useRef<any>(null)
}