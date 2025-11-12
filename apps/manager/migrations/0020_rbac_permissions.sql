-- Migration: Add RBAC permissions and audit logging
-- This migration adds:
-- 1. Viewer role support
-- 2. Last login tracking
-- 3. Audit log table for tracking all user actions

-- Add viewer role to users table
ALTER TABLE users DROP CONSTRAINT IF EXISTS users_role_check;
ALTER TABLE users ADD CONSTRAINT users_role_check CHECK (role IN ('admin', 'user', 'viewer'));

-- Add last_login_at column to track user login times
ALTER TABLE users ADD COLUMN IF NOT EXISTS last_login_at TIMESTAMPTZ;

-- Create index for role-based queries
CREATE INDEX IF NOT EXISTS idx_users_role ON users(role);

-- Create audit_logs table for tracking all user actions
CREATE TABLE IF NOT EXISTS audit_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    username TEXT NOT NULL, -- Store username for historical reference even if user is deleted
    action TEXT NOT NULL, -- e.g., 'login', 'create_vm', 'delete_function', 'update_user'
    resource_type TEXT, -- e.g., 'vm', 'function', 'container', 'network', 'volume', 'user'
    resource_id UUID, -- ID of the resource being acted upon
    details JSONB, -- Additional context (e.g., VM name, previous values, etc.)
    ip_address TEXT, -- Client IP address
    success BOOLEAN NOT NULL DEFAULT true, -- Whether the action succeeded
    error_message TEXT, -- Error message if action failed
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Create indexes for efficient audit log queries
CREATE INDEX IF NOT EXISTS idx_audit_logs_user_id ON audit_logs(user_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_action ON audit_logs(action);
CREATE INDEX IF NOT EXISTS idx_audit_logs_resource_type ON audit_logs(resource_type);
CREATE INDEX IF NOT EXISTS idx_audit_logs_created_at ON audit_logs(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_audit_logs_resource ON audit_logs(resource_type, resource_id);

-- Create composite index for common query patterns
CREATE INDEX IF NOT EXISTS idx_audit_logs_user_action_time ON audit_logs(user_id, action, created_at DESC);

-- Add comment for documentation
COMMENT ON TABLE audit_logs IS 'Audit trail for all user actions in the system';
COMMENT ON COLUMN audit_logs.action IS 'Action performed (e.g., login, create_vm, delete_function)';
COMMENT ON COLUMN audit_logs.details IS 'JSONB field containing action-specific context';
