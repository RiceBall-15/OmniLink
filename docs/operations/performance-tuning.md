# OmniLink 性能调优指南

## 目录

- [系统级调优](#系统级调优)
- [数据库调优](#数据库调优)
- [应用服务调优](#应用服务调优)
- [网络调优](#网络调优)
- [监控指标](#监控指标)

---

## 系统级调优

### 内核参数优化

```bash
# 编辑 /etc/sysctl.conf
cat >> /etc/sysctl.conf << 'EOF'

# OmniLink 性能优化参数

# 网络参数
net.core.somaxconn = 65535
net.ipv4.tcp_max_syn_backlog = 65535
net.ipv4.tcp_fin_timeout = 30
net.ipv4.tcp_tw_reuse = 1
net.ipv4.tcp_keepalive_time = 300
net.ipv4.tcp_keepalive_intvl = 30
net.ipv4.tcp_keepalive_probes = 5

# 文件描述符
fs.file-max = 2097152
fs.nr_open = 2097152

# 虚拟内存
vm.swappiness = 10
vm.overcommit_memory = 1
vm.max_map_count = 262144
EOF

# 应用配置
sudo sysctl -p
```

### 文件描述符限制

```bash
# 编辑 /etc/security/limits.conf
cat >> /etc/security/limits.conf << 'EOF'

# OmniLink 文件描述符限制
* soft nofile 65535
* hard nofile 65535
* soft nproc 65535
* hard nproc 65535
EOF

# 重新登录生效
```

### Docker 配置优化

```bash
# 编辑 /etc/docker/daemon.json
cat > /etc/docker/daemon.json << 'EOF'
{
  "log-driver": "json-file",
  "log-opts": {
    "max-size": "10m",
    "max-file": "3"
  },
  "storage-driver": "overlay2",
  "default-ulimits": {
    "nofile": {
      "Name": "nofile",
      "Hard": 65535,
      "Soft": 65535
    }
  }
}
EOF

# 重启 Docker
sudo systemctl restart docker
```

---

## 数据库调优

### PostgreSQL 配置优化

```bash
# 编辑 postgresql.conf (通过环境变量或挂载配置文件)
# 在 docker-compose.prod.yml 中添加命令参数

command:
  - postgres
  - -c
  - shared_buffers=256MB
  - -c
  - effective_cache_size=768MB
  - -c
  - maintenance_work_mem=64MB
  - -c
  - checkpoint_completion_target=0.9
  - -c
  - wal_buffers=16MB
  - -c
  - default_statistics_target=100
  - -c
  - random_page_cost=1.1
  - -c
  - effective_io_concurrency=200
  - -c
  - work_mem=4MB
  - -c
  - huge_pages=try
  - -c
  - min_wal_size=1GB
  - -c
  - max_wal_size=4GB
  - -c
  - max_worker_processes=4
  - -c
  - max_parallel_workers_per_gather=2
  - -c
  - max_parallel_workers=4
  - -c
  - max_parallel_maintenance_workers=2
```

### 索引优化

```sql
-- 查看未使用的索引
SELECT relname AS table_name,
       indexrelname AS index_name,
       idx_scan AS times_used,
       pg_size_pretty(pg_relation_size(indexrelid)) AS index_size
FROM pg_stat_user_indexes
WHERE idx_scan = 0
ORDER BY pg_relation_size(indexrelid) DESC;

-- 查看表膨胀
SELECT relname AS table_name,
       n_dead_tup AS dead_tuples,
       n_live_tup AS live_tuples,
       ROUND(n_dead_tup * 100.0 / NULLIF(n_live_tup + n_dead_tup, 0), 2) AS dead_ratio
FROM pg_stat_user_tables
WHERE n_dead_tup > 1000
ORDER BY n_dead_tup DESC;

-- 定期维护
VACUUM (VERBOSE, ANALYZE);
```

### 连接池配置

```bash
# 使用 PgBouncer 连接池
# 在 docker-compose.prod.yml 中添加 pgbouncer 服务

pgbouncer:
  image: edoburu/pgbouncer:latest
  container_name: omnilink-pgbouncer
  environment:
    DATABASE_URL: postgres://omnilink:${POSTGRES_PASSWORD}@postgres:5432/omnilink
    POOL_MODE: transaction
    MAX_CLIENT_CONN: 1000
    DEFAULT_POOL_SIZE: 25
    MIN_POOL_SIZE: 5
    RESERVE_POOL_SIZE: 5
  ports:
    - "6432:5432"
  depends_on:
    postgres:
      condition: service_healthy
```

---

## 应用服务调优

### Rust 服务优化

```toml
# Cargo.toml - 编译优化
[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
panic = "abort"
strip = true
```

### 资源限制配置

```yaml
# docker-compose.prod.yml
deploy:
  resources:
    limits:
      cpus: '2.0'
      memory: 1G
    reservations:
      cpus: '0.5'
      memory: 256M
  restart_policy:
    condition: on-failure
    delay: 5s
    max_attempts: 3
    window: 120s
```

### 连接池配置

```rust
// 使用 SQLx 连接池
let pool = PgPoolOptions::new()
    .max_connections(25)
    .min_connections(5)
    .acquire_timeout(Duration::from_secs(30))
    .idle_timeout(Duration::from_secs(600))
    .max_lifetime(Duration::from_secs(1800))
    .connect(&database_url)
    .await?;
```

---

## 网络调优

### Nginx 优化

```nginx
# nginx/nginx.conf 优化配置

worker_processes auto;
worker_rlimit_nofile 65535;

events {
    worker_connections 65535;
    multi_accept on;
    use epoll;
}

http {
    # 启用 gzip 压缩
    gzip on;
    gzip_vary on;
    gzip_proxied any;
    gzip_comp_level 6;
    gzip_types text/plain text/css application/json application/javascript text/xml application/xml;

    # 启用缓存
    proxy_cache_path /var/cache/nginx levels=1:2 keys_zone=app_cache:10m max_size=1g inactive=60m;

    # 优化代理
    proxy_buffering on;
    proxy_buffer_size 4k;
    proxy_buffers 8 4k;
    proxy_busy_buffers_size 8k;

    # 优化静态文件
    location ~* \.(jpg|jpeg|png|gif|ico|css|js)$ {
        expires 1y;
        add_header Cache-Control "public, immutable";
    }
}
```

### WebSocket 优化

```nginx
# WebSocket 配置
location /ws {
    proxy_pass http://im-gateway;
    proxy_http_version 1.1;
    proxy_set_header Upgrade $http_upgrade;
    proxy_set_header Connection "upgrade";
    proxy_read_timeout 86400s;
    proxy_send_timeout 86400s;
}
```

---

## 监控指标

### 关键性能指标 (KPI)

| 指标 | 目标值 | 告警阈值 | 说明 |
|------|--------|---------|------|
| API 响应时间 (P95) | < 200ms | > 500ms | 接口响应速度 |
| 错误率 | < 0.1% | > 1% | 接口稳定性 |
| 并发连接数 | > 1000 | < 100 | 系统吞吐量 |
| CPU 使用率 | < 70% | > 85% | 计算资源 |
| 内存使用率 | < 75% | > 90% | 内存资源 |
| 数据库连接数 | < 100 | > 150 | 数据库负载 |
| 缓存命中率 | > 90% | < 80% | 缓存效率 |

### Prometheus 查询示例

```promql
# API 响应时间 P95
histogram_quantile(0.95, sum by(le) (rate(http_request_duration_seconds_bucket[5m])))

# 错误率
sum(rate(http_requests_total{status=~"5.."}[5m])) / sum(rate(http_requests_total[5m]))

# 并发连接数
sum(websocket_connections_total)

# CPU 使用率
100 - (avg by(instance) (irate(node_cpu_seconds_total{mode="idle"}[5m])) * 100)

# 内存使用率
(1 - node_memory_MemAvailable_bytes / node_memory_MemTotal_bytes) * 100
```

### Grafana 仪表板

访问 Grafana (http://your-server:3000) 查看预置的 OmniLink Overview 仪表板，包含：

- 服务状态总览
- HTTP 请求速率和错误率
- 响应时间分布
- WebSocket 连接数
- 系统资源使用 (CPU、内存、磁盘)
- 数据库连接和查询性能
- Redis 内存和命中率

---

## 压力测试

### 使用 wrk 进行压测

```bash
# 安装 wrk
sudo apt install wrk

# 测试 API 接口
wrk -t4 -c100 -d30s http://localhost:8002/api/v1/health

# 测试 WebSocket 连接
# 需要使用专门的 WebSocket 压测工具
```

### 使用 locust 进行压测

```python
# locustfile.py
from locust import HttpUser, task, between

class OmniLinkUser(HttpUser):
    wait_time = between(1, 3)

    @task
    def health_check(self):
        self.client.get("/health")

    @task(3)
    def get_messages(self):
        self.client.get("/api/v1/messages")
```

```bash
# 运行压测
locust -f locustfile.py --host=http://localhost:8002
```

---

## 性能优化清单

- [ ] 系统内核参数优化
- [ ] 文件描述符限制调整
- [ ] PostgreSQL 配置优化
- [ ] 索引优化和维护
- [ ] 连接池配置
- [ ] Nginx 优化
- [ ] 启用 gzip 压缩
- [ ] 静态资源缓存
- [ ] WebSocket 优化
- [ ] 监控告警配置
- [ ] 定期性能测试
- [ ] 容量规划
