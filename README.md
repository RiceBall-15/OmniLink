# OmniLink

<p align="center">
  <strong>下一代智能即时通讯平台</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/version-2.9-blue.svg" alt="Version">
  <img src="https://img.shields.io/badge/license-MIT-green.svg" alt="License">
  <img src="https://img.shields.io/badge/Rust-1.70+-orange.svg" alt="Rust">
  <img src="https://img.shields.io/badge/React-18+-cyan.svg" alt="React">
  <img src="https://img.shields.io/badge/coverage-TODO-red.svg" alt="Coverage">
</p>

---

## 📖 项目简介

OmniLink 是一个现代化的智能即时通讯平台，支持多模型 AI 对话、文件管理、群聊等企业级功能。采用 Rust 微服务架构后端 + React TypeScript 前端的技术栈。

## ✨ 核心功能

### 💬 即时通讯
- 实时消息传递（WebSocket）
- 消息已读回执
- 消息编辑与撤回
- 端到端加密（基础支持）
- 群聊功能
- 消息搜索

### 🤖 AI 助手
- 多模型支持（OpenAI、Claude、国产模型）
- 流式响应
- 对话历史管理
- Token 用量追踪

### 📁 文件管理
- 文件上传/下载
- 图片/文档/视频预览
- 文件分享链接
- MinIO 对象存储

### 👥 社交功能
- 在线状态同步
- 联系人管理
- 通知系统
- 用户资料管理

### 🏢 企业级特性
- 管理员仪表板
- 系统监控与告警
- API 限流
- 断路器模式
- 优雅停机
- 结构化日志

## 🏗️ 技术架构

```
┌─────────────────────────────────────────────────────────────┐
│                      Frontend (React)                       │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────────────┐  │
│  │  Chat   │ │  Admin  │ │  File   │ │  User Profile   │  │
│  │  Page   │ │Dashboard│ │ Manager │ │     Page        │  │
│  └─────────┘ └─────────┘ └─────────┘ └─────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                     API Gateway (im-api)                     │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────────────┐  │
│  │  Auth   │ │  Rate   │ │ Circuit │ │    Versioning   │  │
│  │  JWT    │ │ Limiter │ │ Breaker │ │    Middleware    │  │
│  └─────────┘ └─────────┘ └─────────┘ └─────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        ▼                     ▼                     ▼
┌───────────────┐    ┌───────────────┐    ┌───────────────┐
│  User Service │    │ Message Svc   │    │  AI Service   │
│  (user-svc)   │    │ (msg-svc)     │    │  (ai-svc)     │
└───────────────┘    └───────────────┘    └───────────────┘
        │                     │                     │
        ▼                     ▼                     ▼
┌───────────────┐    ┌───────────────┐    ┌───────────────┐
│  PostgreSQL   │    │   MongoDB     │    │   Redis       │
└───────────────┘    └───────────────┘    └───────────────┘
```

## 📁 项目结构

```
omnilink/
├── crates/                    # Rust 微服务
│   ├── common/               # 共享库（错误处理、日志、断路器等）
│   ├── im-api/               # API 网关
│   ├── user-svc/             # 用户服务
│   ├── msg-svc/              # 消息服务
│   ├── ai-svc/               # AI 服务
│   ├── file-svc/             # 文件服务
│   ├── push-svc/             # 推送服务
│   ├── usage-svc/            # 用量统计服务
│   └── config-svc/           # 配置服务
├── frontend/                  # 前端应用
│   └── web/                  # React Web 应用
│       └── src/
│           ├── components/   # UI 组件
│           ├── hooks/        # 自定义 Hooks
│           ├── services/     # API 服务
│           ├── stores/       # 状态管理
│           ├── pages/        # 页面组件
│           └── types/        # TypeScript 类型
├── docs/                      # 项目文档
│   ├── api/                  # API 文档
│   ├── deployment/           # 部署指南
│   └── security/             # 安全审计
├── docker/                    # Docker 配置
├── .github/                   # GitHub Actions
└── monitoring/                # 监控配置
```

## 🚀 快速开始

### 环境要求

- **Rust**: 1.70+
- **Node.js**: 18+
- **PostgreSQL**: 14+
- **MongoDB**: 6.0+
- **Redis**: 7.0+
- **MinIO**: 最新版（可选，用于文件存储）

