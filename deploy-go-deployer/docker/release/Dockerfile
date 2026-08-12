# syntax=docker/dockerfile:1.7

FROM rust:1.94.0-alpine AS builder

WORKDIR /build

RUN apk add --no-cache musl-dev

COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY agent/src agent/src
COPY agent-executor/src agent-executor/src
COPY agent-protocol/src agent-protocol/src
COPY api/src api/src
COPY release-authorization/src release-authorization/src
COPY terminal-capability/src terminal-capability/src
COPY agent/Cargo.toml agent/Cargo.toml
COPY agent-executor/Cargo.toml agent-executor/Cargo.toml
COPY agent-protocol/Cargo.toml agent-protocol/Cargo.toml
COPY api/Cargo.toml api/Cargo.toml
COPY api/openapi/external.json api/openapi/external.json
COPY release-authorization/Cargo.toml release-authorization/Cargo.toml
COPY terminal-capability/Cargo.toml terminal-capability/Cargo.toml
COPY deploy-go-deployer deploy-go-deployer
COPY container-template container-template

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build --locked --release -p deploy-go-deployer && \
    cp target/release/deploy-go-deployer /tmp/deploy-go-deployer

FROM alpine:3.22

COPY --from=builder /tmp/deploy-go-deployer /deploy-go-deployer

# 该镜像仅作为本机构建产物容器使用，不用于运行 deployer。
CMD ["/bin/true"]
