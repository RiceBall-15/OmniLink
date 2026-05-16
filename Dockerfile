# Multi-stage build for OmniLink
# Builds all 8 microservices and the frontend

# ============================================================
# Stage 1: Build Rust backend services
# ============================================================
FROM rust:1.77-slim as backend-builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/
COPY migrations/ ./migrations/

# Build all service binaries in release mode
RUN cargo build --release \
    --bin im-api \
    --bin im-gateway \
    --bin ai-service \
    --bin user-service \
    --bin file-service \
    --bin push-service \
    --bin config-service \
    --bin usage-service

# ============================================================
# Stage 2: Build the frontend
# ============================================================
FROM node:20-slim as frontend-builder

WORKDIR /app/frontend

# Copy package files
COPY frontend/web/package.json frontend/web/package-lock.json ./

# Install dependencies
RUN npm ci

# Copy source code
COPY frontend/web/ ./

# Build frontend
RUN npm run build

# ============================================================
# Stage 3: Production image
# ============================================================
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy all service binaries
COPY --from=backend-builder /app/target/release/im-api /usr/local/bin/
COPY --from=backend-builder /app/target/release/im-gateway /usr/local/bin/
COPY --from=backend-builder /app/target/release/ai-service /usr/local/bin/
COPY --from=backend-builder /app/target/release/user-service /usr/local/bin/
COPY --from=backend-builder /app/target/release/file-service /usr/local/bin/
COPY --from=backend-builder /app/target/release/push-service /usr/local/bin/
COPY --from=backend-builder /app/target/release/config-service /usr/local/bin/
COPY --from=backend-builder /app/target/release/usage-service /usr/local/bin/

# Copy frontend build
COPY --from=frontend-builder /app/frontend/dist ./frontend/dist

# Copy migrations
COPY --from=backend-builder /app/migrations ./migrations

# Create non-root user
RUN useradd -m -s /bin/bash omnilink && \
    chown -R omnilink:omnilink /app
USER omnilink

# Default command (overridden by docker-compose)
CMD ["im-api"]
