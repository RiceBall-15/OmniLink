-- 配置表
CREATE TABLE IF NOT EXISTS config (
    id SERIAL PRIMARY KEY,
    key VARCHAR(255) UNIQUE NOT NULL,
    value TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 1,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_by UUID
);

-- 配置历史表
CREATE TABLE IF NOT EXISTS config_history (
    id SERIAL PRIMARY KEY,
    key VARCHAR(255) NOT NULL,
    value TEXT NOT NULL,
    version INTEGER NOT NULL,
    updated_by UUID,
    change_reason TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- 配置订阅表
CREATE TABLE IF NOT EXISTS config_subscription (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    key VARCHAR(255) NOT NULL,
    subscriber VARCHAR(255) NOT NULL, -- 服务名称或用户ID
    callback_url TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    last_notified_at TIMESTAMP WITH TIME ZONE
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_config_key ON config(key);
CREATE INDEX IF NOT EXISTS idx_config_history_key ON config_history(key, version);
CREATE INDEX IF NOT EXISTS idx_config_subscription_key ON config_subscription(key);
CREATE INDEX IF NOT EXISTS idx_config_subscription_subscriber ON config_subscription(subscriber);

-- 添加注释
COMMENT ON TABLE config IS '配置项表';
COMMENT ON TABLE config_history IS '配置历史记录表';
COMMENT ON TABLE config_subscription IS '配置订阅表';

COMMENT ON COLUMN config.version IS '配置版本号';
COMMENT ON COLUMN config_history.change_reason IS '变更原因';
COMMENT ON COLUMN config_subscription.callback_url IS '回调通知URL';