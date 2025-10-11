import { http, HttpResponse } from "msw"
import type { VM, RegistryImage } from "@/types/firecracker"

// Mock data
const mockVMs: VM[] = [
  {
    id: "vm-1",
    name: "web-server-01",
    description: "Production web server",
    created_at: "2024-01-15T10:30:00Z",
    updated_at: "2024-01-15T14:22:00Z",
    config: {
      machine: {
        vcpu_count: 2,
        mem_size_mib: 1024,
        smt: false,
        cpu_template: "T2",
      },
      boot: {
        kernel_image_path: "/images/vmlinux-5.10",
        boot_args: "console=ttyS0 reboot=k panic=1 pci=off",
      },
      metadata: {
        name: "web-server-01",
        description: "Production web server",
        tags: { env: "production", team: "platform" },
      },
    },
    state: "running",
    firecracker_pid: 12345,
    socket_path: "/tmp/firecracker-vm-1.sock",
    tags: { env: "production", team: "platform" },
    owner: "admin",
    environment: "production",
  },
  {
    id: "vm-2",
    name: "test-runner",
    description: "CI/CD test environment",
    created_at: "2024-01-15T09:15:00Z",
    updated_at: "2024-01-15T09:15:00Z",
    config: {
      machine: {
        vcpu_count: 1,
        mem_size_mib: 512,
        smt: false,
        cpu_template: "T2",
      },
      boot: {
        kernel_image_path: "/images/vmlinux-5.10",
        boot_args: "console=ttyS0 reboot=k panic=1 pci=off",
      },
      metadata: {
        name: "test-runner",
        description: "CI/CD test environment",
        tags: { env: "staging", team: "devops" },
      },
    },
    state: "stopped",
    socket_path: "/tmp/firecracker-vm-2.sock",
    tags: { env: "staging", team: "devops" },
    owner: "devops",
    environment: "staging",
  },
]

const mockImages: RegistryImage[] = [
  {
    id: "img-1",
    name: "Ubuntu 22.04 LTS",
    path: "/images/ubuntu-22.04-rootfs.ext4",
    type: "rootfs",
    size_bytes: 2147483648,
    created_at: "2024-01-10T00:00:00Z",
    tags: ["ubuntu", "lts", "22.04"],
  },
  {
    id: "img-2",
    name: "Linux Kernel 5.10",
    path: "/images/vmlinux-5.10",
    type: "kernel",
    size_bytes: 8388608,
    created_at: "2024-01-10T00:00:00Z",
    tags: ["kernel", "5.10"],
  },
]

export const handlers = [
  // VM Management
  http.get("/api/vms", () => {
    return HttpResponse.json(mockVMs)
  }),

  http.get("/api/vms/:id", ({ params }) => {
    const vm = mockVMs.find((v) => v.id === params.id)
    if (!vm) {
      return HttpResponse.json({ error: "VM not found", status: 404, request_id: "mock-req-1" }, { status: 404 })
    }
    return HttpResponse.json(vm)
  }),

  http.post("/api/vms", async ({ request }) => {
    const config = await request.json()
    const newVM: VM = {
      id: `vm-${Date.now()}`,
      name: config.metadata.name,
      description: config.metadata.description,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
      config,
      state: "stopped",
      socket_path: `/tmp/firecracker-${Date.now()}.sock`,
      tags: config.metadata.tags || {},
      owner: "current-user",
      environment: "development",
    }
    mockVMs.push(newVM)
    return HttpResponse.json(newVM, { status: 201 })
  }),

  // Registry
  http.get("/api/registry/images", () => {
    return HttpResponse.json(mockImages)
  }),

  // Firecracker API passthrough
  http.get("/api/firecracker/status", () => {
    return HttpResponse.json({ state: "Running" })
  }),

  http.put("/api/firecracker/machine-config", () => {
    return HttpResponse.json({ success: true })
  }),

  http.put("/api/firecracker/boot-source", () => {
    return HttpResponse.json({ success: true })
  }),

  // Error simulation for testing
  http.put("/api/firecracker/actions", async ({ request }) => {
    const action = await request.json()
    if (action.action_type === "InstanceStart" && Math.random() > 0.8) {
      return HttpResponse.json(
        {
          error: "Cannot start VM",
          fault_message: "VM is already running",
          status: 409,
          suggestion: "Check VM state before starting",
          request_id: "mock-error-1",
        },
        { status: 409 },
      )
    }
    return HttpResponse.json({ success: true })
  }),
]
