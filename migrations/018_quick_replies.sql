-- 快捷回复模板表
CREATE TABLE IF NOT EXISTS quick_replies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title VARCHAR(100) NOT NULL,
    content TEXT NOT NULL,
    category VARCHAR(50) DEFAULT 'general',
    sort_order INTEGER DEFAULT 0,
    is_global BOOLEAN DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 索引
CREATE INDEX idx_quick_replies_user_id ON quick_replies(user_id);
CREATE INDEX idx_quick_replies_category ON quick_replies(category);
CREATE INDEX idx_quick_replies_sort_order ON quick_replies(user_id, sort_order);

-- 全局快捷回复由管理员创建，user_id 设为创建者
COMMENT ON TABLE quick_replies IS '快捷回复模板';
COMMENT ON COLUMN quick_replies.is_global IS '是否为全局模板（管理员设置，所有用户可见）';
