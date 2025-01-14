# Dockerfile
FROM rust:1.75-slim-buster as builder

# Install dependencies
RUN apt-get update && apt-get install -y \
    libudev-dev \
    clang \
    pkg-config \
    libssl-dev \
    build-essential \
    llvm-dev \
    libclang-dev \
    cmake \
    protobuf-compiler
RUN update-ca-certificates

# Set up build directory
WORKDIR /usr/src/app
COPY . .

# Build with cache mounting for faster builds
RUN --mount=type=cache,mode=0777,target=/usr/src/app/target \
    --mount=type=cache,mode=0777,target=/usr/local/cargo/registry \
    cargo build --release --package tip-router-operator-cli

# Production image
FROM debian:buster-slim

# Install necessary runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl1.1 \
    && rm -rf /var/lib/apt/lists/*

# Create necessary directories
RUN mkdir -p /solana/ledger /solana/snapshots /solana/snapshots/autosnapshot

# Copy binary from builder
COPY --from=builder /usr/src/app/target/release/tip-router-operator-cli /usr/local/bin/

# Set up environment
ENV RUST_LOG=info

# Set file descriptor limits
RUN ulimit -n 2000000

# Command will be provided by docker-compose
ENTRYPOINT ["tip-router-operator-cli"]