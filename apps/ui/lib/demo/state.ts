// Seed state + in-memory store for demo mode. All data lives in module scope
// so it persists across the SPA session; localStorage is used to keep it across
// reloads.

import type {
  Vm,
  Container,
  Network,
  Volume,
  Image,
  Template,
  Host,
  Function as Fn,
  User,
  StorageBackend,
  Snapshot,
  AuditLog,
  BackupTarget,
} from "@/lib/types"

const STORAGE_KEY = "nqr-demo-state-v1"

function iso(daysAgo = 0, hoursAgo = 0): string {
  const d = new Date()
  d.setDate(d.getDate() - daysAgo)
  d.setHours(d.getHours() - hoursAgo)
  return d.toISOString()
}

function seedHosts(): Host[] {
  return [
    {
      id: "host-aurora",
      name: "aurora-01",
      addr: "10.0.0.11:9090",
      status: "healthy",
      capabilities_json: { bridge: "fcbr0", run_dir: "/srv/fc", cpus: 32, total_memory_mb: 131072, total_disk_gb: 2048, used_disk_gb: 612 },
      total_cpus: 32,
      total_memory_mb: 131072,
      total_disk_gb: 2048,
      used_disk_gb: 612,
      vm_count: 4,
      last_seen_at: iso(0, 0),
      last_metrics_at: iso(0, 0),
    },
    {
      id: "host-borealis",
      name: "borealis-02",
      addr: "10.0.0.12:9090",
      status: "healthy",
      capabilities_json: { bridge: "fcbr0", run_dir: "/srv/fc", cpus: 64, total_memory_mb: 262144, total_disk_gb: 4096, used_disk_gb: 1320 },
      total_cpus: 64,
      total_memory_mb: 262144,
      total_disk_gb: 4096,
      used_disk_gb: 1320,
      vm_count: 7,
      last_seen_at: iso(0, 0),
      last_metrics_at: iso(0, 0),
    },
    {
      id: "host-corona",
      name: "corona-03",
      addr: "10.0.0.13:9090",
      status: "degraded",
      capabilities_json: { bridge: "fcbr0", run_dir: "/srv/fc", cpus: 32, total_memory_mb: 131072, total_disk_gb: 2048, used_disk_gb: 1980 },
      total_cpus: 32,
      total_memory_mb: 131072,
      total_disk_gb: 2048,
      used_disk_gb: 1980,
      vm_count: 2,
      last_seen_at: iso(0, 1),
      last_metrics_at: iso(0, 1),
    },
  ]
}

function seedImages(): Image[] {
  return [
    { id: "img-vmlinux", kind: "kernel", name: "vmlinux-6.1", host_path: "/srv/images/vmlinux-6.1", sha256: "a1b2c3", size: 13631488, created_at: iso(30), updated_at: iso(30) },
    { id: "img-ubuntu-22", kind: "rootfs", name: "ubuntu-22.04", host_path: "/srv/images/ubuntu-22.04.ext4", sha256: "b2c3d4", size: 1610612736, created_at: iso(28), updated_at: iso(28) },
    { id: "img-ubuntu-24", kind: "rootfs", name: "ubuntu-24.04", host_path: "/srv/images/ubuntu-24.04.ext4", sha256: "c3d4e5", size: 1879048192, created_at: iso(14), updated_at: iso(14) },
    { id: "img-alpine", kind: "rootfs", name: "alpine-3.20", host_path: "/srv/images/alpine-3.20.ext4", sha256: "d4e5f6", size: 268435456, created_at: iso(21), updated_at: iso(21) },
    { id: "img-debian-12", kind: "rootfs", name: "debian-12", host_path: "/srv/images/debian-12.ext4", sha256: "e5f607", size: 1342177280, created_at: iso(20), updated_at: iso(20) },
    { id: "img-runtime", kind: "rootfs", name: "container-runtime", host_path: "/srv/images/container-runtime.ext4", sha256: "f60718", size: 524288000, project: "system", created_at: iso(60), updated_at: iso(60) },
  ]
}

