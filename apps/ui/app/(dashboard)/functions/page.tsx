import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Plus } from "lucide-react"
import Link from "next/link"
import { FunctionTable } from "@/components/function/function-table"
import Image from "next/image"

// Mock data
const mockFunctions = [
  {
    id: "fn-1",
    name: "image-processor",
    runtime: "node" as const,
    code: "// code here",
    handler: "index.handler",
    timeout_seconds: 30,
    memory_mb: 512,
    created_at: new Date(Date.now() - 86400000 * 7).toISOString(),
    updated_at: new Date(Date.now() - 3600000).toISOString(),
    last_invoked_at: new Date(Date.now() - 3600000).toISOString(),
    invocation_count_24h: 1247,
    avg_duration_ms: 234,
  },
  {
    id: "fn-2",
    name: "email-sender",
    runtime: "python" as const,
    code: "# code here",
    handler: "main.handler",
    timeout_seconds: 60,
    memory_mb: 256,
    created_at: new Date(Date.now() - 86400000 * 3).toISOString(),
    updated_at: new Date(Date.now() - 7200000).toISOString(),
    last_invoked_at: new Date(Date.now() - 300000).toISOString(),
    invocation_count_24h: 523,
    avg_duration_ms: 145,
  },
  {
    id: "fn-3",
    name: "data-transformer",
    runtime: "go" as const,
    code: "// code here",
    handler: "main",
    timeout_seconds: 15,
    memory_mb: 1024,
    created_at: new Date(Date.now() - 86400000 * 14).toISOString(),
    updated_at: new Date(Date.now() - 86400000).toISOString(),
    invocation_count_24h: 89,
    avg_duration_ms: 67,
  },
]

export default function FunctionsPage() {
  return (
    <div className="space-y-6">
      <div className="relative overflow-hidden rounded-xl border border-border bg-gradient-to-br from-yellow-50 to-yellow-100/50 p-8">
        <div className="relative z-10 flex items-center justify-between">
          <div className="max-w-xl">
            <h1 className="text-3xl font-bold text-foreground">Serverless Functions</h1>
            <p className="mt-2 text-muted-foreground">
              Deploy and manage Lambda-like functions with automatic scaling and pay-per-use pricing
            </p>
            <Button asChild className="mt-4">
              <Link href="/functions/new">
                <Plus className="mr-2 h-4 w-4" />
                New Function
              </Link>
            </Button>
          </div>
          <div className="hidden lg:block">
            <Image
              src="/serverless-functions-code-lightning-fast-illustrat.jpg"
              alt="Serverless Functions"
              width={300}
              height={200}
              className="rounded-lg"
            />
          </div>
        </div>
        <div className="absolute right-0 top-0 h-64 w-64 translate-x-32 -translate-y-32 rounded-full bg-gradient-to-br from-yellow-400/30 to-yellow-600/30 blur-3xl" />
      </div>

      <Card>
        <CardHeader>
          <CardTitle>All Functions</CardTitle>
        </CardHeader>
        <CardContent>
          <FunctionTable functions={mockFunctions} />
        </CardContent>
      </Card>
    </div>
  )
}
