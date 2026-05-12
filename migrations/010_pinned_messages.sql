-- 创建会话置顶消息表
-- 用户可以将重要消息置顶在会话中，方便快速查看

CREATE TABLE IF NOT EXISTS pinned_messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id UUID NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    message_id UUID NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    pinned_by UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    pinned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(conversation_id, message_id)
);

-- 索引：按会话查询置顶消息（最常用）
CREATE INDEX IF NOT EXISTS idx_pinned_messages_conversation 
    ON pinned_messages(conversation_id, pinned_at DESC);

-- 索引：按消息查询（用于取消置顶）
CREATE INDEX IF NOT EXISTS idx_pinned_messages_message 
    ON pinned_messages(message_id);

-- 索引：按置顶人查询
CREATE INDEX IF NOT EXISTS idx_pinned_messages_pinned_by 
    ON pinned_messages(pinned_by);
