-- Add guest_ip column to vm table for guest agent communication
ALTER TABLE vm ADD COLUMN guest_ip VARCHAR(45);
CREATE INDEX idx_vm_guest_ip ON vm(guest_ip);
