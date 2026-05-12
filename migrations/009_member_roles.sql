-- 添加成员角色字段
ALTER TABLE conversation_members ADD COLUMN IF NOT EXISTS role VARCHAR(20) NOT NULL DEFAULT 'member';

-- 更新现有记录：群主设置为 owner
UPDATE conversation_members cm
SET role = 'owner'
FROM conversations c
WHERE cm.conversation_id = c.id
  AND cm.user_id = c.owner_id
  AND c.type = 'group';

CREATE INDEX IF NOT EXISTS idx_conversation_members_role ON conversation_members(role);
