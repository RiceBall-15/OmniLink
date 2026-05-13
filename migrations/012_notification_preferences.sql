-- 会话通知偏好设置表
CREATE TABLE IF NOT EXISTS conversation_notification_preferences (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    conversation_id UUID NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    muted BOOLEAN NOT NULL DEFAULT false,
    sound VARCHAR(50) NOT NULL DEFAULT 'default',
    badge BOOLEAN NOT NULL DEFAULT true,
    mention_only BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, conversation_id)
);

-- 索引：快速查找用户的会话通知偏好
CREATE INDEX idx_conv_notif_pref_user ON conversation_notification_preferences(user_id);
CREATE INDEX idx_conv_notif_pref_conv ON conversation_notification_preferences(conversation_id);

-- 全局通知设置表
CREATE TABLE IF NOT EXISTS global_notification_settings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE UNIQUE,
    enabled BOOLEAN NOT NULL DEFAULT true,
    sound VARCHAR(50) NOT NULL DEFAULT 'default',
    badge BOOLEAN NOT NULL DEFAULT true,
    preview BOOLEAN NOT NULL DEFAULT true,
    dnd_start VARCHAR(5),  -- HH:MM 格式
    dnd_end VARCHAR(5),    -- HH:MM 格式
    dnd_timezone VARCHAR(50) DEFAULT 'UTC',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 索引：按用户快速查找全局通知设置
CREATE INDEX idx_global_notif_user ON global_notification_settings(user_id);
