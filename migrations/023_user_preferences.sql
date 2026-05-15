-- 用户偏好设置表
-- 支持每个用户存储多个类别、多个键值对的偏好设置

CREATE TABLE IF NOT EXISTS user_preferences (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    category VARCHAR(50) NOT NULL,
    key VARCHAR(100) NOT NULL,
    value JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, category, key)
);

-- 索引：按用户查询所有偏好
CREATE INDEX idx_user_preferences_user_id ON user_preferences(user_id);

-- 索引：按用户+类别查询
CREATE INDEX idx_user_preferences_user_category ON user_preferences(user_id, category);

-- 注释
COMMENT ON TABLE user_preferences IS '用户偏好设置表';
COMMENT ON COLUMN user_preferences.category IS '偏好类别（theme/notification/chat/privacy等）';
COMMENT ON COLUMN user_preferences.key IS '偏好键名';
COMMENT ON COLUMN user_preferences.value IS '偏好值（JSON格式）';
