# syntax=docker/dockerfile:1.7

FROM rust:1.94.0-alpine AS builder

WORKDIR /build

RUN apk add --no-cache musl-dev

ARG TARGETARCH
ARG BUILD_API=0
ARG BUILD_AGENT=0
ARG BUILD_DEPLOYER=0

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
COPY deploy-go-deployer/release deploy-go-deployer/release
COPY agent/release agent/release
COPY agent/install/install.sh agent/install/install.sh
COPY api/migrations api/migrations
COPY api/openapi/external.json api/openapi/external.json
COPY api/src api/src
COPY docs/standards/deploy-artifact-manifest.schema.json docs/standards/deploy-artifact-manifest.schema.json

RUN --mount=type=cache,id=deploy-go-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=deploy-go-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=deploy-go-rust-target-${TARGETARCH},target=/build/target,sharing=locked \
    set -eu; \
    case "$BUILD_API" in 0|1) ;; *) echo "BUILD_API must be 0 or 1" >&2; exit 1 ;; esac; \
    case "$BUILD_AGENT" in 0|1) ;; *) echo "BUILD_AGENT must be 0 or 1" >&2; exit 1 ;; esac; \
    case "$BUILD_DEPLOYER" in 0|1) ;; *) echo "BUILD_DEPLOYER must be 0 or 1" >&2; exit 1 ;; esac; \
    [ "$BUILD_API:$BUILD_AGENT:$BUILD_DEPLOYER" != "0:0:0" ] || \
        { echo "at least one release component must be enabled" >&2; exit 1; }; \
    packages=""; \
    [ "$BUILD_API" = 0 ] || packages="$packages -p deploy-go-api"; \
    [ "$BUILD_AGENT" = 0 ] || packages="$packages -p deploy-go-agent -p deploy-go-agent-executor"; \
    [ "$BUILD_DEPLOYER" = 0 ] || packages="$packages -p deploy-go-deployer"; \
    cargo build --locked --release $packages; \
    mkdir -p /out; \
    [ "$BUILD_API" = 0 ] || cp target/release/deploy-go-api /out/deploy-go-api; \
    if [ "$BUILD_AGENT" = 1 ]; then \
        cp target/release/deploy-go-agent /out/deploy-go-agent; \
        cp target/release/deploy-go-agent-executor /out/deploy-go-agent-executor; \
    fi; \
    [ "$BUILD_DEPLOYER" = 0 ] || cp target/release/deploy-go-deployer /out/deploy-go-deployer

FROM alpine:3.22

COPY --from=builder /out/ /out/
