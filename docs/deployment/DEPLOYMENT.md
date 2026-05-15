# OmniLink 部署指南

## 概述

OmniLink 是一个企业级即时通讯系统，采用 Rust 微服务架构，支持单机和集群部署。

---

## 系统要求

### 最低配置（开发/测试）

| 资源 | 要求 |
|------|------|
| CPU | 2 核 |
| 内存 | 4 GB |
| 磁盘 | 20 GB |
| OS | Ubuntu 22.04+ / Debian 12+ |

### 推荐配置（生产环境）

| 资源 | 要求 |
|------|------|
| CPU | 4+ 核 |
| 内存 | 8+ GB |
| 磁盘 | 100+ GB SSD |
| OS | Ubuntu 22.04 LTS |

### 依赖服务

| 服务 | 版本 | 用途 |
|------|------|------|
| PostgreSQL | 16+ | 主数据库 |
| Redis | 7+ | 缓存、会话、Pub/Sub |
| MinIO | latest | 文件对象存储 |
| Nginx | 1.24+ | 反向代理（可选） |

---

## 快速部署（Docker Compose）

### 1. 克隆项目

```bash
git clone https://github.com/your-org/omnilink.git
cd omnilink
```

### 2. 配置环境变量

```bash
cp .env.example .env
# 编辑 .env 文件，修改以下关键配置：
# - POSTGRES_PASSWORD: 数据库密码
# - REDIS_PASSWORD: Redis 密码
# - MINIO_ROOT_USER/MINIO_ROOT_PASSWORD: MinIO 凭据
# - JWT_SECRET: JWT 签名密钥（务必使用强随机字符串）
```

### 3. 启动服务

```bash
# 构建并启动所有服务
docker-compose up -d --build

# 查看服务状态
docker-compose ps

# 查看日志
docker-compose logs -f im-api
```

### 4. 初始化数据库

```bash
# 数据库迁移会在 PostgreSQL 容器启动时自动执行
# 如果需要手动迁移：
docker-compose exec postgres psql -U omnilink -d omnilink -f /docker-entrypoint-initdb.d/001_initial_schema.sql
```

### 5. 验证部署

```bash
# 检查 API 健康状态
curl http://localhost:8080/health

# 检查 WebSocket Gateway
curl http://localhost:8081/health

# 访问前端
# http://localhost (通过 Nginx)
```

---

## 单机部署（无 Docker）

### 1. 安装依赖

```bash
# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable

# Node.js (前端构建)
curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
sudo apt-get install -y nodejs

# 系统依赖
sudo apt-get install -y pkg-config libssl-dev libpq-dev protobuf-compiler
```

### 2. 安装依赖服务

```bash
# PostgreSQL
sudo apt-get install -y postgresql postgresql-contrib
sudo systemctl enable postgresql

# Redis
sudo apt-get install -y redis-server
sudo systemctl enable redis-server

# MinIO
wget https://dl.min.io/server/minio/release/linux-amd64/minio
chmod +x minio
sudo mv minio /usr/local/bin/
```

### 3. 构建项目

```bash
# 后端
export CARGO_BUILD_JOBS=2
cargo build --release

# 前端
cd frontend/web
npm ci
npm run build
cd ../..
```

### 4. 配置服务

```bash
# 复制配置文件
sudo mkdir -p /etc/omnilink
sudo cp config/*.toml /etc/omnilink/

# 创建 systemd 服务文件
sudo tee /etc/systemd/system/omnilink-api.service << 'EOF'
[Unit]
Description=OmniLink API Server
After=network.target postgresql.service redis-server.service

[Service]
Type=simple
User=omnilink
Group=omnilink
WorkingDirectory=/opt/omnilink
ExecStart=/opt/omnilink/target/release/im-api
Environment=DATABASE_URL=postgresql://omnilink:PASSWORD@localhost:5432/omnilink
Environment=REDIS_URL=redis://:PASSWORD@localhost:6379
Environment=RUST_LOG=info
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

# 类似地创建 im-gateway 和 im-worker 服务
```

### 5. 启动服务

```bash
sudo systemctl daemon-reload
sudo systemctl enable omnilink-api omnilink-gateway omnilink-worker
sudo systemctl start omnilink-api omnilink-gateway omnilink-worker
```

---

## 集群部署

### 架构

```
                    ┌─────────────┐
                    │   Nginx LB  │
                    └──────┬──────┘
                           │
              ┌────────────┼────────────┐
              │            │            │
        ┌─────┴─────┐ ┌────┴──────┐ ┌───┴───────┐
        │  im-api-1 │ │ im-api-2  │ │ im-api-3  │
        └─────┬─────┘ └────┬──────┘ └───┬───────┘
              │            │            │
        ┌─────┴─────┐ ┌────┴──────┐ ┌───┴───────┐
        │im-gateway1│ │im-gateway2│ │im-gateway3│
        └─────┬─────┘ └────┬──────┘ └───┬───────┘
              │            │            │
    ┌─────────┴────────────┴────────────┴─────────┐
    │              Redis Cluster                   │
    └──────────────────┬───────────────────────────┘
                       │
    ┌──────────────────┴───────────────────────────┐
    │         PostgreSQL (Primary + Replica)        │
    └──────────────────────────────────────────────┘
```

