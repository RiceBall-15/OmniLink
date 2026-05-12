# OmniLink 开发任务队列

## 🕐 开发时间窗口
- **开始时间**: 每天 00:00
- **结束时间**: 每天 08:00
- **时长**: 8小时

## 📋 当前任务队列

### 阶段一：核心功能完善（进行中）

#### 1. 消息已读回执系统 ✅
- [x] 实现 `mark_read` handler 的完整逻辑
- [x] 添加消息已读状态数据库查询
- [x] WebSocket 通知发送者消息已读
- [x] 批量标记已读支持

#### 2. 消息编辑/撤回功能 ✅
- [x] 消息编辑 HTTP API
- [x] 消息撤回 HTTP API
- [x] WebSocket 广播通知
- [x] 2分钟时间窗口限制

#### 3. 在线状态同步 ✅
- [x] 用户上线/下线状态管理
- [x] Redis 在线状态存储
- [x] WebSocket 状态广播
- [x] 好友在线状态查询 API

#### 4. WebSocket 认证逻辑完善 ✅
- [x] JWT token 验证
- [x] 连接时认证
- [x] token 过期处理
- [x] 权限检查

#### 5. 文件上传 API 实现 ✅
- [x] 修复 file-service 28个编译错误
- [x] 实现文件上传 handler（单文件 + 批量）
- [x] 文件类型和大小验证（11种MIME类型）
- [x] 实现文件下载 API
- [x] 实现文件删除 API
- [x] 实现文件列表查询 API
- [x] 实现文件信息更新 API
- [x] 实现缩略图获取 API
- [x] 实现存储统计 API
- [x] 认证中间件（auth_middleware + AuthUser 模式）
- [x] Axum 0.7 State 类型兼容性修复
- [ ] MinIO 存储集成（当前使用本地存储）

#### 6. AI 模型对接（基础） ✅
- [x] 完善 OpenAI provider 实现
- [x] 添加基础对话功能
- [x] 流式响应支持
- [x] 错误处理和重试

### 阶段二：AI 服务集成

#### 7. AI 对话管理 ✅
- [x] 对话上下文管理（加载最近20条历史消息）
- [x] Token 用量统计（已有基础实现）
- [x] 对话历史存储（对话历史持久化 + 分页查询）
- [x] 模型切换功能（请求级模型覆盖已实现）

#### 8. 国内模型支持 ✅
- [x] 通义千问集成（qwen-turbo, qwen-plus, qwen-max）
- [x] 智谱AI集成（glm-4, glm-4-flash, glm-4-air）
- [x] 文心一言集成（ernie-3.5-8k, ernie-4.0-8k, ernie-speed-8k）
- [ ] 模型路由策略（可选）

### 阶段三：文件服务

#### 9. 文件下载和预览 ✅
- [x] 文件下载 API（已在 Task 5 中实现）
- [x] 图片缩略图获取
- [x] 文件权限控制（用户所有权验证）
- [ ] CDN 集成（可选）

#### 10. 文件管理功能 ✅
- [x] 文件列表查询（分页、按类型过滤）
- [x] 文件删除功能
- [ ] 文件分享功能
- [x] 存储空间统计

### 阶段四：消息持久化

#### 11. 消息存储优化 ✅
- [x] 消息持久化实现（已完成基础CRUD）
- [x] 历史消息分页加载（支持before_message_id游标分页）
- [x] 消息搜索（后端ILIKE关键词搜索 + GIN索引优化）
- [x] 消息索引优化（pg_trgm扩展 + GIN索引）

#### 12. 群聊功能 ✅
- [x] 群聊创建和管理（群组创建、信息更新、群公告）
- [x] 群成员管理（添加/移除成员、成员列表查询）
- [x] 群消息广播（通过WebSocket实现）
- [x] @提及功能（基础支持）

### 阶段五：高级功能

#### 13. 消息推送通知 ✅
- [x] 重写 push-service 全部代码（models, handlers, services, repository, main）
- [x] 修复 AppState Clone trait 实现
- [x] push-service 编译通过（cargo check）
- [x] 完整的推送 API（发送、批量、模板、历史、统计、清理）
- [x] 模板管理 API（创建、列表、删除）
- [x] 多平台推送模型定义（APNs, FCM, Web Push）
- [x] 移动端推送集成（设备注册/注销、设备列表）
- [x] 桌面通知支持（Web Push 模型、通知偏好管理）
- [x] 推送配置管理（配置 CRUD API）
- [x] 推送统计和监控（健康状态、测试推送）

