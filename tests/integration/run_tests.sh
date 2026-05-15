#!/bin/bash
# OmniLink 集成测试运行脚本
#
# 使用方法:
#   ./run_tests.sh                    # 运行所有测试
#   ./run_tests.sh api               # 运行 API 测试
#   ./run_tests.sh websocket         # 运行 WebSocket 测试
#   ./run_tests.sh user              # 运行用户相关测试

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

# 颜色
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# 检查环境
check_env() {
    if [ -z "$OMNILINK_URL" ]; then
        export OMNILINK_URL="http://localhost:8080"
        echo -e "${YELLOW}警告: OMNILINK_URL 未设置，使用默认: ${OMNILINK_URL}${NC}"
    fi
    
    if [ -z "$AUTH_TOKEN" ]; then
        echo -e "${RED}错误: AUTH_TOKEN 未设置${NC}"
        echo "请先设置: export AUTH_TOKEN=your-token"
        exit 1
    fi
}

# 检查服务是否运行
check_service() {
    echo -e "${YELLOW}检查服务状态...${NC}"
    if curl -s --connect-timeout 5 "${OMNILINK_URL}/health" > /dev/null 2>&1; then
        echo -e "${GREEN}服务运行中${NC}"
    else
        echo -e "${YELLOW}警告: 服务可能未运行，测试可能失败${NC}"
    fi
}

# 运行测试
run_tests() {
    local test_filter=$1
    
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}OmniLink 集成测试${NC}"
    echo -e "${GREEN}========================================${NC}"
    
    export CARGO_BUILD_JOBS=1
    
    case "$test_filter" in
        api)
            echo -e "${GREEN}运行 API 端点测试...${NC}"
            cargo test --test api_endpoints -- --nocapture
            ;;
        websocket|ws)
            echo -e "${GREEN}运行 WebSocket 测试...${NC}"
            cargo test --test websocket_tests -- --nocapture
            ;;
        user)
            echo -e "${GREEN}运行用户测试...${NC}"
            cargo test --test user_tests -- --nocapture
            ;;
        message|msg)
            echo -e "${GREEN}运行消息测试...${NC}"
            cargo test --test message_tests -- --nocapture
            ;;
        conversation|conv)
            echo -e "${GREEN}运行会话测试...${NC}"
            cargo test --test conversation_tests -- --nocapture
            ;;
        file)
            echo -e "${GREEN}运行文件测试...${NC}"
            cargo test --test file_tests -- --nocapture
            ;;
        all|"")
            echo -e "${GREEN}运行所有测试...${NC}"
            cargo test -- --nocapture
            ;;
        *)
            echo -e "${RED}未知测试: $test_filter${NC}"
            echo "可用测试: api, websocket, user, message, conversation, file, all"
            exit 1
            ;;
    esac
    
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}测试完成${NC}"
    echo -e "${GREEN}========================================${NC}"
}

# 主流程
main() {
    check_env
    check_service
    run_tests "$1"
}

main "$@"
