-- Migration 024: Add admin user management and user activity tracking support
-- Adds columns needed for admin user management API and user activity tracking

-- 1. Ensure user_activity table exists (for tracking activity events)
CREATE TABLE IF NOT EXISTS user_activity (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    activity_type VARCHAR(50) NOT NULL,  -- 'login', 'logout', 'message_sent', 'file_upload', etc.
    description TEXT,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_user_activity_user_id ON user_activity(user_id);
CREATE INDEX IF NOT EXISTS idx_user_activity_created_at ON user_activity(created_at);
CREATE INDEX IF NOT EXISTS idx_user_activity_type ON user_activity(activity_type);

-- 2. Add last_active_at column to users if not exists
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'users' AND column_name = 'last_active_at') THEN
        ALTER TABLE users ADD COLUMN last_active_at TIMESTAMPTZ;
    END IF;
END $$;

-- 3. Add status column to users if not exists (for ban/suspend)
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'users' AND column_name = 'status') THEN
        ALTER TABLE users ADD COLUMN status VARCHAR(20) NOT NULL DEFAULT 'active';
    END IF;
END $$;

-- 4. Add online_status column to users if not exists
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'users' AND column_name = 'online_status') THEN
        ALTER TABLE users ADD COLUMN online_status VARCHAR(20) NOT NULL DEFAULT 'offline';
    END IF;
END $$;

-- 5. Create index for admin user queries
CREATE INDEX IF NOT EXISTS idx_users_status ON users(status);
CREATE INDEX IF NOT EXISTS idx_users_last_active_at ON users(last_active_at);

COMMENT ON TABLE user_activity IS '用户活动追踪表';
COMMENT ON COLUMN users.last_active_at IS '最后活跃时间';
COMMENT ON COLUMN users.status IS '用户状态: active, banned, suspended';
COMMENT ON COLUMN users.online_status IS '在线状态: online, offline, away, busy';
