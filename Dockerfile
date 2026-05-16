# Multi-stage build for OmniLink
# Stage 1: Build the Rust backend
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

# Build release binary
RUN cargo build --release --bin omnilink

# Stage 2: Build the frontend
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

# Stage 3: Production image
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy backend binary
COPY --from=backend-builder /app/target/release/omnilink /usr/local/bin/

# Copy frontend build
COPY --from=frontend-builder /app/frontend/dist ./frontend/dist

# Copy migrations
COPY --from=backend-builder /app/migrations ./migrations

# Create non-root user
RUN useradd -m -s /bin/bash omnilink
USER omnilink

# Expose ports
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Run the application
CMD ["omnilink"]
