# syntax=docker/dockerfile:1.7

# 构建阶段的 COPY 全部使用 --link：每个输入独立成层，前面某个输入变化不再连带使后面的层失效，
# 没变的 crate 保持原来的文件修改时间，cargo 不会因为文件被重新复制而重编。
# 例如只改 agent/src 时，api 与 container-template 不需要重编。
# 基础镜像按摘要固定：浮动标签在两次部署之间被上游重新发布时，会让此后的所有层失效并重新下载。
# 更新方法见 docs/runbooks/systemd-deployment-production.md 的“基础镜像摘要”。
FROM rust:1.99.0-alpine@sha256:0cce0a5e0e8ba67b455257a3a02a1d99005f382748789d6464460028810f1627 AS builder

WORKDIR /build

ENV RUSTC_WRAPPER=sccache \
    SCCACHE_DIR=/sccache \
    SCCACHE_CACHE_SIZE=20G

# 构建机（qfy-test2）访问国外源极慢（实测 Alpine 官方源约 16KB/s，国内镜像约 3MB/s），因此依赖统一走国内镜像；
# 均可用 --build-arg 覆盖，APK_MIRROR 设为空字符串则使用官方源。
ARG APK_MIRROR=mirrors.aliyun.com
ARG CARGO_MIRROR_REGISTRY=sparse+https://rsproxy.cn/index/
ARG RUSTUP_MIRROR=https://rsproxy.cn
RUN if [ -n "$APK_MIRROR" ]; then sed -i "s#http[s]*://dl-cdn.alpinelinux.org#https://${APK_MIRROR}#g" /etc/apk/repositories; fi; \
    apk add --no-cache musl-dev sccache
# Cargo 源替换不改变 Cargo.lock 的来源与校验和，--locked 仍然有效。
RUN printf '[source.crates-io]\nreplace-with = "deploy-mirror"\n[source.deploy-mirror]\nregistry = "%s"\n' "$CARGO_MIRROR_REGISTRY" >> "$CARGO_HOME/config.toml"
ENV RUSTUP_DIST_SERVER=$RUSTUP_MIRROR RUSTUP_UPDATE_ROOT=$RUSTUP_MIRROR/rustup

ARG TARGETARCH
ARG BUILD_API=0
ARG BUILD_AGENT=0
ARG BUILD_DEPLOYER=0

COPY --link Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY --link agent/Cargo.toml agent/Cargo.toml
COPY --link agent-executor/Cargo.toml agent-executor/Cargo.toml
COPY --link agent-updater/Cargo.toml agent-updater/Cargo.toml
COPY --link agent-protocol/Cargo.toml agent-protocol/Cargo.toml
COPY --link api/Cargo.toml api/Cargo.toml
COPY --link container-template/Cargo.toml container-template/Cargo.toml
COPY --link deploy-go-deployer/Cargo.toml deploy-go-deployer/Cargo.toml
COPY --link release-authorization/Cargo.toml release-authorization/Cargo.toml
COPY --link terminal-capability/Cargo.toml terminal-capability/Cargo.toml
COPY --link runtime-log/Cargo.toml runtime-log/Cargo.toml

RUN mkdir -p agent/src agent-executor/src agent-updater/src agent-protocol/src api/src container-template/src \
        deploy-go-deployer/src release-authorization/src terminal-capability/src runtime-log/src && \
    touch agent/src/lib.rs agent/src/main.rs agent-executor/src/lib.rs agent-executor/src/main.rs agent-updater/src/main.rs agent-updater/src/transaction.rs \
        agent-protocol/src/lib.rs api/src/lib.rs api/src/main.rs container-template/src/lib.rs \
        deploy-go-deployer/src/main.rs release-authorization/src/lib.rs terminal-capability/src/lib.rs runtime-log/src/lib.rs

RUN --mount=type=cache,id=deploy-go-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=deploy-go-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    cargo fetch --locked

COPY --link agent/src agent/src
COPY --link agent-executor/src agent-executor/src
COPY --link agent-updater/src agent-updater/src
COPY --link agent-protocol/src agent-protocol/src
COPY --link release-authorization/src release-authorization/src
COPY --link terminal-capability/src terminal-capability/src
COPY --link runtime-log/src runtime-log/src
COPY --link container-template/src container-template/src
COPY --link examples/templates examples/templates
COPY --link deploy-go-deployer/src deploy-go-deployer/src
COPY --link deploy-go-deployer/release deploy-go-deployer/release
COPY --link agent/release agent/release
COPY --link agent/install/install.sh agent/install/install.sh
COPY --link api/migrations api/migrations
COPY --link api/openapi/external.json api/openapi/external.json
COPY --link api/src api/src
COPY --link docs/standards/deploy-artifact-manifest.schema.json docs/standards/deploy-artifact-manifest.schema.json

RUN --mount=type=cache,id=deploy-go-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=deploy-go-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=deploy-go-rust-target-${TARGETARCH},target=/build/target,sharing=locked \
    --mount=type=cache,id=deploy-go-sccache,target=/sccache,sharing=locked \
    set -eu; \
    case "$BUILD_API" in 0|1) ;; *) echo "BUILD_API must be 0 or 1" >&2; exit 1 ;; esac; \
    case "$BUILD_AGENT" in 0|1) ;; *) echo "BUILD_AGENT must be 0 or 1" >&2; exit 1 ;; esac; \
    case "$BUILD_DEPLOYER" in 0|1) ;; *) echo "BUILD_DEPLOYER must be 0 or 1" >&2; exit 1 ;; esac; \
    [ "$BUILD_API:$BUILD_AGENT:$BUILD_DEPLOYER" != "0:0:0" ] || \
        { echo "at least one release component must be enabled" >&2; exit 1; }; \
    packages=""; \
    [ "$BUILD_API" = 0 ] || packages="$packages -p deploy-go-api"; \
    [ "$BUILD_AGENT" = 0 ] || packages="$packages -p deploy-go-agent -p deploy-go-agent-executor -p deploy-go-agent-updater"; \
    [ "$BUILD_DEPLOYER" = 0 ] || packages="$packages -p deploy-go-deployer"; \
    cargo build --locked --release $packages; \
    mkdir -p /out; \
    [ "$BUILD_API" = 0 ] || cp target/release/deploy-go-api /out/deploy-go-api; \
    if [ "$BUILD_AGENT" = 1 ]; then \
        cp target/release/deploy-go-agent /out/deploy-go-agent; \
        cp target/release/deploy-go-agent-executor /out/deploy-go-agent-executor; \
        cp target/release/deploy-go-agent-updater /out/deploy-go-agent-updater; \
    fi; \
    [ "$BUILD_DEPLOYER" = 0 ] || cp target/release/deploy-go-deployer /out/deploy-go-deployer; \
    sccache --show-stats | sed -n '1,8p'

# 只含产物的导出阶段：远程构建用 `--target artifacts --output type=local` 直接导出文件，
# 不再 `--load` 成镜像再 docker create / cp（那会在构建机上为每个提交遗留一个约 90MB 的镜像）。
# 必须放在最终的 alpine 阶段之前：本机构建 deploy.sh 用默认目标（最后一个阶段）并经 docker create / cp 取产物。
FROM scratch AS artifacts

COPY --from=builder /out/ /

FROM alpine:3.22@sha256:49340b346798875b5592e5d420241f2f9cfba5f47e62ebb1446a1b34734e898f

COPY --from=builder /out/ /out/
