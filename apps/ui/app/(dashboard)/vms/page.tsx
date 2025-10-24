"use client"

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { useVMs } from "@/lib/queries"
import { VMTable } from "@/components/vm/vm-table"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Plus, Search } from "lucide-react"
import Link from "next/link"
import { useState } from "react"
import Image from "next/image"

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
    <div className="space-y-6">
      <div className="relative overflow-hidden rounded-xl border border-border bg-gradient-to-br from-yellow-50 to-yellow-100/50 p-8">
        <div className="relative z-10 flex items-center justify-between">
          <div className="max-w-xl">
            <h1 className="text-3xl font-bold text-foreground">Virtual Machines</h1>
            <p className="mt-2 text-muted-foreground">
              Manage your microVMs
            </p>
            <Button asChild className="mt-4">
              <Link href="/vms/create">
                <Plus className="mr-2 h-4 w-4" />
                Create VM
              </Link>
            </Button>
          </div>
          <div className="hidden lg:block">
            <Image
              src="/virtual-machine-server-infrastructure-illustration.jpg"
              alt="Virtual Machines"
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
            <VMTable vms={filteredVMs} />
          )}
        </CardContent>
      </Card>

    </div>
  )
}
