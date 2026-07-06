FROM rust:slim AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY shared/                shared/
COPY auth-user-service/     auth-user-service/
COPY crdt-sync-service/     crdt-sync-service/
COPY session-gateway-service/ session-gateway-service/
COPY code-execution-service/ code-execution-service/

RUN cargo build --release --bin code-execution-service

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/code-execution-service /usr/local/bin/

EXPOSE 3003
CMD ["code-execution-service"]