#### 14. 会话管理增强 ✅
- [x] 修复 ConversationEntity 缺少 created_by 字段的 bug
- [x] 会话置顶功能（toggle pin handler + API endpoint）
- [x] 免打扰设置（toggle mute handler + API endpoint）
- [x] 会话归档（toggle archive handler + API endpoint + is_archived model/DB column）
- [x] 会话搜索（search handler + API endpoint + ILIKE query）
- [x] 会话标签/分组（tag CRUD APIs + conversation-tag links）
- [x] 会话排序策略（sort by updated_at, created_at, name, unread_count）

#### 15. 消息加密 🔄
- [ ] 端到端加密设计
- [ ] 密钥交换协议
- [ ] 加密消息存储
- [ ] 解密消息显示

## 🔧 技术债务处理

### 编译错误修复
- [ ] 修复 common crate 依赖问题
- [ ] 修复 im-api 类型推断错误
- [ ] 修复 axum 版本兼容性问题
- [ ] 添加缺失的类型定义

### 测试和文档
- [ ] 添加单元测试
- [ ] 添加集成测试
- [ ] 完善 API 文档
- [ ] 添加代码注释

## 🔧 技术债务处理

### 编译错误修复
- [ ] 修复 common crate 依赖问题
- [ ] 修复 im-api 类型推断错误
- [ ] 修复 axum 版本兼容性问题
- [ ] 添加缺失的类型定义

### 测试和文档
- [ ] 添加单元测试
- [ ] 添加集成测试
- [ ] 完善 API 文档
- [ ] 添加代码注释

## 📊 任务状态说明
- ✅ 已完成
- ✅ 待开发 (commit: c42e0a06)- 🔄 进行中
- ⚠️ 受阻（记录原因）
- ❌ 已放弃

## 📝 开发日志格式

每个任务完成后，添加日志：

```markdown
### YYYY-MM-DD HH:MM - 任务名称
**状态**: ✅ 完成 / ⚠️ 受阻 / ❌ 放弃
**耗时**: X小时
**提交**: commit_hash
**备注**: 
- 完成的功能
- 遇到的问题
- 后续改进
```

## 🎯 今日目标（示例）

### 目标1：实现在线状态同步
- 预计耗时：2小时
- 优先级：高
- 依赖：Redis

### 目标2：完善 WebSocket 认证
- 预计耗时：1.5小时
- 优先级：高
- 依赖：JWT

### 目标3：修复已知编译错误
- 预计耗时：2小时
- 优先级：中
- 依赖：Rust 工具链

### 目标4：添加文件上传基础实现
- 预计耗时：2.5小时
- 优先级：中
- 依赖：MinIO

## ⚡ 快速任务（可穿插执行）

- [ ] 更新 Cargo.toml 依赖版本
- [ ] 添加 .gitignore 规则
- [ ] 清理无用代码
- [ ] 优化导入语句
- [ ] 添加类型注释
- [ ] 修复 linter 警告

## 🚫 已知阻塞项

### 服务器资源限制
- 2核2G 内存，无 Swap
- 编译时需要设置 CARGO_BUILD_JOBS=1
- 大型依赖编译可能 OOM

### 外部依赖
- 需要 OpenAI API Key
- 需要 MinIO 服务
- 需要 Redis/PostgreSQL 运行

## 📈 进度追踪

**总任务数**: 15
**已完成**: 14
**进行中**: 1
**待开发**: 0
**受阻**: 0

**完成率**: 93%

---

*最后更新: 2026-05-13 04:30*

### 2026-05-13 04:30 - 消息推送通知（Task 13）完成
**状态**: ✅ 完成
**耗时**: 0.5小时
**提交**: 37d6514
**备注**:
- 新增设备管理API：注册、注销、获取用户设备列表
- 新增通知偏好管理API：获取/更新通知偏好（免打扰时段等）
- 新增推送配置管理API：配置项的增删改查
- 新增推送健康监控端点：设备统计、失败率、成功率
- 新增测试推送端点
- 所有API包含完整模型定义和路由注册

### 2026-05-13 04:15 - 会话标签/分组和排序策略（Task 14）
**状态**: ✅ 完成
**耗时**: 0.5小时
**提交**: 3d270f0
**备注**:
- 新增会话标签CRUD API：创建、删除、获取用户标签
- 新增会话-标签关联API：添加/移除/获取会话标签
- 新增会话排序支持：按更新时间、创建时间、名称、未读数排序
- 新增 conversation_tags 和 conversation_tag_links 数据库表
- 新增 push_devices 和 push_config 数据库表（为推送集成准备）
- 所有API包含认证中间件和权限检查

