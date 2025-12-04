# syntax=docker/dockerfile:1

# ============================================================================
# Stage 1: Build
# ============================================================================
FROM rust:1.83-slim-bookworm AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY migrations ./migrations

# Build release binary
RUN cargo build --release --package graphql-api

# ============================================================================
# Stage 2: Runtime
# ============================================================================
FROM debian:bookworm-slim AS runtime

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/graphql-api /app/graphql-api

# Copy migrations (embedded at compile time, but useful for reference)
COPY --from=builder /app/migrations /app/migrations

# Create non-root user
RUN useradd -m -u 1000 appuser
USER appuser

# Configuration
ENV LISTEN_ADDR=0.0.0.0:3000
ENV RUST_LOG=graphql_api=info,user_feature=info,todo_feature=info

EXPOSE 3000

CMD ["/app/graphql-api"]
