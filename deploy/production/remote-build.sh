#!/usr/bin/env bash

set -euo pipefail

source_dir=""
output_dir=""
deploy_platform=""
expected_commit=""
api_version=""
agent_version=""
executor_version=""
deployer_version=""
agent_sync="1"
proxy_url=""
registry_mirror=""
cleanup="1"
builder_name=""
web_dockerfile=""
buildkit_config=""

die() {
  printf 'REMOTE_BUILD_ERROR %s\n' "$1" >&2
  exit 1
}

while (($# > 0)); do
  case "$1" in
    --source-dir) source_dir="$2"; shift 2 ;;
    --output-dir) output_dir="$2"; shift 2 ;;
    --deploy-platform) deploy_platform="$2"; shift 2 ;;
    --expected-commit) expected_commit="$2"; shift 2 ;;
    --api-version) api_version="$2"; shift 2 ;;
    --agent-version) agent_version="$2"; shift 2 ;;
    --executor-version) executor_version="$2"; shift 2 ;;
    --deployer-version) deployer_version="$2"; shift 2 ;;
    --agent-sync) agent_sync="$2"; shift 2 ;;
    --proxy-url) proxy_url="$2"; shift 2 ;;
    --registry-mirror) registry_mirror="$2"; shift 2 ;;
    --no-cleanup) cleanup="0"; shift ;;
    *) die "未知参数：$1" ;;
  esac
done

