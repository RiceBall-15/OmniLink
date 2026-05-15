# OmniLink 监控与告警配置指南

## 架构概览

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│  OmniLink    │────▶│  Prometheus  │────▶│   Grafana    │
│  /metrics    │     │  (采集+存储)  │     │  (可视化)    │
└──────────────┘     └──────┬───────┘     └──────────────┘
                            │
                     ┌──────┴───────┐
                     │  AlertManager │
                     │  (告警通知)    │
                     └──────────────┘
```

## 快速部署

### 1. 使用 Docker Compose（推荐）

在 `docker-compose.yml` 中添加以下服务：

```yaml
  # Prometheus
  prometheus:
    image: prom/prometheus:latest
    container_name: omnilink-prometheus
    ports:
      - "9090:9090"
    volumes:
      - ./monitoring/prometheus/prometheus.yml:/etc/prometheus/prometheus.yml:ro
      - ./monitoring/prometheus/rules:/etc/prometheus/rules:ro
      - prometheus_data:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.retention.time=30d'
    networks:
      - omnilink-net

  # Grafana
  grafana:
    image: grafana/grafana:latest
    container_name: omnilink-grafana
    ports:
      - "3000:3000"
    environment:
      GF_SECURITY_ADMIN_PASSWORD: ${GRAFANA_PASSWORD:-admin}
    volumes:
      - grafana_data:/var/lib/grafana
      - ./monitoring/grafana/dashboards:/etc/grafana/provisioning/dashboards:ro
    networks:
      - omnilink-net

  # Node Exporter（系统监控）
  node-exporter:
    image: prom/node-exporter:latest
    container_name: omnilink-node-exporter
    ports:
      - "9100:9100"
    volumes:
      - /proc:/host/proc:ro
      - /sys:/host/sys:ro
    command:
      - '--path.procfs=/host/proc'
      - '--path.sysfs=/host/sys'
    networks:
      - omnilink-net
```

添加 volume：

```yaml
volumes:
  prometheus_data:
  grafana_data:
```

### 2. 启动监控栈

```bash
docker-compose up -d prometheus grafana node-exporter
```

### 3. 访问

- **Prometheus**: http://localhost:9090
- **Grafana**: http://localhost:3000（默认 admin/admin）
- **Node Exporter**: http://localhost:9100

### 4. 导入 Grafana Dashboard

1. 登录 Grafana
2. 左侧菜单 → Dashboards → Import
3. 上传 `monitoring/grafana/dashboards/omnilink.json`
4. 选择 Prometheus 数据源
5. 点击 Import

---

## 暴露的指标

OmniLink API 在 `/metrics` 端点暴露以下 Prometheus 指标：

| 指标名 | 类型 | 说明 |
|--------|------|------|
| `omnilink_uptime_seconds` | gauge | 服务运行时间（秒） |
| `omnilink_requests_total` | counter | HTTP 请求总数 |
| `omnilink_errors_total` | counter | 错误总数 |
| `omnilink_messages_sent_total` | counter | 发送消息总数 |
| `omnilink_messages_received_total` | counter | 接收消息总数 |
| `omnilink_conversations_created_total` | counter | 创建会话总数 |
| `omnilink_users_registered_total` | counter | 注册用户总数 |
| `omnilink_ws_connections` | gauge | 当前 WebSocket 连接数 |
| `omnilink_auth_failures_total` | counter | 认证失败总数 |
| `omnilink_db_pool_size` | gauge | 数据库连接池大小 |
| `omnilink_db_idle_connections` | gauge | 数据库空闲连接数 |

---

## 告警规则

| 告警名称 | 条件 | 严重程度 | 说明 |
|----------|------|----------|------|
| ServiceDown | up == 0, 1min | 🔴 critical | 服务不可用 |
| HighErrorRate | >5%, 5min | 🟡 warning | 错误率过高 |
| HighWSConnections | >5000, 5min | 🟡 warning | WS连接数过高 |
| DBPoolExhausted | idle==0, 2min | 🔴 critical | 连接池耗尽 |
| AuthFailureSpike | >10/s, 5min | 🟡 warning | 认证失败激增 |
| FrequentRestarts | >3/hour | 🟡 warning | 频繁重启 |
| LowMessageThroughput | ~0, 30min | 🔵 info | 消息吞吐量低 |

---

## 日志聚合（可选）

### 使用 Loki + Promtail

```yaml
  # Loki
  loki:
    image: grafana/loki:latest
    container_name: omnilink-loki
    ports:
      - "3100:3100"
    networks:
      - omnilink-net

  # Promtail
  promtail:
    image: grafana/promtail:latest
    container_name: omnilink-promtail
    volumes:
      - /var/log:/var/log:ro
      - ./monitoring/promtail/config.yml:/etc/promtail/config.yml:ro
    networks:
      - omnilink-net
```

在 Grafana 中添加 Loki 数据源即可查询日志。

---

## 性能基准

基于当前服务器（2核2G）的预期指标：

| 指标 | 预期值 | 告警阈值 |
|------|--------|----------|
| 请求延迟 P50 | <50ms | >200ms |
| 请求延迟 P99 | <500ms | >2000ms |
| WebSocket 连接 | <100 | >500 |
| 数据库连接池 | 5-10 | >20 |
| 内存使用 | <80% | >90% |
| CPU 使用 | <50% | >80% |
