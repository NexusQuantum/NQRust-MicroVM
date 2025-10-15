import { z } from "zod"

export const machineConfigSchema = z.object({
  vcpu_count: z.number().min(1, "Must have at least 1 vCPU").max(32, "Cannot exceed 32 vCPUs"),
  mem_size_mib: z.number().min(128, "Memory must be at least 128 MiB"),
  smt: z.boolean(),
  cpu_template: z.string().min(1, "CPU template is required"),
})

export const bootSourceSchema = z.object({
  kernel_image_path: z.string().min(1, "Kernel image path is required"),
  initrd_path: z.string().optional(),
  boot_args: z.string().optional(),
})

export const driveConfigSchema = z.object({
  drive_id: z.string().min(1, "Drive ID is required"),
  path_on_host: z.string().min(1, "Host path is required"),
  is_root_device: z.boolean(),
  is_read_only: z.boolean(),
  cache_type: z.enum(["Unsafe", "Writeback"]),
  io_engine: z.enum(["Sync", "Async"]),
  rate_limiter: z
    .object({
      bandwidth: z
        .object({
          size: z.number().min(0),
          one_time_burst: z.number().min(0),
          refill_time: z.number().min(0),
        })
        .optional(),
      ops: z
        .object({
          size: z.number().min(0),
          one_time_burst: z.number().min(0),
          refill_time: z.number().min(0),
        })
        .optional(),
    })
    .optional(),
})

export const networkConfigSchema = z.object({
  iface_id: z
    .string()
    .min(1, "Interface ID is required")
    .regex(/^eth[1-9]\d*$/, "Interface ID must be eth<index> and start at eth1"),
  host_dev_name: z
    .string()
    .min(1, "Host device name is required")
    .regex(/^tap-[a-zA-Z0-9-]+$/, "Host device must match tap-<identifier>"),
  guest_mac: z.string().optional(),
  allow_mmds_requests: z.boolean(),
  rx_rate_limiter: z
    .object({
      size: z.number().min(0),
      one_time_burst: z.number().min(0),
      refill_time: z.number().min(0),
    })
    .optional(),
  tx_rate_limiter: z
    .object({
      size: z.number().min(0),
      one_time_burst: z.number().min(0),
      refill_time: z.number().min(0),
    })
    .optional(),
})

export const vmMetadataSchema = z.object({
  name: z.string().min(1, "VM name is required").max(100, "Name cannot exceed 100 characters"),
  description: z.string().max(500, "Description cannot exceed 500 characters").optional(),
  tags: z.record(z.string()),
})