### 安装步骤

1. **克隆仓库**
   ```bash
   git clone https://github.com/your-org/omnilink.git
   cd omnilink
   ```

2. **配置环境变量**
   ```bash
   cp .env.example .env
   # 编辑 .env 文件，配置数据库连接等
   ```

3. **启动后端服务**
   ```bash
   # 开发模式
   cargo run --bin im-api
   
   # 或使用 Docker Compose
   docker-compose up -d
   ```

4. **启动前端开发服务器**
   ```bash
   cd frontend/web
   npm install
   npm run dev
   ```

5. **访问应用**
   - 前端: http://localhost:5173
   - API: http://localhost:8001

### Docker 部署

```bash
# 构建并启动所有服务
docker-compose up -d

# 查看日志
docker-compose logs -f

# 停止服务
docker-compose down
```

## 📚 文档

- [API 文档](docs/api/) - RESTful API 接口说明
- [部署指南](docs/deployment/DEPLOYMENT_GUIDE.md) - 单机/集群部署
- [运维手册](docs/deployment/OPERATIONS_MANUAL.md) - 日常运维操作
- [故障排查](docs/deployment/TROUBLESHOOTING.md) - 常见问题解决
- [安全审计](docs/security/SECURITY_AUDIT.md) - 安全检查清单

## 🧪 测试

### 运行后端测试

```bash
# 运行所有测试
cargo test

# 运行特定服务测试
cargo test -p user-svc
cargo test -p msg-svc

# 运行测试并生成覆盖率报告
cargo tarpaulin --out Html
```

### 运行前端测试

```bash
cd frontend/web

# 运行单元测试
npm test

# 运行 E2E 测试
npm run test:e2e

# 生成覆盖率报告
npm run test:coverage
```

## 📊 监控

### Prometheus + Grafana

访问 Grafana 仪表板: http://localhost:3000

- **系统概览**: CPU、内存、磁盘使用率
- **服务状态**: 请求速率、响应时间、错误率
- **业务指标**: 消息发送量、在线用户数

### 告警规则

- 服务宕机告警
- 高错误率告警
- 高延迟告警
- 资源使用率告警

## 🔧 配置

### 环境变量

| 变量名 | 说明 | 默认值 |
|--------|------|--------|
| `DATABASE_URL` | PostgreSQL 连接字符串 | - |
| `MONGODB_URL` | MongoDB 连接字符串 | - |
| `REDIS_URL` | Redis 连接字符串 | - |
| `JWT_SECRET` | JWT 签名密钥 | - |
| `MINIO_ENDPOINT` | MinIO 服务端点 | localhost:9000 |
| `LOG_LEVEL` | 日志级别 | info |
| `RATE_LIMIT_PER_MINUTE` | 每分钟请求限制 | 60 |

### 配置文件

```toml
# config.toml
[server]
host = "0.0.0.0"
port = 8001

[database]
max_connections = 100
min_connections = 5

[redis]
pool_size = 10

[ai]
default_model = "gpt-3.5-turbo"
max_tokens = 4096
```

## 🛡️ 安全

- JWT 认证
- 请求限流
- 输入验证（SQL 注入、XSS 防护）
- HTTPS 支持
- CORS 配置
- 密码 bcrypt 加密

详见 [安全审计文档](docs/security/SECURITY_AUDIT.md)

## 🤝 贡献指南

1. Fork 本仓库
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'feat: add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建 Pull Request

### Commit 规范

使用 [Conventional Commits](https://www.conventionalcommits.org/) 格式：

```
<type>(<scope>): <subject>

<body>

<footer>
```

类型:
- `feat`: 新功能
- `fix`: Bug 修复
- `docs`: 文档更新
- `style`: 代码格式调整
- `refactor`: 代码重构
- `test`: 测试相关
- `chore`: 构建/工具相关

## 📄 许可证

本项目采用 MIT 许可证 - 详见 [LICENSE](LICENSE) 文件

## 📞 联系方式

- 项目主页: https://github.com/your-org/omnilink
- 问题反馈: https://github.com/your-org/omnilink/issues
- 邮箱: team@omnilink.dev

---

<p align="center">
  Made with ❤️ by OmniLink Team
</p>
