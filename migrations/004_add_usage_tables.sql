-- Token使用记录表
CREATE TABLE IF NOT EXISTS token_usage (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    conversation_id UUID,
    model_name VARCHAR(100) NOT NULL,
    provider VARCHAR(50) NOT NULL,
    prompt_tokens INTEGER NOT NULL,
    completion_tokens INTEGER NOT NULL,
    total_tokens INTEGER NOT NULL,
    cost DECIMAL(10, 6) NOT NULL DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    CONSTRAINT fk_token_usage_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    CONSTRAINT fk_token_usage_conversation FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE SET NULL
);

-- API调用记录表
CREATE TABLE IF NOT EXISTS api_call (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    api_endpoint VARCHAR(500) NOT NULL,
    method VARCHAR(10) NOT NULL,
    status_code INTEGER NOT NULL,
    response_time_ms INTEGER NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    CONSTRAINT fk_api_call_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- 统计记录表
CREATE TABLE IF NOT EXISTS usage_stat (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID,
    stat_type VARCHAR(20) NOT NULL, -- 'daily', 'weekly', 'monthly'
    model_name VARCHAR(100),
    provider VARCHAR(50),
    total_tokens BIGINT NOT NULL DEFAULT 0,
    total_cost DECIMAL(12, 6) NOT NULL DEFAULT 0,
    request_count BIGINT NOT NULL DEFAULT 0,
    stat_date DATE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    CONSTRAINT fk_usage_stat_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    CONSTRAINT unique_stat UNIQUE (user_id, stat_type, model_name, provider, stat_date)
);

-- 创建索引以提升查询性能
CREATE INDEX IF NOT EXISTS idx_token_usage_user_id ON token_usage(user_id);
CREATE INDEX IF NOT EXISTS idx_token_usage_conversation_id ON token_usage(conversation_id);
CREATE INDEX IF NOT EXISTS idx_token_usage_created_at ON token_usage(created_at);
CREATE INDEX IF NOT EXISTS idx_token_usage_model ON token_usage(model_name, provider);
CREATE INDEX IF NOT EXISTS idx_api_call_user_id ON api_call(user_id);
CREATE INDEX IF NOT EXISTS idx_api_call_created_at ON api_call(created_at);
CREATE INDEX IF NOT EXISTS idx_usage_stat_user_id ON usage_stat(user_id);
CREATE INDEX IF NOT EXISTS idx_usage_stat_date ON usage_stat(stat_date);

-- 添加注释
COMMENT ON TABLE token_usage IS 'Token使用记录表';
COMMENT ON TABLE api_call IS 'API调用记录表';
COMMENT ON TABLE usage_stat IS '统计记录表';

COMMENT ON COLUMN token_usage.cost IS '费用，单位美元';
COMMENT ON COLUMN api_call.response_time_ms IS '响应时间，单位毫秒';
COMMENT ON COLUMN usage_stat.stat_type IS '统计类型：daily/weekly/monthly';