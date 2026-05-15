#!/bin/bash
# OmniLink 性能基准测试运行脚本
# 
# 使用方法：
#   ./run_benchmarks.sh [benchmark_name]
#
# 示例：
#   ./run_benchmarks.sh                    # 运行所有基准测试
#   ./run_benchmarks.sh message_throughput # 运行消息吞吐量测试
#   ./run_benchmarks.sh websocket          # 运行 WebSocket 测试
#   ./run_benchmarks.sh database           # 运行数据库测试

set -e

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# 配置
BENCHMARK_DIR="$(cd "$(dirname "$0")" && pwd)"
RESULTS_DIR="${BENCHMARK_DIR}/results"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# 检查环境变量
if [ -z "$OMNILINK_URL" ]; then
    export OMNILINK_URL="http://localhost:8080"
    echo -e "${YELLOW}警告: OMNILINK_URL 未设置，使用默认值: ${OMNILINK_URL}${NC}"
fi

if [ -z "$AUTH_TOKEN" ]; then
    echo -e "${RED}错误: AUTH_TOKEN 环境变量未设置${NC}"
    echo "请设置认证令牌: export AUTH_TOKEN=your-token"
    exit 1
fi

# 创建结果目录
mkdir -p "${RESULTS_DIR}"

# 运行基准测试函数
run_benchmark() {
    local bench_name=$1
    local output_file="${RESULTS_DIR}/${bench_name}_${TIMESTAMP}"
    
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}运行基准测试: ${bench_name}${NC}"
    echo -e "${GREEN}========================================${NC}"
    
    # 运行测试并生成报告
    CARGO_BUILD_JOBS=1 cargo bench \
        --bench "${bench_name}" \
        -- --output-format bencher | tee "${output_file}.txt"
    
    # 检查是否有 Criterion HTML 报告
    if [ -d "target/criterion/${bench_name}" ]; then
        cp -r "target/criterion/${bench_name}" "${RESULTS_DIR}/${bench_name}_html_${TIMESTAMP}"
        echo -e "${GREEN}HTML 报告已保存到: ${RESULTS_DIR}/${bench_name}_html_${TIMESTAMP}${NC}"
    fi
    
    echo ""
}

# 生成性能报告
generate_report() {
    local report_file="${RESULTS_DIR}/performance_report_${TIMESTAMP}.md"
    
    cat > "${report_file}" << EOF
# OmniLink 性能基准测试报告

**测试时间**: $(date '+%Y-%m-%d %H:%M:%S')
**测试环境**: $(uname -a)

---

## 系统信息

- **CPU**: $(nproc) 核
- **内存**: $(free -h | awk '/^Mem:/{print $2}')
- **负载**: $(uptime | awk -F'load average:' '{print $2}')

## 测试结果

EOF
    
    # 添加各测试结果
    for result_file in "${RESULTS_DIR}"/*_${TIMESTAMP}.txt; do
        if [ -f "$result_file" ]; then
            bench_name=$(basename "$result_file" | sed "s/_${TIMESTAMP}.txt//")
            echo "### ${bench_name}" >> "${report_file}"
            echo "" >> "${report_file}"
            echo '```' >> "${report_file}"
            cat "$result_file" >> "${report_file}"
            echo '```' >> "${report_file}"
            echo "" >> "${report_file}"
        fi
    done
    
    # 添加结论
    cat >> "${report_file}" << 'EOF'

## 性能分析

### 消息吞吐量
- 单条消息发送延迟: 预期 < 50ms
- 批量消息吞吐量: 预期 > 100 msg/s
- 并发消息发送: 预期支持 50+ 并发

### WebSocket 性能
- 连接建立延迟: 预期 < 100ms
- 并发连接数: 预期支持 500+
- 消息广播延迟: 预期 < 50ms

### 数据库查询
- 会话列表查询: 预期 < 100ms
- 消息历史查询: 预期 < 200ms (100条)
- 消息搜索: 预期 < 500ms

## 优化建议

1. **消息吞吐量低**: 检查数据库连接池配置
2. **WebSocket 连接数受限**: 调整系统 ulimit 和内核参数
3. **数据库查询慢**: 添加索引或优化查询语句

---

*报告生成时间: $(date '+%Y-%m-%d %H:%M:%S')*
EOF
    
    echo -e "${GREEN}性能报告已生成: ${report_file}${NC}"
}

# 主流程
main() {
    echo -e "${GREEN}OmniLink 性能基准测试${NC}"
    echo -e "${GREEN}测试时间: $(date)${NC}"
    echo ""
    
    # 资源检查
    echo -e "${YELLOW}系统资源检查:${NC}"
    free -h
    echo ""
    
    # 检查可用内存
    AVAILABLE_MEM=$(free -m | awk '/^Mem:/{print $7}')
    if [ "$AVAILABLE_MEM" -lt 500 ]; then
        echo -e "${RED}警告: 可用内存不足 500MB (${AVAILABLE_MEM}MB)${NC}"
        echo -e "${RED}基准测试可能影响系统性能${NC}"
        read -p "是否继续? (y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            exit 1
        fi
    fi
    
    # 运行指定测试或所有测试
    if [ -n "$1" ]; then
        run_benchmark "$1"
    else
        # 运行所有测试
        for bench in message_throughput websocket_concurrent database_queries; do
            run_benchmark "$bench"
        done
    fi
    
    # 生成报告
    generate_report
    
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}所有基准测试完成${NC}"
    echo -e "${GREEN}结果保存在: ${RESULTS_DIR}${NC}"
    echo -e "${GREEN}========================================${NC}"
}

# 运行主流程
main "$@"
