# OmniLink 故障排查指南

## 目录

- [快速诊断](#快速诊断)
- [服务故障](#服务故障)
- [数据库故障](#数据库故障)
- [网络故障](#网络故障)
- [性能问题](#性能问题)
- [日志分析](#日志分析)

---

## 快速诊断

### 一键诊断脚本

```bash
#!/bin/bash
# diagnose.sh - 快速诊断脚本

echo "=== OmniLink 诊断报告 ==="
echo ""

echo "1. 系统资源"
echo "---"
free -h
echo ""
df -h /
echo ""

echo "2. Docker 状态"
echo "---"
docker ps -a --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"
echo ""

echo "3. 服务健康检查"
echo "---"
for port in 8002 3002 8003 8004 8006 8007 8008 3005; do
  status=$(curl -s -o /dev/null -w "%{http_code}" http://localhost:$port/health 2>/dev/null)
  if [ "$status" = "200" ]; then
    echo "Port $port: OK"
  else
    echo "Port $port: FAIL ($status)"
  fi
done
echo ""

echo "4. 数据库连接"
echo "---"
docker exec omnilink-postgres pg_isready -U omnilink 2>/dev/null && echo "PostgreSQL: OK" || echo "PostgreSQL: FAIL"
echo ""

echo "5. Redis 连接"
echo "---"
docker exec omnilink-redis redis-cli ping 2>/dev/null && echo "Redis: OK" || echo "Redis: FAIL"
echo ""

echo "6. 最近错误日志"
echo "---"
docker-compose -f /root/omnilink/docker-compose.prod.yml logs --tail=5 im-api 2>/dev/null | grep -i "error" || echo "无错误"
```

---

## 服务故障

### 服务无法启动

**症状**: 容器状态为 `Exited` 或 `Restarting`

**排查步骤**:

```bash
# 1. 查看容器日志
docker-compose -f docker-compose.prod.yml logs [service-name]

# 2. 检查端口冲突
netstat -tlnp | grep [port]

# 3. 检查环境变量
cat .env | grep [VARIABLE]

# 4. 检查资源限制
docker stats --no-stream

# 5. 检查依赖服务
docker-compose -f docker-compose.prod.yml ps
```

**常见原因**:
- 端口被其他进程占用
- 环境变量配置错误
- 依赖服务未启动
- 内存不足

**解决方案**:

```bash
# 杀死占用端口的进程
sudo kill -9 $(sudo lsof -t -i:[port])

# 重启服务
docker-compose -f docker-compose.prod.yml restart [service-name]

# 重建并重启
docker-compose -f docker-compose.prod.yml up -d --build [service-name]
```

### 服务频繁重启

**症状**: 容器状态显示 `Restarting`，重启次数增加

**排查步骤**:

```bash
# 查看重启次数
docker inspect [container-name] --format '{{.RestartCount}}'

# 查看最后退出原因
docker inspect [container-name] --format '{{.State.ExitCode}}'

# 查看 OOM 统计
docker inspect [container-name] --format '{{.State.OOMKilled}}'
```

**常见原因**:
- 内存溢出 (OOM)
- 健康检查失败
- 应用崩溃

**解决方案**:

```bash
# 增加内存限制
# 编辑 docker-compose.prod.yml
deploy:
  resources:
    limits:
      memory: 1G

# 调整健康检查间隔
healthcheck:
  interval: 60s
  timeout: 10s
  retries: 5
```

---

## 数据库故障

### PostgreSQL 连接失败

**症状**: 服务无法连接数据库，日志显示连接超时

**排查步骤**:

```bash
# 1. 检查 PostgreSQL 容器状态
docker ps | grep postgres

# 2. 检查 PostgreSQL 日志
docker-compose -f docker-compose.prod.yml logs postgres

# 3. 测试数据库连接
docker exec omnilink-postgres psql -U omnilink -d omnilink -c "SELECT 1;"

# 4. 检查连接数
docker exec omnilink-postgres psql -U omnilink -c "SELECT count(*) FROM pg_stat_activity;"

# 5. 检查数据库大小
docker exec omnilink-postgres psql -U omnilink -c "SELECT pg_size_pretty(pg_database_size('omnilink'));"
```

**常见原因**:
- PostgreSQL 容器未启动
- 连接数达到上限
- 认证失败
- 磁盘空间不足

**解决方案**:

```bash
# 重启 PostgreSQL
docker-compose -f docker-compose.prod.yml restart postgres

# 增加最大连接数
# 编辑 postgresql.conf
max_connections = 200

# 清理空闲连接
docker exec omnilink-postgres psql -U omnilink -c "
SELECT pg_terminate_backend(pid)
FROM pg_stat_activity
WHERE state = 'idle'
AND query_start < now() - interval '1 hour';"
```

### 数据库性能问题

**症状**: 查询响应慢，CPU 使用率高

**排查步骤**:

```bash
# 查看慢查询
docker exec omnilink-postgres psql -U omnilink -c "
SELECT query, mean_time, calls
FROM pg_stat_statements
ORDER BY mean_time DESC
LIMIT 10;"

# 查看表膨胀
docker exec omnilink-postgres psql -U omnilink -c "
SELECT relname, n_dead_tup, n_live_tup
FROM pg_stat_user_tables
WHERE n_dead_tup > 1000
ORDER BY n_dead_tup DESC;"

# 查看索引使用情况
docker exec omnilink-postgres psql -U omnilink -c "
SELECT relname, indexrelname, idx_scan
FROM pg_stat_user_indexes
ORDER BY idx_scan ASC
LIMIT 10;"
```

**解决方案**:

```bash
# 优化表
docker exec omnilink-postgres psql -U omnilink -c "VACUUM ANALYZE;"

# 重建索引
docker exec omnilink-postgres psql -U omnilink -c "REINDEX DATABASE omnilink;"
```

---

## 网络故障

### 服务间通信失败

**症状**: 服务无法访问其他服务，日志显示连接拒绝

**排查步骤**:

```bash
# 1. 检查 Docker 网络
docker network ls
docker network inspect omnilink-network

# 2. 检查容器网络配置
docker inspect [container-name] --format '{{json .NetworkSettings.Networks}}'

# 3. 测试服务间连通性
docker exec omnilink-im-api ping omnilink-postgres

# 4. 检查 DNS 解析
docker exec omnilink-im-api nslookup omnilink-postgres
```

**常见原因**:
- 容器不在同一网络
- DNS 解析失败
- 防火墙规则阻止

**解决方案**:

```bash
# 确保所有容器在同一网络
docker network connect omnilink-network [container-name]

# 重启网络
docker-compose -f docker-compose.prod.yml down
docker-compose -f docker-compose.prod.yml up -d
```

### 外部访问失败

**症状**: 无法通过域名或公网 IP 访问服务

**排查步骤**:

```bash
# 1. 检查 Nginx 状态
docker ps | grep nginx

# 2. 检查 Nginx 配置
docker exec omnilink-nginx nginx -t

# 3. 检查端口监听
netstat -tlnp | grep -E "80|443"

# 4. 检查防火墙
sudo ufw status
sudo iptables -L -n
```

**解决方案**:

```bash
# 重启 Nginx
docker-compose -f docker-compose.prod.yml restart nginx

# 开放端口
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp

# 检查 SSL 证书
docker exec omnilink-nginx ls -la /etc/nginx/ssl/
```

---

## 性能问题

### 响应时间过长

**症状**: API 响应慢，用户反馈卡顿

**排查步骤**:

```bash
# 1. 查看系统负载
uptime
top -bn1 | head -20

# 2. 查看容器资源使用
docker stats --no-stream

# 3. 查看数据库连接数
docker exec omnilink-postgres psql -U omnilink -c "SELECT count(*) FROM pg_stat_activity;"

# 4. 查看 Redis 内存
docker exec omnilink-redis redis-cli info memory
```

**解决方案**:

```bash
# 增加服务实例数（编辑 docker-compose.prod.yml）
deploy:
  replicas: 3

# 增加资源限制
deploy:
  resources:
    limits:
      cpus: '2.0'
      memory: 1G

# 优化数据库查询（添加索引）
docker exec omnilink-postgres psql -U omnilink -c "
CREATE INDEX idx_messages_created_at ON messages(created_at);"
```

### 内存不足

**症状**: 服务被 OOM Killer 终止

**排查步骤**:

```bash
# 1. 查看系统内存
free -h

# 2. 查看容器内存使用
docker stats --no-stream

# 3. 查看 OOM 日志
dmesg | grep -i "oom"
journalctl -k | grep -i "oom"
```

**解决方案**:

```bash
# 增加服务器内存

# 调整服务内存限制
deploy:
  resources:
    limits:
      memory: 512M

# 添加 swap 空间
sudo fallocate -l 2G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile
echo '/swapfile none swap sw 0 0' | sudo tee -a /etc/fstab
```

---

## 日志分析

### 日志位置

- **应用日志**: `docker-compose logs [service-name]`
- **Nginx 日志**: `/var/log/nginx/`
- **系统日志**: `/var/log/syslog`
- **Docker 日志**: `/var/lib/docker/containers/[container-id]/[container-id]-json.log`

### 常用日志命令

```bash
# 查看实时日志
docker-compose -f docker-compose.prod.yml logs -f [service-name]

# 查看最近 100 行日志
docker-compose -f docker-compose.prod.yml logs --tail=100 [service-name]

# 查看最近 1 小时的日志
docker-compose -f docker-compose.prod.yml logs --since 1h [service-name]

# 搜索错误日志
docker-compose -f docker-compose.prod.yml logs [service-name] | grep -i "error"

# 统计错误数量
docker-compose -f docker-compose.prod.yml logs [service-name] | grep -c "ERROR"

# 导出日志到文件
docker-compose -f docker-compose.prod.yml logs --since 24h > omnilink_logs_$(date +%Y%m%d).log
```

### 日志级别

- **ERROR**: 错误，需要立即处理
- **WARN**: 警告，需要关注
- **INFO**: 信息，正常运行日志
- **DEBUG**: 调试，开发环境使用

**调整日志级别**:

```bash
# 编辑 .env
RUST_LOG=debug  # 开发环境
RUST_LOG=info   # 生产环境
RUST_LOG=warn   # 减少日志量
```

---

## 紧急联系

如果以上方法都无法解决问题：

1. 收集诊断信息
   ```bash
   # 运行诊断脚本
   ./diagnose.sh > diagnosis_$(date +%Y%m%d_%H%M%S).txt
   ```

2. 联系开发团队
   - 提供诊断报告
   - 描述问题现象
   - 提供复现步骤
