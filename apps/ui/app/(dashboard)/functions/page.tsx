"use client"

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Plus, RotateCw, Loader2, CodeXml } from "lucide-react"
import Link from "next/link"
import { FunctionTable } from "@/components/function/function-table"
import { useFunctions } from "@/lib/queries"
import { useState } from "react"

const FunctionFlowDiagram = () => (
  <svg width="300" height="220" viewBox="0 0 300 220" fill="none" xmlns="http://www.w3.org/2000/svg" className="drop-shadow-lg">
    <style>{`
      .function-text { fill: #92400e; }
      .dark .function-text { fill: #f59e0b; }
      .function-bg { fill: #fef3c7; }
      .dark .function-bg { fill: rgba(146, 64, 14, 0.3); }
    `}</style>
    {/* HTTP Request indicator */}
    <text x="150" y="15" textAnchor="middle" className="function-text" fontSize="11" fontWeight="500">HTTP Request → Isolated Execution</text>

    {/* VM 1 with Function - Node.js */}
    <rect x="20" y="30" width="70" height="90" rx="6" className="function-bg" stroke="#f59e0b" strokeWidth="2" />
    <text x="55" y="45" textAnchor="middle" className="function-text" fontWeight="600" fontSize="10">VM 1</text>
    <rect x="28" y="52" width="54" height="60" rx="4" fill="#fbbf24" fillOpacity="0.15" stroke="#f59e0b" strokeWidth="1.5" strokeDasharray="2" />
    <text x="55" y="66" textAnchor="middle" className="function-text" fontWeight="600" fontSize="11">Fn 1</text>
    <text x="55" y="82" textAnchor="middle" className="function-text" fontSize="9">Node.js</text>
    <circle cx="55" cy="98" r="7" fill="#fbbf24" opacity="0.5" />
    <text x="55" y="101.5" textAnchor="middle" className="function-text" fontSize="7" fontWeight="600">JS</text>

    {/* VM 2 with Function - Python */}
    <rect x="115" y="30" width="70" height="90" rx="6" className="function-bg" stroke="#f59e0b" strokeWidth="2" />
    <text x="150" y="45" textAnchor="middle" className="function-text" fontWeight="600" fontSize="10">VM 2</text>
    <rect x="123" y="52" width="54" height="60" rx="4" fill="#fbbf24" fillOpacity="0.15" stroke="#f59e0b" strokeWidth="1.5" strokeDasharray="2" />
    <text x="150" y="66" textAnchor="middle" className="function-text" fontWeight="600" fontSize="11">Fn 2</text>
    <text x="150" y="82" textAnchor="middle" className="function-text" fontSize="9">Python</text>
    <circle cx="150" cy="98" r="7" fill="#fbbf24" opacity="0.5" />
    <text x="150" y="101.5" textAnchor="middle" className="function-text" fontSize="7" fontWeight="600">PY</text>

    {/* VM 3 with Function - Rust */}
    <rect x="210" y="30" width="70" height="90" rx="6" className="function-bg" stroke="#f59e0b" strokeWidth="2" />
    <text x="245" y="45" textAnchor="middle" className="function-text" fontWeight="600" fontSize="10">VM 3</text>
    <rect x="218" y="52" width="54" height="60" rx="4" fill="#fbbf24" fillOpacity="0.15" stroke="#f59e0b" strokeWidth="1.5" strokeDasharray="2" />
    <text x="245" y="66" textAnchor="middle" className="function-text" fontWeight="600" fontSize="11">Fn 3</text>
    <text x="245" y="82" textAnchor="middle" className="function-text" fontSize="9">Rust</text>
    <circle cx="245" cy="98" r="7" fill="#fbbf24" opacity="0.5" />
    <text x="245" y="101.5" textAnchor="middle" className="function-text" fontSize="7" fontWeight="600">RS</text>

    {/* Connection lines from VMs to Host */}
    <line x1="55" y1="120" x2="55" y2="145" stroke="#f59e0b" strokeWidth="2" strokeDasharray="4" />
    <line x1="150" y1="120" x2="150" y2="145" stroke="#f59e0b" strokeWidth="2" strokeDasharray="4" />
    <line x1="245" y1="120" x2="245" y2="145" stroke="#f59e0b" strokeWidth="2" strokeDasharray="4" />

    {/* Arrows */}
    <polygon points="55,145 52,140 58,140" fill="#f59e0b" />
    <polygon points="150,145 147,140 153,140" fill="#f59e0b" />
    <polygon points="245,145 242,140 248,140" fill="#f59e0b" />

    {/* Host Machine (KVM) */}
    <rect x="10" y="145" width="280" height="60" rx="8" className="function-bg" stroke="#f59e0b" strokeWidth="2" />
    <text x="150" y="170" textAnchor="middle" className="function-text" fontWeight="600" fontSize="13">Host Machine (KVM)</text>
    <text x="150" y="188" textAnchor="middle" className="function-text" fontSize="10">Firecracker Hypervisor</text>
  </svg>
)

