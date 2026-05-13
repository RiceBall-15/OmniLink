#!/bin/bash
# CI 测试脚本 - 运行集成测试
set -e

echo "=== OmniLink 集成测试 ==="
echo "时间: $(date)"
echo ""

# 颜色定义
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 检查服务是否运行
check_service() {
    local port=$1
    local name=$2
    if netstat -tlnp 2>/dev/null | grep -q ":$port " || ss -tlnp 2>/dev/null | grep -q ":$port "; then
        echo -e "${GREEN}✓${NC} $name 正在运行 (端口: $port)"
        return 0
    else
        echo -e "${YELLOW}⚠${NC} $name 未运行 (端口: $port)"
        return 1
    fi
}

# 检查必要服务
echo "检查服务状态..."
check_service 5432 "PostgreSQL"
check_service 6379 "Redis"
check_service 8080 "API Server"

echo ""
echo "运行集成测试..."
echo "================================"

# 运行集成测试
cd "$(dirname "$0")"
cargo test --package integration-tests 2>&1 | tee test_output.txt

TEST_EXIT=$?

echo ""
echo "================================"
if [ $TEST_EXIT -eq 0 ]; then
    echo -e "${GREEN}✓ 所有测试通过${NC}"
else
    echo -e "${RED}✗ 测试失败${NC}"
    echo "查看 test_output.txt 获取详细信息"
fi

exit $TEST_EXIT
