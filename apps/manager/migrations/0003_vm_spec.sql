ALTER TABLE vm ADD COLUMN IF NOT EXISTS vcpu INT NOT NULL DEFAULT 1;
ALTER TABLE vm ADD COLUMN IF NOT EXISTS mem_mib INT NOT NULL DEFAULT 512;
ALTER TABLE vm ADD COLUMN IF NOT EXISTS kernel_path TEXT NOT NULL DEFAULT '';
ALTER TABLE vm ADD COLUMN IF NOT EXISTS rootfs_path TEXT NOT NULL DEFAULT '';

-- ensure existing rows have deterministic defaults
UPDATE vm
SET kernel_path = COALESCE(NULLIF(kernel_path, ''), ''),
    rootfs_path = COALESCE(NULLIF(rootfs_path, ''), '');
