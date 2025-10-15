-- Add size_bytes column to vm_drive table
-- This stores the original size for auto-provisioned drives
-- Allows recreating sparse files after VM stop/start cycles
ALTER TABLE vm_drive ADD COLUMN size_bytes BIGINT;