function seedTemplates(): Template[] {
  return [
    { id: "tpl-small", name: "small", description: "1 vCPU · 1 GiB", kernel_path: "/srv/images/vmlinux-6.1", mem_mib: 1024, vcpu: 1, spec: { vcpu: 1, mem_mib: 1024, kernel_image_id: "img-vmlinux", rootfs_image_id: "img-ubuntu-22" }, created_at: iso(40), updated_at: iso(40) },
    { id: "tpl-medium", name: "medium", description: "2 vCPU · 2 GiB", kernel_path: "/srv/images/vmlinux-6.1", mem_mib: 2048, vcpu: 2, spec: { vcpu: 2, mem_mib: 2048, kernel_image_id: "img-vmlinux", rootfs_image_id: "img-ubuntu-22" }, created_at: iso(40), updated_at: iso(40) },
    { id: "tpl-large", name: "large", description: "4 vCPU · 4 GiB", kernel_path: "/srv/images/vmlinux-6.1", mem_mib: 4096, vcpu: 4, spec: { vcpu: 4, mem_mib: 4096, kernel_image_id: "img-vmlinux", rootfs_image_id: "img-ubuntu-24" }, created_at: iso(40), updated_at: iso(40) },
    { id: "tpl-xlarge", name: "xlarge", description: "8 vCPU · 16 GiB", kernel_path: "/srv/images/vmlinux-6.1", mem_mib: 16384, vcpu: 8, spec: { vcpu: 8, mem_mib: 16384, kernel_image_id: "img-vmlinux", rootfs_image_id: "img-ubuntu-24" }, created_at: iso(40), updated_at: iso(40) },
  ]
}

function vmDefaults(): Partial<Vm> {
  return {
    host_addr: "10.0.0.11:9090",
    api_sock: "/srv/fc/sock",
    tap: "tap0",
    log_path: "/srv/fc/log",
    fc_unit: "firecracker@1.service",
    kernel_path: "/srv/images/vmlinux-6.1",
    rootfs_path: "/srv/images/ubuntu-22.04.ext4",
    tags: [],
  }
}

function seedVms(): Vm[] {
  const base = vmDefaults()
  return [
    {
      ...(base as Vm),
      id: "vm-web-01",
      name: "web-01",
      state: "running",
      host_id: "host-aurora",
      template_id: "tpl-medium",
      http_port: 8001,
      vcpu: 2,
      mem_mib: 2048,
      guest_ip: "172.16.0.21",
      tags: ["prod", "web"],
      cpu_usage_percent: 32,
      memory_usage_percent: 48,
      created_at: iso(10),
      updated_at: iso(0, 2),
    },
    {
      ...(base as Vm),
      id: "vm-web-02",
      name: "web-02",
      state: "running",
      host_id: "host-aurora",
      template_id: "tpl-medium",
      http_port: 8002,
      vcpu: 2,
      mem_mib: 2048,
      guest_ip: "172.16.0.22",
      tags: ["prod", "web"],
      cpu_usage_percent: 28,
      memory_usage_percent: 52,
      created_at: iso(10),
      updated_at: iso(0, 2),
    },
    {
      ...(base as Vm),
      id: "vm-api-01",
      name: "api-01",
      state: "running",
      host_id: "host-borealis",
      template_id: "tpl-large",
      http_port: 8101,
      vcpu: 4,
      mem_mib: 4096,
      guest_ip: "172.16.0.31",
      tags: ["prod", "api"],
      cpu_usage_percent: 64,
      memory_usage_percent: 71,
      created_at: iso(8),
      updated_at: iso(0, 1),
    },
    {
      ...(base as Vm),
      id: "vm-worker-01",
      name: "worker-01",
      state: "running",
      host_id: "host-borealis",
      template_id: "tpl-large",
      http_port: 8201,
      vcpu: 4,
      mem_mib: 4096,
      guest_ip: "172.16.0.41",
      tags: ["prod", "worker"],
      cpu_usage_percent: 81,
      memory_usage_percent: 62,
      created_at: iso(5),
      updated_at: iso(0, 0),
    },
    {
      ...(base as Vm),
      id: "vm-db-staging",
      name: "db-staging",
      state: "stopped",
      host_id: "host-borealis",
      template_id: "tpl-xlarge",
      http_port: 8301,
      vcpu: 8,
      mem_mib: 16384,
      guest_ip: "",
      tags: ["staging", "db"],
      cpu_usage_percent: 0,
      memory_usage_percent: 0,
      created_at: iso(20),
      updated_at: iso(2),
    },
    {
      ...(base as Vm),
      id: "vm-ci-runner",
      name: "ci-runner",
      state: "paused",
      host_id: "host-aurora",
      template_id: "tpl-small",
      http_port: 8401,
      vcpu: 1,
      mem_mib: 1024,
      guest_ip: "172.16.0.51",
      tags: ["dev", "ci"],
      cpu_usage_percent: 0,
      memory_usage_percent: 24,
      created_at: iso(3),
      updated_at: iso(0, 3),
    },
  ]
}

