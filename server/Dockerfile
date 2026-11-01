# ==========================================
# Base Stage (Common Rust Environment)
# ==========================================
FROM rust:1-slim-bookworm AS base

WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# ==========================================
# Development Stage (Hot-reloading with cargo-watch)
# ==========================================
FROM base AS dev

RUN cargo install cargo-watch

COPY Cargo.toml Cargo.lock ./
# Pre-build dummy source to cache dependency downloads
RUN mkdir src && echo "fn main() {}" > src/main.rs && \
    cargo build && \
    rm -rf src

COPY . .

EXPOSE 5000
CMD ["cargo", "watch", "-x", "run"]

# ==========================================
# Builder Stage (Production Release Build)
# ==========================================
FROM base AS builder

COPY Cargo.toml Cargo.lock ./

# Cache compiled dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs && \
    cargo build --release --locked && \
    rm -rf src

# Copy source code and build final release binary
COPY src ./src
RUN touch src/main.rs && cargo build --release --locked

# ==========================================
# Production Runtime Stage (Minimal Image)
# ==========================================
FROM debian:bookworm-slim AS prod

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    libssl3 \
    tzdata \
    && rm -rf /var/lib/apt/lists/*

# Run as non-root user
RUN useradd -m -u 1001 appuser
USER appuser

WORKDIR /app

COPY --from=builder --chown=appuser:appuser /app/target/release/secure-chat-server /app/secure-chat-server

EXPOSE 5000
CMD ["/app/secure-chat-server"]
