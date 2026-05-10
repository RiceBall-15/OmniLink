# WebSocket 实现说明

## 概述

im-gateway 服务实现了完整的 WebSocket 基础连接和心跳保活机制，支持多设备连接和在线状态管理。

## 架构

### 核心组件

1. **WebSocket 连接管理器** (`connection_manager.rs`)
   - 管理所有活跃的 WebSocket 连接
   - 支持同一用户多设备同时连接
   - 提供消息路由功能（单播、广播）

2. **在线状态管理器** (`status_manager.rs`)
   - 跟踪用户的在线/离线状态
   - 自动清理过期的在线状态

3. **WebSocket 处理器** (`handlers/ws.rs`)
   - 处理 WebSocket 连接的生命周期
   - 实现 Token 认证
   - 处理各种消息类型
   - 实现心跳机制

4. **主服务** (`main.rs`)
   - 启动 HTTP 和 WebSocket 服务器
   - 启动后台清理任务
   - 启动心跳检测任务

## 消息格式

### 消息类型（匹配前端枚举）

```typescript
enum WSMessageType {
  CONNECT = 'connect',       // 客户端发起连接请求
  CONNECTED = 'connected',   // 服务器确认连接成功
  MESSAGE = 'message',       // 普通消息
  NEW_MESSAGE = 'new_message', // 新消息通知
  PING = 'ping',             // 心跳探测
  PONG = 'pong',             // 心跳响应
  TYPING = 'typing',         // 正在输入
  READ = 'read',             // 已读回执
  ERROR = 'error',           // 错误消息
}
```

### WebSocket 消息结构

```rust
pub struct WSMessage {
    #[serde(rename = "type")]
    pub message_type: WSMessageType,
    pub conversation_id: Option<Uuid>,
    pub message_id: Option<Uuid>,
    pub sender_id: Option<Uuid>,
    pub content: Option<String>,
    pub timestamp: Option<i64>,
    pub data: Option<serde_json::Value>,
}
```

## 连接流程

### 1. 连接认证

```
Client -> Server: {
  "type": "connect",
  "data": {
    "token": "jwt-token",
    "conversation_id": "optional-uuid"
  }
}

Server -> Client: {
  "type": "connected",
  "sender_id": "user-uuid",
  "timestamp": 1234567890,
  "content": "Connected successfully"
}
```

### 2. 心跳保活

- **服务器端**：每 30 秒向所有活跃连接发送 PING 消息
- **客户端**：收到 PING 后应回复 PONG 消息
- **超时检测**：如果连接 60 秒内未活动，服务器将主动断开连接

```
Server -> Client: {"type": "ping", "timestamp": 1234567890}
Client -> Server: {"type": "pong", "timestamp": 1234567891}
```

### 3. 消息发送

```
Client -> Server: {
  "type": "message",
  "conversation_id": "conversation-uuid",
  "content": "Hello, World!",
  "timestamp": 1234567890
}
```

### 4. 输入状态

```
Client -> Server: {
  "type": "typing",
  "conversation_id": "conversation-uuid"
}
```

### 5. 已读回执

```
Client -> Server: {
  "type": "read",
  "conversation_id": "conversation-uuid",
  "message_id": "message-uuid"
}
```

## 心跳机制

### 服务器心跳

- **发送间隔**：30 秒
- **发送内容**：PING 消息
- **目的**：检测连接是否仍然活跃

### 超时检测

- **超时时间**：60 秒
- **检测间隔**：30 秒
- **处理方式**：断开超时连接，清理资源，更新用户状态

### 心跳任务

```rust
// 服务器发送 PING（每30秒）
let heartbeat_task = tokio::spawn(async move {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
    loop {
        interval.tick().await;
        // 发送 PING 消息
    }
});

// 检测超时连接（每30秒检查一次）
tokio::spawn(heartbeat_check_task(connection_manager, status_manager));
```

## 连接管理

### 多设备支持

每个用户可以有多个同时连接：