function seedContainers(): Container[] {
  return [
    {
      id: "c-nginx-01",
      name: "nginx-edge",
      image: "nginx:1.27-alpine",
      args: [],
      env_vars: { TZ: "UTC" },
      volumes: [],
      port_mappings: [{ host: 8080, container: 80, protocol: "tcp" }],
      restart_policy: "unless-stopped",
      state: "running",
      cpu_limit: 1,
      memory_limit_mb: 512,
      cpu_percent: 4,
      memory_used_mb: 84,
      uptime_seconds: 86400 * 3,
      guest_ip: "172.16.0.61",
      created_at: iso(7),
      updated_at: iso(0, 1),
      started_at: iso(3),
    },
    {
      id: "c-redis",
      name: "redis-cache",
      image: "redis:7-alpine",
      args: [],
      env_vars: {},
      volumes: [],
      port_mappings: [{ host: 6379, container: 6379, protocol: "tcp" }],
      restart_policy: "unless-stopped",
      state: "running",
      cpu_limit: 1,
      memory_limit_mb: 1024,
      cpu_percent: 12,
      memory_used_mb: 312,
      uptime_seconds: 86400 * 5,
      guest_ip: "172.16.0.62",
      created_at: iso(8),
      updated_at: iso(0, 1),
      started_at: iso(5),
    },
    {
      id: "c-postgres",
      name: "pg-primary",
      image: "postgres:16",
      args: [],
      env_vars: { POSTGRES_PASSWORD: "•••••••", POSTGRES_DB: "app" },
      volumes: [{ host: "/srv/data/pg", container: "/var/lib/postgresql/data" }],
      port_mappings: [{ host: 5432, container: 5432, protocol: "tcp" }],
      restart_policy: "always",
      state: "running",
      cpu_limit: 2,
      memory_limit_mb: 4096,
      cpu_percent: 18,
      memory_used_mb: 1842,
      uptime_seconds: 86400 * 12,
      guest_ip: "172.16.0.63",
      created_at: iso(14),
      updated_at: iso(0, 1),
      started_at: iso(12),
    },
    {
      id: "c-grafana",
      name: "grafana",
      image: "grafana/grafana:latest",
      args: [],
      env_vars: {},
      volumes: [],
      port_mappings: [{ host: 3000, container: 3000, protocol: "tcp" }],
      restart_policy: "unless-stopped",
      state: "stopped",
      cpu_percent: 0,
      memory_used_mb: 0,
      created_at: iso(2),
      updated_at: iso(1),
    },
  ]
}

function seedNetworks(): Network[] {
  return [
    {
      id: "net-default",
      name: "default-nat",
      description: "Default NAT network for VMs",
      type: "nat",
      bridge_name: "fcbr0",
      host_id: "host-aurora",
      host_name: "aurora-01",
      cidr: "172.16.0.0/24",
      gateway: "172.16.0.1",
      status: "active",
      managed: true,
      dhcp_enabled: true,
      dhcp_range_start: "172.16.0.10",
      dhcp_range_end: "172.16.0.200",
      vm_count: 6,
      created_at: iso(60),
      updated_at: iso(0),
    },
    {
      id: "net-prod",
      name: "prod-bridged",
      description: "Bridged to physical eth0",
      type: "bridged",
      bridge_name: "br-prod",
      host_id: "host-borealis",
      host_name: "borealis-02",
      cidr: "10.10.0.0/24",
      gateway: "10.10.0.1",
      status: "active",
      managed: true,
      dhcp_enabled: false,
      vm_count: 3,
      created_at: iso(30),
      updated_at: iso(2),
    },
    {
      id: "net-isolated",
      name: "ci-isolated",
      description: "Isolated CI/CD network",
      type: "isolated",
      bridge_name: "br-iso",
      host_id: "host-aurora",
      host_name: "aurora-01",
      cidr: "192.168.10.0/24",
      gateway: "192.168.10.1",
      status: "active",
      managed: true,
      dhcp_enabled: true,
      vm_count: 1,
      created_at: iso(7),
      updated_at: iso(0),
    },
  ]
}

