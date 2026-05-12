# OmniLink IM API Documentation

**Version**: 0.1.0  
**Base URL**: `http://localhost:8002`  
**Last Updated**: 2026-05-13

---

## Overview

OmniLink IM API 是一个即时通讯系统的 REST API，支持用户认证、消息管理、会话管理、群组功能、端到端加密等特性。

### 认证方式

大多数 API 需要 JWT Token 认证。在请求头中添加：

```
Authorization: Bearer <your_jwt_token>
```

### 统一响应格式

所有 API 返回统一的 JSON 格式：

```json
{
  "success": true,
  "data": { ... },
  "error": null
}
```

错误响应：

```json
{
  "success": false,
  "data": null,
  "error": {
    "code": "ERROR_CODE",
    "message": "错误描述"
  }
}
```

### 通用错误码

| 错误码 | HTTP 状态码 | 说明 |
|--------|-------------|------|
| `INVALID_INPUT` | 400 | 请求参数验证失败 |
| `INVALID_EMAIL` | 400 | 邮箱格式不正确 |
| `INVALID_ID` | 400 | 无效的 ID 格式 |
| `EMPTY_CONTENT` | 400 | 消息内容不能为空 |
| `EMPTY_KEYWORD` | 400 | 搜索关键词不能为空 |
| `INVALID_CREDENTIALS` | 401 | 邮箱或密码错误 |
| `FORBIDDEN` | 403 | 无权访问此资源 |
| `USER_NOT_FOUND` | 404 | 用户不存在 |
| `MESSAGE_NOT_FOUND` | 404 | 消息不存在 |
| `EMAIL_EXISTS` | 400 | 邮箱已被注册 |
| `USERNAME_EXISTS` | 400 | 用户名已被占用 |

---

## 目录

