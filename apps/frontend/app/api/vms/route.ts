import { NextRequest, NextResponse } from "next/server"

// Mock VM data for testing
const mockVMs = [
  {
    id: "vm-1",
    name: "Web Server",
    description: "Production web server instance",
    state: "running" as const,
    owner: "admin",
    environment: "production" as const,
    tags: { role: "web", tier: "production" },
    config: {
      machine: {
        vcpu_count: 2,
        mem_size_mib: 1024,
      },
    },
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  },
  {
    id: "vm-2", 
    name: "Database Server",
    description: "PostgreSQL database instance",
    state: "paused" as const,
    owner: "admin",
    environment: "staging" as const,
    tags: { role: "database", tier: "staging" },
    config: {
      machine: {
        vcpu_count: 4,
        mem_size_mib: 2048,
      },
    },
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  },
  {
    id: "vm-3",
    name: "Dev Environment",
    description: "Development sandbox",
    state: "stopped" as const,
    owner: "developer",
    environment: "development" as const,
    tags: { role: "dev", tier: "development" },
    config: {
      machine: {
        vcpu_count: 1,
        mem_size_mib: 512,
      },
    },
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  },
]

export async function GET(request: NextRequest) {
  try {
    // Simulate network delay
    await new Promise(resolve => setTimeout(resolve, 500))
    
    return NextResponse.json({
      success: true,
      data: mockVMs,
      count: mockVMs.length,
    })
  } catch (error) {
    return NextResponse.json(
      { success: false, error: "Failed to fetch VMs" },
      { status: 500 }
    )
  }
}