```rust
// 用户连接表: user_id -> Vec<connection_id>
user_connections: HashMap<Uuid, Vec<ConnectionId>>

// 总连接表: connection_id -> connection
connections: HashMap<ConnectionId, WSConnection>
```

### 连接信息

```rust
pub struct WSConnection {
    pub connection_id: ConnectionId,    // 唯一连接ID
    pub user_id: Uuid,                  // 用户ID
    pub conversation_id: Option<Uuid>,  // 当前会话ID
    pub addr: SocketAddr,               // 客户端地址
    pub sender: UnboundedSender<Message>, // 消息发送通道
    pub connected_at: i64,              // 连接时间戳
    pub last_active_at: i64,            // 最后活跃时间戳
}
```

## 在线状态管理

### 状态类型

```rust
pub enum UserStatus {
    Online,   // 在线
    Away,     // 离开
    Busy,     // 忙碌
    Offline,  // 离线
}
```

### 状态更新时机

1. **连接成功**：设置用户在线
2. **收到心跳**：更新最后活跃时间
3. **连接断开**：
   - 如果还有其他连接，保持在线
   - 如果是最后一个连接，设置离线
4. **心跳超时**：设置用户离线

### 清理任务

```rust
// 每60秒清理一次过期的在线状态
tokio::spawn(cleanup_task(status_manager));
```

## 消息路由

### 单播（发送给特定用户）

```rust
connection_manager.send_to_user(user_id, message).await;
```

### 广播（发送给会话所有成员）

```rust
connection_manager.send_to_conversation(conversation_id, message).await;
```

### 全局广播

```rust
connection_manager.broadcast(message).await;
```

## 错误处理

### 认证错误

```rust
{
  "type": "error",
  "content": "Invalid or expired token",
  "timestamp": 1234567890,
  "data": {
    "code": "auth_failed"
  }
}
```

### 格式错误

```rust
{
  "type": "error",
  "content": "Invalid message format",
  "timestamp": 1234567890,
  "data": {
    "code": "format_error"
  }
}
```

## 配置

### 环境变量

- `DATABASE_URL`: PostgreSQL 数据库连接字符串
- `REDIS_URL`: Redis 连接字符串
- `JWT_SECRET`: JWT 密钥
- `IM_GATEWAY_PORT`: HTTP 服务端口（默认：8001）
- `IM_GATEWAY_WS_PORT`: WebSocket 服务端口（默认：8010）

### 关键参数

- **心跳间隔**：30 秒
- **心跳超时**：60 秒
- **清理间隔**：60 秒

## API 端点

### WebSocket

```
ws://localhost:8010/ws
```

### HTTP API

```
POST   /messages                    # 发送消息
GET    /messages/history/:conv_id   # 获取消息历史
POST   /messages/read               # 标记已读
POST   /conversations               # 创建对话
GET    /conversations               # 获取对话列表
GET    /conversations/:conv_id      # 获取对话详情
GET    /online-users                # 获取在线用户
```

## 安全性

1. **Token 认证**：所有 WebSocket 连接必须在第一条消息中提供有效的 JWT token
2. **Token 验证**：使用 `TokenManager::verify_token()` 验证 token
3. **过期处理**：Token 过期或无效时，返回错误并断开连接
4. **超时断开**：心跳超时自动断开连接

## 日志

- `INFO`: 连接建立/断开、用户认证、状态变化
- `DEBUG`: 消息收发、心跳交互
- `WARN`: 错误消息、超时警告
- `ERROR`: 连接错误、认证失败

## 性能考虑

1. **异步处理**：使用 tokio 异步运行时
2. **并发连接**：支持大量并发连接
3. **内存管理**：自动清理过期的连接和状态
4. **消息通道**：使用 unbounded channel 提高吞吐量

## 后续扩展

- [ ] 消息持久化
- [ ] 消息重传机制
- [ ] 离线消息推送
- [ ] 群组消息优化
- [ ] 消息加密
- [ ] 限流和防刷
- [ ] 监控和指标
