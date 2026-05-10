# OmniLink IM API - 用户认证功能实现

## 实现概览

本项目完成了 OmniLink IM API 服务的用户认证功能，包括注册、登录、获取当前用户信息和更新用户资料。

## 已实现的功能

### API 端点

1. **POST /api/auth/register** - 用户注册
   - 用户名验证：3-20 个字符
   - 邮箱验证：有效邮箱格式
   - 密码验证：至少 8 个字符
   - 密码加密：bcrypt 哈希

2. **POST /api/auth/login** - 用户登录
   - 邮箱和密码验证
   - JWT Token 生成（有效期 7 天）
   - 返回 token 和用户信息

3. **GET /api/user/me** - 获取当前用户信息
   - JWT Token 认证
   - 返回完整用户信息

4. **PUT /api/user/me** - 更新用户资料
   - JWT Token 认证
   - 支持部分字段更新
   - 验证邮箱和用户名唯一性

## 文件结构

```
/root/omnilink/crates/im-api/
├── src/
│   ├── main.rs                      # 主入口，路由注册
│   ├── lib.rs                       # 公共导出
│   ├── models/
│   │   ├── mod.rs
│   │   └── auth.rs                  # 数据模型（User, Request, Response）
│   ├── handlers/
│   │   ├── mod.rs
│   │   └── auth.rs                  # API 处理器
│   ├── db/
│   │   ├── mod.rs
│   │   └── user.rs                  # 数据库操作
│   ├── utils/
│   │   ├── mod.rs
│   │   └── jwt.rs                   # JWT 工具函数
│   └── middleware/
│       ├── mod.rs
│       └── auth.rs                  # JWT 认证中间件
├── migrations/
│   └── 001_create_users.sql         # 数据库迁移脚本
├── AUTH_API.md                      # API 文档
└── Cargo.toml                       # 依赖配置
```

## 技术栈

- **Web 框架**: Axum 0.7
- **数据库**: PostgreSQL + SQLx
- **密码加密**: bcrypt
- **JWT**: jsonwebtoken
- **数据验证**: validator
- **邮箱验证**: email-validator

## 数据格式

### 完全匹配前端 TypeScript 类型

```typescript
// 用户类型
interface User {
  id: string              // UUID 字符串
  username: string        // 3-20 个字符
  email: string
  avatar?: string
  createdAt: string       // ISO 8601 格式
  updatedAt: string       // ISO 8601 格式
}

// API 响应包装器
interface ApiResponse<T> {
  success: boolean
  data?: T
  error?: {
    code: string
    message: string
  }
}
```

## 数据库表结构

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

## 安全特性

1. **密码加密**: 使用 bcrypt，默认成本因子
2. **JWT 认证**: HS256 算法，7 天有效期
3. **输入验证**: 使用 validator crate 进行严格验证
4. **SQL 注入防护**: 使用 SQLx 参数化查询
5. **唯一性约束**: 数据库层面的 email 和 username 唯一性

## 错误处理

所有错误都返回统一的 `ApiResponse` 格式，包含错误代码和友好的错误消息。

### 常见错误代码

- `INVALID_INPUT` - 输入数据验证失败
- `EMAIL_EXISTS` - 邮箱已被注册
- `USERNAME_EXISTS` - 用户名已被使用
- `INVALID_CREDENTIALS` - 邮箱或密码错误
- `INVALID_TOKEN` - Token 无效
- `TOKEN_EXPIRED` - Token 已过期
- `USER_NOT_FOUND` - 用户不存在

## 环境变量

```bash
DATABASE_URL=postgresql://postgres:postgres@localhost/omnilink
```

## 运行服务

```bash
# 设置环境变量
export DATABASE_URL="postgresql://postgres:postgres@localhost/omnilink"

# 运行服务
cd /root/omnilink/crates/im-api
cargo run
```

服务将在 `http://0.0.0.0:8002` 启动。

## 测试示例

### 注册新用户
```bash
curl -X POST http://localhost:8002/api/auth/register \n  -H "Content-Type: application/json" \n  -d '{
    "username": "testuser",
    "email": "test@example.com",
    "password": "testpass123"
  }'
```

### 登录
```bash
curl -X POST http://localhost:8002/api/auth/login \n  -H "Content-Type: application/json" \n  -d '{
    "email": "test@example.com",
    "password": "testpass123"
  }'
```

### 获取当前用户信息
```bash
curl -X GET http://localhost:8002/api/user/me \n  -H "Authorization: Bearer YOUR_JWT_TOKEN"
```

### 更新用户资料
```bash
curl -X PUT http://localhost:8002/api/user/me \n  -H "Authorization: Bearer YOUR_JWT_TOKEN" \n  -H "Content-Type: application/json" \n  -d '{
    "username": "newusername",
    "avatar": "https://example.com/avatar.jpg"
  }'
```

## 依赖说明

新增的依赖：
- `sqlx` - 异步 SQL 工具包
- `bcrypt` - 密码哈希
- `email-validator` - 邮箱格式验证

## 注意事项

1. JWT 密钥目前使用硬编码的值，生产环境应该从环境变量读取
2. 服务启动时会自动创建数据库表
3. 所有时间戳都使用 ISO 8601 格式（RFC3339）
4. User.id 使用 UUID 字符串格式，完全匹配前端要求

## 未来改进

1. 集成 user-service 提供的 JWT 服务
2. 添加刷新 token 机制
3. 添加邮件验证功能
4. 添加密码重置功能
5. 添加用户头像上传功能
6. 添加速率限制防止暴力破解

## 文档

详细的 API 文档请参见 [AUTH_API.md](AUTH_API.md)