# OmniLink 运维手册

## 目录

- [日常运维](#日常运维)
- [监控与告警](#监控与告警)
- [备份与恢复](#备份与恢复)
- [性能调优](#性能调优)
- [安全管理](#安全管理)

---

## 日常运维

### 服务管理

```bash
# 查看所有服务状态
docker-compose -f docker-compose.prod.yml ps

# 启动所有服务
docker-compose -f docker-compose.prod.yml up -d

# 停止所有服务
docker-compose -f docker-compose.prod.yml down

# 重启单个服务
docker-compose -f docker-compose.prod.yml restart [service-name]

# 查看服务日志
docker-compose -f docker-compose.prod.yml logs -f [service-name]

# 查看资源使用
docker stats
```

### 服务列表

| 服务名 | 容器名 | 端口 | 说明 |
|--------|--------|------|------|
| im-api | omnilink-im-api | 8002 | HTTP API 服务 |
| im-gateway | omnilink-im-gateway | 3002 | WebSocket 服务 |
| ai-service | omnilink-ai-service | 8003 | AI 服务 |
| user-service | omnilink-user-service | 8004 | 用户服务 |
| usage-service | omnilink-usage-service | 8006 | 用量统计服务 |
| file-service | omnilink-file-service | 8007 | 文件服务 |
| config-service | omnilink-config-service | 8008 | 配置服务 |
| push-service | omnilink-push-service | 3005 | 推送服务 |
| postgres | omnilink-postgres | 5432 | 数据库 |
| redis | omnilink-redis | 6379 | 缓存 |
| minio | omnilink-minio | 9000 | 对象存储 |
| prometheus | omnilink-prometheus | 9090 | 监控 |
| grafana | omnilink-grafana | 3000 | 仪表板 |
| alertmanager | omnilink-alertmanager | 9093 | 告警管理 |

### 健康检查

```bash
# 检查所有服务健康状态
for port in 8002 3002 8003 8004 8006 8007 8008 3005; do
  echo -n "Port $port: "
  curl -s -o /dev/null -w "%{http_code}" http://localhost:$port/health
  echo
done

# 检查数据库连接
docker exec omnilink-postgres pg_isready -U omnilink

# 检查 Redis 连接
docker exec omnilink-redis redis-cli -a $REDIS_PASSWORD ping

# 检查 MinIO 连接
curl -s http://localhost:9000/minio/health/live
```

---

## 监控与告警

### Grafana 访问

- **地址**: http://your-server:3000
- **默认账号**: admin / admin
- **首次登录**: 请立即修改密码

### Prometheus 访问

- **地址**: http://your-server:9090
- **状态页面**: http://your-server:9090/status
- **Targets**: http://your-server:9090/targets

### 告警规则

| 告警名称 | 严重程度 | 触发条件 | 说明 |
|---------|---------|---------|------|
| ServiceDown | critical | 服务离线超过1分钟 | 服务宕机 |
| HighErrorRate | warning | 5xx错误率超过10% | 接口异常 |
| HighMemoryUsage | warning | 内存使用超过85% | 资源告警 |
| CriticalMemoryUsage | critical | 内存使用超过95% | 紧急告警 |
| HighCPUUsage | warning | CPU使用超过80% | 资源告警 |
| DiskSpaceLow | warning | 磁盘空间低于15% | 存储告警 |
| PostgreSQLDown | critical | PostgreSQL离线 | 数据库故障 |
| RedisDown | critical | Redis离线 | 缓存故障 |
| HighResponseTime | warning | P95响应时间超过2秒 | 性能告警 |

### 自定义告警

编辑 `monitoring/prometheus/alerts.yml` 添加自定义告警规则：

```yaml
groups:
  - name: custom_alerts
    rules:
      - alert: CustomAlert
        expr: your_metric > threshold
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Custom alert description"
```

---

## 备份与恢复

### 自动备份脚本

```bash
#!/bin/bash
# backup.sh - 每日备份脚本

BACKUP_DIR="/opt/omnilink/backups"
DATE=$(date +%Y%m%d_%H%M%S)

mkdir -p $BACKUP_DIR

# 备份 PostgreSQL
docker exec omnilink-postgres pg_dump -U omnilink omnilink | gzip > $BACKUP_DIR/postgres_$DATE.sql.gz

# 备份 Redis
docker exec omnilink-redis redis-cli -a $REDIS_PASSWORD BGSAVE
docker cp omnilink-redis:/data/dump.rdb $BACKUP_DIR/redis_$DATE.rdb

# 备份 MinIO 数据
docker cp omnilink-minio:/data $BACKUP_DIR/minio_$DATE

# 清理30天前的备份
find $BACKUP_DIR -type f -mtime +30 -delete

echo "Backup completed: $DATE"
```

### 恢复 PostgreSQL

```bash
# 停止应用服务
docker-compose -f docker-compose.prod.yml stop im-api im-gateway ai-service user-service

# 恢复数据库
gunzip < backup/postgres_20260517_120000.sql.gz | docker exec -i omnilink-postgres psql -U omnilink omnilink

# 重启服务
docker-compose -f docker-compose.prod.yml start im-api im-gateway ai-service user-service
```

### 恢复 Redis

```bash
# 停止 Redis
docker-compose -f docker-compose.prod.yml stop redis

# 恢复数据文件
docker cp backup/redis_20260517_120000.rdb omnilink-redis:/data/dump.rdb

# 启动 Redis
docker-compose -f docker-compose.prod.yml start redis
```

---

## 性能调优

### 数据库优化

```sql
-- 查看慢查询
SELECT query, mean_time, calls
FROM pg_stat_statements
ORDER BY mean_time DESC
LIMIT 10;

-- 查看连接数
SELECT count(*) FROM pg_stat_activity;

-- 优化表
VACUUM ANALYZE;
```

### Redis 优化

```bash
# 查看内存使用
docker exec omnilink-redis redis-cli -a $REDIS_PASSWORD INFO memory

# 查看慢日志
docker exec omnilink-redis redis-cli -a $REDIS_PASSWORD SLOWLOG GET 10

# 清理过期数据
docker exec omnilink-redis redis-cli -a $REDIS_PASSWORD DBSIZE
```

### 应用服务优化

```bash
# 查看服务资源使用
docker stats --no-stream

# 调整服务资源限制（编辑 docker-compose.prod.yml）
deploy:
  resources:
    limits:
      cpus: '2.0'
      memory: 1G
    reservations:
      cpus: '1.0'
      memory: 512M
```

---

## 安全管理

### 定期安全检查

```bash
# 更新系统包
sudo apt update && sudo apt upgrade -y

# 更新 Docker 镜像
docker-compose -f docker-compose.prod.yml pull
docker-compose -f docker-compose.prod.yml up -d

# 检查容器漏洞
docker scout cves omnilink-im-api:latest
```

### 防火墙配置

```bash
# 只开放必要端口
sudo ufw allow 80/tcp    # HTTP
sudo ufw allow 443/tcp   # HTTPS
sudo ufw allow 22/tcp    # SSH

# 禁止直接访问内部服务端口
sudo ufw deny 5432/tcp   # PostgreSQL
sudo ufw deny 6379/tcp   # Redis
sudo ufw deny 9000/tcp   # MinIO
```

### 密钥管理

- 定期轮换 JWT_SECRET（每90天）
- 定期轮换数据库密码（每90天）
- 使用强密码（至少16字符，包含大小写字母、数字、特殊字符）
- 不要在代码中硬编码密钥
- 使用环境变量或密钥管理服务

### 日志审计

```bash
# 查看登录日志
docker-compose -f docker-compose.prod.yml logs im-api | grep "login"

# 查看错误日志
docker-compose -f docker-compose.prod.yml logs --tail=100 im-api | grep "ERROR"

# 导出日志
docker-compose -f docker-compose.prod.yml logs --since 24h > omnilink_logs_$(date +%Y%m%d).log
```

---

## 故障排查

### 常见问题

1. **服务无法启动**
   - 检查端口是否被占用: `netstat -tlnp | grep [port]`
   - 检查环境变量是否正确: `cat .env`
   - 查看服务日志: `docker-compose logs [service]`

2. **数据库连接失败**
   - 检查 PostgreSQL 是否运行: `docker ps | grep postgres`
   - 检查连接参数: `cat .env | grep POSTGRES`
   - 测试连接: `docker exec omnilink-postgres pg_isready`

3. **内存不足**
   - 查看内存使用: `free -h`
   - 查看容器内存: `docker stats`
   - 调整资源限制或增加服务器内存

4. **磁盘空间不足**
   - 查看磁盘使用: `df -h`
   - 清理 Docker: `docker system prune -a`
   - 清理旧日志: `journalctl --vacuum-time=7d`
