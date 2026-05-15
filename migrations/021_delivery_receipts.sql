-- 消息投递状态跟踪表
CREATE TABLE IF NOT EXISTS message_delivery_receipts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    message_id UUID NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status VARCHAR(20) NOT NULL DEFAULT 'delivered' CHECK (status IN ('sent', 'delivered', 'read')),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    UNIQUE(message_id, user_id)
);

-- 索引：按消息查询投递状态
CREATE INDEX IF NOT EXISTS idx_delivery_receipts_message_id ON message_delivery_receipts(message_id);

-- 索引：按用户查询投递状态
CREATE INDEX IF NOT EXISTS idx_delivery_receipts_user_id ON message_delivery_receipts(user_id);

-- 索引：按状态查询
CREATE INDEX IF NOT EXISTS idx_delivery_receipts_status ON message_delivery_receipts(status);
