# OmniLink API 文档

> 版本: v0.1.0 | 最后更新: 2026-05-16

## 概述

OmniLink 即时通讯系统 REST API，支持用户认证、即时消息、会话管理、端到端加密等功能。

### 基础信息

| 项目 | 值 |
|------|-----|
| Base URL | `http://localhost:8080/api` |
| 认证方式 | Bearer Token (JWT) |
| 内容类型 | `application/json` |
| WebSocket | `ws://localhost:8080/ws` |

### 认证说明

所有需要认证的 API 需要在请求头中携带 JWT Token：

```
Authorization: Bearer <your-jwt-token>
```

---

## 认证 API

### 注册用户

```
POST /api/auth/register
```

**请求体：**
```json
{
  "username": "testuser",
  "email": "test@example.com",
  "password": "TestPassword123!",
  "displayName": "Test User"
}
```

**响应 (200):**
```json
{
  "id": "uuid",
  "username": "testuser",
  "email": "test@example.com",
  "displayName": "Test User",
  "token": "jwt-token"
}
```

### 用户登录

```
POST /api/auth/login
```

**请求体：**
```json
{
  "username": "testuser",
  "password": "TestPassword123!"
}
```

**响应 (200):**
```json
{
  "token": "jwt-token",
  "user": {
    "id": "uuid",
    "username": "testuser",
    "email": "test@example.com"
  }
}
```

### 获取当前用户

```
GET /api/auth/me
Authorization: Bearer <token>
```

---

## 消息 API

### 发送消息

```
POST /api/im/messages
Authorization: Bearer <token>
```

**请求体：**
```json
{
  "conversationId": "uuid",
  "content": "消息内容",
  "contentType": "text",
  "metadata": {
    "replyTo": "uuid",
    "mentions": ["user1", "user2"]
  }
}
```

**contentType 支持：**
- `text` - 文本消息
- `image` - 图片消息
- `file` - 文件消息
- `audio` - 语音消息
- `video` - 视频消息
- `location` - 位置消息
- `system` - 系统消息

### 获取消息历史

```
GET /api/im/conversations/{conversation_id}/messages?limit=20&offset=0&before=timestamp
Authorization: Bearer <token>
```

**查询参数：**
| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| limit | int | 20 | 返回数量限制 |
| offset | int | 0 | 偏移量 |
| before | timestamp | - | 获取此时间之前的消息 |
| after | timestamp | - | 获取此时间之后的消息 |

### 编辑消息

```
PUT /api/im/messages/{message_id}
Authorization: Bearer <token>
```

**请求体：**
```json
{
  "content": "编辑后的消息内容"
}
```

### 撤回消息

```
DELETE /api/im/messages/{message_id}
Authorization: Bearer <token>
```

### 批量发送消息

```
POST /api/im/messages/batch
Authorization: Bearer <token>
```

**请求体：**
```json
{
  "messages": [
    {
      "conversationId": "uuid1",
      "content": "消息1",
      "contentType": "text"
    },
    {
      "conversationId": "uuid2",
      "content": "消息2",
      "contentType": "text"
    }
  ]
}
```

### 标记消息已读

```
POST /api/im/messages/{message_id}/read
Authorization: Bearer <token>
```

### 获取已读回执

```
GET /api/im/messages/{message_id}/receipts
Authorization: Bearer <token>
```

### 搜索消息

```
GET /api/im/messages/search?q=keyword&limit=20
Authorization: Bearer <token>
```

### 全局搜索

```
GET /api/im/messages/search/global?q=keyword
Authorization: Bearer <token>
```

---

## 会话 API

### 获取会话列表

```
GET /api/im/conversations?limit=20&offset=0
Authorization: Bearer <token>
```

### 创建会话

```
POST /api/im/conversations
Authorization: Bearer <token>
```

**请求体（单聊）：**
```json
{
  "type": "direct",
  "participantIds": ["user-uuid"],
  "name": null,
  "metadata": {}
}
```

**请求体（群聊）：**
```json
{
  "type": "group",
  "participantIds": ["user1-uuid", "user2-uuid", "user3-uuid"],
  "name": "群组名称",
  "metadata": {
    "description": "群组描述"
  }
}
```

### 搜索会话

```
GET /api/im/conversations/search?q=keyword
Authorization: Bearer <token>
```

### 置顶会话

```
POST /api/im/conversations/{conversation_id}/pin
Authorization: Bearer <token>
```

### 免打扰

```
POST /api/im/conversations/{conversation_id}/mute
Authorization: Bearer <token>
```

### 归档会话

```
POST /api/im/conversations/{conversation_id}/archive
Authorization: Bearer <token>
```

---

## 端到端加密 API

### 生成密钥对

```
POST /api/im/encryption/keys
Authorization: Bearer <token>
```

**请求体：**
```json
{
  "keyType": "identity",
  "keyVersion": 1
}
```

### 注册公钥

```
POST /api/im/encryption/register-key
Authorization: Bearer <token>
```

**请求体：**
```json
{
  "keyType": "identity",
  "publicKey": "base64-encoded-public-key",
  "keyVersion": 1
}
```

**keyType 支持：**
- `identity` - 身份密钥
- `signed_pre_key` - 签名预密钥
- `one_time_pre_key` - 一次性预密钥

**响应 (200):**
```json
{
  "id": "uuid",
  "userId": "uuid",
  "publicKey": "base64-encoded",
  "keyType": "identity",
  "keyVersion": 1,
  "isActive": true,
  "createdAt": "2026-05-16T00:00:00Z"
}
```

### 获取用户公钥

```
GET /api/im/encryption/public-key/{user_id}?keyType=identity
Authorization: Bearer <token>
```