### 使用 Docker Swarm

```bash
# 初始化 Swarm
docker swarm init

# 部署 Stack
docker stack deploy -c docker-compose.yml omnilink

# 扩展服务
docker service scale omnilink_im-api=3 omnilink_im-gateway=3
```

### 使用 Kubernetes

参考 `deploy/kubernetes/` 目录中的 Helm Chart。

---

## Nginx 配置

```nginx
upstream omnilink_api {
    least_conn;
    server 127.0.0.1:8080;
    # 添加更多 API 实例：
    # server 127.0.0.1:8082;
    # server 127.0.0.1:8084;
}

upstream omnilink_ws {
    ip_hash;  # WebSocket 需要会话粘性
    server 127.0.0.1:8081;
    # 添加更多 Gateway 实例：
    # server 127.0.0.1:8083;
}

server {
    listen 80;
    server_name im.example.com;

    # API 请求
    location /api/ {
        proxy_pass http://omnilink_api;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    # WebSocket 连接
    location /ws {
        proxy_pass http://omnilink_ws;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_read_timeout 86400s;
        proxy_send_timeout 86400s;
    }

    # 前端静态文件
    location / {
        root /app/static;
        try_files $uri $uri/ /index.html;
    }
}
```

---

## 备份与恢复

### 数据库备份

```bash
# 每日备份脚本
#!/bin/bash
BACKUP_DIR="/backup/postgresql"
DATE=$(date +%Y%m%d_%H%M%S)
docker-compose exec -T postgres pg_dump -U omnilink omnilink | gzip > "$BACKUP_DIR/omnilink_$DATE.sql.gz"

# 保留最近 30 天
find "$BACKUP_DIR" -name "*.sql.gz" -mtime +30 -delete
```

### 数据库恢复

```bash
gunzip < backup.sql.gz | docker-compose exec -T postgres psql -U omnilink -d omnilink
```

### MinIO 备份

```bash
# 使用 mc 客户端
mc mirror local/omnilink-files backup/omnilink-files
```

---

## 监控

### Prometheus 指标

API 服务暴露 Prometheus 指标端点：

```
GET /metrics
```

### 关键指标

| 指标 | 说明 | 告警阈值 |
|------|------|----------|
| `http_requests_total` | HTTP 请求总数 | - |
| `http_request_duration_seconds` | 请求延迟 | P99 > 2s |
| `websocket_connections` | WebSocket 连接数 | > 10000 |
| `database_pool_active` | 数据库连接池活跃数 | > 80% |
| `redis_connected_clients` | Redis 连接数 | > 500 |

### Grafana Dashboard

导入 `monitoring/grafana/dashboards/omnilink.json` 获取预配置仪表板。

---

## 故障排查

### 常见问题

| 问题 | 可能原因 | 解决方案 |
|------|----------|----------|
| 服务启动失败 | 数据库未就绪 | 检查 PostgreSQL 状态和连接字符串 |
| WebSocket 连接断开 | Nginx 超时 | 增加 `proxy_read_timeout` |
| 文件上传失败 | MinIO 连接问题 | 检查 MinIO 凭据和 bucket 权限 |
| 内存占用过高 | 连接泄漏 | 检查数据库连接池配置 |
| 编译 OOM | 资源不足 | 设置 `CARGO_BUILD_JOBS=1` |

### 日志查看

```bash
# Docker 环境
docker-compose logs -f im-api --tail 100

# Systemd 环境
journalctl -u omnilink-api -f --no-pager -n 100

# 日志级别调整
export RUST_LOG=debug  # 启用调试日志
```

### 性能调优

```bash
# PostgreSQL
# 调整 shared_buffers = 25% of RAM
# 调整 effective_cache_size = 75% of RAM
# 调整 max_connections = 200

# Redis
# 调整 maxmemory-policy allkeys-lru
# 启用 appendonly yes

# 系统内核参数
sysctl -w net.core.somaxconn=65535
sysctl -w net.ipv4.tcp_max_syn_backlog=65535
```

---

## 安全建议

1. **修改所有默认密码** — 生产环境必须使用强密码
2. **启用 HTTPS** — 使用 Let's Encrypt 或内部 CA
3. **限制网络访问** — 数据库和 Redis 仅允许内网访问
4. **定期更新依赖** — 运行 `cargo audit` 检查安全漏洞
5. **启用审计日志** — 记录所有管理员操作
6. **配置防火墙** — 仅开放必要端口（80, 443）

---

## 更新与升级

```bash
# 拉取最新代码
git pull origin master

# 重新构建
docker-compose build

# 滚动更新
docker-compose up -d --no-deps im-api
docker-compose up -d --no-deps im-gateway
docker-compose up -d --no-deps im-worker

# 验证
curl http://localhost:8080/health
```
