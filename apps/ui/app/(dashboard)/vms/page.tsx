"use client"

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { useVMs } from "@/lib/queries"
import { VMTable } from "@/components/vm/vm-table"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Plus, Search } from "lucide-react"
import Link from "next/link"
import { useState } from "react"
import { useAuthStore, canCreateResource } from "@/lib/auth/store"

const VMFlowDiagram = () => (
  <svg width="300" height="200" viewBox="0 0 300 200" fill="none" xmlns="http://www.w3.org/2000/svg" className="drop-shadow-lg">
    <style>{`
      .vm-text { fill: #92400e; }
      .dark .vm-text { fill: #f59e0b; }
      .vm-bg { fill: #fef3c7; }
      .dark .vm-bg { fill: rgba(146, 64, 14, 0.3); }
    `}</style>
    {/* Host Machine */}
    <rect x="10" y="120" width="280" height="70" rx="8" className="vm-bg" stroke="#f59e0b" strokeWidth="2" />
    <text x="150" y="150" textAnchor="middle" className="vm-text" fontWeight="600" fontSize="14">Host Machine (KVM)</text>
    <text x="150" y="165" textAnchor="middle" className="vm-text" fontSize="11">Firecracker Hypervisor</text>

    {/* VM 1 */}
    <rect x="20" y="30" width="70" height="70" rx="6" className="vm-bg" stroke="#f59e0b" strokeWidth="2" />
    <circle cx="55" cy="55" r="15" fill="#fbbf24" opacity="0.3" />
    <text x="55" y="58" textAnchor="middle" className="vm-text" fontWeight="600" fontSize="12">VM 1</text>
    <text x="55" y="85" textAnchor="middle" className="vm-text" fontSize="9">Guest OS</text>

    {/* VM 2 */}
    <rect x="115" y="30" width="70" height="70" rx="6" className="vm-bg" stroke="#f59e0b" strokeWidth="2" />
    <circle cx="150" cy="55" r="15" fill="#fbbf24" opacity="0.3" />
    <text x="150" y="58" textAnchor="middle" className="vm-text" fontWeight="600" fontSize="12">VM 2</text>
    <text x="150" y="85" textAnchor="middle" className="vm-text" fontSize="9">Container</text>

    {/* VM 3 */}
    <rect x="210" y="30" width="70" height="70" rx="6" className="vm-bg" stroke="#f59e0b" strokeWidth="2" />
    <circle cx="245" cy="55" r="15" fill="#fbbf24" opacity="0.3" />
    <text x="245" y="58" textAnchor="middle" className="vm-text" fontWeight="600" fontSize="12">VM 3</text>
    <text x="245" y="85" textAnchor="middle" className="vm-text" fontSize="9">Function</text>

    {/* Connection lines */}
    <line x1="55" y1="100" x2="55" y2="120" stroke="#f59e0b" strokeWidth="2" strokeDasharray="4" />
    <line x1="150" y1="100" x2="150" y2="120" stroke="#f59e0b" strokeWidth="2" strokeDasharray="4" />
    <line x1="245" y1="100" x2="245" y2="120" stroke="#f59e0b" strokeWidth="2" strokeDasharray="4" />

    {/* Arrows */}
    <polygon points="55,120 52,115 58,115" fill="#f59e0b" />
    <polygon points="150,120 147,115 153,115" fill="#f59e0b" />
    <polygon points="245,120 242,115 248,115" fill="#f59e0b" />
  </svg>
)

export default function VMsPage() {
  const { data: vms, isLoading, error } = useVMs()
  const [searchTerm, setSearchTerm] = useState("")
  const { user } = useAuthStore()

  const filteredVMs = vms?.filter(vm =>
    vm.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
    vm.id.toLowerCase().includes(searchTerm.toLowerCase())
  ) || []

  if (isLoading) {
    return (
      <div className="container mx-auto py-6">
        <div className="animate-pulse space-y-4">
          <div className="h-8 bg-muted rounded w-1/4"></div>
          <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
            {[...Array(6)].map((_, i) => (
              <div key={i} className="h-48 bg-muted rounded-lg"></div>
            ))}
          </div>
        </div>
      </div>
    )
  }

  if (error) {
    return (
      <div className="container mx-auto py-6">
        <div className="text-center space-y-4">
          <h1 className="text-2xl font-bold text-destructive">Failed to load VMs</h1>
          <p className="text-muted-foreground">
            Unable to fetch VM list. Please check your connection and try again.
          </p>
        </div>
      </div>
    )
  }

  return (
    <div className="space-y-6">
      <div className="relative overflow-hidden rounded-xl border border-border bg-gradient-to-br from-yellow-50 to-yellow-100/50 dark:from-yellow-950/30 dark:to-yellow-900/20 p-8">
        <div className="relative z-10 flex items-center justify-between">
          <div className="max-w-xl">
            <h1 className="text-3xl font-bold text-foreground dark:text-primary">Virtual Machines</h1>
            <p className="mt-2 text-foreground font-medium">
              Manage your microVMs
            </p>
            {canCreateResource(user) && (
              <Button asChild className="mt-4">
                <Link href="/vms/create">
                  <Plus className="mr-2 h-4 w-4" />
                  Create VM
                </Link>
              </Button>
            )}
          </div>
          <div className="hidden lg:block">
            <VMFlowDiagram />
          </div>
        </div>
        <div className="absolute right-0 top-0 h-64 w-64 translate-x-32 -translate-y-32 rounded-full bg-gradient-to-br from-yellow-400/30 to-yellow-600/30 dark:from-yellow-500/20 dark:to-yellow-600/10 blur-3xl" />
      </div>

      <Card>
        <CardHeader>
          <CardTitle>All Virtual Machine</CardTitle>
        </CardHeader>
        <CardContent>
          {filteredVMs.length === 0 ? (
            <div className="text-center py-12">
              <h3 className="text-lg font-medium">No VMs found</h3>
              <p className="text-muted-foreground mt-2">
                {searchTerm
                  ? "No VMs match your search criteria."
                  : "Get started by creating your first VM."
                }
              </p>
              {!searchTerm && canCreateResource(user) && (
                <Button asChild className="mt-4">
                  <Link href="/vms/create">
                    <Plus className="mr-2 h-4 w-4" />
                    Create your first VM
                  </Link>
                </Button>
              )}
            </div>
          ) : (
            <VMTable vms={filteredVMs} />
          )}
        </CardContent>
      </Card>

    </div>
  )
}
