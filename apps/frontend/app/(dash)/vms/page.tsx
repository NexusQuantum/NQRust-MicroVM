"use client"

import { useVMs } from "@/lib/queries"
import { VMCard } from "@/components/vm-card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Plus, Search } from "lucide-react"
import Link from "next/link"
import { useState } from "react"

export default function VMsPage() {
  const { data: vms, isLoading, error } = useVMs()
  const [searchTerm, setSearchTerm] = useState("")

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
    <div className="container mx-auto py-6">
      <div className="space-y-6">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-3xl font-bold">Virtual Machines</h1>
            <p className="text-muted-foreground">
              Manage your Firecracker microVMs
            </p>
          </div>
          <Button asChild>
            <Link href="/vms/create">
              <Plus className="mr-2 h-4 w-4" />
              Create VM
            </Link>
          </Button>
        </div>

        <div className="flex items-center space-x-2">
          <div className="relative flex-1 max-w-sm">
            <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
            <Input
              placeholder="Search VMs..."
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
              className="pl-10"
            />
          </div>
        </div>

        {filteredVMs.length === 0 ? (
          <div className="text-center py-12">
            <h3 className="text-lg font-medium">No VMs found</h3>
            <p className="text-muted-foreground mt-2">
              {searchTerm 
                ? "No VMs match your search criteria."
                : "Get started by creating your first VM."
              }
            </p>
            {!searchTerm && (
              <Button asChild className="mt-4">
                <Link href="/vms/create">
                  <Plus className="mr-2 h-4 w-4" />
                  Create your first VM
                </Link>
              </Button>
            )}
          </div>
        ) : (
          <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
            {filteredVMs.map((vm) => (
              <VMCard key={vm.id} vm={vm} />
            ))}
          </div>
        )}
      </div>
    </div>
  )
}