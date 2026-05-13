-- Migration 016: Add burn-after-reading support to messages
-- 阅后即焚功能：消息被阅读后在指定时间后自动删除

-- 添加阅后即焚相关字段到 messages 表
ALTER TABLE messages
ADD COLUMN IF NOT EXISTS burn_after_reading BOOLEAN DEFAULT FALSE,
ADD COLUMN IF NOT EXISTS burn_after_seconds INTEGER DEFAULT NULL,
ADD COLUMN IF NOT EXISTS burned_at TIMESTAMP WITH TIME ZONE DEFAULT NULL;

-- 创建 burn_after_reading_config 表（可选：全局默认配置）
CREATE TABLE IF NOT EXISTS burn_after_reading_config (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    default_enabled BOOLEAN DEFAULT FALSE,
    default_ttl_seconds INTEGER DEFAULT 30,  -- 默认30秒后焚毁
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(user_id)
);

-- 创建索引：加速查询待焚毁消息
CREATE INDEX IF NOT EXISTS idx_messages_burn_pending
ON messages (burn_after_reading, read_at, burn_after_seconds)
WHERE burn_after_reading = TRUE AND burned_at IS NULL AND read_at IS NOT NULL;

-- 注释
COMMENT ON COLUMN messages.burn_after_reading IS '是否为阅后即焚消息';
COMMENT ON COLUMN messages.burn_after_seconds IS '阅读后多少秒焚毁（NULL表示不焚毁）';
COMMENT ON COLUMN messages.burned_at IS '消息被焚毁的时间（NULL表示未焚毁）';
