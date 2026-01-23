# Build stage
FROM docker.xuanyuan.run/rust:1.85-bookworm AS builder

WORKDIR /app

# Install dependencies for Pingora
RUN apt-get update && apt-get install -y \
    cmake \
    libclang-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Create dummy src for dependency caching
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Downgrade incompatible deps and build
RUN cargo update home@0.5.12 --precise 0.5.9 && \
    cargo build --release && rm -rf src target/release/deps/arc_auth*

# Copy actual source
COPY src ./src
COPY migrations ./migrations
COPY config ./config
COPY web/dist ./web/dist

# Build the application
RUN cargo build --release

# Runtime stage
FROM docker.xuanyuan.run/debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy binary
COPY --from=builder /app/target/release/arc_auth /app/arc_auth

# Copy config, migrations and web assets
COPY --from=builder /app/config ./config
COPY --from=builder /app/migrations ./migrations
COPY --from=builder /app/web/dist ./web/dist

# Expose ports
EXPOSE 8080 3001

CMD ["./arc_auth"]
