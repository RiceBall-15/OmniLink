-- 消息重试队列表
-- 用于跟踪消息发送失败后的重试状态

CREATE TABLE IF NOT EXISTS message_retry_queue (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    message_id UUID NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    conversation_id UUID NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    sender_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    retry_count INTEGER NOT NULL DEFAULT 0,
    max_retries INTEGER NOT NULL DEFAULT 3,
    next_retry_at TIMESTAMP WITH TIME ZONE NOT NULL,
    last_error TEXT,
    status VARCHAR(20) NOT NULL DEFAULT 'pending', -- pending, retrying, succeeded, failed
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- 索引：快速查询待重试的消息
CREATE INDEX IF NOT EXISTS idx_message_retry_queue_status_next_retry 
ON message_retry_queue(status, next_retry_at) 
WHERE status IN ('pending', 'retrying');

-- 索引：按消息ID查询重试记录
CREATE INDEX IF NOT EXISTS idx_message_retry_queue_message_id 
ON message_retry_queue(message_id);

-- 索引：按发送者查询重试记录
CREATE INDEX IF NOT EXISTS idx_message_retry_queue_sender_id 
ON message_retry_queue(sender_id);

COMMENT ON TABLE message_retry_queue IS '消息发送重试队列';
COMMENT ON COLUMN message_retry_queue.retry_count IS '当前重试次数';
COMMENT ON COLUMN message_retry_queue.max_retries IS '最大重试次数（默认3次）';
COMMENT ON COLUMN message_retry_queue.next_retry_at IS '下次重试时间（指数退避）';
COMMENT ON COLUMN message_retry_queue.status IS '队列状态：pending, retrying, succeeded, failed';
