-- User blocking system
CREATE TABLE IF NOT EXISTS blocked_users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    blocker_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    blocked_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(blocker_id, blocked_id)
);

-- Index for fast lookup of blocked users
CREATE INDEX idx_blocked_users_blocker ON blocked_users(blocker_id);
CREATE INDEX idx_blocked_users_blocked ON blocked_users(blocked_id);

-- Prevent self-blocking (check constraint)
ALTER TABLE blocked_users ADD CONSTRAINT chk_no_self_block CHECK (blocker_id != blocked_id);