### 2026-05-12 00:45 - WebSocket认证逻辑完善
**状态**: ✅ 完成
**耗时**: 0.25小时
**提交**: 10840cd
**备注**: 
- Added TokenRefresh and RefreshOk WebSocket message types
- Enhanced error handling with specific error codes
- Token refresh without disconnecting WebSocket session
- Better authentication error messages

### 2026-05-13 00:25 - 文件上传API实现（Task 5, 9, 10）
**状态**: ✅ 完成
**耗时**: 0.5小时
**提交**: 7cd30de
**备注**:
- 修复 file-service 全部28个编译错误
- 重写 middleware.rs：采用 im-gateway 的 auth_middleware + AuthUser 模式
- 修复 handlers.rs：使用 State<Arc<AppState>> 兼容 Axum 0.7
- 实现完整的文件服务 API：上传、下载、删除、列表、更新、缩略图、存储统计
- 添加 tracing-subscriber 依赖
- 所有 cargo check warning 清零（file-service 范围内）

### 2026-05-13 01:15 - AI模型对接（Task 6）进行中
**状态**: 🔄 进行中
**耗时**: 1小时
**提交**: 1de9c1d
**备注**:
- 修复 ai-service 全部编译错误（E0615, E0382, E0308, E0599, E0601）
- 重写 main.rs：添加 #[tokio::main]、修复 DatabaseManager 参数、pg_pool().clone()
- 重写 chat_stream：从模拟数据切换到真实 AI provider 流式调用
- 更新 handlers.rs：使用 service.chat_stream() 获取 provider stream，映射为 SSE 事件
- 清理所有 ai-service 范围内 warning
- 3个 provider（OpenAI、Anthropic、Google）均已实现 chat_completion_stream
- 剩余：错误重试逻辑

### 2026-05-13 03:00 - 群聊功能（Task 12）完成
**状态**: ✅ 完成
**耗时**: 0.5小时
**提交**: 1d6a579
**备注**:
- 新增群组管理API：成员列表、添加/移除成员、更新群信息
- 新增群公告API：获取/更新群公告
- 实现群主权限控制：只有群主可以管理成员和群信息
- 普通成员可以主动退出群聊

### 2026-05-13 02:30 - 消息存储优化（Task 11）完成
**状态**: ✅ 完成
**耗时**: 0.5小时
**提交**: 2c5dc0d
**备注**:
- 新增消息搜索API：GET /api/im/conversations/:id/messages/search
- 新增消息统计API：GET /api/im/conversations/:id/messages/stats
- 使用pg_trgm扩展 + GIN索引优化ILIKE搜索性能
- 支持会话内关键词搜索和跨会话搜索

### 2026-05-13 02:00 - 国内模型支持（Task 8）完成
**状态**: ✅ 完成
**耗时**: 0.5小时
**提交**: ffae638
**备注**:
- 新增 QwenProvider：通义千问（qwen-turbo, qwen-plus, qwen-max）
- 新增 ZhipuProvider：智谱AI（glm-4, glm-4-flash, glm-4-air）
- 新增 ErnieProvider：文心一言（ernie-3.5-8k, ernie-4.0-8k, ernie-speed-8k）
- 所有 provider 支持流式响应
- 所有 provider 包含中文定价信息
- 更新 main.rs 读取 QWEN_API_KEY, ZHIPU_API_KEY, ERNIE_API_KEY, ERNIE_SECRET_KEY

### 2026-05-13 01:30 - AI对话管理（Task 7）进行中
**状态**: 🔄 进行中
**耗时**: 0.5小时
**提交**: 3c55964, b448129
**备注**:
- 实现对话历史持久化：对话消息的创建、查询、删除
- 实现对话上下文加载：chat 和 chat_stream 自动加载最近20条历史消息
- 请求级模型覆盖：request.model_id 优先于 assistant.model_id
- 对话历史分页查询 API：GET /conversations/:id/messages
- 对话清除 API：DELETE /conversations/:id
- 添加 PaginationQuery 结构体
- 新增 get_conversation_history 和 clear_conversation 路由

### 2026-05-12 00:30 - 在线状态同步
**状态**: ✅ 完成
**耗时**: 0.5小时
**提交**: 6b1c5dc
**备注**: 
- Enhanced OnlineStatusManager with Redis-backed status storage
- Added StatusChange WebSocket message type for real-time status broadcasts
- Users now broadcast online/offline status to all connected clients
- Added batch status query API (POST /users/status/batch)
- Added background cleanup task for expired status (30s interval)
- Status manager loads previous state from Redis on startup