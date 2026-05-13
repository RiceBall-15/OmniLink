-- Migration 018: 用户反馈系统
-- 创建 user_feedbacks 表

CREATE TABLE IF NOT EXISTS user_feedbacks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    feedback_type VARCHAR(20) NOT NULL DEFAULT 'other' CHECK (feedback_type IN ('bug', 'feature', 'other')),
    content TEXT NOT NULL,
    contact_email VARCHAR(255),
    status VARCHAR(20) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'processing', 'resolved', 'rejected')),
    priority VARCHAR(20) NOT NULL DEFAULT 'medium' CHECK (priority IN ('low', 'medium', 'high', 'urgent')),
    admin_reply TEXT,
    replied_by UUID REFERENCES users(id),
    replied_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_user_feedbacks_user_id ON user_feedbacks(user_id);
CREATE INDEX IF NOT EXISTS idx_user_feedbacks_status ON user_feedbacks(status);
CREATE INDEX IF NOT EXISTS idx_user_feedbacks_type ON user_feedbacks(feedback_type);
CREATE INDEX IF NOT EXISTS idx_user_feedbacks_created_at ON user_feedbacks(created_at DESC);

-- 自动更新 updated_at 触发器
CREATE OR REPLACE TRIGGER update_user_feedbacks_updated_at
    BEFORE UPDATE ON user_feedbacks
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
