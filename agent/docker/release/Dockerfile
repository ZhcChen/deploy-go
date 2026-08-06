# syntax=docker/dockerfile:1.7

FROM rust:1.94.0-alpine AS builder

WORKDIR /build

RUN apk add --no-cache musl-dev

COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY agent agent
COPY agent-protocol agent-protocol
COPY api/Cargo.toml api/Cargo.toml
COPY api/src api/src

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build --locked --release -p deploy-go-agent && \
    cp target/release/deploy-go-agent /tmp/deploy-go-agent

FROM alpine:3.22

COPY --from=builder /tmp/deploy-go-agent /deploy-go-agent

# 该镜像仅作为本机构建产物容器使用，不用于运行 Agent。
CMD ["/bin/true"]
