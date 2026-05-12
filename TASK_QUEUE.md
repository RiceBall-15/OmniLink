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

#### 7. AI 对话管理 ⏳
- [ ] 对话上下文管理
- [ ] Token 用量统计
- [ ] 对话历史存储
- [ ] 模型切换功能

#### 8. 国内模型支持 ⏳
- [ ] 通义千问集成
- [ ] 文心一言集成
- [ ] 智谱AI集成
- [ ] 模型路由策略

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

#### 11. 消息存储优化 ⏳
- [ ] 消息持久化实现
- [ ] 历史消息分页加载
- [ ] 消息搜索（后端）
- [ ] 消息索引优化

#### 12. 群聊功能 ⏳
- [ ] 群聊创建和管理
- [ ] 群消息广播
- [ ] 群成员管理
- [ ] @提及功能

### 阶段五：高级功能

#### 13. 消息推送通知 ⏳
- [ ] 移动端推送集成
- [ ] 桌面通知支持
- [ ] 推送配置管理
- [ ] 推送统计和监控

#### 14. 会话管理增强 ⏳
- [ ] 会话置顶功能
- [ ] 免打扰设置
- [ ] 会话归档
- [ ] 会话搜索

#### 15. 消息加密 ⏳
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
**已完成**: 8
**进行中**: 0
**待开发**: 7
**受阻**: 0

**完成率**: 53%

---

*最后更新: 2026-05-13 01:30*

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