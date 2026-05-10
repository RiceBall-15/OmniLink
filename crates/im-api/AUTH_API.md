# OmniLink IM API - 用户认证 API 文档

## 概述

本服务提供用户认证相关的 API，包括注册、登录、获取当前用户信息和更新用户资料。

## 基础信息

- **Base URL**: `http://localhost:8002`
- **认证方式**: Bearer Token (JWT)

## 数据格式

### 用户类型 (User)
```typescript
interface User {
  id: string              // UUID 字符串
  username: string        // 3-20 个字符
  email: string           // 有效邮箱
  avatar?: string         // 可选
  createdAt: string       // ISO 8601 格式
  updatedAt: string       // ISO 8601 格式
}
```

### API 响应格式 (ApiResponse)
```typescript
interface ApiResponse<T> {
  success: boolean
  data?: T
  error?: {
    code: string
    message: string
  }
}
```

## API 端点

### 1. 用户注册

**端点**: `POST /api/auth/register`

**请求体**:
```json
{
  "username": "johndoe",
  "email": "john@example.com",
  "password": "securepassword123"
}
```

**验证规则**:
- `username`: 3-20 个字符
- `email`: 有效邮箱格式
- `password`: 至少 8 个字符

**响应 (201 Created)**:
```json
{
  "success": true,
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "username": "johndoe",
    "email": "john@example.com",
    "avatar": null,
    "createdAt": "2026-05-02T12:00:00Z",
    "updatedAt": "2026-05-02T12:00:00Z"
  }
}
```

**错误响应**:
```json
{
  "success": false,
  "error": {
    "code": "EMAIL_EXISTS",
    "message": "该邮箱已被注册"
  }
}
```

---

### 2. 用户登录

**端点**: `POST /api/auth/login`

**请求体**:
```json
{
  "email": "john@example.com",
  "password": "securepassword123"
}
```

**响应 (200 OK)**:
```json
{
  "success": true,
  "data": {
    "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "user": {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "username": "johndoe",
      "email": "john@example.com",
      "avatar": null,
      "createdAt": "2026-05-02T12:00:00Z",
      "updatedAt": "2026-05-02T12:00:00Z"
    }
  }
}
```

**错误响应**:
```json
{
  "success": false,
  "error": {
    "code": "INVALID_CREDENTIALS",
    "message": "邮箱或密码错误"
  }
}
```

---

### 3. 获取当前用户信息

**端点**: `GET /api/user/me`

**请求头**:
```
Authorization: Bearer {token}
```

**响应 (200 OK)**:
```json
{
  "success": true,
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "username": "johndoe",
    "email": "john@example.com",
    "avatar": "https://example.com/avatar.jpg",
    "createdAt": "2026-05-02T12:00:00Z",
    "updatedAt": "2026-05-02T13:00:00Z"
  }
}
```

**错误响应**:
```json
{
  "success": false,
  "error": {
    "code": "INVALID_TOKEN",
    "message": "无效的 token"
  }
}
```

---

### 4. 更新用户资料

**端点**: `PUT /api/user/me`

**请求头**:
```
Authorization: Bearer {token}
```

**请求体**:
```json
{
  "username": "johndoe2",
  "email": "john2@example.com",
  "avatar": "https://example.com/new-avatar.jpg"
}
```

注意：所有字段都是可选的，可以只更新部分字段。

**响应 (200 OK)**:
```json
{
  "success": true,
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "username": "johndoe2",
    "email": "john2@example.com",
    "avatar": "https://example.com/new-avatar.jpg",
    "createdAt": "2026-05-02T12:00:00Z",
    "updatedAt": "2026-05-02T14:00:00Z"
  }
}
```

**错误响应**:
```json
{
  "success": false,
  "error": {
    "code": "EMAIL_EXISTS",
    "message": "该邮箱已被注册"
  }
}
```

---

## 错误代码

| 代码 | 描述 |
|------|------|
| `INVALID_INPUT` | 输入数据验证失败 |
| `INVALID_EMAIL` | 邮箱格式不正确 |
| `EMAIL_EXISTS` | 邮箱已被注册 |
| `USERNAME_EXISTS` | 用户名已被使用 |
| `INVALID_CREDENTIALS` | 邮箱或密码错误 |
| `INVALID_TOKEN` | Token 无效 |
| `TOKEN_EXPIRED` | Token 已过期 |
| `USER_NOT_FOUND` | 用户不存在 |
| `REGISTER_FAILED` | 注册失败 |
| `LOGIN_FAILED` | 登录失败 |
| `GET_USER_FAILED` | 获取用户信息失败 |
| `UPDATE_FAILED` | 更新用户信息失败 |
| `PASSWORD_VERIFY_FAILED` | 密码验证失败 |
| `TOKEN_GENERATION_FAILED` | Token 生成失败 |

## 数据库配置

服务启动时会自动创建 `users` 表，如果不存在的话。表结构如下：

```sql
CREATE TABLE users (
    id UUID PRIMARY KEY,
    username VARCHAR(20) UNIQUE NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    avatar VARCHAR(500),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);
```

## 安全说明

1. **密码加密**: 使用 bcrypt 进行密码哈希，成本因子为默认值
2. **JWT Token**: 使用 HS256 算法，Token 有效期为 7 天
3. **输入验证**: 所有输入都经过严格验证
4. **数据库索引**: email 和 username 字段有唯一索引

## 环境变量

- `DATABASE_URL`: PostgreSQL 数据库连接字符串
  - 默认值: `postgresql://postgres:postgres@localhost/omnilink`

## 开发说明

服务监听端口: `8002`
健康检查端点: `GET /health`
