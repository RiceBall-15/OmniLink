#!/bin/bash
# run_coverage.sh - 运行代码覆盖率测试
#
# 使用方法：
#   ./run_coverage.sh [--open]
#
# 注意：服务器资源有限（2核2G），建议在本地或 CI 环境运行

set -e

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# 检查 tarpaulin 是否安装
if ! command -v cargo-tarpaulin &> /dev/null; then
    echo -e "${RED}错误: cargo-tarpaulin 未安装${NC}"
    echo "请先安装: cargo install cargo-tarpaulin"
    echo ""
    echo "注意：在 2核2G 服务器上安装可能需要较长时间"
    echo "建议在本地或 CI 环境运行覆盖率测试"
    exit 1
fi

# 资源检查
echo -e "${YELLOW}系统资源检查:${NC}"
free -h
echo ""

AVAILABLE_MEM=$(free -m | awk '/^Mem:/{print $7}')
if [ "$AVAILABLE_MEM" -lt 500 ]; then
    echo -e "${RED}警告: 可用内存不足 500MB (${AVAILABLE_MEM}MB)${NC}"
    echo -e "${RED}覆盖率测试可能需要较多内存${NC}"
    exit 1
fi

# 设置环境变量
export CARGO_BUILD_JOBS=1

# 创建输出目录
mkdir -p target/coverage

echo -e "${GREEN}开始运行代码覆盖率测试...${NC}"
echo -e "${GREEN}开始时间: $(date)${NC}"
echo ""

# 运行 tarpaulin
cargo tarpaulin \
    --config tarpaulin.toml \
    --skip-clean \
    --timeout 300 \
    --out Html \
    --out Xml \
    --out Lcov \
    --output-dir target/coverage \
    2>&1 | tee target/coverage/tarpaulin.log

echo ""
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}代码覆盖率测试完成${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo "报告位置："
echo "  - HTML: target/coverage/tarpaulin-report.html"
echo "  - XML:  target/coverage/cobertura.xml"
echo "  - LCOV: target/coverage/lcov.info"
echo ""

# 如果指定了 --open 参数，尝试打开报告
if [ "$1" = "--open" ]; then
    if command -v xdg-open &> /dev/null; then
        xdg-open target/coverage/tarpaulin-report.html
    elif command -v open &> /dev/null; then
        open target/coverage/tarpaulin-report.html
    else
        echo -e "${YELLOW}无法自动打开报告，请手动打开: target/coverage/tarpaulin-report.html${NC}"
    fi
fi