function seedVolumes(): Volume[] {
  return [
    { id: "vol-pg-data", name: "pg-data", description: "Postgres data volume", path: "/srv/volumes/pg-data.qcow2", size_bytes: 53687091200, size_gb: 50, type: "qcow2", status: "attached", host_id: "host-borealis", host_name: "borealis-02", attached_to_vm_id: "vm-api-01", attached_to_vm_name: "api-01", created_at: iso(14) },
    { id: "vol-logs", name: "shared-logs", description: "Centralized log volume", path: "/srv/volumes/logs.ext4", size_bytes: 21474836480, size_gb: 20, type: "ext4", status: "available", host_id: "host-aurora", host_name: "aurora-01", created_at: iso(7) },
    { id: "vol-cache", name: "build-cache", description: "CI build cache", path: "/srv/volumes/cache.raw", size_bytes: 107374182400, size_gb: 100, type: "raw", status: "attached", host_id: "host-aurora", host_name: "aurora-01", attached_to_vm_id: "vm-ci-runner", attached_to_vm_name: "ci-runner", created_at: iso(3) },
    { id: "vol-snapshots", name: "snapshot-store", description: "VM snapshot vault", path: "/srv/volumes/snapshots.qcow2", size_bytes: 214748364800, size_gb: 200, type: "qcow2", status: "available", host_id: "host-corona", host_name: "corona-03", created_at: iso(45) },
  ]
}

function seedFunctions(): Fn[] {
  return [
    { id: "fn-resize-img", name: "resize-image", runtime: "javascript", handler: "index.handler", timeout_seconds: 30, code: "export const handler = async (e) => ({ ok: true, in: e })", vcpu: 1, memory_mb: 256, state: "ready", invocation_count_24h: 1284, avg_duration_ms: 142, last_invoked_at: iso(0, 0), created_at: iso(14), updated_at: iso(2), guest_ip: "172.16.0.81", port: 9100 },
    { id: "fn-webhook", name: "github-webhook", runtime: "python", handler: "main.handler", timeout_seconds: 10, code: "def handler(event):\n    return { 'ok': True }", vcpu: 1, memory_mb: 128, state: "ready", invocation_count_24h: 386, avg_duration_ms: 68, last_invoked_at: iso(0, 1), created_at: iso(7), updated_at: iso(1), guest_ip: "172.16.0.82", port: 9101 },
    { id: "fn-classify", name: "classify-event", runtime: "typescript", handler: "index.handler", timeout_seconds: 60, code: "export const handler = async (e) => ({ class: 'A' })", vcpu: 2, memory_mb: 512, state: "ready", invocation_count_24h: 52, avg_duration_ms: 412, last_invoked_at: iso(0, 2), created_at: iso(3), updated_at: iso(0, 4), guest_ip: "172.16.0.83", port: 9102 },
  ]
}

function seedUsers(): User[] {
  return [
    { id: "u-root", username: "root", role: "admin", created_at: iso(120), last_login_at: iso(0, 0), timezone: "UTC", theme: "dark" },
    { id: "u-alice", username: "alice", role: "admin", created_at: iso(80), last_login_at: iso(1), timezone: "Europe/Berlin", theme: "dark" },
    { id: "u-bob", username: "bob", role: "user", created_at: iso(45), last_login_at: iso(0, 3), timezone: "America/New_York", theme: "light" },
    { id: "u-carol", username: "carol", role: "viewer", created_at: iso(15), last_login_at: iso(2), timezone: "Asia/Singapore", theme: "system" },
  ]
}

function seedStorageBackends(): StorageBackend[] {
  return [
    { id: "sb-local", name: "Local Disk", kind: "local_file", capabilities: {} as any, is_default: true, created_at: iso(120) },
    { id: "sb-nfs", name: "NAS-NFS", kind: "nfs", capabilities: {} as any, is_default: false, created_at: iso(60) },
    { id: "sb-iscsi", name: "Pure-iSCSI", kind: "iscsi", capabilities: {} as any, is_default: false, created_at: iso(30) },
  ]
}

function seedBackupTargets(): BackupTarget[] {
  return [
    { id: "bt-s3", name: "AWS S3 (us-east-1)", endpoint: "https://s3.us-east-1.amazonaws.com", region: "us-east-1", bucket: "nqr-backups", prefix: "prod/", access_key_id: "AKIA••••••EXAMPLE", gc_hour: 3, created_at: iso(40) },
    { id: "bt-minio", name: "MinIO (on-prem)", endpoint: "https://minio.internal:9000", bucket: "nqr-backups", prefix: "lab/", access_key_id: "minio-admin", gc_hour: 4, created_at: iso(15) },
  ]
}

