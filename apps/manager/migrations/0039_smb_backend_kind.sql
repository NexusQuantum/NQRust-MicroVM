-- Allow 'smb' as a storage_backend.kind. Parallel to nfs as a file-protocol
-- backend; uses mount.cifs on the agent and Proxmox-style per-backend
-- credential files outside the DB.

ALTER TABLE storage_backend
    DROP CONSTRAINT IF EXISTS storage_backend_kind_check;

ALTER TABLE storage_backend
    ADD CONSTRAINT storage_backend_kind_check
    CHECK (kind IN ('local_file', 'iscsi', 'truenas_iscsi', 'spdk_lvol', 'nfs', 'iscsi_lvm', 'smb'));
