-- Migration: Add resource ownership tracking
-- This migration adds created_by_user_id to all resource tables
-- to support role-based access control with resource ownership

-- Add created_by_user_id to vm table
ALTER TABLE vm ADD COLUMN IF NOT EXISTS created_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS idx_vm_created_by ON vm(created_by_user_id);

-- Add created_by_user_id to containers table
ALTER TABLE containers ADD COLUMN IF NOT EXISTS created_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS idx_containers_created_by ON containers(created_by_user_id);

-- Add created_by_user_id to function table
ALTER TABLE function ADD COLUMN IF NOT EXISTS created_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS idx_function_created_by ON function(created_by_user_id);

-- Add created_by_user_id to network table
ALTER TABLE network ADD COLUMN IF NOT EXISTS created_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS idx_network_created_by ON network(created_by_user_id);

-- Add created_by_user_id to volume table
ALTER TABLE volume ADD COLUMN IF NOT EXISTS created_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS idx_volume_created_by ON volume(created_by_user_id);

-- Add comments for documentation
COMMENT ON COLUMN vm.created_by_user_id IS 'User who created this VM (for ownership-based access control)';
COMMENT ON COLUMN containers.created_by_user_id IS 'User who created this container (for ownership-based access control)';
COMMENT ON COLUMN function.created_by_user_id IS 'User who created this function (for ownership-based access control)';
COMMENT ON COLUMN network.created_by_user_id IS 'User who created this network (for ownership-based access control)';
COMMENT ON COLUMN volume.created_by_user_id IS 'User who created this volume (for ownership-based access control)';

-- Note: Existing resources will have NULL created_by_user_id (no owner)
-- Admins can access resources with NULL owners
