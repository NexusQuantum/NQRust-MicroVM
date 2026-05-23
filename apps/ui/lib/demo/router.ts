// Mock router for demo mode. Pattern-matches (method, path) tuples against the
// in-memory store and returns the same shapes the real manager would.
//
// Coverage strategy: hand-handle the high-traffic endpoints (auth, eula,
// license, vms, containers, networks, volumes, images, templates, hosts,
// functions, users, dashboard, settings, audit, backups, snapshots, storage
// backends). Unmatched endpoints fall through to a safe default that won't
// crash list/detail screens (`{ items: [] }`, `{ item: null }`, etc.).

import { DEMO_MODE } from "./flag"
import { getState, mutateState, newId, nowIso } from "./state"
import type { Vm, Container, Network, Volume, Template } from "@/lib/types"

export interface MockRequest {
  method: string
  path: string // path after the base URL, e.g. "/vms"
  body?: any
}

function ok() {
  return { ok: true }
}

function notFound(): never {
  throw new Error(
    JSON.stringify({
      error: "Not found",
      status: 404,
      suggestion: "Demo data does not include this resource",
      request_id: "demo",
    }),
  )
}

function asStateVm(req: any): Vm {
  const tpl = getState().templates.find((t) => t.id === req.template_id) || getState().templates[0]
  const vcpu = req.vcpu ?? tpl?.spec.vcpu ?? 1
  const mem = req.mem_mib ?? tpl?.spec.mem_mib ?? 1024
  const host = getState().hosts[Math.floor(Math.random() * getState().hosts.length)]
  const ipSuffix = 100 + Math.floor(Math.random() * 100)
  const id = newId("vm")
  return {
    id,
    name: req.name || `demo-${id.slice(-4)}`,
    state: "running",
    host_id: host.id,
    template_id: req.template_id,
    host_addr: host.addr,
    api_sock: "/srv/fc/sock",
    tap: "tap0",
    log_path: "/srv/fc/log",
    http_port: 9000 + Math.floor(Math.random() * 999),
    fc_unit: "firecracker@demo.service",
    vcpu,
    mem_mib: mem,
    kernel_path: "/srv/images/vmlinux-6.1",
    rootfs_path: "/srv/images/ubuntu-22.04.ext4",
    guest_ip: `172.16.0.${ipSuffix}`,
    tags: req.tags ?? [],
    cpu_usage_percent: 10 + Math.floor(Math.random() * 60),
    memory_usage_percent: 20 + Math.floor(Math.random() * 60),
    created_at: nowIso(),
    updated_at: nowIso(),
  }
}

function fakeMetricsSeries(now = Date.now(), n = 60, base = 50, jitter = 30) {
  return Array.from({ length: n }, (_, i) => {
    const t = new Date(now - (n - i) * 60_000).toISOString()
    return { recorded_at: t, value: Math.max(0, Math.min(100, base + Math.sin(i / 4) * jitter + (Math.random() - 0.5) * 10)) }
  })
}

function parseId(path: string, prefix: string): string | null {
  const m = path.match(new RegExp(`^${prefix}/([^/?]+)`))
  return m ? m[1] : null
}

