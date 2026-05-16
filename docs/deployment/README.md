# OmniLink 部署指南

## 目录

- [环境要求](#环境要求)
- [单机部署](#单机部署)
- [集群部署](#集群部署)
- [SSL/TLS 配置](#ssltls-配置)
- [环境变量说明](#环境变量说明)

---

## 环境要求

### 硬件要求

| 部署方式 | CPU | 内存 | 磁盘 | 说明 |
|---------|-----|------|------|------|
| 开发环境 | 2核 | 4GB | 20GB | 本地开发测试 |
| 单机生产 | 4核 | 8GB | 50GB | 小规模部署 (< 1000 用户) |
| 集群生产 | 8核+ | 16GB+ | 100GB+ | 大规模部署 (> 1000 用户) |

### 软件要求

- **操作系统**: Ubuntu 20.04+ / CentOS 7+ / Debian 10+
- **Docker**: 20.10+
- **Docker Compose**: 2.0+
- **Git**: 2.0+
- **Nginx** (可选): 1.18+ (用于反向代理)

---

## 单机部署

### 1. 环境准备

```bash
# 更新系统包
sudo apt update && sudo apt upgrade -y

# 安装 Docker
curl -fsSL https://get.docker.com | sudo sh
sudo usermod -aG docker $USER

# 安装 Docker Compose
sudo curl -L "https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
sudo chmod +x /usr/local/bin/docker-compose

# 验证安装
docker --version
docker-compose --version
```

### 2. 克隆项目

```bash
git clone https://github.com/your-org/omnilink.git
cd omnilink
```

### 3. 配置环境变量

```bash
# 复制环境变量模板
cp .env.example .env

# 编辑环境变量（必须修改以下配置）
vim .env
```

**必须修改的配置项**:
- `POSTGRES_PASSWORD` - 数据库密码
- `REDIS_PASSWORD` - Redis 密码
- `JWT_SECRET` - JWT 签名密钥（至少32字符）
- `MINIO_ROOT_PASSWORD` - MinIO 密码

### 4. 启动服务

```bash
# 使用生产环境配置启动
docker-compose -f docker-compose.prod.yml up -d

# 查看服务状态
docker-compose -f docker-compose.prod.yml ps

# 查看日志
docker-compose -f docker-compose.prod.yml logs -f [service-name]
```

### 5. 验证部署

```bash
# 检查所有服务是否健康
docker-compose -f docker-compose.prod.yml ps

# 测试 API
curl http://localhost:8002/health

# 测试 WebSocket
curl http://localhost:3002/health

# 访问 Grafana 监控面板
# http://localhost:3000 (默认账号: admin/admin)
```

---

## 集群部署

### 架构概览

```
                    ┌─────────────────┐
                    │   Load Balancer │
                    │   (Nginx/HAProxy)│
                    └────────┬────────┘
                             │
         ┌───────────────────┼───────────────────┐
         │                   │                   │
    ┌────▼────┐         ┌────▼────┐         ┌────▼────┐
    │ Node 1  │         │ Node 2  │         │ Node 3  │
    │ API+GW  │         │ API+GW  │         │ API+GW  │
    └────┬────┘         └────┬────┘         └────┬────┘
         │                   │                   │
         └───────────────────┼───────────────────┘
                             │
         ┌───────────────────┼───────────────────┐
         │                   │                   │
    ┌────▼────┐         ┌────▼────┐         ┌────▼────┐
    │PostgreSQL│         │  Redis  │         │  MinIO  │
    │ Primary │         │ Cluster │         │ Cluster │
    └─────────┘         └─────────┘         └─────────┘
```

### 使用 Docker Swarm

```bash
# 初始化 Swarm
docker swarm init

# 创建配置文件 secret
echo "your_postgres_password" | docker secret create postgres_password -
echo "your_redis_password" | docker secret create redis_password -
echo "your_jwt_secret" | docker secret create jwt_secret -

# 部署 stack
docker stack deploy -c docker-compose.prod.yml omnilink
```

### 使用 Kubernetes

参见 `k8s/` 目录中的 Kubernetes 配置文件。

---

## SSL/TLS 配置

### 使用 Let's Encrypt

```bash
# 安装 certbot
sudo apt install certbot python3-certbot-nginx

# 获取证书
sudo certbot --nginx -d your-domain.com

# 自动续期
sudo crontab -e
# 添加: 0 12 * * * /usr/bin/certbot renew --quiet
```

### 手动配置 SSL

```bash
# 创建 SSL 目录
mkdir -p nginx/ssl

# 复制证书文件
cp your-cert.pem nginx/ssl/cert.pem
cp your-key.pem nginx/ssl/key.pem

# 重启 Nginx
docker-compose -f docker-compose.prod.yml restart nginx
```

---

## 环境变量说明

| 变量名 | 说明 | 默认值 | 必填 |
|--------|------|--------|------|
| `POSTGRES_USER` | 数据库用户名 | `omnilink` | 否 |
| `POSTGRES_PASSWORD` | 数据库密码 | - | **是** |
| `POSTGRES_DB` | 数据库名称 | `omnilink` | 否 |
| `REDIS_PASSWORD` | Redis 密码 | - | **是** |
| `JWT_SECRET` | JWT 签名密钥 | - | **是** |
| `MINIO_ROOT_USER` | MinIO 用户名 | `minioadmin` | 否 |
| `MINIO_ROOT_PASSWORD` | MinIO 密码 | - | **是** |
| `GRAFANA_USER` | Grafana 用户名 | `admin` | 否 |
| `GRAFANA_PASSWORD` | Grafana 密码 | `admin` | 否 |
| `RUST_LOG` | 日志级别 | `info` | 否 |
| `APP_ENV` | 运行环境 | `production` | 否 |

---

## 常见问题

### Q: 服务启动失败怎么办？

```bash
# 查看服务日志
docker-compose -f docker-compose.prod.yml logs [service-name]

# 检查资源使用
docker stats

# 重启服务
docker-compose -f docker-compose.prod.yml restart [service-name]
```

### Q: 如何更新服务？

```bash
# 拉取最新代码
git pull

# 重新构建并部署
docker-compose -f docker-compose.prod.yml up -d --build
```

### Q: 如何备份数据？

```bash
# 备份 PostgreSQL
docker exec omnilink-postgres pg_dump -U omnilink omnilink > backup_$(date +%Y%m%d).sql

# 备份 MinIO
docker cp omnilink-minio:/data ./minio_backup_$(date +%Y%m%d)
```