1. [健康检查](#1-健康检查)
2. [认证 API](#2-认证-api)
3. [用户 API](#3-用户-api)
4. [会话 API](#4-会话-api)
5. [消息 API](#5-消息-api)
6. [群组管理 API](#6-群组管理-api)
7. [会话管理增强 API](#7-会话管理增强-api)
8. [标签管理 API](#8-标签管理-api)
9. [加密 API](#9-加密-api)
10. [数据模型](#10-数据模型)

---

## 1. 健康检查

### `GET /health`

检查服务是否正常运行。

**认证**: 不需要

**响应**:
```
IM API is healthy
```

---

## 2. 认证 API

### 2.1 用户注册

`POST /api/auth/register`

**认证**: 不需要

**请求体**:
```json
{
  "username": "alice",
  "email": "alice@example.com",
  "password": "password123"
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `username` | string | ✅ | 用户名，3-20个字符 |
| `email` | string | ✅ | 邮箱地址 |
| `password` | string | ✅ | 密码，至少8个字符 |

**响应** (201 Created):
```json
{
  "success": true,
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "username": "alice",
    "email": "alice@example.com",
    "avatar": null,
    "createdAt": "2026-05-13T00:00:00Z",
    "updatedAt": "2026-05-13T00:00:00Z"
  }
}
```

**错误响应**:
- `400` - 参数验证失败、邮箱已存在、用户名已存在

---

### 2.2 用户登录

`POST /api/auth/login`

**认证**: 不需要

**请求体**:
```json
{
  "email": "alice@example.com",
  "password": "password123"
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `email` | string | ✅ | 邮箱地址 |
| `password` | string | ✅ | 密码 |

**响应** (200 OK):
```json
{
  "success": true,
  "data": {
    "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "user": {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "username": "alice",
      "email": "alice@example.com",
      "avatar": null,
      "createdAt": "2026-05-13T00:00:00Z",
      "updatedAt": "2026-05-13T00:00:00Z"
    }
  }
}
```

**错误响应**:
- `400` - 参数验证失败
- `401` - 邮箱或密码错误

---

## 3. 用户 API

### 3.1 获取当前用户信息

`GET /api/user/me`

**认证**: ✅ 需要

**响应** (200 OK):
```json
{
  "success": true,
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "username": "alice",
    "email": "alice@example.com",
    "avatar": "https://example.com/avatar.png",
    "createdAt": "2026-05-13T00:00:00Z",
    "updatedAt": "2026-05-13T00:00:00Z"
  }
}
```

**错误响应**:
- `404` - 用户不存在

---

### 3.2 更新用户资料

`PUT /api/user/me`

**认证**: ✅ 需要

**请求体**:
```json
{
  "username": "new_name",
  "email": "new@example.com",
  "avatar": "https://example.com/new-avatar.png"
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `username` | string? | ❌ | 新用户名，3-20个字符 |
| `email` | string? | ❌ | 新邮箱地址 |
| `avatar` | string? | ❌ | 新头像 URL |

**响应** (200 OK):
```json
{
  "success": true,
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "username": "new_name",
    "email": "new@example.com",
    "avatar": "https://example.com/new-avatar.png",
    "createdAt": "2026-05-13T00:00:00Z",
    "updatedAt": "2026-05-13T01:00:00Z"
  }
}
```

**错误响应**:
- `400` - 参数验证失败、邮箱已存在、用户名已存在

---

## 4. 会话 API

### 4.1 获取会话列表

`GET /api/im/conversations`

**认证**: ✅ 需要

**查询参数**:

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `sort_by` | string | `updated_at` | 排序字段：`updated_at`, `created_at`, `name`, `unread_count` |
| `order` | string | `desc` | 排序方向：`asc`, `desc` |
| `tag_id` | string | - | 按标签过滤 |
| `include_archived` | bool | `false` | 是否包含已归档会话 |

**响应** (200 OK):
```json
{
  "success": true,
  "data": [
    {
      "id": "660e8400-e29b-41d4-a716-446655440000",
      "type": "direct",
      "name": null,
      "avatar": null,
      "lastMessage": {
        "id": "770e8400-e29b-41d4-a716-446655440000",
        "content": "Hello!",
        "type": "text",
        "status": "sent",
        "senderId": "550e8400-e29b-41d4-a716-446655440000",
        "conversationId": "660e8400-e29b-41d4-a716-446655440000",
        "createdAt": "2026-05-13T00:00:00Z",
        "updatedAt": "2026-05-13T00:00:00Z"
      },
      "unreadCount": 3,
      "isPinned": false,
      "isMuted": false,
      "isArchived": false,
      "createdAt": "2026-05-13T00:00:00Z",
      "updatedAt": "2026-05-13T00:00:00Z"
    }
  ]
}
```

---

### 4.2 创建会话

`POST /api/im/conversations`

**认证**: ✅ 需要

**请求体**:
```json
{
  "type": "direct",
  "name": null,
  "participantIds": ["user-id-1", "user-id-2"]
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `type` | string | ✅ | 会话类型：`direct`, `group`, `ai` |
| `name` | string? | ❌ | 会话名称（群聊必填） |
| `participantIds` | string[] | ✅ | 参与者 ID 列表 |

**响应** (201 Created):
```json
{
  "success": true,
  "data": {
    "id": "660e8400-e29b-41d4-a716-446655440000",
    "type": "direct",
    "name": null,
    "avatar": null,
    "lastMessage": null,
    "unreadCount": 0,
    "isPinned": false,
    "isMuted": false,
    "isArchived": false,
    "createdAt": "2026-05-13T00:00:00Z",
    "updatedAt": "2026-05-13T00:00:00Z"
  }
}
```

---

### 4.3 搜索会话

`GET /api/im/conversations/search`

**认证**: ✅ 需要

**查询参数**:

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `q` | string | ✅ | 搜索关键词 |
| `include_archived` | bool | ❌ | 是否包含已归档会话，默认 `false` |

**响应** (200 OK):
```json
{
  "success": true,
  "data": [
    {
      "id": "660e8400-e29b-41d4-a716-446655440000",
      "type": "group",
      "name": "开发团队",
      ...
    }
  ]
}
```

---

## 5. 消息 API

### 5.1 获取会话消息列表

`GET /api/im/conversations/:id/messages`

**认证**: ✅ 需要

**路径参数**:

| 参数 | 类型 | 说明 |
|------|------|------|
| `id` | string | 会话 ID |

**查询参数**:

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `page` | i64 | `1` | 页码 |
| `limit` | i64 | `50` | 每页数量 |

**响应** (200 OK):
```json
{
  "success": true,
  "data": [
    {
      "id": "770e8400-e29b-41d4-a716-446655440000",
      "conversationId": "660e8400-e29b-41d4-a716-446655440000",
      "senderId": "550e8400-e29b-41d4-a716-446655440000",
      "content": "Hello!",
      "type": "text",
      "status": "sent",
      "createdAt": "2026-05-13T00:00:00Z",
      "updatedAt": "2026-05-13T00:00:00Z",
      "readAt": null,
      "replyTo": null,
      "metadata": null
    }
  ]
}
```

**错误响应**:
- `400` - 无效的会话 ID
- `403` - 不是会话参与者

---

### 5.2 发送消息

`POST /api/im/conversations/:id/messages`

**认证**: ✅ 需要

**路径参数**:

| 参数 | 类型 | 说明 |
|------|------|------|
| `id` | string | 会话 ID |

**请求体**:
```json
{
  "content": "Hello, World!",
  "type": "text"
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `content` | string | ✅ | 消息内容（不能为空） |
| `type` | string | ✅ | 消息类型：`text`, `image`, `file`, `system` |

**响应** (201 Created):
```json
{
  "success": true,
  "data": {
    "id": "770e8400-e29b-41d4-a716-446655440000",
    "conversationId": "660e8400-e29b-41d4-a716-446655440000",
    "senderId": "550e8400-e29b-41d4-a716-446655440000",
    "content": "Hello, World!",
    "type": "text",
    "status": "sent",
    "createdAt": "2026-05-13T00:00:00Z",
    "updatedAt": "2026-05-13T00:00:00Z"
  }
}
```

**错误响应**:
- `400` - 无效的会话 ID、消息内容为空
- `403` - 不是会话参与者

---

### 5.3 编辑消息

`PUT /api/im/conversations/:id/messages/:msg_id`

**认证**: ✅ 需要

**限制**: 只能编辑自己的消息，且在发送后 2 分钟内

**路径参数**:

| 参数 | 类型 | 说明 |
|------|------|------|
| `id` | string | 会话 ID |
| `msg_id` | string | 消息 ID |

**请求体**:
```json
{
  "content": "Updated message content"
}
```

**响应** (200 OK):
```json
{
  "success": true,
  "data": {
    "id": "770e8400-e29b-41d4-a716-446655440000",
    "content": "Updated message content",
    "updatedAt": "2026-05-13T00:02:00Z",
    ...
  }
}
```

**错误响应**:
- `400` - 无效的 ID、消息内容为空
- `403` - 无法编辑此消息（非本人或超过2分钟）
- `404` - 消息不存在

---

### 5.4 撤回消息

`POST /api/im/conversations/:id/messages/:msg_id/recall`

**认证**: ✅ 需要

**限制**: 只能撤回自己的消息，且在发送后 2 分钟内

**路径参数**:

| 参数 | 类型 | 说明 |
|------|------|------|
| `id` | string | 会话 ID |
| `msg_id` | string | 消息 ID |

**响应** (200 OK):
```json
{
  "success": true,
  "data": {
    "id": "770e8400-e29b-41d4-a716-446655440000",
    "content": "",
    "status": "failed",
    ...
  }
}
```

**错误响应**:
- `400` - 无效的 ID
- `403` - 无法撤回此消息
- `404` - 消息不存在

---

### 5.5 标记会话已读

`POST /api/im/conversations/:id/read`

**认证**: ✅ 需要

**路径参数**:

| 参数 | 类型 | 说明 |
|------|------|------|
| `id` | string | 会话 ID |

**响应** (200 OK):
```json
{
  "success": true,
  "data": {
    "success": true
  }
}
```

**错误响应**:
- `400` - 无效的会话 ID
- `403` - 不是会话参与者

---

### 5.6 搜索消息

`GET /api/im/conversations/:id/messages/search`

**认证**: ✅ 需要

**路径参数**:

| 参数 | 类型 | 说明 |
|------|------|------|
| `id` | string | 会话 ID |

**查询参数**:

| 参数 | 类型 | 必填 | 默认值 | 说明 |
|------|------|------|--------|------|
| `keyword` | string | ✅ | - | 搜索关键词 |
| `page` | i64 | ❌ | `1` | 页码 |
| `limit` | i64 | ❌ | `50` | 每页数量 |

**响应** (200 OK):
```json
{
  "success": true,
  "data": {
    "messages": [
      {
        "id": "770e8400-e29b-41d4-a716-446655440000",
        "content": "Hello, World!",
        ...
      }
    ],
    "keyword": "Hello",
    "page": 1,
    "limit": 50
  }
}
```

**错误响应**:
- `400` - 无效的会话 ID、搜索关键词为空
- `403` - 不是会话参与者

---

### 5.7 获取消息统计

`GET /api/im/conversations/:id/messages/stats`

**认证**: ✅ 需要

**路径参数**:

| 参数 | 类型 | 说明 |
|------|------|------|
| `id` | string | 会话 ID |

**响应** (200 OK):
```json
{
  "success": true,
  "data": {
    "total_messages": 150,
    "today_messages": 23,
    "participants": 5
  }
}
```

---

## 6. 群组管理 API

### 6.1 获取群组成员列表

`GET /api/im/conversations/:id/members`

**认证**: ✅ 需要

**路径参数**:

| 参数 | 类型 | 说明 |
|------|------|------|
| `id` | string | 会话 ID |

**响应** (200 OK):
```json
{
  "success": true,
  "data": [
    {
      "user_id": "550e8400-e29b-41d4-a716-446655440000",
      "username": "alice",
      "role": "owner",
      "joined_at": "2026-05-13T00:00:00Z"
    },
    {
      "user_id": "880e8400-e29b-41d4-a716-446655440000",
      "username": "bob",
      "role": "member",
      "joined_at": "2026-05-13T01:00:00Z"
    }
  ]
}
```

---

### 6.2 添加群组成员

`POST /api/im/conversations/:id/members`

**认证**: ✅ 需要（仅群主可操作）

**路径参数**:

| 参数 | 类型 | 说明 |
|------|------|------|
| `id` | string | 会话 ID |

**请求体**:
```json
{
  "user_ids": ["user-id-1", "user-id-2"]
}
```

**响应** (200 OK):
```json
{
  "success": true,
  "data": {
    "added": 2
  }
}
```

---

### 6.3 移除群组成员

`DELETE /api/im/conversations/:id/members/:member_id`

**认证**: ✅ 需要（仅群主可操作，或成员主动退出）

**路径参数**:

| 参数 | 类型 | 说明 |
|------|------|------|
| `id` | string | 会话 ID |
| `member_id` | string | 成员用户 ID |

**响应** (200 OK):
```json
{
  "success": true,
  "data": {
    "removed": true
  }
}
```

---

### 6.4 更新群组信息

`PUT /api/im/conversations/:id/group`

**认证**: ✅ 需要（仅群主可操作）

**路径参数**:

| 参数 | 类型 | 说明 |
|------|------|------|
| `id` | string | 会话 ID |

**请求体**:
```json
{
  "name": "新群名称",
  "avatar": "https://example.com/group-avatar.png"
}
```

**响应** (200 OK):
```json
{
  "success": true,
  "data": {
    "id": "660e8400-e29b-41d4-a716-446655440000",
    "name": "新群名称",
    "avatar": "https://example.com/group-avatar.png",
    ...
  }
}
```

---

### 6.5 获取群公告

`GET /api/im/conversations/:id/announcement`

**认证**: ✅ 需要

**路径参数**:

| 参数 | 类型 | 说明 |
|------|------|------|
| `id` | string | 会话 ID |

**响应** (200 OK):
```json
{
  "success": true,
  "data": {
    "announcement": "欢迎加入本群！"
  }
}
```

---

### 6.6 更新群公告

`PUT /api/im/conversations/:id/announcement`

**认证**: ✅ 需要（仅群主可操作）

**路径参数**:

| 参数 | 类型 | 说明 |
|------|------|------|
| `id` | string | 会话 ID |

**请求体**:
```json
{
  "announcement": "新的群公告内容"
}
```

**响应** (200 OK):
```json
{
  "success": true,
  "data": {
    "announcement": "新的群公告内容"
  }
}
```

---

## 7. 会话管理增强 API

### 7.1 切换会话置顶状态

`PUT /api/im/conversations/:id/pin`

**认证**: ✅ 需要

**路径参数**:

| 参数 | 类型 | 说明 |
|------|------|------|
| `id` | string | 会话 ID |

**请求体**:
```json
{
  "is_pinned": true
}
```

**响应** (200 OK):
```json
{
  "success": true,
  "data": {
    "id": "660e8400-e29b-41d4-a716-446655440000",
    "isPinned": true,
    ...
  }
}
```

---

### 7.2 切换会话免打扰状态

`PUT /api/im/conversations/:id/mute`

**认证**: ✅ 需要

**路径参数**:

| 参数 | 类型 | 说明 |
|------|------|------|
| `id` | string | 会话 ID |

**请求体**:
```json
{
  "is_muted": true
}
```

**响应** (200 OK):
```json
{
  "success": true,
  "data": {
    "id": "660e8400-e29b-41d4-a716-446655440000",
    "isMuted": true,
    ...
  }
}
```

---

### 7.3 切换会话归档状态

`PUT /api/im/conversations/:id/archive`

**认证**: ✅ 需要

**路径参数**:

| 参数 | 类型 | 说明 |
|------|------|------|
| `id` | string | 会话 ID |

**请求体**:
```json
{
  "is_archived": true
}
```

**响应** (200 OK):
```json
{
  "success": true,
  "data": {
    "id": "660e8400-e29b-41d4-a716-446655440000",
    "isArchived": true,
    ...
  }
}
```

---

## 8. 标签管理 API

### 8.1 创建标签

`POST /api/im/tags`

**认证**: ✅ 需要

**请求体**:
```json
{
  "name": "工作",
  "color": "#FF5722"
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `name` | string | ✅ | 标签名称 |
| `color` | string? | ❌ | 标签颜色（十六进制） |

**响应** (201 Created):
```json
{
  "success": true,
  "data": {
    "id": "tag-id",
    "user_id": "user-id",
    "name": "工作",
    "color": "#FF5722",
    "created_at": "2026-05-13T00:00:00Z"
  }
}
```

---

### 8.2 获取用户的所有标签

`GET /api/im/tags`

**认证**: ✅ 需要

**响应** (200 OK):
```json
{
  "success": true,
  "data": [
    {
      "id": "tag-id-1",
      "name": "工作",
      "color": "#FF5722",
      "created_at": "2026-05-13T00:00:00Z"
    },
    {
      "id": "tag-id-2",
      "name": "朋友",
      "color": "#4CAF50",
      "created_at": "2026-05-13T00:00:00Z"
    }
  ]
}
```

---

### 8.3 删除标签

`DELETE /api/im/tags/:tag_id`

**认证**: ✅ 需要

**路径参数**:

| 参数 | 类型 | 说明 |
|------|------|------|
| `tag_id` | string | 标签 ID |

**响应** (200 OK):
```json
{
  "success": true,
  "data": {
    "deleted": true
  }
}
```

---

### 8.4 给会话添加标签

`POST /api/im/conversations/:id/tags/:tag_id`

**认证**: ✅ 需要

**路径参数**:

| 参数 | 类型 | 说明 |
|------|------|------|
| `id` | string | 会话 ID |
| `tag_id` | string | 标签 ID |

**响应** (200 OK):
```json
{
  "success": true,
  "data": {
    "conversation_id": "conv-id",
    "tag_id": "tag-id",
    "created_at": "2026-05-13T00:00:00Z"
  }
}
```

---

### 8.5 移除会话的标签

`DELETE /api/im/conversations/:id/tags/:tag_id`

**认证**: ✅ 需要

**路径参数**:

| 参数 | 类型 | 说明 |
|------|------|------|
| `id` | string | 会话 ID |
| `tag_id` | string | 标签 ID |

**响应** (200 OK):
```json
{
  "success": true,
  "data": {
    "removed": true
  }
}
```

---

### 8.6 获取会话的所有标签

`GET /api/im/conversations/:id/tags`

**认证**: ✅ 需要

**路径参数**:

| 参数 | 类型 | 说明 |
|------|------|------|
| `id` | string | 会话 ID |

**响应** (200 OK):
```json
{
  "success": true,
  "data": [
    {
      "id": "tag-id-1",
      "name": "工作",
      "color": "#FF5722",
      "created_at": "2026-05-13T00:00:00Z"
    }
  ]
}
```

---

## 9. 加密 API

### 9.1 生成加密密钥对

`POST /api/im/encryption/keys`

**认证**: ✅ 需要

**响应** (200 OK):
```json
{
  "success": true,
  "data": {
    "public_key": "base64-encoded-public-key",
    "key_id": "key-id"
  }
}
```

---

### 9.2 获取会话密钥

`GET /api/im/encryption/session-key/:conversation_id`

**认证**: ✅ 需要

**路径参数**:

| 参数 | 类型 | 说明 |
|------|------|------|
| `conversation_id` | string | 会话 ID |

**响应** (200 OK):
```json
{
  "success": true,
  "data": {
    "session_key": "base64-encoded-session-key",
    "conversation_id": "conv-id"
  }
}
```

---

### 9.3 加密消息

`POST /api/im/encryption/encrypt`

**认证**: ✅ 需要

**请求体**:
```json
{
  "conversation_id": "conv-id",
  "plaintext": "Hello, this is a secret message"
}
```

**响应** (200 OK):
```json
{
  "success": true,
  "data": {
    "ciphertext": "base64-encoded-ciphertext",
    "nonce": "base64-encoded-nonce"
  }
}
```

---

### 9.4 解密消息

`POST /api/im/encryption/decrypt`

**认证**: ✅ 需要

**请求体**:
```json
{
  "conversation_id": "conv-id",
  "ciphertext": "base64-encoded-ciphertext",
  "nonce": "base64-encoded-nonce"
}
```

**响应** (200 OK):
```json
{
  "success": true,
  "data": {
    "plaintext": "Hello, this is a secret message"
  }
}
```

---

### 9.5 获取加密信息

`GET /api/im/encryption/info`

**认证**: ✅ 需要

**响应** (200 OK):
```json
{
  "success": true,
  "data": {
    "has_keys": true,
    "public_key": "base64-encoded-public-key",
    "key_created_at": "2026-05-13T00:00:00Z"
  }
}
```

---

### 9.6 密钥交换

`POST /api/im/encryption/key-exchange`

**认证**: ✅ 需要

**请求体**:
```json
{
  "conversation_id": "conv-id",
  "public_key": "base64-encoded-public-key"
}
```

**响应** (200 OK):
```json
{
  "success": true,
  "data": {
    "session_key": "base64-encoded-shared-session-key"
  }
}
```

---

### 9.7 存储加密消息

`POST /api/im/encryption/store`

**认证**: ✅ 需要

**请求体**:
```json
{
  "conversation_id": "conv-id",
  "ciphertext": "base64-encoded-ciphertext",
  "nonce": "base64-encoded-nonce",
  "sender_public_key": "base64-encoded-public-key"
}
```

**响应** (200 OK):
```json
{
  "success": true,
  "data": {
    "message_id": "encrypted-msg-id",
    "stored_at": "2026-05-13T00:00:00Z"
  }
}
```

---

### 9.8 获取加密消息历史

`GET /api/im/encryption/messages/:conversation_id`

**认证**: ✅ 需要

**路径参数**:

| 参数 | 类型 | 说明 |
|------|------|------|
| `conversation_id` | string | 会话 ID |

**响应** (200 OK):
```json
{
  "success": true,
  "data": [
    {
      "id": "encrypted-msg-id",
      "conversation_id": "conv-id",
      "sender_id": "user-id",
      "ciphertext": "base64-encoded-ciphertext",
      "nonce": "base64-encoded-nonce",
      "created_at": "2026-05-13T00:00:00Z"
    }
  ]
}
```

---

## 10. 数据模型

### 10.1 User

```typescript
interface User {
  id: string;              // UUID
  username: string;        // 用户名
  email: string;           // 邮箱
  avatar?: string;         // 头像 URL
  createdAt: string;       // ISO 8601
  updatedAt: string;       // ISO 8601
}
```

### 10.2 Message

```typescript
interface Message {
  id: string;              // UUID
  conversationId: string;  // 会话 ID
  senderId: string;        // 发送者 ID
  content: string;         // 消息内容
  type: MessageType;       // 消息类型
  status: MessageStatus;   // 消息状态
  createdAt: string;       // ISO 8601
  updatedAt: string;       // ISO 8601
  readAt?: string;         // 已读时间
  replyTo?: string;        // 回复的消息 ID
  metadata?: object;       // 附加元数据
}

type MessageType = "text" | "image" | "file" | "system";
type MessageStatus = "sending" | "sent" | "delivered" | "read" | "failed";
```

### 10.3 Conversation

```typescript
interface Conversation {
  id: string;              // UUID
  type: ConversationType;  // 会话类型
  name?: string;           // 会话名称
  avatar?: string;         // 会话头像
  lastMessage?: Message;   // 最后一条消息
  unreadCount: number;     // 未读消息数
  isPinned: boolean;       // 是否置顶
  isMuted: boolean;        // 是否免打扰
  isArchived: boolean;     // 是否已归档
  createdAt: string;       // ISO 8601
  updatedAt: string;       // ISO 8601
}

type ConversationType = "direct" | "group" | "ai";
```

### 10.4 OnlineStatus

```typescript
type OnlineStatus = "offline" | "online" | "away" | "busy";
```

### 10.5 ConversationTag

```typescript
interface ConversationTag {
  id: string;              // UUID
  user_id: string;         // 所属用户 ID
  name: string;            // 标签名称
  color?: string;          // 标签颜色
  created_at: string;      // ISO 8601
}
```

---

## 附录

### A. WebSocket 消息类型

WebSocket 连接地址: `ws://localhost:8003/ws`

#### 客户端 -> 服务器

| 类型 | 说明 | 数据 |
|------|------|------|
| `Auth` | 认证 | `{ "token": "jwt_token" }` |
| `Message` | 发送消息 | `{ "conversation_id": "...", "content": "...", "type": "text" }` |
| `Edit` | 编辑消息 | `{ "message_id": "...", "content": "..." }` |
| `Recall` | 撤回消息 | `{ "message_id": "..." }` |
| `Typing` | 输入状态 | `{ "conversation_id": "...", "is_typing": true }` |
| `Ping` | 心跳 | `{}` |

#### 服务器 -> 客户端

| 类型 | 说明 |
|------|------|
| `AuthOk` | 认证成功 |
| `AuthError` | 认证失败 |
| `Message` | 新消息 |
| `MessageEdited` | 消息已编辑 |
| `MessageRecalled` | 消息已撤回 |
| `StatusChange` | 用户状态变更 |
| `Typing` | 对方正在输入 |
| `Pong` | 心跳响应 |
| `Error` | 错误通知 |

### B. 服务端口

| 服务 | 端口 | 说明 |
|------|------|------|
| im-api | 8002 | REST API |
| im-gateway | 8003 | WebSocket 网关 |
| ai-service | 8004 | AI 对话服务 |
| file-service | 8005 | 文件服务 |
| user-service | 8006 | 用户服务 |
| usage-service | 8007 | 用量统计服务 |
| push-service | 8008 | 推送服务 |
| config-service | 8009 | 配置服务 |

### C. 数据库表

| 表名 | 说明 |
|------|------|
| `users` | 用户表 |
| `conversations` | 会话表 |
| `conversation_participants` | 会话参与者表 |
| `messages` | 消息表 |
| `conversation_tags` | 会话标签表 |
| `conversation_tag_links` | 会话-标签关联表 |
| `encrypted_messages` | 加密消息表 |
| `push_devices` | 推送设备表 |
| `push_config` | 推送配置表 |

---

*文档生成时间: 2026-05-13 05:30*
