# OmniLink - 通用IM AI聊天应用

> 一个类似"微信 + OpenAI"的跨平台智能对话应用

## 项目简介

OmniLink是一款功能强大的跨平台IM应用，集成了多个大语言模型（LLM）提供商，为用户提供类似微信的即时通讯体验，同时支持与多个AI助手的对话。

### 核心功能

- **多模型对话**：支持OpenAI、Anthropic、Google、国内大模型等多厂商LLM接入
- **即时通讯**：类似微信的单聊、群聊、消息收发、已读回执
- **AI助手管理**：创建、配置、管理不同的AI助手角色
- **用量统计**：Token消耗、调用次数、费用统计可视化
- **跨平台同步**：手机端与电脑端消息、配置实时同步
- **文件传输**：支持图片、文档等多媒体消息

### 技术架构

#### 后端技术栈
- **语言**：Rust
- **Web框架**：Axum + Tokio
- **数据库**：PostgreSQL 16 + Redis 7 + MongoDB
- **消息队列**：Apache Kafka
- **对象存储**：MinIO
- **搜索引擎**：Elasticsearch 8
- **分析数据库**：ClickHouse

#### 前端技术栈
- **iOS/Android**：Flutter 3.x
- **Windows/macOS**：Tauri (Rust + Web)
- **Web**：React 18 + TypeScript
- **小程序**：UniApp

## 项目结构

```
omnilink/
├── crates/                      # Rust crates
│   ├── common/                  # 公共库
│   ├── im-gateway/             # WebSocket网关
│   ├── im-api/                 # REST API
│   ├── ai-service/             # AI对话服务
│   ├── user-service/           # 用户服务
│   ├── file-service/           # 文件服务
│   ├── usage-service/          # 用量统计服务
│   ├── push-service/           # 推送服务
│   └── config-service/         # 配置服务
├── migrations/                 # 数据库迁移
├── monitoring/                 # 监控配置
├── docs/                       # 文档
├── docker-compose.yml           # Docker Compose配置
├── Cargo.toml                  # Cargo工作区配置
├── .env.example               # 环境变量示例
└── README.md                  # 项目说明
```

## 快速开始

### 环境要求

- Rust 1.75+
- Docker & Docker Compose
- Node.js 18+ (前端开发)

### 安装步骤

1. 克隆项目
```bash
git clone https://github.com/RiceBall-15/OmniLink.git
cd OmniLink
```

2. 配置环境变量
```bash
cp .env.example .env
# 编辑.env文件，填入相关配置
```

3. 启动依赖服务
```bash
docker-compose up -d
```

4. 运行数据库迁移
```bash
# 等待PostgreSQL启动
sqlx migrate run
```

5. 启动服务
```bash
# 开发模式
cargo run -p im-gateway
cargo run -p im-api
cargo run -p ai-service
cargo run -p user-service
```

## 开发计划

详细开发计划请查看 [PROJECT_PLAN.md](PROJECT_PLAN.md)

## 贡献指南

欢迎提交Issue和Pull Request！

## 许可证

MIT License

## 联系方式

- GitHub: [@RiceBall-15](https://github.com/RiceBall-15)
- 项目地址: [https://github.com/RiceBall-15/OmniLink](https://github.com/RiceBall-15/OmniLink)

---

*OmniLink - 让AI对话更简单*