export default function FunctionsPage() {
  const { data: functions, isLoading, error, refetch, isFetching } = useFunctions() // useFunctions()
  const [searchTerm, setSearchTerm] = useState("")

  const filteredFunctions = functions?.filter(fn =>
    fn.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
    fn?.id.toLowerCase().includes(searchTerm.toLowerCase())
  ) || []

  if (isLoading) {
    return (
      <div className="container mx-auto py-6">
        <div className="animate-pulse space-y-4">
          <div className="h-8 bg-muted rounded w-1/4" />
          <div className="grid gap-4">
            {[...Array(6)].map((_, i) => <div key={i} className="h-24 bg-muted rounded-lg" />)}
          </div>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="container mx-auto py-6 text-center space-y-4">
        <h1 className="text-2xl font-bold text-destructive">Failed to load Functions</h1>
        <p className="text-muted-foreground">Unable to fetch function list. Please check your connection and try again.</p>
        <Button variant="outline" onClick={() => location.reload()}>Try again</Button>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="relative overflow-hidden rounded-xl border border-border bg-gradient-to-br from-yellow-50 to-yellow-100/50 dark:from-yellow-950/30 dark:to-yellow-900/20 p-8">
        <div className="relative z-10 flex items-center justify-between">
          <div className="max-w-xl">
            <h1 className="text-3xl font-bold text-foreground">Serverless Functions</h1>
            <p className="mt-2 text-muted-foreground">
              Deploy and manage Lambda-like functions with automatic scaling and pay-per-use pricing
            </p>
            <div className="flex gap-4">
              <Button asChild className="mt-4">
                <Link href="/functions/new">
                  <Plus className="mr-2 h-4 w-4" />
                  New Function
                </Link>
              </Button>
              <Button asChild className="mt-4" variant="outline">
                <Link href="/functions/playground">
                  <CodeXml className="mr-2 h-4 w-5" />
                  Playground
                </Link>
              </Button>
            </div>
          </div>
          <div className="hidden lg:block">
            <FunctionFlowDiagram />
          </div>
        </div>
        <div className="absolute right-0 top-0 h-64 w-64 translate-x-32 -translate-y-32 rounded-full bg-gradient-to-br from-yellow-400/30 to-yellow-600/30 dark:from-yellow-500/20 dark:to-yellow-600/10 blur-3xl" />
      </div>

      <Card>
        <CardHeader className="flex items-center justify-between">
          <CardTitle>All Functions</CardTitle>
          <div className="flex items-center gap-2">
            {/* (Opsional) Search di header */}
            <Button
              variant="outline"
              onClick={() => refetch()}
              disabled={isFetching}
              title="Refresh function list"
            >
              {isFetching ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  Refreshing...
                </>
              ) : (
                <>
                  <RotateCw className="mr-2 h-4 w-4" />
                  Refresh
                </>
              )}
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          {filteredFunctions.length === 0 ? (
            <div className="text-center py-12">
              <h3 className="text-lg font-medium">No Functions found</h3>
              <p className="text-muted-foreground mt-2">
                {searchTerm
                  ? "No Functions match your search criteria."
                  : "Get started by creating your first Function."
                }
              </p>
              {!searchTerm && (
                <Button asChild className="mt-4">
                  <Link href="/functions/new">
                    <Plus className="mr-2 h-4 w-4" />
                    Create your first Function
                  </Link>
                </Button>
              )}
            </div>
          ) : (
            <FunctionTable functions={filteredFunctions} />
          )}
        </CardContent>
      </Card>
    </div>
  )
}
