##### Builder image
FROM rust:1.80.0-slim-bullseye as builder

RUN apt-get update && apt-get install -y \
    libudev-dev \
    clang \
    pkg-config \
    libssl-dev \
    build-essential \
    cmake \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/* \
    && update-ca-certificates

WORKDIR /usr/src/app

COPY . .

RUN cargo build --release --bin jito-tip-router-cli
RUN cargo install --path ./cli --bin jito-tip-router-cli --locked

##### Final image
FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl1.1 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/app/target/release/jito-tip-router-cli /usr/local/bin/jito-tip-router-cli

ENTRYPOINT ["jito-tip-router-cli keeper"]
