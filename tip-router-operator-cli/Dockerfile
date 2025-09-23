# Dockerfile
FROM rust:1.85.0-slim-bookworm as builder

# Install dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    llvm-dev \
    libclang-dev \
    clang \
    protobuf-compiler \
    git \
    pkg-config \
    libssl-dev \
    libudev-dev \
    cmake
RUN update-ca-certificates

# Set up build directory
WORKDIR /usr/src/app
COPY . .

RUN echo "Contents of /usr/src/app:" && \
    ls -la && \
    echo "Cargo workspace info:" && \
    cargo metadata --format-version=1

# Build with cache mounting for faster builds
RUN echo "Starting cargo build..." && \
    cargo build --release --bin tip-router-operator-cli -vv && \
    echo "Build completed, checking results:" && \
    find /usr/src/app/target -type f -name "tip-router-operator-cli*" && \
    ls -la /usr/src/app/target/release/

# Production image
FROM debian:bookworm-slim

# Install necessary runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create necessary directories
RUN mkdir -p /solana/ledger /solana/snapshots /solana/snapshots/autosnapshot

# Copy binary from builder
COPY --from=builder /usr/src/app/target/release/tip-router-operator-cli /usr/local/bin/

# Set up environment
ENV RUST_LOG=info

# Command will be provided by docker-compose
ENTRYPOINT ["tip-router-operator-cli"]