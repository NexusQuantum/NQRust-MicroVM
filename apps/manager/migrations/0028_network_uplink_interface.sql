-- Add uplink_interface column to network table for bridged network reconciliation
ALTER TABLE network ADD COLUMN IF NOT EXISTS uplink_interface TEXT;