export async function handleMockRequest({ method, path, body }: MockRequest): Promise<any> {
  // Strip query string for matching, but keep for filters
  const [rawPath, _query] = path.split("?")
  const p = rawPath

  // Auth ----------------------------------------------------------------
  if (p === "/auth/login" && method === "POST") {
    const username = body?.username || "demo"
    return {
      token: "demo-token-" + Math.random().toString(36).slice(2),
      user: {
        id: "u-demo",
        username,
        role: "admin",
        email: `${username}@demo.local`,
        created_at: nowIso(),
      },
    }
  }
  if (p === "/auth/me" && method === "GET") {
    return { id: "u-demo", username: "demo", role: "admin", created_at: nowIso(), timezone: "UTC" }
  }
  if (p === "/auth/me/avatar" && method === "DELETE") return ok()
  if (p === "/auth/me/profile" && method === "PATCH") return ok()
  if (p === "/auth/me/password" && method === "POST") return ok()
  if (p === "/auth/me/preferences" && method === "GET") {
    return {
      preferences: {
        timezone: "UTC",
        theme: "dark",
        date_format: "YYYY-MM-DD",
        notifications: { email: true, browser: true, desktop: false },
        vm_defaults: { vcpu: 2, mem_mib: 2048, disk_gb: 10 },
        auto_refresh: 30,
        metrics_retention: 7,
      },
    }
  }
  if (p === "/auth/me/preferences" && method === "PATCH") return ok()

  // EULA + License + SSO providers (gating endpoints)
  if (p === "/admin/eula/status" || p === "/eula/status") {
    return { needs_acceptance: false, latest_accepted_version: "1.0.0" }
  }
  if (p === "/admin/eula/info" || p === "/eula/info") {
    return { version: "1.0.0", languages: ["en"] }
  }
  if (p === "/admin/eula/accept" && method === "POST") return { success: true }
  if (p === "/admin/license/status" || p === "/license/status") {
    return {
      is_licensed: true,
      status: "active",
      is_grace_period: false,
      customer_name: "NQR-MicroVM Demo",
      product: "NQR-MicroVM",
      features: ["vm", "container", "function", "network", "volume", "snapshot", "backup"],
      expires_at: new Date(Date.now() + 365 * 86400_000).toISOString(),
      activations: 1,
      max_activations: 1000,
      verified_at: nowIso(),
      license_key: "DEMO-****-****-MODE",
    }
  }
  if (p === "/admin/license/activate" && method === "POST") return { success: true }
  if (p === "/admin/license/upload" && method === "POST") return { success: true }
  if (p === "/sso/providers" && method === "GET") return { providers: [] }
  if (p === "/admin/sso/providers" && method === "GET") return { items: [] }

  // Dashboard ----------------------------------------------------------
  if (p === "/dashboard/stats" || p === "/stats" || p === "/admin/stats") {
    const s = getState()
    return {
      total_vms: s.vms.length,
      running_vms: s.vms.filter((v) => v.state === "running").length,
      total_functions: s.functions.length,
      invocations_24h: s.functions.reduce((a, f) => a + (f.invocation_count_24h ?? 0), 0),
      total_containers: s.containers.length,
      running_containers: s.containers.filter((c) => c.state === "running").length,
      total_hosts: s.hosts.length,
    }
  }

  // System --------------------------------------------------------------
  if (p === "/admin/db/info" || p === "/db/info") {
    return { kind: "postgres", host: "demo-db.internal", port: 5432, database: "nqr", version: "16.2", uptime_seconds: 86400 * 30 }
  }
  if (p === "/admin/system/stats" || p === "/system/stats") {
    return { cpu_count: 32, mem_total_mb: 65536, disk_total_gb: 1024, manager_version: "demo", agent_version: "demo" }
  }

  // Audit Logs ----------------------------------------------------------
  if (p === "/admin/audit" || p === "/audit-logs") {
    const items = getState().auditLogs
    return { items, total: items.length }
  }

  // VMs -----------------------------------------------------------------
  if (p === "/vms" && method === "GET") return { items: getState().vms }
  if (p === "/vms" && method === "POST") {
    const vm = asStateVm(body)
    mutateState((s) => s.vms.unshift(vm))
    return { id: vm.id, item: vm }
  }
  const vmId = parseId(p, "/vms")
  if (vmId) {
    const isAction = (a: string) => p === `/vms/${vmId}/${a}`
    if (p === `/vms/${vmId}` && method === "GET") {
      const item = getState().vms.find((v) => v.id === vmId)
      if (!item) notFound()
      return { item }
    }
    if (p === `/vms/${vmId}` && method === "PATCH") {
      mutateState((s) => {
        const v = s.vms.find((x) => x.id === vmId)
        if (v && body) {
          if (body.name) v.name = body.name
          if (body.tags) v.tags = body.tags
          v.updated_at = nowIso()
        }
      })
      return ok()
    }
    if (p === `/vms/${vmId}` && method === "DELETE") {
      mutateState((s) => {
        s.vms = s.vms.filter((v) => v.id !== vmId)
      })
      return ok()
    }
    if (isAction("start") || isAction("resume")) {
      mutateState((s) => {
        const v = s.vms.find((x) => x.id === vmId)
        if (v) { v.state = "running"; v.updated_at = nowIso(); v.cpu_usage_percent = 20; v.memory_usage_percent = 40 }
      })
      return ok()
    }
    if (isAction("stop")) {
      mutateState((s) => {
        const v = s.vms.find((x) => x.id === vmId)
        if (v) { v.state = "stopped"; v.updated_at = nowIso(); v.cpu_usage_percent = 0; v.memory_usage_percent = 0; v.guest_ip = "" }
      })
      return ok()
    }
    if (isAction("pause")) {
      mutateState((s) => {
        const v = s.vms.find((x) => x.id === vmId)
        if (v) { v.state = "paused"; v.updated_at = nowIso(); v.cpu_usage_percent = 0 }
      })
      return ok()
    }
    if (isAction("flush-metrics") || isAction("ctrl-alt-del") || isAction("initialize")) return ok()
    if (p === `/vms/${vmId}/shell` && method === "GET") return { username: "demo", password: "demo" }
    if (p === `/vms/${vmId}/drives` && method === "GET") return { items: getState().drives[vmId] || [] }
    if (p === `/vms/${vmId}/drives` && method === "POST") {
      const d = { id: newId("drv"), vm_id: vmId, ...body, created_at: nowIso() }
      mutateState((s) => { (s.drives[vmId] ||= []).push(d) })
      return d
    }
    if (p === `/vms/${vmId}/nics` && method === "GET") return { items: getState().nics[vmId] || [] }
    if (p === `/vms/${vmId}/nics` && method === "POST") {
      const n = { id: newId("nic"), vm_id: vmId, ...body, created_at: nowIso() }
      mutateState((s) => { (s.nics[vmId] ||= []).push(n) })
      return n
    }
    if (p === `/vms/${vmId}/port-forwards` && method === "GET") return { items: getState().portForwards[vmId] || [] }
    if (p === `/vms/${vmId}/port-forwards` && method === "POST") {
      const pf = { id: newId("pf"), vm_id: vmId, ...body, created_at: nowIso() }
      mutateState((s) => { (s.portForwards[vmId] ||= []).push(pf) })
      return pf
    }
    if (p === `/vms/${vmId}/snapshots` && method === "GET") {
      return { items: getState().snapshots.filter((s) => s.vm_id === vmId) }
    }
    if (p === `/vms/${vmId}/snapshots` && method === "POST") {
      const v = getState().vms.find((x) => x.id === vmId)
      const snap = {
        id: newId("snap"),
        vm_id: vmId,
        name: body?.name || `snap-${Date.now()}`,
        host_addr: v?.host_addr || "10.0.0.11:9090",
        mem_size_mib: v?.mem_mib || 1024,
        vcpu: v?.vcpu || 1,
        kernel_path: v?.kernel_path || "/srv/images/vmlinux-6.1",
        rootfs_path: `/srv/snapshots/${vmId}-${Date.now()}.ext4`,
        mem_path: `/srv/snapshots/${vmId}-${Date.now()}.mem`,
        host_id: v?.host_id,
        created_at: nowIso(),
      }
      mutateState((s) => s.snapshots.unshift(snap as any))
      return { id: snap.id, item: snap }
    }
  }

  // Snapshots top-level
  if (p.startsWith("/snapshots/") && method === "DELETE") {
    const id = p.slice("/snapshots/".length)
    mutateState((s) => { s.snapshots = s.snapshots.filter((x) => x.id !== id) })
    return ok()
  }

  // Containers ----------------------------------------------------------
  if (p === "/containers" && method === "GET") return { items: getState().containers }
  if (p === "/containers" && method === "POST") {
    const c: Container = {
      id: newId("c"),
      name: body?.name || "demo-container",
      image: body?.image || "alpine:latest",
      args: body?.args ?? [],
      env_vars: body?.env_vars ?? {},
      volumes: body?.volumes ?? [],
      port_mappings: body?.port_mappings ?? [],
      restart_policy: body?.restart_policy ?? "no",
      state: "running",
      cpu_limit: body?.cpu_limit,
      memory_limit_mb: body?.memory_limit_mb,
      cpu_percent: 5 + Math.floor(Math.random() * 20),
      memory_used_mb: 50 + Math.floor(Math.random() * 200),
      uptime_seconds: 1,
      guest_ip: `172.16.0.${150 + Math.floor(Math.random() * 50)}`,
      created_at: nowIso(),
      updated_at: nowIso(),
      started_at: nowIso(),
    }
    mutateState((s) => s.containers.unshift(c))
    return { id: c.id }
  }
  const cId = parseId(p, "/containers")
  if (cId) {
    const cAct = (a: string) => p === `/containers/${cId}/${a}`
    if (p === `/containers/${cId}` && method === "GET") {
      const item = getState().containers.find((x) => x.id === cId)
      if (!item) notFound()
      return { item }
    }
    if (p === `/containers/${cId}` && method === "PUT") {
      mutateState((s) => { const c = s.containers.find((x) => x.id === cId); if (c && body) Object.assign(c, body) })
      const item = getState().containers.find((x) => x.id === cId)
      return { item }
    }
    if (p === `/containers/${cId}` && method === "DELETE") {
      mutateState((s) => { s.containers = s.containers.filter((c) => c.id !== cId) })
      return ok()
    }
    if (cAct("start")) { mutateState((s) => { const c = s.containers.find((x) => x.id === cId); if (c) { c.state = "running"; c.started_at = nowIso() } }); return ok() }
    if (cAct("stop")) { mutateState((s) => { const c = s.containers.find((x) => x.id === cId); if (c) { c.state = "stopped"; c.stopped_at = nowIso(); c.cpu_percent = 0; c.memory_used_mb = 0 } }); return ok() }
    if (cAct("restart")) { mutateState((s) => { const c = s.containers.find((x) => x.id === cId); if (c) c.state = "running" }); return ok() }
    if (cAct("pause")) { mutateState((s) => { const c = s.containers.find((x) => x.id === cId); if (c) c.state = "paused" }); return ok() }
    if (cAct("resume")) { mutateState((s) => { const c = s.containers.find((x) => x.id === cId); if (c) c.state = "running" }); return ok() }
    if (cAct("exec") && method === "POST") return { stdout: "demo-mode: exec is not available", stderr: "", exit_code: 0 }
    if (p.startsWith(`/containers/${cId}/logs`)) {
      const samples = [
        "[info] starting service",
        "[info] listening on 0.0.0.0:8080",
        "[info] accepted connection from 10.0.0.5",
        "[debug] request /healthz returned 200",
        "[debug] cache miss for key=user:42",
        "[info] running gc cycle",
      ]
      return {
        items: samples.map((message, i) => ({
          container_id: cId,
          timestamp: new Date(Date.now() - (samples.length - i) * 1500).toISOString(),
          stream: i % 5 === 0 ? "stderr" : "stdout",
          message,
        })),
      }
    }
    if (p === `/containers/${cId}/stats`) {
      return {
        items: Array.from({ length: 30 }, (_, i) => ({
          cpu_percent: 8 + Math.sin(i / 3) * 6 + Math.random() * 3,
          memory_used_mb: 200 + Math.sin(i / 4) * 50,
          memory_limit_mb: 1024,
          network_rx_bytes: i * 1024 * 16,
          network_tx_bytes: i * 1024 * 12,
          block_read_bytes: i * 4096,
          block_write_bytes: i * 4096,
          pids: 12,
          recorded_at: new Date(Date.now() - (30 - i) * 60_000).toISOString(),
        })),
      }
    }
  }

  // Docker Hub mocks ----------------------------------------------------
  if (p === "/images/dockerhub/search" && method === "POST") {
    const q = body?.query || ""
    const items = [
      { name: "nginx", description: "Official build of NGINX.", star_count: 19000, is_official: true, is_automated: false, pull_count: 1_000_000_000 },
      { name: "redis", description: "Redis is the world's fastest data platform.", star_count: 12000, is_official: true, is_automated: false, pull_count: 800_000_000 },
      { name: "postgres", description: "The PostgreSQL object-relational database.", star_count: 13000, is_official: true, is_automated: false, pull_count: 1_200_000_000 },
      { name: "node", description: "Node.js JavaScript runtime.", star_count: 11000, is_official: true, is_automated: false, pull_count: 900_000_000 },
    ].filter((x) => !q || x.name.includes(q))
    return { items }
  }
  if (p === "/images/dockerhub/tags" && method === "POST") {
    return { items: ["latest", "alpine", "1.27", "1.26", "stable"].map((tag) => ({ name: tag, last_updated: nowIso(), images: [] })) }
  }
  if (p.startsWith("/images/dockerhub/download")) return { task_id: "demo-task", status: "completed", progress: 100 }

  // Images / Registry ---------------------------------------------------
  if (p === "/images" && method === "GET") return { items: getState().images }
  if (p === "/images" && method === "POST") {
    const img = { id: newId("img"), kind: body?.kind || "rootfs", name: body?.name || "demo-image", host_path: body?.host_path || "/srv/images/demo", sha256: "demo", size: body?.size || 1_000_000, project: body?.project, created_at: nowIso(), updated_at: nowIso() }
    mutateState((s) => s.images.unshift(img))
    return { id: img.id }
  }
  const imageId = parseId(p, "/images")
  if (imageId) {
    if (method === "DELETE") {
      mutateState((s) => { s.images = s.images.filter((i) => i.id !== imageId) })
      return ok()
    }
    if (method === "GET") {
      const item = getState().images.find((i) => i.id === imageId)
      if (item) return { item }
    }
  }

  // Templates -----------------------------------------------------------
  if (p === "/templates" && method === "GET") return { items: getState().templates }
  if (p === "/templates" && method === "POST") {
    const t: Template = {
      id: newId("tpl"),
      name: body?.name || "new-template",
      description: body?.description || "",
      kernel_path: "/srv/images/vmlinux-6.1",
      mem_mib: body?.spec?.mem_mib || 1024,
      vcpu: body?.spec?.vcpu || 1,
      spec: body?.spec || { vcpu: 1, mem_mib: 1024 },
      created_at: nowIso(),
      updated_at: nowIso(),
    }
    mutateState((s) => s.templates.unshift(t))
    return { id: t.id }
  }
  const tplId = parseId(p, "/templates")
  if (tplId) {
    if (method === "DELETE") { mutateState((s) => { s.templates = s.templates.filter((t) => t.id !== tplId) }); return ok() }
    if (method === "PUT" || method === "PATCH") {
      mutateState((s) => { const t = s.templates.find((x) => x.id === tplId); if (t && body) Object.assign(t, body) })
      const item = getState().templates.find((t) => t.id === tplId)
      return { item }
    }
    if (method === "GET") {
      const item = getState().templates.find((t) => t.id === tplId)
      if (item) return { item }
    }
    if (p === `/templates/${tplId}/instantiate` && method === "POST") {
      const tpl = getState().templates.find((t) => t.id === tplId)
      const vm = asStateVm({ name: body?.name || `from-${tpl?.name}`, template_id: tplId, vcpu: tpl?.spec.vcpu, mem_mib: tpl?.spec.mem_mib })
      mutateState((s) => s.vms.unshift(vm))
      return { id: vm.id, item: vm }
    }
  }

  // Hosts ---------------------------------------------------------------
  if (p === "/hosts" && method === "GET") return { items: getState().hosts }
  const hostId = parseId(p, "/hosts")
  if (hostId) {
    if (method === "GET") {
      const item = getState().hosts.find((h) => h.id === hostId)
      if (item) return { item }
    }
    if (method === "DELETE") { mutateState((s) => { s.hosts = s.hosts.filter((h) => h.id !== hostId) }); return ok() }
  }

  // Networks ------------------------------------------------------------
  if (p === "/networks" && method === "GET") return { items: getState().networks }
  if (p === "/networks" && method === "POST") {
    const n: Network = {
      id: newId("net"),
      name: body?.name || "new-network",
      description: body?.description,
      type: body?.type || "nat",
      bridge_name: `br-${body?.name || Math.random().toString(36).slice(2, 6)}`,
      host_id: body?.host_id,
      cidr: body?.cidr || "192.168.100.0/24",
      gateway: body?.gateway,
      status: "active",
      managed: true,
      dhcp_enabled: body?.dhcp_enabled ?? true,
      dhcp_range_start: body?.dhcp_range_start,
      dhcp_range_end: body?.dhcp_range_end,
      vm_count: 0,
      vlan_id: body?.vlan_id,
      created_at: nowIso(),
      updated_at: nowIso(),
    }
    mutateState((s) => s.networks.unshift(n))
    return { item: n }
  }
  if (p === "/networks/suggest") {
    return { bridge_name: "fcbr1", cidr: "172.16.50.0/24", gateway: "172.16.50.1", dhcp_range_start: "172.16.50.10", dhcp_range_end: "172.16.50.200" }
  }
  if (p.match(/^\/hosts\/[^/]+\/interfaces$/)) {
    return {
      interfaces: [
        { name: "eth0", mac: "52:54:00:12:34:01", state: "up", addresses: ["10.0.0.11/24"], is_management: true },
        { name: "eth1", mac: "52:54:00:12:34:02", state: "up", addresses: [], is_management: false },
        { name: "fcbr0", mac: "52:54:00:12:34:03", state: "up", addresses: ["172.16.0.1/24"], is_management: false, master: "fcbr0" },
      ],
    }
  }
  const netId = parseId(p, "/networks")
  if (netId) {
    if (p === `/networks/${netId}` && method === "GET") {
      const item = getState().networks.find((n) => n.id === netId)
      if (item) return { item }
    }
    if (p === `/networks/${netId}` && method === "PATCH") {
      mutateState((s) => { const n = s.networks.find((x) => x.id === netId); if (n && body) Object.assign(n, body) })
      const item = getState().networks.find((n) => n.id === netId)
      return { item }
    }
    if (p === `/networks/${netId}` && method === "DELETE") {
      mutateState((s) => { s.networks = s.networks.filter((n) => n.id !== netId) }); return ok()
    }
    if (p === `/networks/${netId}/vms`) {
      const vm_ids = getState().vms.slice(0, Math.min(getState().vms.length, 2)).map((v) => v.id)
      return { vm_ids }
    }
    if (p === `/networks/${netId}/retry` && method === "POST") {
      const item = getState().networks.find((n) => n.id === netId)
      return { item }
    }
  }

  // Volumes -------------------------------------------------------------
  if (p === "/volumes" && method === "GET") return { items: getState().volumes }
  if (p === "/volumes" && method === "POST") {
    const v: Volume = {
      id: newId("vol"),
      name: body?.name || "new-volume",
      description: body?.description,
      path: `/srv/volumes/${body?.name || "demo"}.${body?.type || "qcow2"}`,
      size_bytes: (body?.size_gb || 10) * 1024 * 1024 * 1024,
      size_gb: body?.size_gb || 10,
      type: body?.type || "qcow2",
      status: "available",
      host_id: body?.host_id || getState().hosts[0].id,
      host_name: getState().hosts.find((h) => h.id === body?.host_id)?.name,
      created_at: nowIso(),
    }
    mutateState((s) => s.volumes.unshift(v))
    return { item: v }
  }
  const volId = parseId(p, "/volumes")
  if (volId) {
    if (p === `/volumes/${volId}` && method === "GET") {
      const item = getState().volumes.find((v) => v.id === volId)
      if (item) return { item }
    }
    if (p === `/volumes/${volId}` && method === "DELETE") {
      mutateState((s) => { s.volumes = s.volumes.filter((v) => v.id !== volId) }); return ok()
    }
    if (p === `/volumes/${volId}/attach` && method === "POST") {
      mutateState((s) => {
        const v = s.volumes.find((x) => x.id === volId)
        if (v) {
          v.status = "attached"
          v.attached_to_vm_id = body?.vm_id
          v.attached_to_vm_name = s.vms.find((x) => x.id === body?.vm_id)?.name
        }
      })
      return ok()
    }
    if (p === `/volumes/${volId}/detach` && method === "POST") {
      mutateState((s) => {
        const v = s.volumes.find((x) => x.id === volId)
        if (v) { v.status = "available"; v.attached_to_vm_id = undefined; v.attached_to_vm_name = undefined }
      })
      return ok()
    }
  }

  // Functions -----------------------------------------------------------
  if (p === "/functions" && method === "GET") return getState().functions
  if (p === "/functions" && method === "POST") {
    const f = {
      id: newId("fn"),
      name: body?.name || "new-function",
      runtime: body?.runtime || "javascript",
      handler: body?.handler || "index.handler",
      timeout_seconds: 30,
      code: body?.code || "",
      vcpu: body?.vcpu ?? 1,
      memory_mb: body?.memory_mb ?? 128,
      state: "ready" as const,
      created_at: nowIso(),
      updated_at: nowIso(),
      invocation_count_24h: 0,
      avg_duration_ms: 0,
      guest_ip: `172.16.0.${180 + Math.floor(Math.random() * 20)}`,
      port: 9100 + Math.floor(Math.random() * 100),
    }
    mutateState((s) => s.functions.unshift(f as any))
    return f
  }
  const fnId = parseId(p, "/functions")
  if (fnId) {
    if (p === `/functions/${fnId}` && method === "GET") {
      const item = getState().functions.find((f) => f.id === fnId)
      if (item) return item
    }
    if (p === `/functions/${fnId}` && method === "PUT") {
      mutateState((s) => { const f = s.functions.find((x) => x.id === fnId); if (f && body) Object.assign(f, body) })
      return getState().functions.find((f) => f.id === fnId)
    }
    if (p === `/functions/${fnId}` && method === "DELETE") {
      mutateState((s) => { s.functions = s.functions.filter((f) => f.id !== fnId) }); return ok()
    }
    if (p === `/functions/${fnId}/invoke` && method === "POST") {
      return {
        request_id: newId("req"),
        status: "success",
        duration_ms: 80 + Math.floor(Math.random() * 200),
        response: { ok: true, echo: body?.event ?? null },
        logs: ["[info] cold start: 18ms", "[info] handler returned"],
      }
    }
    if (p.startsWith(`/functions/${fnId}/invocations`)) {
      return { items: Array.from({ length: 12 }, (_, i) => ({
        id: newId("inv"),
        function_id: fnId,
        status: i % 9 === 0 ? "error" : "success",
        duration_ms: 60 + Math.floor(Math.random() * 400),
        memory_used_mb: 60 + Math.floor(Math.random() * 120),
        request_id: newId("req"),
        event: { hello: "world" },
        response: { ok: true },
        logs: ["[info] invocation completed"],
        invoked_at: new Date(Date.now() - i * 60_000).toISOString(),
      })) }
    }
  }
  if (p === "/functions/test" && method === "POST") {
    return { request_id: newId("req"), status: "success", duration_ms: 84, response: { ok: true }, logs: ["[info] test invocation"] }
  }

  // Users ---------------------------------------------------------------
  if ((p === "/admin/users" || p === "/users") && method === "GET") return { items: getState().users }
  if ((p === "/admin/users" || p === "/users") && method === "POST") {
    const u = { id: newId("u"), username: body?.username || "newuser", role: body?.role || "user", created_at: nowIso() }
    mutateState((s) => s.users.unshift(u as any))
    return { id: u.id }
  }
  const userId = parseId(p, "/admin/users") || parseId(p, "/users")
  if (userId && (p.startsWith("/admin/users/") || p.startsWith("/users/"))) {
    if (method === "GET") {
      const item = getState().users.find((u) => u.id === userId)
      if (item) return { item }
    }
    if (method === "PATCH" || method === "PUT") {
      mutateState((s) => { const u = s.users.find((x) => x.id === userId); if (u && body) Object.assign(u, body) })
      return ok()
    }
    if (method === "DELETE") {
      mutateState((s) => { s.users = s.users.filter((u) => u.id !== userId) }); return ok()
    }
  }

  // Storage backends ----------------------------------------------------
  if ((p === "/admin/storage-backends" || p === "/storage-backends") && method === "GET") {
    return { items: getState().storageBackends }
  }
  if ((p === "/admin/storage-backends" || p === "/storage-backends") && method === "POST") {
    const sb = { id: newId("sb"), name: body?.name || "new-backend", kind: body?.kind || "local_file", capabilities: {} as any, is_default: !!body?.is_default, created_at: nowIso() }
    mutateState((s) => s.storageBackends.unshift(sb as any))
    return { id: sb.id }
  }
  // Network scan endpoints (NFS/iSCSI/SMB)
  if (p.endsWith("/scan") && (p.includes("nfs") || p.includes("iscsi"))) {
    if (p.includes("iscsi")) return { targets: [{ portal: "10.0.0.50:3260", iqn: "iqn.2024-01.com.example:storage1" }] }
    return { exports: [{ path: "/exports/nqr", allowed: "10.0.0.0/24" }] }
  }
  if (p.endsWith("/health") && p.includes("storage")) {
    return { reachable: true, status: "ok", used_bytes: 50 * 1024 ** 3, total_bytes: 1024 ** 4 }
  }

  // Backup targets ------------------------------------------------------
  if (p === "/admin/backup-targets" || p === "/backup-targets") {
    if (method === "GET") return { items: getState().backupTargets }
    if (method === "POST") {
      const bt = { id: newId("bt"), ...body, created_at: nowIso() }
      mutateState((s) => s.backupTargets.unshift(bt as any))
      return bt
    }
  }
  if ((p.startsWith("/admin/backup-targets/") || p.startsWith("/backup-targets/")) && method === "DELETE") {
    const id = p.split("/").pop()!
    mutateState((s) => { s.backupTargets = s.backupTargets.filter((b) => b.id !== id) })
    return ok()
  }

  // Metrics -------------------------------------------------------------
  if (p.includes("/metrics")) {
    if (p.startsWith("/hosts/")) {
      const hostId = p.split("/")[2]
      return fakeMetricsSeries().map((m) => ({ host_id: hostId, recorded_at: m.recorded_at, cpu_usage_percent: m.value, memory_used_mb: 30000 + (m.value * 100), memory_total_mb: 65536, disk_used_gb: 400, disk_total_gb: 1024 }))
    }
    if (p.startsWith("/vms/")) {
      const vmId = p.split("/")[2]
      return fakeMetricsSeries(Date.now(), 60, 40, 25).map((m) => ({ vm_id: vmId, recorded_at: m.recorded_at, cpu_usage_percent: m.value, memory_usage_percent: m.value, memory_used_kb: 1024 * 1024, memory_total_kb: 2 * 1024 * 1024, load_average: m.value / 25 }))
    }
    if (p.startsWith("/containers/")) {
      const cid = p.split("/")[2]
      return fakeMetricsSeries(Date.now(), 60, 30, 15).map((m) => ({ container_id: cid, recorded_at: m.recorded_at, cpu_percent: m.value, memory_used_mb: m.value * 5, memory_limit_mb: 1024, network_rx_bytes: 1024 * 1024, network_tx_bytes: 1024 * 1024, block_read_bytes: 4096, block_write_bytes: 4096, pids: 8 }))
    }
  }

  // Generic safe default: list-looking shape vs single-item -------------
  if (method === "GET") {
    if (p.endsWith("s") || p.includes("list")) return { items: [], total: 0 }
    return { item: null }
  }
  if (method === "DELETE") return ok()
  if (method === "POST" || method === "PUT" || method === "PATCH") return { ok: true, demo: true }

  return null
}

export { DEMO_MODE }
