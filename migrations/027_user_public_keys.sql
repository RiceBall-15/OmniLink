-- 用户公钥注册表（E2E加密支持）
-- 存储用户的身份公钥，用于端到端加密密钥交换

CREATE TABLE IF NOT EXISTS user_public_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    public_key TEXT NOT NULL,
    key_type VARCHAR(20) NOT NULL DEFAULT 'identity',  -- identity, signed_pre_key, one_time_pre_key
    key_version INTEGER NOT NULL DEFAULT 1,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMP,
    UNIQUE(user_id, key_type, key_version)
);

-- 索引：快速查找用户的活跃公钥
CREATE INDEX idx_user_public_keys_user_active 
    ON user_public_keys(user_id, is_active) 
    WHERE is_active = true;

-- 索引：按密钥类型查询
CREATE INDEX idx_user_public_keys_type 
    ON user_public_keys(key_type, is_active);

-- 公钥注册日志表（审计用）
CREATE TABLE IF NOT EXISTS key_exchange_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    initiator_id UUID NOT NULL REFERENCES users(id),
    responder_id UUID NOT NULL REFERENCES users(id),
    conversation_id UUID,
    exchange_type VARCHAR(20) NOT NULL DEFAULT 'initial',  -- initial, rotation
    status VARCHAR(20) NOT NULL DEFAULT 'pending',  -- pending, completed, failed
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMP
);

CREATE INDEX idx_key_exchange_log_initiator 
    ON key_exchange_log(initiator_id, created_at DESC);
CREATE INDEX idx_key_exchange_log_responder 
    ON key_exchange_log(responder_id, created_at DESC);
