# Multi-stage Dockerfile for OmniLink Rust services
# Stage 1: Build
FROM rust:1.82-bookworm AS builder

WORKDIR /app

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/

# Build the specified binary
ARG SERVICE_NAME=im-api
ENV CARGO_BUILD_JOBS=1

RUN cargo build --release --bin ${SERVICE_NAME} && \
    strip target/release/${SERVICE_NAME}

# Stage 2: Runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -s /bin/false appuser

WORKDIR /app

# Copy binary from builder
ARG SERVICE_NAME=im-api
COPY --from=builder /app/target/release/${SERVICE_NAME} /app/${SERVICE_NAME}

# Copy migrations
COPY migrations/ /app/migrations/

# Set ownership
RUN chown -R appuser:appuser /app

USER appuser

# Health check
HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
    CMD curl -f http://localhost:${SERVICE_PORT:-8002}/health || exit 1

# Default port (overridable per service)
ENV SERVICE_PORT=8002
EXPOSE ${SERVICE_PORT}

ENTRYPOINT ["/app/im-api"]
