# Build stage
FROM rustlang/rust:nightly-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Create dummy project for dependency caching
RUN USER=root cargo new --bin nate-status
WORKDIR /app/nate-status

# Copy manifests ONLY first
COPY Cargo.toml Cargo.lock ./
COPY rust-toolchain.toml* ./

# Build dependencies only - this layer will be cached
RUN cargo build --release && rm -rf src target/release/deps/nate_status*

# Now copy actual source code
COPY src ./src
COPY templates ./templates
COPY lexicons ./lexicons
COPY static ./static

# Touch main.rs to ensure rebuild with our actual code
RUN touch src/main.rs

# Build for release
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the built binary
COPY --from=builder /app/target/release/nate-status /app/nate-status

# Copy templates and lexicons
COPY templates ./templates
COPY lexicons ./lexicons
# Copy static files
COPY static ./static

# Create directory for SQLite database
RUN mkdir -p /data

# Set environment variables
ENV DB_PATH=/data/status.db
ENV ENABLE_FIREHOSE=true

# Expose port
EXPOSE 8080

# Run the binary
CMD ["./nate-status"]