function seedSnapshots(): Snapshot[] {
  return [
    { id: "snap-1", vm_id: "vm-web-01", name: "pre-deploy", host_addr: "10.0.0.11:9090", mem_size_mib: 2048, vcpu: 2, kernel_path: "/srv/images/vmlinux-6.1", rootfs_path: "/srv/snapshots/web-01-1.ext4", mem_path: "/srv/snapshots/web-01-1.mem", host_id: "host-aurora", created_at: iso(2) } as Snapshot,
    { id: "snap-2", vm_id: "vm-api-01", name: "v1.4-baseline", host_addr: "10.0.0.12:9090", mem_size_mib: 4096, vcpu: 4, kernel_path: "/srv/images/vmlinux-6.1", rootfs_path: "/srv/snapshots/api-01-2.ext4", mem_path: "/srv/snapshots/api-01-2.mem", host_id: "host-borealis", created_at: iso(5) } as Snapshot,
  ]
}

function seedAuditLogs(): AuditLog[] {
  const rows: AuditLog[] = [
    { id: "al-1", user_id: "u-root", username: "root", action: "vm.start", resource_type: "vm", resource_id: "vm-web-01", details: {}, ip_address: "10.0.0.4", success: true, error_message: null, created_at: iso(0, 1) },
    { id: "al-2", user_id: "u-alice", username: "alice", action: "vm.create", resource_type: "vm", resource_id: "vm-worker-01", details: { template: "tpl-large" }, ip_address: "10.0.0.6", success: true, error_message: null, created_at: iso(5) },
    { id: "al-3", user_id: "u-bob", username: "bob", action: "container.deploy", resource_type: "container", resource_id: "c-nginx-01", details: {}, ip_address: "10.0.0.7", success: true, error_message: null, created_at: iso(0, 4) },
    { id: "al-4", user_id: "u-root", username: "root", action: "user.login", resource_type: "user", resource_id: "u-root", details: {}, ip_address: "10.0.0.4", success: true, error_message: null, created_at: iso(0, 0) },
    { id: "al-5", user_id: "u-carol", username: "carol", action: "vm.view", resource_type: "vm", resource_id: "vm-api-01", details: {}, ip_address: "10.0.0.9", success: true, error_message: null, created_at: iso(0, 2) },
  ]
  return rows
}

export type DemoStateShape = {
  vms: Vm[]
  containers: Container[]
  networks: Network[]
  volumes: Volume[]
  images: Image[]
  templates: Template[]
  hosts: Host[]
  functions: Fn[]
  users: User[]
  storageBackends: StorageBackend[]
  snapshots: Snapshot[]
  auditLogs: AuditLog[]
  backupTargets: BackupTarget[]
  // Per-VM sub-resources are stored by parent id for simplicity.
  drives: Record<string, any[]>
  nics: Record<string, any[]>
  portForwards: Record<string, any[]>
}

function fresh(): DemoStateShape {
  return {
    vms: seedVms(),
    containers: seedContainers(),
    networks: seedNetworks(),
    volumes: seedVolumes(),
    images: seedImages(),
    templates: seedTemplates(),
    hosts: seedHosts(),
    functions: seedFunctions(),
    users: seedUsers(),
    storageBackends: seedStorageBackends(),
    snapshots: seedSnapshots(),
    auditLogs: seedAuditLogs(),
    backupTargets: seedBackupTargets(),
    drives: {},
    nics: {},
    portForwards: {},
  }
}

let _state: DemoStateShape | null = null

function load(): DemoStateShape {
  if (typeof window === "undefined") return fresh()
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (raw) return JSON.parse(raw) as DemoStateShape
  } catch {
    // fall through
  }
  return fresh()
}

function persist() {
  if (typeof window === "undefined" || !_state) return
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(_state))
  } catch {
    // quota etc — ignore, demo data is rebuildable
  }
}

export function getState(): DemoStateShape {
  if (!_state) _state = load()
  return _state
}

export function mutateState(fn: (s: DemoStateShape) => void) {
  if (!_state) _state = load()
  fn(_state)
  persist()
}

export function resetState() {
  _state = fresh()
  persist()
}

export function newId(prefix: string): string {
  return `${prefix}-${Math.random().toString(36).slice(2, 8)}`
}

export function nowIso(): string {
  return new Date().toISOString()
}