[[ -d "$source_dir" && ! -L "$source_dir" ]] || die "源码目录无效"
[[ "$output_dir" == /var/lib/deploy-go-builder/* ]] || die "构建输出目录必须位于独立构建根目录"
[[ "$deploy_platform" == linux/amd64 ]] || die "正式环境仅支持 linux/amd64"
[[ "$expected_commit" =~ ^[0-9a-f]{40}$ ]] || die "源码 commit 无效"
[[ "$agent_sync" == 0 || "$agent_sync" == 1 ]] || die "Agent 同步参数无效"
[[ "$api_version" == "$agent_version" && "$agent_version" == "$executor_version" && "$executor_version" == "$deployer_version" ]] ||
  die "发布版本不一致"

command -v docker >/dev/null 2>&1 || die "qfy-test2 缺少 docker"
docker info >/dev/null 2>&1 || die "qfy-test2 Docker daemon 不可用"
docker buildx version >/dev/null 2>&1 || die "qfy-test2 缺少 docker buildx"
command -v jq >/dev/null 2>&1 || die "qfy-test2 缺少 jq"
command -v sha256sum >/dev/null 2>&1 || die "qfy-test2 缺少 sha256sum"
command -v curl >/dev/null 2>&1 || die "qfy-test2 缺少 curl"
[[ "$(cat "$source_dir/.deploy-go-source-commit")" == "$expected_commit" ]] || die "源码快照 commit 校验失败"

build_root=/var/lib/deploy-go-builder
warn() {
  printf 'REMOTE_BUILD_WARN %s\n' "$1" >&2
}

# 构建串行化：下面会删除残留的旧 builder 和镜像，必须保证没有另一个构建正在使用它们。
command -v flock >/dev/null 2>&1 || die "qfy-test2 缺少 flock"
exec 9>"$build_root/.build.lock"
flock -w 1800 9 || die "等待另一个构建结束超时（30 分钟）"

# BuildKit 缓存上限。不配置时 BuildKit 的默认回收阈值是 min(磁盘 60%, 100GB)，大磁盘永远触发不到，
# 缓存只增不减（实测旧 builder 累积到 30GB，其中 68% 是历史 npm 层）。有工作集下限，不会把热缓存清掉。
cache_reserved="6GB"
cache_max="20GB"
[[ -z "$registry_mirror" || "$registry_mirror" =~ ^[A-Za-z0-9._:-]+$ ]] || die "Docker Registry 镜像地址无效"
buildkit_config_body="$(
  printf '[worker.oci]\n  gc = true\n  reservedSpace = "%s"\n  maxUsedSpace = "%s"\n' "$cache_reserved" "$cache_max"
  if [[ -n "$registry_mirror" ]]; then
    printf '[registry."docker.io"]\n  mirrors = ["%s"]\n' "$registry_mirror"
  fi
)"
# builder 的代理、镜像站和缓存配置只在创建时生效。名字随这些配置的摘要变化：
# 配置变了就换新 builder（避免复用已持久化的旧参数），旧的由下面的清理删除。
config_digest="$(printf '%s\n%s\n%s' "$proxy_url" "$registry_mirror" "$buildkit_config_body" | sha256sum | cut -c1-12)"
builder_name="deploy-go-production-$config_digest"
buildkit_config="$build_root/buildkitd.$config_digest.toml"
if ! docker buildx inspect "$builder_name" >/dev/null 2>&1; then
  printf '%s\n' "$buildkit_config_body" > "$buildkit_config"
  docker buildx create \
    --name "$builder_name" \
    --driver docker-container \
    --driver-opt network=host \
    --driver-opt "env.HTTP_PROXY=$proxy_url" \
    --driver-opt "env.HTTPS_PROXY=$proxy_url" \
    --driver-opt "env.ALL_PROXY=$proxy_url" \
    --driver-opt "env.http_proxy=$proxy_url" \
    --driver-opt "env.https_proxy=$proxy_url" \
    --driver-opt "env.all_proxy=$proxy_url" \
    --buildkitd-config "$buildkit_config" >/dev/null
fi
docker buildx inspect --bootstrap "$builder_name" >/dev/null

# 清理本脚本自己遗留的东西。名字匹配得很严（固定前缀 + 12 位十六进制摘要），不会碰同一台机器上其它项目的 builder 和镜像。
prune_stale_builders() {
  local name
  while read -r name; do
    [[ "$name" =~ ^deploy-go-production(-[0-9a-f]{12}|-direct)?$ ]] || continue
    [[ "$name" != "$builder_name" ]] || continue
    if docker buildx rm --force "$name" </dev/null >/dev/null 2>&1; then
      printf 'REMOTE_BUILD_CLEANUP 已删除过期 builder %s\n' "$name"
    else
      warn "删除过期 builder $name 失败，已跳过"
    fi
  done < <(docker buildx ls 2>/dev/null | awk 'NR > 1 && /^[^ ]/ { name = $1; sub(/\*$/, "", name); print name }')
}
# 旧脚本用 `--load` 把每个提交的产物留成镜像（约 90MB 一个，从不删除）。
prune_leaked_images() {
  local image removed=0
  while read -r image; do
    [[ "$image" =~ ^deploy-go-rust-release-remote-[0-9a-f]{40}-x86_64:latest$ ]] || continue
    docker rmi "$image" </dev/null >/dev/null 2>&1 && removed=$((removed + 1))
  done < <(docker images --format '{{.Repository}}:{{.Tag}}' 2>/dev/null)
  ((removed == 0)) || printf 'REMOTE_BUILD_CLEANUP 已删除 %d 个遗留的构建产物镜像\n' "$removed"
}
if [[ "$cleanup" == 1 ]]; then
  prune_stale_builders
  prune_leaked_images
fi

mkdir -p "$output_dir/deployer-release" "$output_dir/web"
if [[ "$agent_sync" == 1 ]]; then
  mkdir -p "$output_dir/agent-release"
fi
export BUILDKIT_NO_CLIENT_TOKEN="${BUILDKIT_NO_CLIENT_TOKEN:-1}"
build_proxy_args=(
  --build-arg "HTTP_PROXY=$proxy_url"
  --build-arg "HTTPS_PROXY=$proxy_url"
  --build-arg "ALL_PROXY=$proxy_url"
  --build-arg "http_proxy=$proxy_url"
  --build-arg "https_proxy=$proxy_url"
  --build-arg "all_proxy=$proxy_url"
)

build_rust_release() {
  local arch="$1"
  local platform="$2"
  local build_api="$3"
  local build_agent="$4"
  local build_deployer="$5"
  local export_dir="$output_dir/.export-$arch"

  rm -rf "$export_dir"
  mkdir -p "$export_dir"
  # 直接把 artifacts 阶段导出成文件：不 --load 成镜像，构建机上不会再留下每个提交一个的产物镜像。
  docker buildx build --builder "$builder_name" \
    --platform "$platform" \
    --network host \
    "${build_proxy_args[@]}" \
    --build-arg "BUILD_API=$build_api" \
    --build-arg "BUILD_AGENT=$build_agent" \
    --build-arg "BUILD_DEPLOYER=$build_deployer" \
    --target artifacts \
    --output "type=local,dest=$export_dir" \
    --file "$source_dir/deploy/docker/release/Dockerfile" \
    "$source_dir"
  if [[ "$build_api" == 1 && "$platform" == "$deploy_platform" ]]; then
    cp "$export_dir/deploy-go-api" "$output_dir/deploy-go-api"
    chmod 0755 "$output_dir/deploy-go-api"
  fi
  if [[ "$build_agent" == 1 ]]; then
    cp "$export_dir/deploy-go-agent" "$output_dir/agent-release/deploy-go-agent-linux-$arch"
    cp "$export_dir/deploy-go-agent-executor" \
      "$output_dir/agent-release/deploy-go-agent-executor-linux-$arch"
    cp "$export_dir/deploy-go-agent-updater" \
      "$output_dir/agent-release/deploy-go-agent-updater-linux-$arch"
    chmod 0755 \
      "$output_dir/agent-release/deploy-go-agent-linux-$arch" \
      "$output_dir/agent-release/deploy-go-agent-executor-linux-$arch" \
      "$output_dir/agent-release/deploy-go-agent-updater-linux-$arch"
  fi
  if [[ "$build_deployer" == 1 ]]; then
    cp "$export_dir/deploy-go-deployer" "$output_dir/deployer-release/deploy-go-deployer-linux-$arch"
    chmod 0755 "$output_dir/deployer-release/deploy-go-deployer-linux-$arch"
  fi
  rm -rf "$export_dir"
}

build_rust_release x86_64 linux/amd64 1 "$agent_sync" 1

web_dockerfile="$source_dir/deploy/production/web-build.Dockerfile"
cat > "$web_dockerfile" <<'EOF'
# syntax=docker/dockerfile:1.7
FROM node:22-alpine AS builder
ARG HTTP_PROXY
ARG HTTPS_PROXY
ARG ALL_PROXY
ARG http_proxy
ARG https_proxy
ARG all_proxy
ENV HTTP_PROXY=$HTTP_PROXY HTTPS_PROXY=$HTTPS_PROXY ALL_PROXY=$ALL_PROXY \
    http_proxy=$http_proxy https_proxy=$https_proxy all_proxy=$all_proxy
WORKDIR /workspace
# 依赖层只依赖 package.json 与锁文件：改 Rust 或前端源码都不会让 npm ci 重跑
# （此前整个源码目录先于 npm ci 被复制进来，任何文件变化都会让它重跑 40 多秒，并在构建缓存里多留一个约 400MB 的层）。
COPY admin/package.json admin/package-lock.json admin/
RUN --mount=type=cache,target=/root/.npm npm ci --prefix admin --ignore-scripts
# tsc -b 会类型检查 admin/src/test，其中用 ?raw 引用了仓库根的 examples/templates 与 test-fixtures。
COPY examples/templates examples/templates
COPY test-fixtures test-fixtures
COPY scripts/check-client-sensitive-data.mjs scripts/check-client-sensitive-data.mjs
COPY admin admin
RUN npm run build --prefix admin && \
    node scripts/check-client-sensitive-data.mjs admin/dist && \
    mkdir -p /web && cp -R admin/dist/. /web/
FROM scratch
COPY --from=builder /web/ /
EOF
docker buildx build --builder "$builder_name" \
  --platform "$deploy_platform" \
  --network host \
  "${build_proxy_args[@]}" \
  --output "type=local,dest=$output_dir/web" \
  --file "$web_dockerfile" \
  "$source_dir"

rm -f "$web_dockerfile"

cp "$source_dir/deploy-go-deployer/release/generate-manifest.sh" "$output_dir/deployer-release/"

if [[ "$agent_sync" == 1 ]]; then
  cp "$source_dir/agent/install/deploy-go-agent.service" "$output_dir/agent-release/"
  cp "$source_dir/agent/install/deploy-go-agent-runner.service" "$output_dir/agent-release/"
  cp "$source_dir/agent/install/deploy-go-agent-executor.service" "$output_dir/agent-release/"
  cp "$source_dir/agent/install/deploy-go-agent-updater.service" "$output_dir/agent-release/"
  cp "$source_dir/agent/install/executor.json.in" "$output_dir/agent-release/"
  cp "$source_dir/agent/release/generate-manifest.sh" "$output_dir/agent-release/"
  (
    cd "$source_dir"
    bash agent/release/generate-manifest.sh \
      "$output_dir/agent-release" \
      "https://deploy-go.invalid/agent-releases/$agent_version" \
      "$agent_version"
  )
  rm "$output_dir/agent-release/generate-manifest.sh"
fi
(
  cd "$output_dir/deployer-release"
  bash generate-manifest.sh . "https://deploy-go.invalid/deployer-releases/$deployer_version" "$deployer_version"
  rm generate-manifest.sh
)

if [[ "$agent_sync" == 1 ]]; then
  jq -e --arg version "$agent_version" \
    '.agent_version == $version and .executor_version == $version' \
    "$output_dir/agent-release/deploy-go-agent-manifest.json" >/dev/null || die "Agent manifest 校验失败"
fi
jq -e --arg version "$deployer_version" \
  '.deployer_version == $version' \
  "$output_dir/deployer-release/deploy-go-deployer-manifest.json" >/dev/null || die "deployer manifest 校验失败"
printf '%s\n' "$expected_commit" > "$output_dir/build-commit"
if [[ "$cleanup" == 1 ]]; then
  # 构建成功后把缓存收敛到上限（BuildKit 自身的回收是周期性的，这里不等它）。清理失败不影响本次构建结果。
  docker buildx prune --builder "$builder_name" \
    --reserved-space "$cache_reserved" --max-used-space "$cache_max" --force >/dev/null 2>&1 ||
    warn "收敛构建缓存失败，已跳过"
fi
printf '远程构建完成：%s\n' "$expected_commit"
