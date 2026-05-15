-- 数据保留策略表
CREATE TABLE IF NOT EXISTS data_retention_policies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) NOT NULL UNIQUE,
    description TEXT,
    retention_days INTEGER NOT NULL CHECK (retention_days > 0),
    target_table VARCHAR(50) NOT NULL,
    is_enabled BOOLEAN NOT NULL DEFAULT true,
    last_run_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE data_retention_policies IS '数据保留策略配置，用于自动清理过期数据';

-- 索引
CREATE INDEX IF NOT EXISTS idx_retention_policies_enabled ON data_retention_policies(is_enabled) WHERE is_enabled = true;
