-- 0040_vmm_backends.sql
-- Pluggable VMM backend foundation (Firecracker + QEMU).
--
-- All changes are additive / NULLABLE / DEFAULT-backed so that a release that
-- ships this migration can be rolled back to the prior manager binary without
-- schema fixes — per the DB Migration Policy in CLAUDE.md.

-- 1. Per-VM VMM backend, guest OS, boot mode, console transport.
ALTER TABLE vm ADD COLUMN IF NOT EXISTS vmm_kind TEXT NOT NULL DEFAULT 'firecracker';
ALTER TABLE vm ADD COLUMN IF NOT EXISTS guest_os TEXT NOT NULL DEFAULT 'linux_kernel';
ALTER TABLE vm ADD COLUMN IF NOT EXISTS boot_mode JSONB;
ALTER TABLE vm ADD COLUMN IF NOT EXISTS console_kind TEXT NOT NULL DEFAULT 'unix_serial';
ALTER TABLE vm ADD COLUMN IF NOT EXISTS vnc_listen TEXT;
ALTER TABLE vm ADD COLUMN IF NOT EXISTS firmware_path TEXT;
ALTER TABLE vm ADD COLUMN IF NOT EXISTS nvram_path TEXT;

DO $$
BEGIN
    BEGIN
        ALTER TABLE vm
            ADD CONSTRAINT vm_vmm_kind_chk
            CHECK (vmm_kind IN ('firecracker', 'qemu'));
    EXCEPTION WHEN duplicate_object THEN NULL; END;

    BEGIN
        ALTER TABLE vm
            ADD CONSTRAINT vm_guest_os_chk
            CHECK (guest_os IN ('linux_kernel', 'linux_disk', 'windows', 'other'));
    EXCEPTION WHEN duplicate_object THEN NULL; END;

    BEGIN
        ALTER TABLE vm
            ADD CONSTRAINT vm_console_kind_chk
            CHECK (console_kind IN ('unix_serial', 'pty', 'vnc'));
    EXCEPTION WHEN duplicate_object THEN NULL; END;
END$$;

-- Backfill boot_mode for existing rows from the implicit Linux-kernel layout.
-- Existing rows have kernel_path and rootfs_path set; their cmdline is the
-- standard FC default applied at boot time, so we leave it empty and let the
-- manager's selection logic fill it as before.
UPDATE vm
SET boot_mode = jsonb_build_object(
    'mode', 'linux_kernel',
    'kernel', kernel_path,
    'initrd', NULL,
    'cmdline', ''
)
WHERE boot_mode IS NULL AND kernel_path IS NOT NULL AND kernel_path <> '';

CREATE INDEX IF NOT EXISTS idx_vm_vmm_kind ON vm(vmm_kind);

-- 2. Snapshots are per-backend; restore refuses to load a snapshot whose
--    vmm_kind does not match the target VM's.
ALTER TABLE snapshot ADD COLUMN IF NOT EXISTS vmm_kind TEXT NOT NULL DEFAULT 'firecracker';
DO $$
BEGIN
    BEGIN
        ALTER TABLE snapshot
            ADD CONSTRAINT snapshot_vmm_kind_chk
            CHECK (vmm_kind IN ('firecracker', 'qemu'));
    EXCEPTION WHEN duplicate_object THEN NULL; END;
END$$;

-- 3. Image registry discriminator. `image_kind` is the strict enum that
--    drives VMM routing. The pre-existing `kind` column (free-form, used for
--    "docker", "kernel", etc.) is preserved untouched.
ALTER TABLE image ADD COLUMN IF NOT EXISTS image_kind TEXT NOT NULL DEFAULT 'linux_kernel';
ALTER TABLE image ADD COLUMN IF NOT EXISTS nvram_template_path TEXT;
ALTER TABLE image ADD COLUMN IF NOT EXISTS guest_os_hint TEXT;
ALTER TABLE image ADD COLUMN IF NOT EXISTS disk_format TEXT;

DO $$
BEGIN
    BEGIN
        ALTER TABLE image
            ADD CONSTRAINT image_image_kind_chk
            CHECK (image_kind IN ('linux_kernel', 'linux_disk', 'uefi_disk', 'installer_iso'));
    EXCEPTION WHEN duplicate_object THEN NULL; END;

    BEGIN
        ALTER TABLE image
            ADD CONSTRAINT image_uefi_nvram_chk
            CHECK (image_kind <> 'uefi_disk' OR nvram_template_path IS NOT NULL);
    EXCEPTION WHEN duplicate_object THEN NULL; END;
END$$;

CREATE INDEX IF NOT EXISTS idx_image_image_kind ON image(image_kind);

-- 4. Per-host inventory of which VMM kinds are installed. Manager refuses to
--    schedule a VM onto a host whose set doesn't include the requested kind.
ALTER TABLE host ADD COLUMN IF NOT EXISTS vmm_kinds_installed TEXT[] NOT NULL
    DEFAULT '{firecracker}';

COMMENT ON COLUMN host.vmm_kinds_installed IS
  'Array of VmmKind db strings (e.g. {firecracker,qemu}) the agent advertises support for. Updated on agent registration / heartbeat by probing for the corresponding VMM binary.';

-- 5. Per-host capacity tracking so the manager refuses to over-commit when
--    scheduling new VMs across both Firecracker and QEMU. Stored as
--    aggregates that the manager refreshes on VM create/delete.
ALTER TABLE host ADD COLUMN IF NOT EXISTS reserved_vcpu INT NOT NULL DEFAULT 0;
ALTER TABLE host ADD COLUMN IF NOT EXISTS reserved_mem_mib BIGINT NOT NULL DEFAULT 0;
ALTER TABLE host ADD COLUMN IF NOT EXISTS total_vcpu INT;
ALTER TABLE host ADD COLUMN IF NOT EXISTS total_mem_mib BIGINT;

COMMENT ON COLUMN host.reserved_vcpu IS
  'Sum of vcpu reserved across all running/paused VMs on this host (both Firecracker and QEMU). Manager refuses to create a new VM that would push reserved_vcpu past total_vcpu unless over-commit is explicitly allowed.';
COMMENT ON COLUMN host.reserved_mem_mib IS
  'Sum of mem_mib reserved across all running/paused VMs on this host (both Firecracker and QEMU).';