**响应 (200):**
```json
{
  "id": "uuid",
  "userId": "uuid",
  "publicKey": "base64-encoded",
  "keyType": "identity",
  "keyVersion": 1,
  "createdAt": "2026-05-16T00:00:00Z"
}
```

### 批量获取公钥

```
POST /api/im/encryption/public-keys/batch
Authorization: Bearer <token>
```

**请求体：**
```json
{
  "userIds": ["uuid1", "uuid2", "uuid3"],
  "keyType": "identity"
}
```

**响应 (200):**
```json
{
  "keys": [
    {
      "userId": "uuid1",
      "publicKey": "base64-encoded",
      "keyType": "identity",
      "keyVersion": 1
    }
  ]
}
```

### 加密消息

```
POST /api/im/encryption/encrypt
Authorization: Bearer <token>
```

### 解密消息

```
POST /api/im/encryption/decrypt
Authorization: Bearer <token>
```

### 密钥交换

```
POST /api/im/encryption/key-exchange
Authorization: Bearer <token>
```

### 获取加密消息历史

```
GET /api/im/encryption/messages/{conversation_id}
Authorization: Bearer <token>
```

---

## 用户状态 API

### 更新在线状态

```
POST /api/im/presence
Authorization: Bearer <token>
```

**请求体：**
```json
{
  "status": "online",
  "statusText": "在线状态描述"
}
```

**status 支持：**
- `online` - 在线
- `away` - 离开
- `busy` - 忙碌
- `offline` - 离线

### 获取用户状态

```
GET /api/im/presence/{user_id}
Authorization: Bearer <token>
```

### 批量获取状态

```
POST /api/im/presence/batch
Authorization: Bearer <token>
```

**请求体：**
```json
{
  "userIds": ["uuid1", "uuid2", "uuid3"]
}
```

---

## 联系人 API

### 添加联系人

```
POST /api/im/contacts
Authorization: Bearer <token>
```

### 获取联系人列表

```
GET /api/im/contacts
Authorization: Bearer <token>
```

### 搜索用户

```
GET /api/im/contacts/search?q=keyword
Authorization: Bearer <token>
```

---

## 公告 API

### 创建公告

```
POST /api/im/announcements
Authorization: Bearer <token>
```

### 获取所有公告

```
GET /api/im/announcements
Authorization: Bearer <token>
```

### 获取活跃公告

```
GET /api/im/announcements/active
Authorization: Bearer <token>
```

### 标记公告已读

```
POST /api/im/announcements/{announcement_id}/read
Authorization: Bearer <token>
```

---

## 快捷回复 API

### 创建快捷回复

```
POST /api/im/quick-replies
Authorization: Bearer <token>
```

### 获取快捷回复列表

```
GET /api/im/quick-replies
Authorization: Bearer <token>
```

---

## 反馈 API

### 提交反馈

```
POST /api/im/feedbacks
Authorization: Bearer <token>
```

### 获取所有反馈

```
GET /api/im/feedbacks
Authorization: Bearer <token>
```

---

## 聊天导出 API

### 创建导出任务

```
POST /api/im/chat-export
Authorization: Bearer <token>
```

### 下载导出文件

```
GET /api/im/chat-export/{export_id}/download
Authorization: Bearer <token>
```

---

## 消息重试 API

### 重试失败消息

```
POST /api/im/messages/{message_id}/retry
Authorization: Bearer <token>
```

### 获取失败消息列表

```
GET /api/im/messages/failed
Authorization: Bearer <token>
```

---

## 健康检查 API

### 服务健康检查

```
GET /api/health
```

**响应 (200):**
```json
{
  "status": "healthy",
  "version": "0.1.0",
  "uptime": 3600,
  "dependencies": {
    "database": "connected",
    "redis": "connected"
  }
}
```

---

## WebSocket API

### 连接

```
ws://localhost:8080/ws?token=<jwt-token>
```

### 消息格式

**发送消息：**
```json
{
  "type": "message",
  "conversationId": "uuid",
  "content": "消息内容",
  "contentType": "text"
}
```

**接收消息：**
```json
{
  "type": "message",
  "id": "uuid",
  "conversationId": "uuid",
  "senderId": "uuid",
  "content": "消息内容",
  "contentType": "text",
  "timestamp": "2026-05-16T00:00:00Z"
}
```

**心跳：**
```json
{"type": "ping"}
```
响应：
```json
{"type": "pong"}
```

---

## 错误响应格式

所有错误响应遵循统一格式：

```json
{
  "error": {
    "code": "ERROR_CODE",
    "message": "错误描述",
    "details": {}
  }
}
```

### 常见错误码

| HTTP 状态码 | 错误码 | 说明 |
|------------|--------|------|
| 400 | BAD_REQUEST | 请求参数错误 |
| 401 | UNAUTHORIZED | 未认证或令牌无效 |
| 403 | FORBIDDEN | 权限不足 |
| 404 | NOT_FOUND | 资源不存在 |
| 409 | CONFLICT | 资源冲突（如重复注册） |
| 429 | TOO_MANY_REQUESTS | 请求过于频繁 |
| 500 | INTERNAL_ERROR | 服务器内部错误 |

---

## 速率限制

| API | 限制 |
|-----|------|
| 认证 API | 10 次/分钟 |
| 消息 API | 100 次/分钟 |
| 文件上传 | 10 次/分钟 |
| WebSocket 消息 | 60 条/分钟 |

---

## 附录：Swagger UI

启动服务后访问 Swagger UI：

```
http://localhost:8080/swagger-ui/
```

OpenAPI JSON 文档：

```
http://localhost:8080/api-docs/openapi.json
```
