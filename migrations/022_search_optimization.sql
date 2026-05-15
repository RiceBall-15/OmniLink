-- 搜索优化：启用 pg_trgm 扩展和添加复合索引
-- Task 88: 搜索结果排序优化
-- Task 89: 会话未读计数优化
-- Task 92: 数据库查询优化

-- Step 1: 启用 pg_trgm 扩展（用于 similarity() 函数）
CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- Step 2: 为 messages 表的 content 字段添加 GIN 索引（支持 trigram 搜索）
CREATE INDEX IF NOT EXISTS idx_messages_content_trgm ON messages USING GIN (content gin_trgm_ops);

-- Step 3: 为 conversation_user_state 表添加复合索引
-- 优化批量未读计数查询
CREATE INDEX IF NOT EXISTS idx_conversation_user_state_user_conv
    ON conversation_user_state(user_id, conversation_id);

-- 优化未读消息会话列表查询（部分索引，只索引有未读消息的行）
CREATE INDEX IF NOT EXISTS idx_conversation_user_state_user_unread
    ON conversation_user_state(user_id, unread_count) WHERE unread_count > 0;

-- Step 4: 为 conversation_participants 表添加复合索引
-- 优化会话参与者的查询
CREATE INDEX IF NOT EXISTS idx_conversation_participants_user_conv
    ON conversation_participants(user_id, conversation_id);

-- Step 5: 为 messages 表添加复合索引
-- 优化按会话和时间排序的查询
CREATE INDEX IF NOT EXISTS idx_messages_conversation_created
    ON messages(conversation_id, created_at DESC);

-- 优化按发送者查询
CREATE INDEX IF NOT EXISTS idx_messages_sender_created
    ON messages(sender_id, created_at DESC);

-- Step 6: 为 conversations 表添加索引
-- 优化按更新时间排序的会话列表查询
CREATE INDEX IF NOT EXISTS idx_conversations_updated_at
    ON conversations(updated_at DESC);

-- Step 7: 分析表以更新统计信息
ANALYZE messages;
ANALYZE conversation_user_state;
ANALYZE conversation_participants;
ANALYZE conversations;
