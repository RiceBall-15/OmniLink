#!/bin/bash
# run_compression_benchmarks.sh
# 运行压缩性能基准测试并生成报告

set -e

echo "=== OmniLink 压缩性能基准测试 ==="
echo "开始时间: $(date)"
echo ""

# 检查资源
echo "=== 系统资源 ==="
free -h
echo ""

# 运行基准测试
echo "=== 运行压缩基准测试 ==="
cd /root/omnilink/benchmarks

# 使用单线程避免 OOM
export CARGO_BUILD_JOBS=1

# 运行测试
cargo bench --bench compression_performance 2>&1 | tee /tmp/compression_benchmark.log

echo ""
echo "=== 测试完成 ==="
echo "完成时间: $(date)"
echo ""
echo "结果已保存到 target/criterion/ 目录"
