-- Add user preferences and avatar support
-- Migration: 0022_user_preferences.sql

-- Add preferences JSONB column for flexible user settings
ALTER TABLE users ADD COLUMN IF NOT EXISTS preferences JSONB DEFAULT '{}';

-- Add timezone preference
ALTER TABLE users ADD COLUMN IF NOT EXISTS timezone VARCHAR(50) DEFAULT 'UTC';

-- Add theme preference
ALTER TABLE users ADD COLUMN IF NOT EXISTS theme VARCHAR(20) DEFAULT 'dark';

-- Add avatar path (stores path to avatar file)
ALTER TABLE users ADD COLUMN IF NOT EXISTS avatar_path TEXT;

-- Create index for faster preference queries
CREATE INDEX IF NOT EXISTS idx_user_preferences ON users USING gin(preferences);

-- Add comment for documentation
COMMENT ON COLUMN users.preferences IS 'User preferences stored as JSONB (notifications, vmDefaults, etc.)';
COMMENT ON COLUMN users.timezone IS 'User timezone (IANA timezone identifier)';
COMMENT ON COLUMN users.theme IS 'User theme preference (dark/light)';
COMMENT ON COLUMN users.avatar_path IS 'Path to user avatar image (PNG, 500x500px)';
