# syntax=docker/dockerfile:1.7

FROM rust:1.94.0-alpine AS builder

WORKDIR /build

RUN apk add --no-cache musl-dev

ARG TARGETARCH

COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY agent/Cargo.toml agent/Cargo.toml
COPY agent-executor/Cargo.toml agent-executor/Cargo.toml
COPY agent-protocol/Cargo.toml agent-protocol/Cargo.toml
COPY api/Cargo.toml api/Cargo.toml
COPY container-template/Cargo.toml container-template/Cargo.toml
COPY deploy-go-deployer/Cargo.toml deploy-go-deployer/Cargo.toml
COPY release-authorization/Cargo.toml release-authorization/Cargo.toml
COPY terminal-capability/Cargo.toml terminal-capability/Cargo.toml

RUN mkdir -p agent/src agent-executor/src agent-protocol/src api/src container-template/src \
        deploy-go-deployer/src release-authorization/src terminal-capability/src && \
    touch agent/src/lib.rs agent/src/main.rs agent-executor/src/lib.rs agent-executor/src/main.rs \
        agent-protocol/src/lib.rs api/src/lib.rs api/src/main.rs container-template/src/lib.rs \
        deploy-go-deployer/src/main.rs release-authorization/src/lib.rs terminal-capability/src/lib.rs

RUN --mount=type=cache,id=deploy-go-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=deploy-go-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    cargo fetch --locked

COPY agent/src agent/src
COPY agent-executor/src agent-executor/src
COPY agent-protocol/src agent-protocol/src
COPY release-authorization/src release-authorization/src
COPY terminal-capability/src terminal-capability/src
COPY container-template/src container-template/src
COPY examples/templates examples/templates
COPY deploy-go-deployer/src deploy-go-deployer/src
COPY api/src api/src
COPY docs/standards/deploy-artifact-manifest.schema.json docs/standards/deploy-artifact-manifest.schema.json

RUN --mount=type=cache,id=deploy-go-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=deploy-go-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=deploy-go-rust-target-${TARGETARCH},target=/build/target,sharing=locked \
    cargo build --locked --release -p deploy-go-agent -p deploy-go-agent-executor && \
    cp target/release/deploy-go-agent /tmp/deploy-go-agent && \
    cp target/release/deploy-go-agent-executor /tmp/deploy-go-agent-executor

FROM alpine:3.22

COPY --from=builder /tmp/deploy-go-agent /deploy-go-agent
COPY --from=builder /tmp/deploy-go-agent-executor /deploy-go-agent-executor

# 该镜像仅作为本机构建产物容器使用，不用于运行 Agent。
CMD ["/bin/true"]
