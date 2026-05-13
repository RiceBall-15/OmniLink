-- 会话最后活跃时间优化
-- 1. 添加 last_message_at 列到 conversations 表
-- 2. 创建 conversation_user_state 表实现精确的每用户未读计数
-- 3. 添加 last_message_preview 列

-- Step 1: Add last_message_at column to conversations
ALTER TABLE conversations ADD COLUMN IF NOT EXISTS last_message_at TIMESTAMPTZ DEFAULT NOW();

-- Step 2: Add last_message_preview column to conversations
ALTER TABLE conversations ADD COLUMN IF NOT EXISTS last_message_preview TEXT;

-- Step 3: Create per-user conversation state table
CREATE TABLE IF NOT EXISTS conversation_user_state (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id UUID NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    user_id UUID NOT NULL,
    last_read_at TIMESTAMPTZ DEFAULT NOW(),
    unread_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(conversation_id, user_id)
);

-- Step 4: Create index for efficient queries
CREATE INDEX IF NOT EXISTS idx_conversation_user_state_user ON conversation_user_state(user_id);
CREATE INDEX IF NOT EXISTS idx_conversation_user_state_conv ON conversation_user_state(conversation_id);
CREATE INDEX IF NOT EXISTS idx_conversations_last_message_at ON conversations(last_message_at DESC);

-- Step 5: Initialize last_message_at from existing messages
UPDATE conversations c
SET last_message_at = COALESCE(
    (SELECT MAX(m.created_at) FROM messages m WHERE m.conversation_id = c.id),
    c.updated_at
),
last_message_preview = COALESCE(
    (SELECT LEFT(m.content, 50) FROM messages m WHERE m.conversation_id = c.id ORDER BY m.created_at DESC LIMIT 1),
    ''
);

-- Step 6: Initialize conversation_user_state for existing participants
INSERT INTO conversation_user_state (conversation_id, user_id, last_read_at, unread_count)
SELECT
    cp.conversation_id,
    cp.user_id,
    NOW(),
    0
FROM conversation_participants cp
ON CONFLICT (conversation_id, user_id) DO NOTHING;
