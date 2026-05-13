#!/bin/bash
#
# OmniLink 集成测试脚本
# 用于在本地或资源充足的环境运行完整集成测试
#
# 使用方法:
#   chmod +x integration_test.sh
#   ./integration_test.sh
#

set -e  # 遇到错误立即退出

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 日志函数
log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[✓]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[!]${NC} $1"; }
log_error() { echo -e "${RED}[✗]${NC} $1"; }

# 测试结果统计
TESTS_PASSED=0
TESTS_FAILED=0
TESTS_SKIPPED=0

# 检查依赖
check_dependencies() {
    log_info "检查依赖..."
    
    local missing=()
    
    command -v cargo >/dev/null 2>&1 || missing+=("cargo (Rust)")
    command -v node >/dev/null 2>&1 || missing+=("node")
    command -v npm >/dev/null 2>&1 || missing+=("npm")
    command -v docker >/dev/null 2>&1 || missing+=("docker")
    command -v docker-compose >/dev/null 2>&1 || missing+=("docker-compose")
    
    if [ ${#missing[@]} -ne 0 ]; then
        log_error "缺少依赖: ${missing[*]}"
        echo ""
        echo "请安装以下依赖:"
        echo "  - Rust: https://rustup.rs/"
        echo "  - Node.js: https://nodejs.org/"
        echo "  - Docker: https://docs.docker.com/get-docker/"
        echo "  - Docker Compose: https://docs.docker.com/compose/install/"
        exit 1
    fi
    
    log_success "所有依赖已安装"
}

# 检查资源
check_resources() {
    log_info "检查系统资源..."
    
    # 检查内存（需要至少 4GB）
    local total_mem=$(free -g | awk '/^Mem:/{print $2}')
    if [ "$total_mem" -lt 4 ]; then
        log_warn "内存不足 4GB（当前 ${total_mem}GB），编译可能较慢或失败"
        read -p "是否继续？(y/N) " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            exit 1
        fi
    else
        log_success "内存充足: ${total_mem}GB"
    fi
    
    # 检查磁盘空间（需要至少 10GB）
    local avail_disk=$(df -BG . | awk 'NR==2{print $4}' | sed 's/G//')
    if [ "$avail_disk" -lt 10 ]; then
        log_warn "磁盘空间不足 10GB（当前可用 ${avail_disk}GB）"
    else
        log_success "磁盘空间充足: ${avail_disk}GB"
    fi
}

# 启动数据库服务
start_databases() {
    log_info "启动数据库服务..."
    
    cd docker
    
    # 检查 docker-compose.yml 是否存在
    if [ ! -f "docker-compose.yml" ]; then
        log_error "未找到 docker-compose.yml"
        exit 1
    fi
    
    # 启动服务
    docker-compose up -d
    
    # 等待服务就绪
    log_info "等待数据库服务就绪..."
    sleep 10
    
    # 检查 PostgreSQL
    if docker-compose exec -T postgres pg_isready -U omnilink >/dev/null 2>&1; then
        log_success "PostgreSQL 就绪"
    else
        log_error "PostgreSQL 未就绪"
        docker-compose logs postgres
        exit 1
    fi
    
    # 检查 Redis
    if docker-compose exec -T redis redis-cli ping | grep -q PONG; then
        log_success "Redis 就绪"
    else
        log_warn "Redis 未就绪（可选）"
    fi
    
    cd ..
}

# 运行后端单元测试
run_backend_tests() {
    log_info "运行后端单元测试..."
    
    cd omnilink
    
    # 设置编译并行度（避免 OOM）
    export CARGO_BUILD_JOBS=2
    
    # 运行测试
    if cargo test --workspace 2>&1 | tee /tmp/omnilink_test.log; then
        # 统计测试结果
        local passed=$(grep -c "test result: ok" /tmp/omnilink_test.log || echo 0)
        local failed=$(grep -c "test result: FAILED" /tmp/omnilink_test.log || echo 0)
        
        if [ "$failed" -eq 0 ]; then
            log_success "后端单元测试全部通过"
            TESTS_PASSED=$((TESTS_PASSED + passed))
        else
            log_error "后端单元测试有失败"
            TESTS_FAILED=$((TESTS_FAILED + failed))
        fi
    else
        log_error "后端测试运行失败"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
    
    cd ..
}

# 运行后端编译检查
run_backend_check() {
    log_info "运行后端编译检查..."
    
    cd omnilink
    
    export CARGO_BUILD_JOBS=2
    
    if cargo check --workspace 2>&1 | tee /tmp/omnilink_check.log; then
        log_success "后端编译检查通过"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        log_error "后端编译检查失败"
        TESTS_FAILED=$((TESTS_FAILED + 1))
        
        # 显示错误信息
        echo ""
        log_error "编译错误详情:"
        grep -A 5 "error\[" /tmp/omnilink_check.log | head -50
    fi
    
    cd ..
}

# 运行前端测试
run_frontend_tests() {
    log_info "运行前端测试..."
    
    cd omnilink/web
    
    # 检查 node_modules
    if [ ! -d "node_modules" ]; then
        log_info "安装前端依赖..."
        npm install
    fi
    
    # TypeScript 类型检查
    log_info "运行 TypeScript 类型检查..."
    if npm run type-check 2>&1 | tee /tmp/omnilink_typecheck.log; then
        log_success "TypeScript 类型检查通过"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        log_error "TypeScript 类型检查失败"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
    
    # ESLint 检查
    log_info "运行 ESLint 检查..."
    if npm run lint 2>&1 | tee /tmp/omnilink_lint.log; then
        log_success "ESLint 检查通过"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        log_warn "ESLint 有警告（非阻塞）"
        TESTS_SKIPPED=$((TESTS_SKIPPED + 1))
    fi
    
    # 前端构建
    log_info "运行前端构建..."
    if npm run build 2>&1 | tee /tmp/omnilink_build.log; then
        log_success "前端构建成功"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        log_error "前端构建失败"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
    
    cd ../..
}

# 运行 API 集成测试
run_api_integration_tests() {
    log_info "运行 API 集成测试..."
    
    cd omnilink
    
    # 启动后端服务（后台）
    log_info "启动后端服务..."
    export DATABASE_URL="postgresql://omnilink:omnilink@localhost:5432/omnilink"
    export REDIS_URL="redis://localhost:6379"
    export JWT_SECRET="test-secret-key-for-integration-testing"
    
    # 启动 im-api
    cd crates/im-api
    cargo run --release &
    IM_API_PID=$!
    cd ../..
    
    # 等待服务启动
    log_info "等待 im-api 启动..."
    sleep 15
    
    # 健康检查
    if curl -s http://localhost:8080/health | grep -q "ok"; then
        log_success "im-api 健康检查通过"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        log_error "im-api 健康检查失败"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
    
    # 测试用户注册
    log_info "测试用户注册 API..."
    REGISTER_RESPONSE=$(curl -s -X POST http://localhost:8080/api/auth/register \
        -H "Content-Type: application/json" \
        -d '{"email":"test@example.com","username":"testuser","password":"Test123!"}')
    
    if echo "$REGISTER_RESPONSE" | grep -q "id"; then
        log_success "用户注册 API 测试通过"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        log_error "用户注册 API 测试失败"
        echo "Response: $REGISTER_RESPONSE"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
    
    # 测试用户登录
    log_info "测试用户登录 API..."
    LOGIN_RESPONSE=$(curl -s -X POST http://localhost:8080/api/auth/login \
        -H "Content-Type: application/json" \
        -d '{"email":"test@example.com","password":"Test123!"}')
    
    if echo "$LOGIN_RESPONSE" | grep -q "token"; then
        log_success "用户登录 API 测试通过"
        TESTS_PASSED=$((TESTS_PASSED + 1))
        
        # 提取 token
        TOKEN=$(echo "$LOGIN_RESPONSE" | grep -o '"token":"[^"]*"' | cut -d'"' -f4)
        
        # 测试获取用户信息
        log_info "测试获取用户信息 API..."
        USER_RESPONSE=$(curl -s http://localhost:8080/api/user/me \
            -H "Authorization: Bearer $TOKEN")
        
        if echo "$USER_RESPONSE" | grep -q "test@example.com"; then
            log_success "获取用户信息 API 测试通过"
            TESTS_PASSED=$((TESTS_PASSED + 1))
        else
            log_error "获取用户信息 API 测试失败"
            TESTS_FAILED=$((TESTS_FAILED + 1))
        fi
    else
        log_error "用户登录 API 测试失败"
        echo "Response: $LOGIN_RESPONSE"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
    
    # 停止后端服务
    log_info "停止后端服务..."
    kill $IM_API_PID 2>/dev/null || true
    
    cd ..
}

# 清理资源
cleanup() {
    log_info "清理资源..."
    
    # 停止数据库服务
    if [ -f "omnilink/docker/docker-compose.yml" ]; then
        cd omnilink/docker
        docker-compose down
        cd ../..
    fi
    
    # 停止可能残留的进程
    pkill -f "cargo run" 2>/dev/null || true
}

# 显示测试报告
show_report() {
    echo ""
    echo "=========================================="
    echo "        OmniLink 集成测试报告"
    echo "=========================================="
    echo ""
    echo -e "${GREEN}通过: $TESTS_PASSED${NC}"
    echo -e "${RED}失败: $TESTS_FAILED${NC}"
    echo -e "${YELLOW}跳过: $TESTS_SKIPPED${NC}"
    echo ""
    
    if [ $TESTS_FAILED -eq 0 ]; then
        echo -e "${GREEN}🎉 所有测试通过！${NC}"
        echo ""
        echo "下一步："
        echo "  1. 启动后端: cd omnilink/crates/im-api && cargo run"
        echo "  2. 启动前端: cd omnilink/web && npm run dev"
        echo "  3. 访问 http://localhost:5173"
        return 0
    else
        echo -e "${RED}❌ 有测试失败，请检查日志${NC}"
        echo ""
        echo "日志文件："
        echo "  - /tmp/omnilink_test.log"
        echo "  - /tmp/omnilink_check.log"
        echo "  - /tmp/omnilink_typecheck.log"
        echo "  - /tmp/omnilink_build.log"
        return 1
    fi
}

# 主函数
main() {
    echo "=========================================="
    echo "     OmniLink 集成测试脚本"
    echo "=========================================="
    echo ""
    
    # 设置退出时清理
    trap cleanup EXIT
    
    # 执行测试步骤
    check_dependencies
    check_resources
    start_databases
    run_backend_check
    run_backend_tests
    run_frontend_tests
    run_api_integration_tests
    
    # 显示报告
    show_report
}

# 运行主函数
main
