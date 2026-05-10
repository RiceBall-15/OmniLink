# OmniLink 后端开发计划

## 📅 更新日期：2026-05-09

## 🎯 本次开发目标

### 阶段一：核心功能完善（优先级最高）

#### 1. 消息已读回执系统
- [ ] 实现 `mark_read` handler 的完整逻辑
- [ ] 添加消息已读状态数据库查询
- [ ] WebSocket 通知发送者消息已读
- [ ] 批量标记已读支持

#### 2. 消息转发/在线通知
- [ ] 实现消息转发到对话参与者
- [ ] 在线用户状态同步
- [ ] 输入状态指示器（正在输入...）

#### 3. 用户信息缓存
- [ ] 实现 `get_user_info` 从数据库加载
- [ ] Redis 缓存层
- [ ] 缓存失效策略

### 阶段二：AI 服务集成

#### 4. AI 模型对接
- [ ] 完善 OpenAI provider 实现
- [ ] 添加 Anthropic provider
- [ ] 添加国内模型支持（通义千问/文心一言）
- [ ] 流式响应支持

### 阶段三：文件服务

#### 5. 文件上传 API
- [ ] 实现文件上传 handler
- [ ] 文件类型验证
- [ ] 文件大小限制
- [ ] MinIO 集成

---

## 🔧 技术实现细节

### 消息已读回执

```rust
// 新增数据库查询
pub async fn mark_messages_read(
    &self,
    conversation_id: Uuid,
    user_id: Uuid,
    message_ids: Vec<Uuid>,
) -> Result<()> {
    sqlx::query(
        "UPDATE messages SET read_at = NOW() 
         WHERE id = ANY($1) AND conversation_id = $2"
    )
    .bind(&message_ids)
    .bind(conversation_id)
    .execute(&self.pool)
    .await?;
    Ok(())
}
```

### WebSocket 通知

```rust
// 通知发送者消息已读
let read_receipt = WSMessage {
    message_type: WSMessageType::ReadReceipt,
    conversation_id: Some(conversation_id),
    message_id: None,
    sender_id: Some(reader_id),
    content: None,
    timestamp: Some(Utc::now().timestamp()),
    data: Some(serde_json::json!({
        "message_ids": message_ids,
        "read_by": reader_id,
    })),
};

self.connection_manager
    .send_to_user(message.sender_id, read_receipt)
    .await;
```

---

## 📊 预期完成时间

- 阶段一：2-3 小时
- 阶段二：3-4 小时
- 阶段三：2-3 小时

总计：7-10 小时
