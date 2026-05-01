# OmniLink 部署指南

本文档提供 OmniLink 项目的完整部署流程，包括本地开发环境和生产环境的部署方案。

## 📋 目录

- [系统要求](#系统要求)
- [快速启动](#快速启动)
- [本地开发部署](#本地开发部署)
- [生产环境部署](#生产环境部署)
- [服务配置说明](#服务配置说明)
- [常见问题](#常见问题)

---

## 系统要求

### 硬件要求

| 组件 | 开发环境 | 生产环境（推荐） |
|------|----------|------------------|
| CPU | 4核+ | 8核+ |
| 内存 | 8GB+ | 16GB+ |
| 磁盘 | 50GB+ | 100GB+ SSD |

### 软件要求

- **Docker**: 20.10+
- **Docker Compose**: 2.0+
- **Git**: 2.30+
- **Node.js**: 18+ (前端开发)
- **Rust**: 1.75+ (后端开发)

### 操作系统

- Linux (推荐: Ubuntu 22.04 / CentOS 8+)
- macOS (开发环境)
- Windows 10/11 (开发环境，需 WSL2)

---

## 快速启动

### 一键启动所有服务

```bash
# 1. 克隆项目
git clone https://github.com/RiceBall-15/OmniLink.git
cd OmniLink

# 2. 配置环境变量
cp .env.example .env
# 编辑 .env 文件，根据需要修改配置

# 3. 启动所有服务（包括数据库、缓存、消息队列等）
docker compose up -d

# 4. 等待服务启动完成（约 30-60 秒）
docker compose ps

# 5. 运行数据库迁移
docker compose exec -T postgres psql -U im_chat -d im_chat -f /docker-entrypoint-initdb.d/001_initial.sql

# 6. 启动后端服务
cargo run --bin omnilink

# 7. 启动前端服务（新终端）
cd frontend/web
npm install
npm run dev
```

### 访问服务

| 服务 | 地址 | 说明 |
|------|------|------|
| **Web 前端** | http://localhost:3000 | React 应用 |
| **IM API** | http://localhost:8002 | REST API |
| **IM Gateway (WebSocket)** | ws://localhost:8001 | WebSocket 连接 |
| **AI Service** | http://localhost:8003 | AI 对话服务 |
| **MinIO 控制台** | http://localhost:9001 | 对象存储管理 (admin/minioadmin) |
| **Grafana** | http://localhost:3001 | 监控面板 (admin/admin123) |
| **Prometheus** | http://localhost:9090 | 指标查询 |

---

## 本地开发部署

### 前端开发

```bash
cd frontend/web

# 安装依赖
npm install

# 开发模式（热更新）
npm run dev

# 生产构建
npm run build

# 预览生产构建
npm run preview
```

### 后端开发

```bash
# 开发模式（自动重载）
cargo watch -x run

# 运行特定服务
cargo run -p im-gateway
cargo run -p im-api
cargo run -p ai-service
cargo run -p user-service

# 运行测试
cargo test

# 运行测试并查看输出
cargo test -- --nocapture
```

### 数据库迁移

```bash
# 运行所有迁移
sqlx migrate run

# 创建新迁移
sqlx migrate add <migration_name>

# 回滚最后一条迁移
sqlx migrate revert
```

---

## 生产环境部署

### 1. 环境准备

```bash
# 创建部署目录
mkdir -p /opt/omnilink
cd /opt/omnilink

# 克隆代码
git clone https://github.com/RiceBall-15/OmniLink.git .
git checkout main
```

### 2. 配置环境变量

```bash
cp .env.example .env

# 编辑生产配置
vim .env
```

**关键配置项：**

```bash
# 数据库密码（必须修改）
POSTGRES_PASSWORD=<your-strong-password>
REDIS_PASSWORD=<your-strong-password>
MONGO_PASSWORD=<your-strong-password>

# JWT 密钥（必须修改）
JWT_SECRET=<your-256-bit-secret>

# AI API 密钥
OPENAI_API_KEY=<your-openai-key>
ANTHROPIC_API_KEY=<your-anthropic-key>
```

### 3. 启动基础设施

```bash
# 启动所有依赖服务
docker compose up -d postgres redis mongodb kafka clickhouse minio prometheus grafana

# 检查服务健康状态
docker compose ps
```

### 4. 构建并部署后端

```bash
# 生产构建
cargo build --release

# 启动后端服务
./target/release/omnilink
```

**使用 systemd 管理（推荐）：**

```bash
# 创建 systemd 服务文件
sudo vim /etc/systemd/system/omnilink.service
```

```ini
[Unit]
Description=OmniLink Backend Service
After=network.target docker-compose.service

[Service]
Type=simple
User=root
WorkingDirectory=/opt/omnilink
ExecStart=/opt/omnilink/target/release/omnilink
Restart=always
RestartSec=10
Environment="RUST_LOG=info"
Environment="RUST_BACKTRACE=1"

[Install]
WantedBy=multi-user.target
```

```bash
# 启用并启动服务
sudo systemctl daemon-reload
sudo systemctl enable omnilink
sudo systemctl start omnilink

# 查看日志
sudo journalctl -u omnilink -f
```

### 5. 部署前端

```bash
cd frontend/web

# 构建生产版本
npm run build

# 使用 Nginx 部署
sudo cp -r dist/* /var/www/omnilink/
```

**Nginx 配置示例：**

```nginx
server {
    listen 80;
    server_name your-domain.com;

    root /var/www/omnilink;
    index index.html;

    # 前端路由
    location / {
        try_files $uri $uri/ /index.html;
    }

    # API 代理
    location /api/ {
        proxy_pass http://localhost:8002/;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }

    # WebSocket 代理
    location /ws {
        proxy_pass http://localhost:8001;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
    }

    # 静态文件缓存
    location ~* \.(js|css|png|jpg|jpeg|gif|ico|svg|woff|woff2|ttf|eot)$ {
        expires 1y;
        add_header Cache-Control "public, immutable";
    }
}
```

### 6. 配置 SSL 证书（Let's Encrypt）

```bash
# 安装 Certbot
sudo apt install certbot python3-certbot-nginx

# 获取证书
sudo certbot --nginx -d your-domain.com

# 自动续期
sudo certbot renew --dry-run
```

### 7. 配置防火墙

```bash
# 允许 HTTP/HTTPS
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp

# 允许 SSH
sudo ufw allow 22/tcp

# 启用防火墙
sudo ufw enable
```

---

## 服务配置说明

### PostgreSQL (端口 5432)

**用途：** 用户数据、会话、关系型数据存储

**连接字符串：**
```bash
postgres://im_chat:password@localhost:5432/im_chat
```

**管理命令：**
```bash
# 连接数据库
docker compose exec postgres psql -U im_chat -d im_chat

# 备份数据库
docker compose exec postgres pg_dump -U im_chat im_chat > backup.sql

# 恢复数据库
docker compose exec -T postgres psql -U im_chat im_chat < backup.sql
```

### Redis (端口 6379)

**用途：** 缓存、会话存储、消息队列

**连接字符串：**
```bash
redis://:password@localhost:6379/0
```

**管理命令：**
```bash
# 连接 Redis
docker compose exec redis redis-cli -a password

# 查看所有 key
docker compose exec redis redis-cli -a password KEYS '*'

# 清空所有数据
docker compose exec redis redis-cli -a password FLUSHALL
```

### MongoDB (端口 27017)

**用途：** AI 对话记录、非结构化数据

**连接字符串：**
```bash
mongodb://admin:password@localhost:27017/im_chat?authSource=admin
```

**管理命令：**
```bash
# 连接 MongoDB
docker compose exec mongodb mongosh -u admin -p password

# 备份数据库
docker compose exec mongodb mongodump -u admin -p password --db im_chat --out /backup

# 恢复数据库
docker compose exec mongodb mongorestore -u admin -p password --db im_chat /backup/im_chat
```

### Kafka (端口 9092)

**用途：** 消息队列、事件流处理

**管理命令：**
```bash
# 查看所有 Topic
docker compose exec kafka kafka-topics --list --bootstrap-server localhost:9092

# 创建 Topic
docker compose exec kafka kafka-topics --create --bootstrap-server localhost:9092 --topic messages --partitions 3 --replication-factor 1

# 查看消息
docker compose exec kafka kafka-console-consumer --bootstrap-server localhost:9092 --topic messages --from-beginning
```

### MinIO (端口 9000/9001)

**用途：** 对象存储、文件上传

**访问信息：**
- API: http://localhost:9000
- 控制台: http://localhost:9001
- 用户名: minioadmin
- 密码: minioadmin

**管理命令：**
```bash
# 使用 mc 客户端
wget https://dl.min.io/client/mc/release/linux-amd64/mc
chmod +x mc
./mc alias set local http://localhost:9000 minioadmin minioadmin

# 创建 Bucket
./mc mb local/omnilink-files

# 设置访问策略
./mc policy set download local/omnilink-files
```

### ClickHouse (端口 8123/9000)

**用途：** 分析数据库、日志存储

**管理命令：**
```bash
# 连接数据库
docker compose exec clickhouse clickhouse-client

# 执行查询
docker compose exec clickhouse clickhouse-client --query "SELECT * FROM system.tables LIMIT 10"
```

### Prometheus + Grafana

**用途：** 监控、指标可视化

- **Prometheus**: http://localhost:9090
- **Grafana**: http://localhost:3001 (admin/admin123)

**导入 Dashboard：**
1. 登录 Grafana
2. 创建数据源 (Prometheus: http://prometheus:9090)
3. 导入 Dashboard ID: 1860 (Node Exporter Full)

---

## 常见问题

### 1. Docker 容器启动失败

```bash
# 查看容器日志
docker compose logs postgres
docker compose logs redis

# 重新启动
docker compose down
docker compose up -d
```

### 2. 端口冲突

```bash
# 修改 docker-compose.yml 中的端口映射
ports:
  - "5433:5432"  # PostgreSQL 使用 5433 端口
```

### 3. 内存不足

```bash
# 限制 Docker 内存使用
vim /etc/docker/daemon.json

{
  "default-runtime": "runc",
  "default-ulimits": {
    "memlock": {
      "Name": "memlock",
      "Hard": -1,
      "Soft": -1
    }
  },
  "max-concurrent-downloads": 3,
  "max-download-attempts": 5
}

sudo systemctl restart docker
```

### 4. 数据库连接失败

```bash
# 检查网络连接
docker network ls
docker network inspect omnilink_omnilink-network

# 测试连接
docker compose exec postgres pg_isready -U im_chat
```

### 5. WebSocket 连接断开

```bash
# 检查反向代理配置
# 确保 Nginx 正确代理 WebSocket 连接
proxy_http_version 1.1;
proxy_set_header Upgrade $http_upgrade;
proxy_set_header Connection "upgrade";
```

### 6. 文件上传失败

```bash
# 检查 MinIO 服务状态
docker compose logs minio

# 验证访问密钥
docker compose exec minio mc alias set local http://localhost:9000 minioadmin minioadmin
```

---

## 监控与日志

### 查看服务日志

```bash
# 查看所有服务日志
docker compose logs -f

# 查看特定服务日志
docker compose logs -f postgres
docker compose logs -f im-gateway

# 查看后端服务日志
sudo journalctl -u omnilink -f
```

### 性能监控

```bash
# 查看容器资源使用
docker stats

# 查看系统资源
htop
```

### 数据库性能分析

```bash
# PostgreSQL 慢查询
docker compose exec postgres psql -U im_chat -d im_chat -c "SELECT * FROM pg_stat_statements ORDER BY total_time DESC LIMIT 10;"

# Redis 性能
docker compose exec redis redis-cli -a password INFO stats
```

---

## 备份与恢复

### 数据备份

```bash
#!/bin/bash
# backup.sh

BACKUP_DIR="/backup/omnilink/$(date +%Y%m%d_%H%M%S)"
mkdir -p $BACKUP_DIR

# 备份 PostgreSQL
docker compose exec -T postgres pg_dump -U im_chat im_chat > $BACKUP_DIR/postgres.sql

# 备份 MongoDB
docker compose exec mongodb mongodump -u admin -p password --db im_chat --out $BACKUP_DIR/mongodb

# 备份 Redis
docker compose exec redis redis-cli -a password --rdb $BACKUP_DIR/redis/dump.rdb

# 备份 MinIO
./mc mirror local/omnilink-files $BACKUP_DIR/minio

echo "Backup completed: $BACKUP_DIR"
```

### 数据恢复

```bash
#!/bin/bash
# restore.sh

BACKUP_DIR=$1

# 恢复 PostgreSQL
docker compose exec -T postgres psql -U im_chat im_chat < $BACKUP_DIR/postgres.sql

# 恢复 MongoDB
docker compose exec mongodb mongorestore -u admin -p password --db im_chat $BACKUP_DIR/mongodb/im_chat

# 恢复 Redis
docker compose exec redis redis-cli -a password --rdb $BACKUP_DIR/redis/dump.rdb

echo "Restore completed from: $BACKUP_DIR"
```

---

## 更新与维护

### 更新代码

```bash
# 拉取最新代码
git pull origin main

# 重新构建后端
cargo build --release

# 重启服务
sudo systemctl restart omnilink
```

### 数据库迁移

```bash
# 运行新的迁移
sqlx migrate run

# 回滚（如需要）
sqlx migrate revert
```

---

## 安全建议

1. **修改默认密码**
   - 修改所有数据库和服务的默认密码
   - 使用强密码（16+ 字符）

2. **启用 SSL/TLS**
   - 使用 Let's Encrypt 免费证书
   - 强制 HTTPS 访问

3. **防火墙配置**
   - 只开放必要的端口
   - 限制数据库端口仅本地访问

4. **定期备份**
   - 设置自动备份任务（每天）
   - 验证备份可用性

5. **监控告警**
   - 配置 Grafana 告警
   - 监控服务健康状态

---

## 技术支持

- **GitHub Issues**: https://github.com/RiceBall-15/OmniLink/issues
- **文档**: https://github.com/RiceBall-15/OmniLink/wiki
- **邮件**: support@omnilink.com

---

## 许可证

MIT License - 详见 [LICENSE](LICENSE) 文件
