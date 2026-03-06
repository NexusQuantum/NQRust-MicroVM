import type { ResolvedSchema } from "./openapi-types"

const FIELD_SAMPLES: Record<string, unknown> = {
  id: "550e8400-e29b-41d4-a716-446655440000",
  vm_id: "550e8400-e29b-41d4-a716-446655440000",
  host_id: "661f9500-f3ac-52e5-b827-557766551111",
  drive_id: "rootfs",
  nic_id: "772e0611-04bd-63f6-c938-668877662222",
  network_id: "883f1722-15ce-74g7-da49-779988773333",
  function_id: "994g2833-26df-85h8-eb5a-880099884444",
  container_id: "aa5h3944-37eg-96i9-fc6b-991100995555",
  snapshot_id: "bb6i4a55-48fh-a7ja-gd7c-aa2211aa6666",
  template_id: "cc7j5b66-59gi-b8kb-he8d-bb3322bb7777",
  image_id: "dd8k6c77-6ahj-c9lc-if9e-cc4433cc8888",
  user_id: "ee9l7d88-7bik-damd-jgaf-dd5544dd9999",
  parent_id: "ff0m8e99-8cjl-ebne-khbg-ee6655eeaaaa",
  source_snapshot_id: "ff0m8e99-8cjl-ebne-khbg-ee6655eeaaaa",
  request_id: "req_abc123",
  name: "my-vm",
  username: "admin",
  password: "secure-password",
  token: "eyJhbGciOiJIUzI1NiIs...",
  handler: "handler.main",
  code: "exports.handler = async (event) => { return { statusCode: 200 }; }",
  runtime: "nodejs18",
  image: "nginx:latest",
  kind: "rootfs",
  host_path: "/srv/images/ubuntu-22.04.ext4",
  sha256: "e3b0c44298fc1c149afb...",
  rootfs_path: "/srv/images/ubuntu-22.04.ext4",
  kernel_path: "/srv/images/vmlinux-5.10",
  log_path: "/var/log/firecracker/vm.log",
  api_sock: "/tmp/firecracker.socket",
  tap: "fc-tap0",
  fc_unit: "fc-vm-001",
  host_addr: "192.168.1.100:9090",
  guest_ip: "172.16.0.2",
  assigned_ip: "172.16.0.2",
  iface_id: "eth0",
  host_dev_name: "fc-tap0",
  guest_mac: "AA:FC:00:00:00:01",
  state: "running",
  status: "success",
  command: "/bin/sh",
  exec_id: "exec_abc123",
  addr: "192.168.1.100:9090",
  path: "/var/log/app.log",
  level: "Info",
  module: "api_server",
  stream: "stdout",
  message: "Container started successfully",
  text: "log output here...",
  restart_policy: "always",
  snapshot_type: "full",
  snapshot_path: "/srv/fc/snapshots/snap-001",
  mem_path: "/srv/fc/snapshots/snap-001/mem",
  protocol: "tcp",
  error: null,
  output: "command output",
  role: "admin",
}

const TYPE_SAMPLES: Record<string, unknown> = {
  string: "string",
  "string:uuid": "550e8400-e29b-41d4-a716-446655440000",
  "string:date-time": "2026-03-02T10:00:00Z",
  "integer:int32": 0,
  "integer:int64": 0,
  number: 0,
  "number:float": 0.0,
  boolean: true,
}

const NUMERIC_FIELD_SAMPLES: Record<string, number> = {
  vcpu: 4,
  mem_mib: 512,
  amount_mib: 256,
  memory_mb: 256,
  memory_limit_mb: 512,
  memory_used_mb: 128,
  timeout_seconds: 30,
  port: 8080,
  http_port: 8080,
  host: 8080,
  container: 80,
  duration_ms: 150,
  size: 1073741824,
  size_bytes: 1073741824,
  rootfs_size_mb: 2048,
  stats_polling_interval_s: 5,
  cpu_percent: 25.5,
  cpu_limit: 2.0,
  pids: 12,
  exit_code: 0,
  block_read_bytes: 1048576,
  block_write_bytes: 524288,
  network_rx_bytes: 2097152,
  network_tx_bytes: 1048576,
  uptime_seconds: 3600,
  grace_days_remaining: 14,
}

export function sampleValue(name: string, schema: ResolvedSchema): unknown {
  // Check field-specific samples first
  if (name in FIELD_SAMPLES) return FIELD_SAMPLES[name]
  if (name in NUMERIC_FIELD_SAMPLES) return NUMERIC_FIELD_SAMPLES[name]

  // Enums
  if (schema.enum && schema.enum.length > 0) return schema.enum[0]

  // Nullable
  if (schema.nullable && !schema.properties) return null

  // Arrays
  if (schema.type === "array" && schema.items) {
    return [sampleValue("item", schema.items)]
  }

  // Objects
  if (schema.type === "object" && schema.properties) {
    return buildSampleObject(schema)
  }

  // additionalProperties (maps)
  if (schema.additionalProperties && typeof schema.additionalProperties === "object") {
    return { key: sampleValue("value", schema.additionalProperties) }
  }

  // Type-based fallback
  const typeKey = schema.format ? `${schema.type}:${schema.format}` : (schema.type ?? "string")
  return TYPE_SAMPLES[typeKey] ?? "string"
}

export function buildSampleObject(schema: ResolvedSchema): Record<string, unknown> {
  if (!schema.properties) return {}
  const result: Record<string, unknown> = {}
  for (const [key, prop] of Object.entries(schema.properties)) {
    if (prop.nullable && !schema.required?.includes(key)) continue
    result[key] = sampleValue(key, prop)
  }
  return result
}
