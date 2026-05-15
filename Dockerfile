# OmniLink IM System - Dockerfile
# 多阶段构建，优化镜像大小

# ============================================
# 阶段 1: 构建 Rust 后端
# ============================================
FROM rust:1.75-slim as backend-builder

WORKDIR /app

# 安装构建依赖
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# 配置 cargo 使用 sparse 协议加速
ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
ENV CARGO_BUILD_JOBS=2

# 复制 Cargo 配置和清单文件
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/

# 构建后端服务
RUN cargo build --release --bin im-api && \
    cargo build --release --bin im-gateway && \
    cargo build --release --bin im-worker

# ============================================
# 阶段 2: 构建前端
# ============================================
FROM node:20-alpine as frontend-builder

WORKDIR /app

# 复制 package.json 和 lock 文件
COPY frontend/package.json frontend/package-lock.json ./

# 安装依赖
RUN npm ci

# 复制源代码
COPY frontend/ .

# 构建前端
RUN npm run build

# ============================================
# 阶段 3: 运行时镜像
# ============================================
FROM debian:bookworm-slim

# 安装运行时依赖
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# 创建非 root 用户
RUN groupadd -r omnilink && useradd -r -g omnilink -d /app -s /sbin/nologin omnilink

WORKDIR /app

# 从构建阶段复制二进制文件
COPY --from=backend-builder /app/target/release/im-api /usr/local/bin/
COPY --from=backend-builder /app/target/release/im-gateway /usr/local/bin/
COPY --from=backend-builder /app/target/release/im-worker /usr/local/bin/

# 从前端构建阶段复制静态文件
COPY --from=frontend-builder /app/build /app/static

# 复制配置文件
COPY config/ /app/config/

# 复制迁移文件
COPY migrations/ /app/migrations/

# 设置权限
RUN chown -R omnilink:omnilink /app

# 切换到非 root 用户
USER omnilink

# 暴露端口
EXPOSE 8080 8081

# 健康检查
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# 默认启动命令
CMD ["im-api"]
