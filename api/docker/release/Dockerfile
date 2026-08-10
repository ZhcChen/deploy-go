# syntax=docker/dockerfile:1.7

FROM rust:1.94.0-alpine AS builder

WORKDIR /build

RUN apk add --no-cache musl-dev

COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY agent agent
COPY agent-executor agent-executor
COPY agent-protocol agent-protocol
COPY release-authorization release-authorization
COPY terminal-capability terminal-capability
COPY deploy-go-deployer/Cargo.toml deploy-go-deployer/Cargo.toml
COPY deploy-go-deployer/release/manifest.schema.json deploy-go-deployer/release/manifest.schema.json
COPY agent/release agent/release
COPY agent/install/install.sh agent/install/install.sh
COPY api/Cargo.toml api/Cargo.toml
COPY api/migrations api/migrations
COPY api/src api/src
COPY docs/standards/deploy-artifact-manifest.schema.json docs/standards/deploy-artifact-manifest.schema.json

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build --locked --release -p deploy-go-api && \
    cp target/release/deploy-go-api /tmp/deploy-go-api

FROM alpine:3.22

WORKDIR /app

RUN apk add --no-cache ca-certificates curl tzdata && \
    addgroup -g 1000 app && \
    adduser -D -u 1000 -G app app && \
    mkdir -p /data && \
    mkdir -p /var/lib/deploy-go/agent-releases && \
    chown -R app:app /data /var/lib/deploy-go

COPY --from=builder --chown=app:app /tmp/deploy-go-api /app/deploy-go-api

ENV DEPLOY_GO_BIND_ADDR=0.0.0.0:30100 \
    DEPLOY_GO_DATABASE_URL=sqlite:///data/deploy-go.db \
    RUST_LOG=info

USER app

EXPOSE 30100

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl --fail --silent http://127.0.0.1:30100/healthz || exit 1

CMD ["/app/deploy-go-api"]
