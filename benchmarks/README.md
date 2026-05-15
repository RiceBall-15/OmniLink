# OmniLink 性能基准测试套件

## 概述

本测试套件用于评估 OmniLink IM 系统的性能表现，包括消息吞吐量、WebSocket 并发能力和数据库查询性能。

## 测试分类

### 1. 消息吞吐量测试 (`message_throughput`)

测试消息发送和接收的性能：

- **单条消息发送延迟**: 测量单条消息从发送到响应的时间
- **消息大小影响**: 测试不同大小消息（10B-5KB）的性能差异
- **批量消息吞吐量**: 测试批量发送（10-200条）的处理能力
- **并发消息发送**: 测试多客户端同时发送消息的性能
- **历史消息查询**: 测试不同分页大小的消息查询性能

### 2. WebSocket 并发测试 (`websocket_concurrent`)

测试 WebSocket 连接和消息广播：

- **连接建立延迟**: 测量 WebSocket 握手时间
- **并发连接数**: 测试系统支持的最大并发连接数
- **消息发送延迟**: 测量通过 WebSocket 发送消息的延迟
- **并发消息发送**: 测试多连接同时发送消息的性能
- **消息接收延迟**: 测量消息从发送到接收的延迟

### 3. 数据库查询测试 (`database_queries`)

测试常见数据库操作的性能：

- **会话列表查询**: 测试不同分页大小的会话查询
- **消息历史查询**: 测试分页和偏移量对查询性能的影响
- **消息搜索**: 测试不同搜索关键词的查询性能
- **用户状态查询**: 测试单个和批量用户状态查询
- **已读状态查询**: 测试消息已读状态查询性能

## 使用方法

### 前置条件

1. OmniLink 服务正在运行
2. 已获取有效的认证令牌
3. Rust 工具链已安装

### 运行测试

```bash
# 设置环境变量
export OMNILINK_URL=http://localhost:8080
export AUTH_TOKEN=your-auth-token-here

# 运行所有测试
cd /root/omnilink/benchmarks
./scripts/run_benchmarks.sh

# 运行特定测试
./scripts/run_benchmarks.sh message_throughput
./scripts/run_benchmarks.sh websocket_concurrent
./scripts/run_benchmarks.sh database_queries
```

### 查看结果

测试结果保存在 `results/` 目录：

```
results/
├── message_throughput_20260516_0645.txt
├── websocket_concurrent_20260516_0645.txt
├── database_queries_20260516_0645.txt
├── performance_report_20260516_0645.md
└── message_throughput_html_20260516_0645/
    └── report/
        └── index.html
```

## 性能指标

### 预期性能基准

| 指标 | 目标值 | 说明 |
|------|--------|------|
| 单条消息发送延迟 | < 50ms | 95th percentile |
| 批量消息吞吐量 | > 100 msg/s | 100条消息批量发送 |
| WebSocket 连接延迟 | < 100ms | 握手完成时间 |
| 并发连接数 | > 500 | 同时在线连接 |
| 会话列表查询 | < 100ms | 20条分页 |
| 消息历史查询 | < 200ms | 100条消息 |
| 消息搜索 | < 500ms | 全文搜索 |

### 性能瓶颈识别

如果测试结果未达到预期，可能的原因：

1. **消息吞吐量低**
   - 数据库连接池配置不足
   - 消息队列处理延迟
   - 网络带宽限制

2. **WebSocket 连接数受限**
   - 系统 ulimit 限制
   - 内核参数未优化
   - 内存不足

3. **数据库查询慢**
   - 缺少必要的索引
   - 查询语句未优化
   - 连接池配置不当

## 优化建议

### 系统级优化

```bash
# 增加文件描述符限制
ulimit -n 65535

# 优化内核参数
sysctl -w net.core.somaxconn=65535
sysctl -w net.ipv4.tcp_max_syn_backlog=65535
sysctl -w net.core.netdev_max_backlog=65535
```

### 应用级优化

1. **数据库连接池**
   ```toml
   [database]
   max_connections = 50
   min_connections = 10
   ```

2. **消息队列配置**
   ```toml
   [message_queue]
   batch_size = 100
   flush_interval_ms = 50
   ```

3. **WebSocket 配置**
   ```toml
   [websocket]
   max_connections = 10000
   ping_interval_secs = 30
   ```

## 持续集成

建议将基准测试集成到 CI/CD 流程：

```yaml
# .github/workflows/benchmark.yml
name: Performance Benchmark
on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Run benchmarks
        run: |
          cd benchmarks
          ./scripts/run_benchmarks.sh
      - name: Upload results
        uses: actions/upload-artifact@v3
        with:
          name: benchmark-results
          path: benchmarks/results/
```

## 贡献指南

添加新的基准测试：

1. 在 `benches/` 目录创建新的测试文件
2. 在 `Cargo.toml` 添加 `[[bench]]` 配置
3. 更新 `run_benchmarks.sh` 脚本
4. 更新本文档的测试分类说明

## 许可证

MIT License - OmniLink Team
