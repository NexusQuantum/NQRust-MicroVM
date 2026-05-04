-- Add 'iscsi_lvm' to the allowed storage_backend.kind values. Mirrors
-- Proxmox VE's LVM-on-iSCSI mode: vendor-agnostic auto-provisioning of
-- per-VM block devices on top of any iSCSI target.
--
-- Prior migrations did not define an explicit CHECK constraint on
-- storage_backend.kind (allowed values were enforced in application code).
-- This migration introduces one so the database also rejects unknown kinds,
-- including the new 'iscsi_lvm' value. The DROP IF EXISTS keeps the migration
-- idempotent in case a constraint was added out-of-band.

ALTER TABLE storage_backend
    DROP CONSTRAINT IF EXISTS storage_backend_kind_check;

ALTER TABLE storage_backend
    ADD CONSTRAINT storage_backend_kind_check
    CHECK (kind IN ('local_file', 'iscsi', 'truenas_iscsi', 'spdk_lvol', 'nfs', 'iscsi_lvm'));
